# Golden fixtures: TreeHelper#render_tree_items (HTML) and RecentFilesHelper.
# Writes tree_helper.json and recent_files_helper.json.
require_relative "notes_fs_common"

H = ApplicationController.helpers
LOCALES = I18n.available_locales.map(&:to_s).freeze

# ---------------- tree_helper ----------------
tcases = []
def tree_html(items, expanded, selected, locale = "en")
  I18n.with_locale(locale) { H.render_tree_items(items, expanded_folders: Set.new(expanded), selected_file: selected).to_s }
end
file = ->(name, path, **kw) { { name: name, path: path, type: "file", file_type: "markdown" }.merge(kw) }
folder = ->(name, path, children) { { name: name, path: path, type: "folder", children: children } }

T = [
  ["empty state", [], [], ""],
  ["file item", [file.("test", "test.md")], [], ""],
  ["selected file", [file.("test", "test.md")], [], "test.md"],
  ["folder no children", [folder.("folder1", "folder1", [])], [], ""],
  ["folder with child collapsed", [folder.("folder1", "folder1", [file.("note", "folder1/note.md")])], [], ""],
  ["folder expanded", [folder.("folder1", "folder1", [file.("note", "folder1/note.md")])], ["folder1"], ""],
  ["config file item", [{ name: ".fed", path: ".fed", type: "file", file_type: "config" }], [], ""],
  ["config file selected", [{ name: ".fed", path: ".fed", type: "file", file_type: "config" }], [], ".fed"],
  ["string keys", [{ "name" => "test", "path" => "test.md", "type" => "file", "file_type" => "markdown" }], [], ""],
  ["string-key folder", [{ "name" => "f", "path" => "f", "type" => "folder", "children" => [{ "name" => "n", "path" => "f/n.md", "type" => "file" }] }], ["f"], "f/n.md"],
  ["file with mtime", [file.("m", "m.md", mtime: 1_700_000_000)], [], ""],
  ["file without file_type defaults to markdown", [{ name: "x", path: "x.md", type: "file" }], [], ""],
  ["folder with nil children", [{ name: "n", path: "n", type: "folder", children: nil }], ["n"], ""],
  ["HTML escaping in names and paths", [file.("<b>&\"'</b>", "<b>&\"'</b>.md"), folder.("a&b", "a&b", [])], [], "<b>&\"'</b>.md"],
  ["unicode names", [folder.("日本語", "日本語", [file.("ノート", "日本語/ノート.md")]), file.("שלום", "שלום.md"), file.("👨‍👩‍👧", "👨‍👩‍👧.md"), file.("é", "é.md")], ["日本語"], "שלום.md"],
  ["nested 3 levels mixed expansion", [folder.("a", "a", [folder.("b", "a/b", [folder.("c", "a/b/c", [file.("deep", "a/b/c/deep.md")])]), file.("x", "a/x.md")])], ["a", "a/b/c"], "a/b/c/deep.md"],
  ["unknown type rendered as file", [{ name: "u", path: "u", type: "weird" }], [], ""],
  ["selected folder path does not mark folder", [folder.("f", "f", [])], [], "f"]
].freeze
T.each do |name, items, expanded, selected|
  tcases << { "name" => name, "fn" => "render_tree_items",
              "input" => { "items" => NotesFS.json(items), "expanded_folders" => expanded, "selected_file" => selected, "locale" => "en" },
              "expected" => tree_html(items, expanded, selected) }
end
(LOCALES - ["en"]).each do |loc|
  tcases << { "name" => "empty state locale #{loc}", "fn" => "render_tree_items",
              "input" => { "items" => [], "expanded_folders" => [], "selected_file" => "", "locale" => loc },
              "expected" => tree_html([], [], "", loc) }
end
# Real tree from list_tree rendered
NotesFS.sandbox({ "Inbox/todo.md" => "t", "Projects/a/spec.md" => "s", "journal.md" => "j", "e/" => nil }) do |root, _|
  items = NotesService.new(base_path: root).list_tree
  tcases << { "name" => "render list_tree output", "fn" => "render_tree_items",
              "input" => { "items" => NotesFS.json(items), "expanded_folders" => ["Projects"], "selected_file" => "journal.md", "locale" => "en" },
              "expected" => tree_html(items, ["Projects"], "journal.md") }
end

Golden.write("tree_helper", Golden.fixture(
  source: "app/helpers/tree_helper.rb#render_tree_items",
  generator: NotesFS.gen_path("helpers.rb"),
  notes: "expected is the exact HTML string produced by Rails content_tag/tag helpers (attribute order and escaping as Rails emits, " \
         "data-action '->' escaped as '-&gt;'). items accept symbol or string keys (input shows JSON form; string-key cases used string keys in Ruby, others symbols). " \
         "Empty top-level list renders the localized sidebar.no_notes_yet div; empty nested children render ''.",
  cases: tcases))

# ---------------- recent_files_helper ----------------
rcases = []
NOW = NotesFS::PINNED_NOW
OFFSETS = [0, 1, 2, 59, 60, 61, 119, 120, 600, 3599, 3600, 3601, 7199, 7200, 86_399, 86_400, 86_401, 172_799, 172_800, 518_400, 604_799, 604_800, 604_801, 2_592_000, -1, -60, -86_400, 0.4, 0.999, 1.5, 59.999].freeze
LOCALES.each do |loc|
  OFFSETS.each do |sec|
    v = NotesFS.travel_to(NOW) { I18n.with_locale(loc) { H.recent_file_opened_ago(Time.current - sec) } }
    rcases << { "name" => "opened_ago #{loc} #{sec}s", "fn" => "recent_file_opened_ago",
                "input" => { "now" => NOW.iso8601(6), "seconds_ago" => sec, "locale" => loc }, "expected" => v }
  end
end
TS = ["2026-10-26T09:30:00-03:00", "2026-10-26T12:30:00Z", "2026-10-26T12:30:00.123456Z", "2026-10-26T12:30:00+05:30", "2026-10-26T12:30:00",
      "2026-10-26", "2026-10-26 12:30:00Z", "2026-10-26T12:30Z", "not a timestamp", "", " 2026-10-26T12:30:00Z", "2026-02-30T00:00:00Z",
      "2026-10-26T24:00:00Z", "2026-10-26T12:30:60Z", "+002026-10-26T12:30:00Z", "2026-10-26t12:30:00z", "2026-10-26T12:30:00.5+00:00", "１２"].freeze
TS.each do |s|
  v = NotesFS.capture do
    t = H.recent_file_opened_at(s)
    t && { "iso8601" => t.iso8601(6), "utc" => t.getutc.iso8601(6), "utc_offset" => t.utc_offset }
  end
  rcases << { "name" => "opened_at #{s.inspect}", "fn" => "recent_file_opened_at", "input" => s, "expected" => v }
end

Golden.write("recent_files_helper", Golden.fixture(
  source: "app/helpers/recent_files_helper.rb",
  generator: NotesFS.gen_path("helpers.rb"),
  notes: "recent_file_opened_ago(opened_at) evaluated with Time.current frozen at #{NOW.iso8601(6)} (travel_to; note travel_to truncates usec so now is " \
         "#{NOW.change(usec: 0).iso8601(6)}) and opened_at = now - seconds_ago. seconds = max(0, (now - opened_at).to_i) (truncation toward zero). " \
         "Localized via I18n home.opened_ago.* for each available locale. recent_file_opened_at = Time.iso8601(value) or nil on ArgumentError; " \
         "expected shows the parsed time (original offset), its UTC form and utc_offset (seconds).",
  cases: rcases))
