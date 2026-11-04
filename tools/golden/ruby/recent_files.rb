# Golden fixtures: RecentFilesService (persisted as `recent_files = <json>` in <NOTES_PATH>/.fed).
require_relative "notes_fs_common"

T0 = NotesFS::PINNED_NOW
cases = []

# ops: {"op":"record","path":..,"at_offset":seconds} | entries | clear | remove | remap | fs_delete | fs_write | fs_chmod |
#      service_rename | service_delete | service_delete_folder | http
def run_op(o, root)
  at = T0 + (o["at_offset"] || 0)
  NotesFS.travel_to(at) do
    Golden.with_env("NOTES_PATH" => root) do
      hist = -> { RecentFilesService.new(config: Config.new(base_path: root)) }
      case o["op"]
      when "record" then NotesFS.capture { hist.().record(o["path"]) }
      when "entries" then hist.().entries
      when "clear" then hist.().clear
      when "remove" then hist.().remove(o["path"])
      when "remap" then hist.().remap(o["old_path"], o["new_path"])
      when "fs_delete" then File.delete(File.join(root, o["path"])) && true
      when "fs_write" then File.binwrite(File.join(root, o["path"]), o["content"]) && true
      when "fs_chmod" then File.chmod(o["mode"], File.join(root, o["path"])) && true
      when "service_rename" then NotesFS.capture { NotesService.new(base_path: root).rename(o["old_path"], o["new_path"]) }
      when "service_delete" then NotesFS.capture { NotesService.new(base_path: root).delete(o["path"]) }
      when "service_delete_folder" then NotesFS.capture { NotesService.new(base_path: root).delete_folder(o["path"]) }
      when "http"
        r = NotesFS.http(o["method"].downcase.to_sym, o["url"], params: o["params"], headers: o["headers"] || {})
        r
      else raise "unknown #{o['op']}"
      end
    end
  end
end

def fed_recent_line(root)
  fed = File.join(root, ".fed")
  return nil unless File.exist?(fed)
  File.read(fed).lines.map(&:chomp).grep(/\Arecent_files\s*=/)
end

def rcase(cases, name, tree, ops, note: nil)
  NotesFS.sandbox(tree) do |root, _|
    results = ops.map { |o| NotesFS.norm(run_op(o, root), root) }
    # restore permissions so cleanup and snapshot work
    Dir.glob("**/*", File::FNM_DOTMATCH, base: root).each { |r| File.chmod(0o755, File.join(root, r)) if File.directory?(File.join(root, r)) && !File.symlink?(File.join(root, r)) }
    entries = NotesFS.travel_to(T0 + 3600) { RecentFilesService.new(config: Config.new(base_path: root)).entries }
    c = { "name" => name, "fn" => "RecentFilesService",
          "input" => { "tree" => tree, "ops" => ops, "now" => T0.iso8601(6) },
          "expected" => { "results" => results, "entries_after" => entries, "fed_recent_files_lines" => fed_recent_line(root),
                          "tree_after" => NotesFS.snap(root) } }
    c["notes"] = note if note
    cases << c
  end
end
def rec(path, off = 0) = { "op" => "record", "path" => path, "at_offset" => off }
def fed_with(json) = "<FED_TEMPLATE>recent_files = #{json}\n"

notes12 = (0...12).to_h { |i| ["#{i}.md", "# Test\n\nContent"] }
rcase(cases, "bounded (10) deduplicated most-recent-first", notes12, (0...12).map { |i| rec("#{i}.md", i) } + [rec("4.md", 100), { "op" => "entries" }])
rcase(cases, "record on fresh notes dir creates .fed from template", { "a.md" => "a" }, [rec("a.md"), { "op" => "entries" }])
rcase(cases, "record same path twice moves to front with new time", { "a.md" => "a", "b.md" => "b" }, [rec("a.md", 0), rec("b.md", 1), rec("a.md", 2)])
rcase(cases, "external deletion filtered without removing unrelated", { "gone.md" => "g", "keep.md" => "k" },
      [rec("gone.md", 0), rec("keep.md", 1), { "op" => "fs_delete", "path" => "gone.md" }, { "op" => "entries" }, rec("gone.md", 2)])
rcase(cases, "file and folder mutations update shared history", { "docs/first.md" => "f", "docs-other/other.md" => "o" },
      [rec("docs/first.md", 0), rec("docs-other/other.md", 1), { "op" => "service_rename", "old_path" => "docs", "new_path" => "renamed" }, { "op" => "entries" },
       { "op" => "service_rename", "old_path" => "renamed/first.md", "new_path" => "renamed/second.md" }, { "op" => "entries" },
       { "op" => "service_delete", "path" => "renamed/second.md" }, { "op" => "entries" }])
INVALID = ["../README.md", "/tmp/secret.md", ".secret.md", "docs/../safe.md", "outside.md", "image.png", nil, "", "safe", "a//b.md", "a\\b.md",
           "x\0.md", "dir.md", "sub/.hidden/x.md", "./safe.md", "safe.md/", "SAFE.md", "#{'a' * 4100}.md", 42, ["safe.md"]].freeze
rcase(cases, "rejects traversal, hidden, outside symlinks, non-markdown and malformed paths",
      { "safe.md" => "s", "outside.md" => { "symlink" => "<OUTSIDE>/x.md" }, "dir.md/" => nil, "sub/.hidden/x.md" => "h", "image.png" => "p", "a/b.md" => "ab" },
      INVALID.map { |p| rec(p) } + [rec("safe.md", 1), rec("a/b.md", 2)])
rcase(cases, "unicode, spaces and internal symlink paths are accepted",
      { "日本語/ノート.md" => "j", "with space.md" => "s", "שלום.md" => "h", "👨‍👩‍👧.md" => "e", "real.md" => "r", "alias.md" => { "symlink" => "real.md" } },
      [rec("日本語/ノート.md", 0), rec("with space.md", 1), rec("שלום.md", 2), rec("👨‍👩‍👧.md", 3), rec("alias.md", 4), { "op" => "entries" }])
rcase(cases, "permission errors (EACCES on stat) do not erase an entry", { "locked/a.md" => "a", "b.md" => "b" },
      [rec("locked/a.md", 0), rec("b.md", 1), { "op" => "fs_chmod", "path" => "locked", "mode" => 0 }, { "op" => "entries" }, rec("b.md", 2)],
      note: "locked/ chmod 000 makes stat raise EACCES (non-root user); present? treats EACCES as present.")
rcase(cases, "malformed history JSON does not discard other configuration",
      { ".fed" => "theme = nord\nrecent_files = [broken\n" }, [{ "op" => "entries" }, { "op" => "clear" }, { "op" => "entries" }])
rcase(cases, "non-array history value", { ".fed" => fed_with('{"path":"a.md"}'), "a.md" => "a" }, [{ "op" => "entries" }, rec("a.md")])
crafted = [
  { "path" => "a.md", "opened_at" => "2026-01-01T00:00:09.000000Z", "extra" => "dropped" },
  "not a hash", 42, nil,
  { "path" => "b.md" },
  { "path" => "c.md", "opened_at" => 12_345 },
  { "path" => "a.md", "opened_at" => "2026-01-01T00:00:08.000000Z" },
  { "path" => "missing.md", "opened_at" => "2026-01-01T00:00:07.000000Z" },
  { "path" => "../x.md", "opened_at" => "2026-01-01T00:00:06.000000Z" },
  { "path" => "d.md", "opened_at" => "not a timestamp" },
  { "path" => "e.md", "opened_at" => "" },
  { "path" => "f.md", "opened_at" => "2026-10-26T09:30:00-03:00" }
]
rcase(cases, "normalization of crafted history (types, dupes, missing files, extra keys)",
      { ".fed" => fed_with(JSON.generate(crafted)), "a.md" => "a", "b.md" => "b", "c.md" => "c", "d.md" => "d", "e.md" => "e", "f.md" => "f", "g.md" => "g" },
      [{ "op" => "entries" }, rec("g.md")])
many = (0...120).map { |i| { "path" => "n#{i}.md", "opened_at" => "2026-01-01T00:00:00.000000Z" } }
rcase(cases, "history longer than 100 items is truncated before filtering",
      (0...120).to_h { |i| ["n#{i}.md", "x"] }.reject { |k, _| k.match?(/\An[0-9]\.md\z/) }.merge(".fed" => fed_with(JSON.generate(many))),
      [{ "op" => "entries" }, rec("n110.md")],
      note: "n0..n9 do not exist; entries scan only the first 100 stored items, so n10..n19 are the 10 returned.")
rcase(cases, "clear", { "a.md" => "a" }, [rec("a.md"), { "op" => "clear" }, { "op" => "entries" }])
rcase(cases, "remove file and folder scope (prefix-safe)", { "docs/a.md" => "a", "docs/sub/b.md" => "b", "docs-x/c.md" => "c", "docs.md" => "d" },
      [rec("docs/a.md", 0), rec("docs/sub/b.md", 1), rec("docs-x/c.md", 2), rec("docs.md", 3), { "op" => "remove", "path" => "docs" }, { "op" => "entries" },
       { "op" => "remove", "path" => "docs.md" }])
rcase(cases, "remap before fs move keeps entry only if new path exists", { "a.md" => "a", "b/c.md" => "c" },
      [rec("a.md", 0), rec("b/c.md", 1), { "op" => "remap", "old_path" => "a.md", "new_path" => "z.md" }, { "op" => "entries" }])
rcase(cases, "remap after external move", { "a.md" => "a", "b/c.md" => "c" },
      [rec("a.md", 0), rec("b/c.md", 1), { "op" => "fs_write", "path" => "z.md", "content" => "a" }, { "op" => "fs_delete", "path" => "a.md" },
       { "op" => "remap", "old_path" => "a.md", "new_path" => "z.md" }, { "op" => "entries" }])
rcase(cases, "delete_folder (empty) removes scoped entries", { "d/x.md" => "x" },
      [rec("d/x.md"), { "op" => "fs_delete", "path" => "d/x.md" }, { "op" => "service_delete_folder", "path" => "d" }])
rcase(cases, "service ops without .fed never create it", { "a.md" => "a" },
      [{ "op" => "service_rename", "old_path" => "a.md", "new_path" => "b.md" }, { "op" => "service_delete", "path" => "b.md" }])
rcase(cases, "existing settings and comments preserved when history is written",
      { ".fed" => "# my comment\ntheme = nord\n\nrecent_files = []\nlocale = pt-BR\n", "a.md" => "a" }, [rec("a.md")])
rcase(cases, "commented-out recent_files key gets replaced in place",
      { ".fed" => "# recent_files = []\ntheme = nord\n", "a.md" => "a" }, [rec("a.md")])
rcase(cases, "uppercase key in .fed", { ".fed" => "RECENT_FILES = [{\"path\":\"a.md\",\"opened_at\":\"2026-01-01T00:00:00Z\"}]\n", "a.md" => "a", "b.md" => "b" },
      [{ "op" => "entries" }, rec("b.md")])
rcase(cases, "HTTP POST/DELETE /recent_files", { "a.md" => "a", "b.md" => "b" },
      [{ "op" => "http", "method" => "POST", "url" => "/recent_files", "params" => { "path" => "a.md" } },
       { "op" => "http", "method" => "POST", "url" => "/recent_files", "params" => { "path" => "missing.md" }, "at_offset" => 1 },
       { "op" => "http", "method" => "POST", "url" => "/recent_files", "params" => { "path" => ".fed" }, "at_offset" => 2 },
       { "op" => "http", "method" => "POST", "url" => "/recent_files", "at_offset" => 3 },
       { "op" => "http", "method" => "POST", "url" => "/recent_files", "params" => { "path" => "b.md" }, "at_offset" => 4 },
       { "op" => "entries" },
       { "op" => "http", "method" => "DELETE", "url" => "/recent_files", "at_offset" => 5 }, { "op" => "entries" }])
seven = (1..7).to_h { |i| ["dir/n#{i} <&\">.md", "x"] }
rcase(cases, "HTTP GET /recent_files HTML panel (ages, escaping, >5 show more)", seven,
      seven.keys.each_with_index.map { |p, i| rec(p, -[0, 1, 59, 3600, 86_400, 604_800, 604_801][i]) } +
      [{ "op" => "http", "method" => "GET", "url" => "/recent_files", "headers" => { "Accept" => "application/json" } }],
      note: "Records are made at negative offsets from now (#{T0.iso8601(6)}) so the panel shows 'Just now', seconds, minutes, ... 'Over a week ago'. " \
            "Only the 7 most recent within LIMIT 10 are present; first 5 visible.")
rcase(cases, "HTTP GET /recent_files empty history renders empty body", {}, [{ "op" => "http", "method" => "GET", "url" => "/recent_files" }])

Golden.write("recent_files", Golden.fixture(
  source: "app/services/recent_files_service.rb (+ Config#modify persistence, NotesService side effects, RecentFilesController)",
  generator: NotesFS.gen_path("recent_files.rb"),
  notes: "Storage: a single line `recent_files = <compact JSON array of {path, opened_at}>` in <NOTES_PATH>/.fed (written via Config surgical save: replaces " \
         "an existing or commented key line in place, else appends at end; whole file rewritten atomically with trailing \\n). If .fed is missing it is first created " \
         "from the template (shown as <FED_TEMPLATE>; text in fed_template). opened_at = Time.current.utc.iso8601(6). Time pinned with travel_to: each op runs at " \
         "#{T0.iso8601(6)} + op.at_offset seconds; entries_after is computed at now+3600s. Results: record/clear/remove/remap return Config#set result (true/false) " \
         "or {error} (NotFoundError 'Note not found'). Lock file .frankmd-config.lock created in root.",
  fed_template: NotesFS.fed_template, cases: cases))
