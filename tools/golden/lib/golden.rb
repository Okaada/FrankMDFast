# Shared helpers for Ruby golden generators. Load with:
#   require_relative "../lib/golden"
require "json"
require "fileutils"
require "tmpdir"

module Golden
  FRANKMD = File.expand_path(ENV.fetch("FRANKMD", "~/Projects/FrankMD"))
  OUT_DIR = File.expand_path("../../../tests/fixtures/golden", __dir__)
  TMP_BASE = ENV["GOLDEN_TMP"]

  module_function

  def sha
    @sha ||= `git -C #{FRANKMD} rev-parse HEAD`.strip
  end

  # Build a fixture hash. `source` like "app/services/hugo_service.rb#slugify"
  def fixture(source:, generator:, cases:, notes: nil, **extra)
    { "source" => "#{source} @ #{sha}", "frankmd_sha" => sha,
      "generator" => generator, "notes" => notes }.merge(extra.transform_keys(&:to_s)).merge("cases" => cases)
  end

  def write(area, data)
    FileUtils.mkdir_p(OUT_DIR)
    path = File.join(OUT_DIR, "#{area}.json")
    File.write(path, JSON.pretty_generate(data) + "\n")
    warn "wrote #{path} (#{data["cases"]&.size} cases)"
    path
  end

  # Run block, returning its value or {"error"=>class, "message"=>msg}
  def capture
    yield
  rescue StandardError => e
    { "error" => e.class.name, "message" => e.message }
  end

  def mktmp(prefix = "golden")
    if TMP_BASE
      FileUtils.mkdir_p(TMP_BASE)
      Dir.mktmpdir(prefix, TMP_BASE)
    else
      Dir.mktmpdir(prefix)
    end
  end

  # Materialize a tree spec under root.
  #   "a/b.md" => "content"   file
  #   "dir/"   => nil         empty dir
  #   "l"      => {"symlink" => "target"}
  #   "f"      => {"content" => "...", "mtime" => 1700000000}
  def build_tree(root, spec)
    spec.each do |rel, val|
      path = File.join(root, rel)
      if rel.end_with?("/")
        FileUtils.mkdir_p(path)
      elsif val.is_a?(Hash) && val["symlink"]
        FileUtils.mkdir_p(File.dirname(path))
        File.symlink(val["symlink"], path)
      else
        FileUtils.mkdir_p(File.dirname(path))
        content = val.is_a?(Hash) ? val["content"].to_s : val.to_s
        File.binwrite(path, content)
        File.utime(val["mtime"], val["mtime"], path) if val.is_a?(Hash) && val["mtime"]
      end
    end
    root
  end

  # Snapshot of a directory: sorted map rel_path => content | nil (dir, key ends with "/")
  # | {"symlink"=>target}. Does not follow symlinks.
  def snapshot(root)
    out = {}
    Dir.glob("**/*", File::FNM_DOTMATCH, base: root).sort.each do |rel|
      next if File.basename(rel) == "." || File.basename(rel) == ".."
      path = File.join(root, rel)
      if File.symlink?(path)
        out[rel] = { "symlink" => File.readlink(path) }
      elsif File.directory?(path)
        out["#{rel}/"] = nil
      else
        data = File.binread(path)
        out[rel] = data.dup.force_encoding("UTF-8").valid_encoding? ? data.force_encoding("UTF-8") : { "base64" => [data].pack("m0") }
      end
    end
    out
  end

  # Recursively replace root path strings with <ROOT>
  def normalize(obj, root)
    real = File.realpath(root) rescue root
    case obj
    when String then obj.gsub(real, "<ROOT>").gsub(root, "<ROOT>")
    when Array then obj.map { |v| normalize(v, root) }
    when Hash then obj.to_h { |k, v| [normalize(k.to_s, root), normalize(v, root)] }
    when Pathname then normalize(obj.to_s, root)
    when Symbol then obj.to_s
    when Time, Date, DateTime then obj.iso8601
    else obj
    end
  end

  # Temporarily set ENV vars (nil deletes), restoring afterwards.
  def with_env(vars)
    saved = vars.keys.to_h { |k| [k, ENV[k]] }
    vars.each { |k, v| v.nil? ? ENV.delete(k) : ENV[k] = v }
    yield
  ensure
    saved.each { |k, v| v.nil? ? ENV.delete(k) : ENV[k] = v }
  end
end
