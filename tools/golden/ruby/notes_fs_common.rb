# Shared helpers for the notes-filesystem golden generators
# (path_safety, notes_tree, notes_crud, notes_rename, search, recent_files,
# backlinks, tree_helper, recent_files_helper).
#
# Run every generator with RAILS_ENV=test TZ=UTC FRANKMD_LOCALE=en, e.g.
#   cd ~/Projects/FrankMD && RAILS_ENV=test TZ=UTC FRANKMD_LOCALE=en \
#     GOLDEN_TMP=$SCRATCH/notes/fs mise exec ruby@3.4.8 -- \
#     bundle exec rails runner /mnt/data/Projects/FrankMDFast/tools/golden/ruby/<area>.rb
require_relative "../lib/golden"
require "active_support/testing/time_helpers"

ENV["TZ"] = "UTC"
I18n.locale = :en

module NotesFS
  extend ActiveSupport::Testing::TimeHelpers
  module_function

  # Default pinned mtime for tree-spec files without an explicit mtime. Files
  # get BASE_MTIME - index*100 in spec order, so earlier spec entries are newer.
  BASE_MTIME = 1_700_000_000
  PINNED_NOW = Time.utc(2026, 1, 30, 12, 34, 56.789012r)

  def gen_path(name) = "tools/golden/ruby/#{name}"

  # Fresh sandbox: <tmp>/notes (root) and <tmp>/outside (escape target).
  # Symlink targets / contents may use <ROOT> and <OUTSIDE> placeholders.
  def sandbox(tree = {}, outside: {}, pin_mtimes: true)
    tmp = File.realpath(Golden.mktmp("nfs"))
    root = File.join(tmp, "notes")
    out = File.join(tmp, "outside")
    FileUtils.mkdir_p(root)
    FileUtils.mkdir_p(out)
    build(out, outside, root, out, pin_mtimes)
    build(root, tree, root, out, pin_mtimes)
    yield root, out
  ensure
    FileUtils.rm_rf(tmp) if tmp
  end

  def subst(str, root, out) = str.to_s.gsub("<ROOT>", root).gsub("<OUTSIDE>", out)

  def build(base, spec, root, out, pin_mtimes)
    i = 0
    spec.each do |rel, val|
      path = File.join(base, rel)
      if rel.end_with?("/")
        FileUtils.mkdir_p(path)
      elsif val.is_a?(Hash) && val.key?("symlink")
        FileUtils.mkdir_p(File.dirname(path))
        File.symlink(subst(val["symlink"], root, out), path)
      else
        FileUtils.mkdir_p(File.dirname(path))
        if val.is_a?(Hash) && val.key?("base64")
          File.binwrite(path, val["base64"].unpack1("m0"))
        else
          content = val.is_a?(Hash) ? val["content"].to_s : val.to_s
          content = fed_template + content.delete_prefix("<FED_TEMPLATE>") if content.start_with?("<FED_TEMPLATE>")
          File.binwrite(path, content)
        end
        File.chmod(val["mode"], path) if val.is_a?(Hash) && val["mode"]
        mtime = val.is_a?(Hash) && val["mtime"] ? val["mtime"] : (pin_mtimes ? BASE_MTIME - i * 100 : nil)
        File.utime(mtime, mtime, path) if mtime
        i += 1
      end
    end
  end

  def fed_template
    @fed_template ||= begin
      tmp = Golden.mktmp("fedtpl")
      Config.new(base_path: tmp)
      File.read(File.join(tmp, ".fed"))
    ensure
      FileUtils.rm_rf(tmp)
    end
  end

  # Snapshot root; .fed content that begins with the generated template is
  # shortened to "<FED_TEMPLATE>" + remainder.
  def snap(root)
    s = Golden.snapshot(root)
    s.each do |k, v|
      if k == ".fed" && v.is_a?(String) && v.start_with?(fed_template)
        s[k] = "<FED_TEMPLATE>" + v.delete_prefix(fed_template)
      end
    end
    norm(s, root)
  end

  def norm(obj, root, out = nil)
    out ||= File.join(File.dirname(root), "outside")
    r = Golden.normalize(obj, root)
    tmp = File.dirname(root)
    deep_str(r) { |s| s.gsub(out, "<OUTSIDE>").gsub(tmp, "<TMP>").gsub(/\.[0-9a-f]{12}\.tmp\b/, ".<HEX>.tmp") }
  end

  def deep_str(obj, &blk)
    case obj
    when String then blk.call(obj)
    when Array then obj.map { |v| deep_str(v, &blk) }
    when Hash then obj.to_h { |k, v| [blk.call(k.to_s), deep_str(v, &blk)] }
    else obj
    end
  end

  # JSON round trip (exactly what a controller `render json:` emits).
  def json(obj) = JSON.parse(obj.to_json)

  # Replace unpinned (wall-clock) integer mtimes in tree output with "<TIME>".
  def mask_mtimes(obj)
    case obj
    when Array then obj.map { |v| mask_mtimes(v) }
    when Hash
      obj.to_h do |k, v|
        if k.to_s == "mtime" && v.is_a?(Integer) && v > BASE_MTIME + 10_000_000
          [k, "<TIME>"]
        else
          [k, mask_mtimes(v)]
        end
      end
    else obj
    end
  end

  def capture(&blk)
    Golden.capture(&blk)
  rescue SystemCallError => e
    { "error" => e.class.name, "message" => e.message }
  end

  def model_result(ok, model)
    { "ok" => ok, "errors" => model.errors.full_messages, "path" => model.path }
  end

  # Integration session against the real Rails app (test env: no CSRF/host checks).
  def http(method, url, params: nil, headers: {})
    s = ActionDispatch::Integration::Session.new(Rails.application)
    s.host = "localhost"
    begin
      s.public_send(method, url, params: params, headers: { "Accept" => "application/json" }.merge(headers))
    rescue StandardError => e
      # Unrescued controller exception (production would answer 500).
      return { "status" => 500, "unhandled_exception" => { "error" => e.class.name, "message" => e.message } }
    end
    body = s.response.body
    parsed = begin
      JSON.parse(body)
    rescue JSON::ParserError
      body
    end
    { "status" => s.response.status, "body" => parsed }
  end
end

# Sequential operation executor shared by notes_crud / notes_rename.
module NotesOps
  module_function

  def sha(s) = Digest::SHA256.hexdigest(s)

  # op: {"op"=>name, ...args}. Returns a JSON-able result.
  def run(op, root)
    svc = NotesService.new(base_path: root)
    a = op
    rev = a.key?("expected_revision_of") ? sha(a["expected_revision_of"]) : a["expected_revision"]
    Golden.with_env("NOTES_PATH" => root) do
      case a["op"]
      when "read" then NotesFS.capture { svc.read(a["path"]) }
      when "write" then NotesFS.capture { svc.write(a["path"], a["content"]) }
      when "create" then NotesFS.capture { svc.create(a["path"], a["content"]) }
      when "update" then NotesFS.capture { svc.update(a["path"], a["content"]) }
      when "delete" then NotesFS.capture { svc.delete(a["path"], expected_revision: rev) }
      when "rename" then NotesFS.capture { svc.rename(a["old_path"], a["new_path"]) }
      when "create_folder" then NotesFS.capture { svc.create_folder(a["path"]) }
      when "delete_folder" then NotesFS.capture { svc.delete_folder(a["path"]) }
      when "exists?", "file?", "directory?" then NotesFS.capture { svc.public_send(a["op"], a["path"]) }
      when "list_tree" then NotesFS.mask_mtimes(NotesFS.json(svc.list_tree))
      when "stat_mode" then format("%o", File.lstat(File.join(root, a["path"])).mode & 0o7777)
      when "chmod" then File.chmod(a["mode"], File.join(root, a["path"])) && true
      when "Note.find"
        NotesFS.capture { n = Note.find(a["path"]); { "path" => n.path, "name" => n.name, "content" => n.content, "revision" => n.revision, "as_json" => NotesFS.json(n.as_json) } }
      when "Note#save"
        n = Note.new(path: a["path"], content: a["content"])
        kw = {}
        kw[:existing_only] = true if a["mode"] == "existing_only"
        kw[:create_only] = true if a["mode"] == "create_only"
        ok = NotesFS.capture { n.save(**kw) }
        NotesFS.model_result(ok, n)
      when "Note#destroy"
        n = Note.new(path: a["path"])
        ok = NotesFS.capture { n.destroy(expected_revision: rev) }
        NotesFS.model_result(ok, n).merge("revision_conflict" => n.revision_conflict?)
      when "Note#rename"
        n = Note.new(path: a["path"])
        ok = NotesFS.capture { n.rename(a["new_path"]) }
        NotesFS.model_result(ok, n)
      when "Note#exists?"
        n = Note.new(path: a["path"])
        NotesFS.capture { { "exists" => n.exists?, "persisted" => n.persisted? } }
      when "Folder#create"
        fo = Folder.new(path: a["path"])
        ok = NotesFS.capture { fo.create }
        NotesFS.model_result(ok, fo)
      when "Folder#destroy"
        fo = Folder.new(path: a["path"])
        ok = NotesFS.capture { fo.destroy }
        NotesFS.model_result(ok, fo)
      when "Folder#rename"
        fo = Folder.new(path: a["path"])
        ok = NotesFS.capture { fo.rename(a["new_path"]) }
        NotesFS.model_result(ok, fo)
      when "http"
        r = NotesFS.http(a["method"].downcase.to_sym, a["url"], params: a["params"], headers: a["headers"] || {})
        r["body"] = "<HTML>" if r["body"].is_a?(String) && r["body"].include?("<html")
        r
      else raise "unknown op #{a['op']}"
      end
    end
  end

  def scenario(name, tree, ops, outside: {}, pin_time: true, fn: nil)
    res = nil
    NotesFS.sandbox(tree, outside: outside) do |root, _|
      body = lambda do
        results = ops.map { |op| NotesFS.norm(run(op, root), root) }
        { "results" => results, "tree_after" => NotesFS.snap(root) }
      end
      res = pin_time ? NotesFS.travel_to(NotesFS::PINNED_NOW) { body.call } : body.call
    end
    { "name" => name, "fn" => fn || ops.map { |o| o["op"] }.uniq.join(","),
      "input" => { "tree" => tree, "outside" => outside, "ops" => ops }, "expected" => res }
  end
end
