# Golden fixtures: rename/move notes and folders (service, models, HTTP, side effects).
require_relative "notes_fs_common"

def sc(...) = NotesOps.scenario(...)
def op(name, **kw) = { "op" => name }.merge(kw.transform_keys(&:to_s))
def rn(o, n) = op("rename", old_path: o, new_path: n)
def http(method, url, params = nil) = { "op" => "http", "method" => method, "url" => url, "params" => params }
def recent_fed(*paths)
  items = paths.each_with_index.map { |p, i| { "path" => p, "opened_at" => "2026-01-01T00:00:%02d.000000Z" % (59 - i) } }
  "<FED_TEMPLATE>recent_files = #{JSON.generate(items)}\n"
end

cases = []
# --- NotesService#rename ---
cases << sc("rename file", { "old.md" => "Content" }, [rn("old.md", "new.md")])
cases << sc("rename into existing folder", { "root.md" => "Content", "subfolder/" => nil }, [rn("root.md", "subfolder/moved.md")])
cases << sc("rename into missing nested folders (created)", { "a.md" => "a" }, [rn("a.md", "x/y/z/a.md")])
cases << sc("move out of folder leaves empty folder", { "f/a.md" => "a" }, [rn("f/a.md", "a.md")])
cases << sc("rename folder with contents", { "old_folder/note.md" => "Content", "old_folder/sub/deep.md" => "d" }, [rn("old_folder", "new_folder")])
cases << sc("move folder into another folder", { "a/n.md" => "n", "b/" => nil }, [rn("a", "b/a")])
cases << sc("rename missing source", {}, [rn("nonexistent.md", "new.md")])
cases << sc("rename onto existing file is rejected", { "old.md" => "original", "new.md" => "destination" }, [rn("old.md", "new.md")])
cases << sc("rename file onto existing directory is rejected", { "a.md" => "a", "dir/" => nil }, [rn("a.md", "dir")])
cases << sc("rename folder onto existing empty folder is rejected", { "a/x.md" => "x", "b/" => nil }, [rn("a", "b")])
cases << sc("rename onto broken symlink is rejected", { "a.md" => "a", "b.md" => { "symlink" => "missing.md" } }, [rn("a.md", "b.md")])
cases << sc("rename to same path is rejected", { "a.md" => "a" }, [rn("a.md", "a.md")])
cases << sc("case-only rename of file (case-sensitive fs)", { "Note.md" => "N" }, [rn("Note.md", "note.md")])
cases << sc("case-only rename of folder", { "Folder/n.md" => "n" }, [rn("Folder", "folder")])
cases << sc("case-only rename when both exist", { "Note.md" => "N", "note.md" => "n" }, [rn("Note.md", "note.md")])
cases << sc("move folder into itself", { "a/n.md" => "n" }, [rn("a", "a/b")])
cases << sc("move folder into its existing descendant", { "a/b/n.md" => "n" }, [rn("a", "a/b/c")])
cases << sc("move folder into a sibling with shared prefix", { "a/n.md" => "n", "ab/" => nil }, [rn("a", "ab/a")])
cases << sc("rename file under itself (parent would be a file)", { "a.md" => "a" }, [rn("a.md", "a.md/x.md")])
cases << sc("service rename does not touch extensions", { "a.md" => "a", "b.md" => "b" }, [rn("a.md", "a.txt"), rn("b.md", "b")])
cases << sc("service rename requires exact source name (no .md added)", { "a.md" => "a" }, [rn("a", "b.md")])
cases << sc("rename unicode names", { "café.md" => "c", "日本語/ノート.md" => "j" },
            [rn("café.md", "שלום/👨‍👩‍👧.md"), rn("日本語", "étude")])
cases << sc("rename with spaces and trailing slashes", { "my folder/n.md" => "n", "x.md" => "x" }, [rn("my folder/", "other folder/"), rn("x.md", "  spaced.md")])
cases << sc("rename hidden file and to hidden name", { ".hidden.md" => "h", "v.md" => "v" }, [rn(".hidden.md", "visible.md"), rn("v.md", ".v.md")])
cases << sc("rename onto .fed via service (no guard at service level)", { "a.md" => "theme = dark\n" }, [rn("a.md", ".fed")])
cases << sc("rename reserved lock path", { "a.md" => "a" }, [rn("a.md", ".frankmd-config.lock"), rn(".frankmd-config.lock", "x.md")])
cases << sc("rename traversal", { "a.md" => "a" }, [rn("../outside/secret.md", "x.md"), rn("a.md", "../a.md"), rn("a.md", "/tmp/a.md")], outside: { "secret.md" => "s" })
cases << sc("rename into symlinked dir escaping root", { "a.md" => "a", "escape" => { "symlink" => "<OUTSIDE>" } }, [rn("a.md", "escape/a.md")])
cases << sc("rename internal symlink moves the link", { "real.md" => "r", "alias.md" => { "symlink" => "real.md" } }, [rn("alias.md", "sub/alias.md")])
cases << sc("rename root folder", { "a.md" => "a" }, [rn("", "x"), rn(".", "y")])
cases << sc("rename preserves content bytes (CRLF/BOM)", { "a.md" => "﻿a\r\nb\r" }, [rn("a.md", "b.md")])
cases << sc("rename does not rewrite links in other notes", { "target.md" => "t", "src.md" => "See [[target]] and [t](target.md)" }, [rn("target.md", "renamed.md")])

# --- recent_files remap side effects (only when .fed exists) ---
cases << sc("rename folder remaps scoped recent entries only",
            { ".fed" => recent_fed("docs/first.md", "docs-other/other.md"), "docs/first.md" => "f", "docs-other/other.md" => "o" },
            [rn("docs", "renamed")])
cases << sc("rename file remaps entry then delete removes it",
            { ".fed" => recent_fed("renamed/first.md", "docs-other/other.md"), "renamed/first.md" => "f", "docs-other/other.md" => "o" },
            [rn("renamed/first.md", "renamed/second.md"), op("delete", path: "renamed/second.md")])
cases << sc("rename remap drops entries whose files are gone and dedups",
            { ".fed" => recent_fed("a.md", "gone.md", "b.md"), "a.md" => "a", "b.md" => "b" }, [rn("a.md", "b2.md")])
cases << sc("rename to non-md keeps entry? (remapped then filtered by valid_path)",
            { ".fed" => recent_fed("a.md", "keep.md"), "a.md" => "a", "keep.md" => "k" }, [rn("a.md", "a.txt")])
cases << sc("rename without .fed does not create one", { "a.md" => "a" }, [rn("a.md", "b.md")])
cases << sc("rename with .fed lacking recent_files writes recent_files line",
            { ".fed" => "<FED_TEMPLATE>", "a.md" => "a" }, [rn("a.md", "b.md")])
cases << sc("rename prefix-sharing folder 'doc' must not remap 'docs/'",
            { ".fed" => recent_fed("doc/a.md", "docs/b.md", "doc.md"), "doc/a.md" => "a", "docs/b.md" => "b", "doc.md" => "d" }, [rn("doc", "folder")])

# --- Note#rename ---
cases << sc("Note#rename normalizes new path (.md added)", { "old.md" => "x" }, [op("Note#rename", path: "old.md", new_path: "new")])
cases << sc("Note#rename into subdirectory", { "root.md" => "x", "subfolder/" => nil }, [op("Note#rename", path: "root.md", new_path: "subfolder/moved.md")])
cases << sc("Note#rename source without extension", { "old.md" => "x" }, [op("Note#rename", path: "old", new_path: "new.md")])
cases << sc("Note#rename errors", { "a.md" => "a", "b.md" => "b" },
            [op("Note#rename", path: "a.md", new_path: "b"), op("Note#rename", path: "missing.md", new_path: "c"),
             op("Note#rename", path: "a.md", new_path: "../x"), op("Note#rename", path: "a.md", new_path: "x.txt")])
cases << sc("Note#rename to .fed keeps name", { "a.md" => "a" }, [op("Note#rename", path: "a.md", new_path: ".fed")])

# --- Folder#rename (incl. Hugo index.md slug update) ---
HUGO = <<~FM
  ---
  title: "My Blog Post"
  slug: "old-slug"
  date: 2026-02-01T10:00:00-03:00
  draft: true
  tags:
  -
  ---

  Content here
FM
cases << sc("Folder#rename simple", { "old_folder/" => nil }, [op("Folder#rename", path: "old_folder", new_path: "new_folder")])
cases << sc("Folder#rename with contents", { "folder/note.md" => "Content" }, [op("Folder#rename", path: "folder", new_path: "renamed")])
cases << sc("Folder#rename updates Hugo slug", { "old-slug/index.md" => HUGO }, [op("Folder#rename", path: "old-slug", new_path: "new-slug")])
cases << sc("Folder#rename accented folder name slug", { "old-post/index.md" => "---\ntitle: \"Test\"\nslug: \"old-post\"\n---\n\nBody\n" },
            [op("Folder#rename", path: "old-post", new_path: "café-açaí")])
cases << sc("Folder#rename non-Hugo index.md untouched", { "regular-folder/index.md" => "# Just a regular note\n\nNo frontmatter here" },
            [op("Folder#rename", path: "regular-folder", new_path: "renamed-folder")])
cases << sc("Folder#rename frontmatter without slug untouched", { "no-slug-folder/index.md" => "---\ntitle: \"No Slug\"\ndate: 2026-02-01\n---\n\nContent\n" },
            [op("Folder#rename", path: "no-slug-folder", new_path: "renamed-no-slug")])
cases << sc("Folder#rename single-quoted and unquoted slugs", { "a/index.md" => "---\nslug: 'old'\n---\nx", "b/index.md" => "---\nslug: old-b\n---\ny" },
            [op("Folder#rename", path: "a", new_path: "Nova Pasta"), op("Folder#rename", path: "b", new_path: "日本語 Post 2026")])
cases << sc("Folder#rename CRLF frontmatter", { "c/index.md" => "---\r\nslug: \"old\"\r\ntitle: x\r\n---\r\nbody\r\n" }, [op("Folder#rename", path: "c", new_path: "crlf-new")])
cases << sc("Folder#rename slug already equal", { "x/index.md" => "---\nslug: \"same-name\"\n---\n" }, [op("Folder#rename", path: "x", new_path: "Same Name")])
cases << sc("Folder#rename nested move keeps slug from basename", { "p/old/index.md" => "---\nslug: \"old\"\n---\n" }, [op("Folder#rename", path: "p/old", new_path: "q/Deep Move")])
cases << sc("Folder#rename slug in body only (after frontmatter) untouched", { "d/index.md" => "---\ntitle: t\n---\nslug: \"body\"\n" }, [op("Folder#rename", path: "d", new_path: "e")])
cases << sc("Folder#rename errors", { "a/" => nil, "b/" => nil },
            [op("Folder#rename", path: "a", new_path: "b"), op("Folder#rename", path: "missing", new_path: "x"), op("Folder#rename", path: "a", new_path: "../x")])

# --- HTTP ---
cases << sc("POST /notes/:path/rename", { "old.md" => "x" }, [http("POST", "/notes/old.md/rename", { new_path: "new.md" })])
cases << sc("POST /notes/:path/rename adds .md and moves to subfolder", { "old.md" => "x", "subfolder/" => nil }, [http("POST", "/notes/old.md/rename", { new_path: "subfolder/new" })])
cases << sc("POST /notes/:path/rename conflict", { "old.md" => "source", "new.md" => "dest" }, [http("POST", "/notes/old.md/rename", { new_path: "new.md" })])
cases << sc("POST /notes/:path/rename missing", {}, [http("POST", "/notes/missing.md/rename", { new_path: "x.md" })])
cases << sc("POST /notes/:path/rename blank new_path", { "a.md" => "a" }, [http("POST", "/notes/a.md/rename", { new_path: "" }), http("POST", "/notes/a.md/rename")])
cases << sc("POST /folders/:path/rename", { "old_folder/n.md" => "n" }, [http("POST", "/folders/old_folder/rename", { new_path: "new_folder" })])
cases << sc("POST /folders/:path/rename to different parent", { "a/n.md" => "n", "b/" => nil }, [http("POST", "/folders/a/rename", { new_path: "b/a" })])
cases << sc("POST /folders/:path/rename missing / conflict", { "a/" => nil, "b/" => nil },
            [http("POST", "/folders/missing/rename", { new_path: "x" }), http("POST", "/folders/a/rename", { new_path: "b" })])

# --- FoldersController expanded-folder remap after rename (turbo stream) ---
[
  ["old", "new", "old,old/sub,older,other/old"],
  ["a/b", "c", "a,a/b,a/b/c,a/bb"],
  ["x", "y", ""],
  ["日本", "nihon", "日本,日本/語"]
].each do |from, to, expanded|
  NotesFS.sandbox({}) do |root, _|
    Golden.with_env("NOTES_PATH" => root) do
      c = FoldersController.new
      c.params = ActionController::Parameters.new(expanded: expanded, selected: "")
      c.send(:load_tree_for_turbo_stream, selected: to, remap_from: from, remap_to: to)
      cases << { "name" => "expanded folder remap #{from}->#{to} [#{expanded}]", "fn" => "FoldersController#load_tree_for_turbo_stream",
                 "input" => { "remap_from" => from, "remap_to" => to, "expanded" => expanded },
                 "expected" => { "expanded_folders" => c.instance_variable_get(:@expanded_folders).to_a, "selected_file" => c.instance_variable_get(:@selected_file) } }
    end
  end
end

Golden.write("notes_rename", Golden.fixture(
  source: "app/services/notes_service.rb#rename, app/models/note.rb#rename, app/models/folder.rb#rename/update_hugo_index_slug, app/services/recent_files_service.rb#remap, controllers",
  generator: NotesFS.gen_path("notes_rename.rb"),
  notes: "Same scenario format as notes_crud.json (ops run sequentially; tree_after snapshot; <ROOT>/<OUTSIDE>/<TMP>/<FED_TEMPLATE>/<HEX> placeholders; " \
         "time frozen at #{NotesFS::PINNED_NOW.iso8601(6)} UTC). Rename never rewrites links inside other notes; the only side effects are " \
         "(1) recent_files remap in .fed (only when <ROOT>/.fed exists) and (2) Folder#rename rewriting the Hugo slug in <new>/index.md frontmatter. " \
         "Case-only renames were captured on a case-sensitive Linux filesystem. Expanded-folder remap cases call the private controller method directly; " \
         "expanded_folders is a Ruby Set rendered as an array in insertion order.",
  fed_template: NotesFS.fed_template, cases: cases))
