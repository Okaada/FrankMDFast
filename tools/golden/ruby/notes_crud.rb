# Golden fixtures: create/read/update/delete notes and folders (service, models, HTTP).
require_relative "notes_fs_common"

def sc(...) = NotesOps.scenario(...)
def op(name, **kw) = { "op" => name }.merge(kw.transform_keys(&:to_s))
def http(method, url, params = nil, headers = nil)
  h = { "op" => "http", "method" => method, "url" => url, "params" => params }
  h["headers"] = headers if headers
  h
end

cases = []
LONG = ("lorem ipsum ção 日本語 " * 400)
CONTENTS = {
  "plain" => "New content", "empty" => "", "trailing newline" => "line\n", "no trailing newline" => "line",
  "CRLF" => "a\r\nb\r\n", "lone CR" => "a\rb\r", "mixed endings" => "a\nb\r\nc\rd", "BOM" => "﻿# Title\n",
  "unicode" => "ção ü 日本語 שלום 👨‍👩‍👧 é  nbsp\ttab", "NUL byte" => "a\0b", "only whitespace" => " \t \n ",
  "long" => LONG, "frontmatter" => "---\ntitle: \"x\"\n---\n\nbody\n"
}.freeze

# --- read ---
cases << sc("read existing", { "test.md" => "Hello World" }, [op("read", path: "test.md")])
cases << sc("read missing", {}, [op("read", path: "nonexistent.md")])
cases << sc("read without extension does not auto-add .md (service)", { "test.md" => "x" }, [op("read", path: "test")])
cases << sc("read a directory", { "dir/" => nil }, [op("read", path: "dir")])
cases << sc("read traversal", { "safe.md" => "Safe content" }, [op("read", path: "../../../etc/passwd"), op("read", path: "../outside/secret.md")], outside: { "secret.md" => "s" })
cases << sc("read through internal symlink", { "real.md" => "r", "alias.md" => { "symlink" => "real.md" } }, [op("read", path: "alias.md")])
cases << sc("read through symlink escaping root", { "out.md" => { "symlink" => "<OUTSIDE>/secret.md" } }, [op("read", path: "out.md")], outside: { "secret.md" => "s" })
cases << sc("read preserves CRLF / CR / BOM / NUL bytes", CONTENTS.slice("CRLF", "lone CR", "BOM", "NUL byte").transform_keys { |k| "#{k.tr(' ', '_')}.md" },
            CONTENTS.slice("CRLF", "lone CR", "BOM", "NUL byte").keys.map { |k| op("read", path: "#{k.tr(' ', '_')}.md") })
cases << sc("read filename with double dots", { "notes..v2.md" => "kept intact" }, [op("read", path: "notes..v2.md")])
cases << sc("read hidden and .fed via service (service does not block)", { ".hidden.md" => "h", ".fed" => "theme = dark\n" }, [op("read", path: ".hidden.md"), op("read", path: ".fed")])

# --- write (upsert) ---
CONTENTS.each do |label, content|
  cases << sc("write new file content=#{label}", {}, [op("write", path: "new.md", content: content), op("read", path: "new.md")])
end
cases << sc("write overwrites existing", { "existing.md" => "Old content" }, [op("write", path: "existing.md", content: "New content")])
cases << sc("write does not add extension", {}, [op("write", path: "plain", content: "x"), op("write", path: "x.txt", content: "t")])
cases << sc("write creates parent directories", {}, [op("write", path: "deep/nested/note.md", content: "Content")])
cases << sc("write Hugo dated path", {}, [op("write", path: "2026/01/30/my-first-post/index.md", content: "---\ntitle: \"My First Post\"\n---\n")])
cases << sc("write preserves file mode across atomic swap", { "note.md" => "original" },
            [op("chmod", path: "note.md", mode: 0o600), op("write", path: "note.md", content: "updated"), op("stat_mode", path: "note.md")])
cases << sc("write new file mode (umask dependent)", {}, [op("write", path: "n.md", content: "x"), op("stat_mode", path: "n.md")])
cases << sc("write filename with double dots", {}, [op("write", path: "notes..v2.md", content: "kept intact"), op("read", path: "notes..v2.md")])
cases << sc("write traversal rejected", {}, [op("write", path: "../escape.md", content: "x"), op("write", path: "/etc/x.md", content: "x"), op("write", path: "a/../../x.md", content: "x")])
cases << sc("write through symlinked dir escaping root rejected", { "escape" => { "symlink" => "<OUTSIDE>" } }, [op("write", path: "escape/new.md", content: "x")])
cases << sc("write over internal symlink replaces the link with a regular file", { "real.md" => "r", "alias.md" => { "symlink" => "real.md" } },
            [op("write", path: "alias.md", content: "via alias")])
cases << sc("write onto a directory", { "dir.md/" => nil }, [op("write", path: "dir.md", content: "x")])
cases << sc("write under a path whose parent is a file", { "file.md" => "f" }, [op("write", path: "file.md/child.md", content: "x")])
cases << sc("write unicode paths", {}, [op("write", path: "日本語/ノート.md", content: "j"), op("write", path: "שלום.md", content: "h"),
                                         op("write", path: "👨‍👩‍👧.md", content: "e"), op("write", path: "é.md", content: "c"), op("write", path: "with space.md", content: "s")])
cases << sc("write reserved config lock path", {}, [op("write", path: ".frankmd-config.lock", content: "x"), op("write", path: "./.frankmd-config.lock", content: "x"),
                                                     op("write", path: ".frankmd-filesystem.lock", content: "x")])
cases << sc("write NUL in path", {}, [op("write", path: "a\0b.md", content: "x")])
cases << sc("write empty path / root", {}, [op("write", path: "", content: "x"), op("write", path: ".", content: "x")])

# --- create (no replace) ---
cases << sc("create new", {}, [op("create", path: "fresh.md", content: "Fresh")])
cases << sc("create refuses to replace existing", { "existing.md" => "Original content" }, [op("create", path: "existing.md", content: "replacement")])
cases << sc("create refuses broken symlink destination", { "dangling.md" => { "symlink" => "nowhere.md" } }, [op("create", path: "dangling.md", content: "x")])
cases << sc("create refuses directory destination", { "d.md/" => nil }, [op("create", path: "d.md", content: "x")])
cases << sc("create case-variant of existing (case-sensitive fs)", { "Note.md" => "N" }, [op("create", path: "note.md", content: "n")])
cases << sc("create with parents", {}, [op("create", path: "a/b/c/new.md", content: "abc")])
cases << sc("create content variants", {}, CONTENTS.keys.map { |k| op("create", path: "#{k.tr(' ', '_')}.md", content: CONTENTS[k]) })
cases << sc("create traversal", {}, [op("create", path: "../x.md", content: "x")])

# --- update (existing only) ---
cases << sc("update existing", { "existing.md" => "Old content" }, [op("update", path: "existing.md", content: "New content")])
cases << sc("update missing does not create file or folder", {}, [op("update", path: "missing-folder/missing.md", content: "Must not be created")])
cases << sc("update a directory", { "dir/" => nil }, [op("update", path: "dir", content: "x")])
cases << sc("update keeps CRLF content verbatim", { "n.md" => "a\nb\n" }, [op("update", path: "n.md", content: "x\r\ny\r\n")])
cases << sc("update preserves mode", { "n.md" => "a" }, [op("chmod", path: "n.md", mode: 0o640), op("update", path: "n.md", content: "b"), op("stat_mode", path: "n.md")])

# --- delete ---
cases << sc("delete removes file", { "to_delete.md" => "x" }, [op("delete", path: "to_delete.md")])
cases << sc("delete with matching revision", { "to_delete.md" => "# Loaded content" }, [op("delete", path: "to_delete.md", expected_revision_of: "# Loaded content")])
cases << sc("delete with stale revision", { "to_delete.md" => "updated content" }, [op("delete", path: "to_delete.md", expected_revision_of: "loaded content")])
cases << sc("delete with empty-string revision", { "d.md" => "x" }, [op("delete", path: "d.md", expected_revision: "")])
cases << sc("delete missing", {}, [op("delete", path: "nonexistent.md")])
cases << sc("delete a directory via note delete", { "dir/" => nil }, [op("delete", path: "dir")])
cases << sc("delete internal symlink removes link only", { "real.md" => "r", "alias.md" => { "symlink" => "real.md" } }, [op("delete", path: "alias.md")])
cases << sc("delete traversal", {}, [op("delete", path: "../x.md")])
cases << sc("delete removes recent_files entries when .fed exists",
            { ".fed" => "<FED_TEMPLATE>recent_files = [{\"path\":\"a.md\",\"opened_at\":\"2026-01-01T00:00:00.000000Z\"},{\"path\":\"b.md\",\"opened_at\":\"2026-01-01T00:00:01.000000Z\"}]\n",
              "a.md" => "a", "b.md" => "b" },
            [op("delete", path: "a.md")])

# --- folders (service) ---
cases << sc("create_folder", {}, [op("create_folder", path: "new_folder")])
cases << sc("create_folder nested", {}, [op("create_folder", path: "deep/nested/folder")])
cases << sc("create_folder existing", { "f/" => nil }, [op("create_folder", path: "f")])
cases << sc("create_folder where file exists", { "f" => "file" }, [op("create_folder", path: "f")])
cases << sc("create_folder trailing slash / unicode / spaces", {}, [op("create_folder", path: "slash/"), op("create_folder", path: "日本語 フォルダ"), op("create_folder", path: "  sp  ")])
cases << sc("create_folder traversal / hidden", {}, [op("create_folder", path: "../x"), op("create_folder", path: ".hidden")])
cases << sc("create_folder empty path (root exists)", {}, [op("create_folder", path: "")])
cases << sc("delete_folder empty", { "empty_folder/" => nil }, [op("delete_folder", path: "empty_folder")])
cases << sc("delete_folder non-empty", { "folder/note.md" => "n" }, [op("delete_folder", path: "folder")])
cases << sc("delete_folder containing only hidden file counts as non-empty", { "folder/.DS_Store" => "x" }, [op("delete_folder", path: "folder")])
cases << sc("delete_folder missing", {}, [op("delete_folder", path: "nope")])
cases << sc("delete_folder on a file", { "f.md" => "x" }, [op("delete_folder", path: "f.md")])
cases << sc("delete_folder root", { "a.md" => "a" }, [op("delete_folder", path: "")])
cases << sc("delete_folder removes scoped recent entries",
            { ".fed" => "<FED_TEMPLATE>recent_files = [{\"path\":\"docs/a.md\",\"opened_at\":\"2026-01-01T00:00:00.000000Z\"}]\n", "docs/" => nil, "x.md" => "x" },
            [op("delete_folder", path: "docs")])

# --- exists?/file?/directory? ---
cases << sc("existence predicates", { "a.md" => "a", "d/" => nil, "l.md" => { "symlink" => "a.md" } },
            %w[a.md a d l.md missing].flat_map { |p| %w[exists? file? directory?].map { |m| op(m, path: p) } })

# --- Note model ---
cases << sc("Note.find returns content", { "test.md" => "# Test Note\n\nContent here" }, [op("Note.find", path: "test.md")])
cases << sc("Note.find normalizes missing extension", { "test.md" => "x" }, [op("Note.find", path: "test")])
cases << sc("Note.find missing", {}, [op("Note.find", path: "nonexistent.md")])
cases << sc("Note.find nested", { "folder/nested.md" => "Nested content" }, [op("Note.find", path: "folder/nested.md"), op("Note.find", path: "folder/nested")])
cases << sc("Note.find .fed (model does not block)", { ".fed" => "theme = dark\n" }, [op("Note.find", path: ".fed")])
cases << sc("Note.find unicode content revision", { "u.md" => CONTENTS["unicode"] }, [op("Note.find", path: "u")])
cases << sc("Note#save upsert creates / updates / parents", { "existing.md" => "Old" },
            [op("Note#save", path: "new_note.md", content: "New content"), op("Note#save", path: "existing.md", content: "Updated"),
             op("Note#save", path: "deep/nested/note.md", content: "Content"), op("Note#save", path: "nil_content.md", content: nil)])
cases << sc("Note#save validations", {}, [op("Note#save", path: "../outside.md", content: "x"), op("Note#save", path: "", content: "x"),
                                          op("Note#save", path: nil, content: "x"), op("Note#save", path: "notes..v2.md", content: "x")])
cases << sc("Note#save does not normalize extension", {}, [op("Note#save", path: "noext", content: "x")])
cases << sc("Note#save create_only on existing", { "e.md" => "keep" }, [op("Note#save", path: "e.md", content: "x", mode: "create_only")])
cases << sc("Note#save existing_only on missing", {}, [op("Note#save", path: "m.md", content: "x", mode: "existing_only")])
cases << sc("Note#save parent is a file", { "f.md" => "f" }, [op("Note#save", path: "f.md/c.md", content: "x")])
cases << sc("Note#save absolute path escaping", {}, [op("Note#save", path: "/etc/x.md", content: "x")])
cases << sc("Note#destroy", { "d.md" => "c" }, [op("Note#destroy", path: "d.md")])
cases << sc("Note#destroy matching / stale revision", { "a.md" => "loaded", "b.md" => "changed" },
            [op("Note#destroy", path: "a.md", expected_revision_of: "loaded"), op("Note#destroy", path: "b.md", expected_revision_of: "loaded")])
cases << sc("Note#destroy missing / traversal / no extension", { "noext.md" => "n" },
            [op("Note#destroy", path: "missing.md"), op("Note#destroy", path: "../x.md"), op("Note#destroy", path: "noext")])
cases << sc("Note#exists?", { "a.md" => "a", "d/" => nil }, [op("Note#exists?", path: "a"), op("Note#exists?", path: "a.md"), op("Note#exists?", path: "d"), op("Note#exists?", path: ""), op("Note#exists?", path: "../a")])

# --- Folder model ---
cases << sc("Folder#create / nested / existing / validation", { "exists/" => nil },
            [op("Folder#create", path: "new_folder"), op("Folder#create", path: "parent/child/grandchild"), op("Folder#create", path: "exists"),
             op("Folder#create", path: "../outside"), op("Folder#create", path: ""), op("Folder#create", path: "file.md")])
cases << sc("Folder#create where a file with same name exists", { "f" => "x" }, [op("Folder#create", path: "f")])
cases << sc("Folder#destroy empty / non-empty / missing / file", { "empty/" => nil, "full/n.md" => "n", "f.md" => "f" },
            [op("Folder#destroy", path: "empty"), op("Folder#destroy", path: "full"), op("Folder#destroy", path: "missing"), op("Folder#destroy", path: "f.md")])

# --- HTTP (NotesController / FoldersController JSON API) ---
cases << sc("POST /notes/:path creates note", {}, [http("POST", "/notes/new_note.md", { content: "# New Note" })])
cases << sc("POST /notes/:path adds .md extension", {}, [http("POST", "/notes/no_extension", { content: "Content" })])
cases << sc("POST /notes/:path without content param", {}, [http("POST", "/notes/blank")])
cases << sc("POST /notes/:path in subfolder and nested", { "subfolder/" => nil },
            [http("POST", "/notes/subfolder/note.md", { content: "x" }), http("POST", "/notes/a/b/c/deep.md", { content: "d" })])
cases << sc("POST /notes/:path existing -> 422", { "existing.md" => "Original" }, [http("POST", "/notes/existing.md", { content: "x" })])
cases << sc("POST /notes/:path traversal", {}, [http("POST", "/notes/..%2Fescape.md", { content: "x" }), http("POST", "/notes/a%2F..%2F..%2Fx", { content: "x" })])
cases << sc("POST /notes/:path unicode + space + CRLF content", {},
            [http("POST", "/notes/#{ERB::Util.url_encode('日本語 ノート')}", { content: "a\r\nb" }), http("POST", "/notes/%D7%A9%D7%9C%D7%95%D7%9D", { content: "h" })])
cases << sc("POST /notes/:path with dots in name keeps extension rule", {}, [http("POST", "/notes/v1.2", { content: "x" }), http("POST", "/notes/file.txt", { content: "x" })])
cases << sc("POST /notes hugo template", {}, [http("POST", "/notes", { template: "hugo", title: "My First Post" })])
cases << sc("POST /notes hugo template with parent", { "blog/" => nil }, [http("POST", "/notes", { template: "hugo", title: "Nested Post", parent: "blog" })])
cases << sc("POST /notes hugo template accented title", {}, [http("POST", "/notes", { template: "hugo", title: "Ação e Café \"quoted\"" })])
cases << sc("POST /notes hugo template missing title", {}, [http("POST", "/notes", { template: "hugo" }), http("POST", "/notes", { template: "hugo", title: "  " })])
cases << sc("POST /notes hugo template flat path style + custom template file",
            { ".fed" => "hugo_path_style = flat\n", ".hugo_template.md" => "---\ntitle: \"{{title}}\"\nslug: {{slug}}\ndate: {{date}}\n---\n" },
            [http("POST", "/notes", { template: "hugo", title: "Flat Post" })])
cases << sc("PATCH /notes/:path updates", { "test.md" => "Original" }, [http("PATCH", "/notes/test.md", { content: "Updated content" })])
cases << sc("PATCH /notes/:path without content clears note", { "test.md" => "Original" }, [http("PATCH", "/notes/test.md")])
cases << sc("PATCH /notes/:path missing -> 404 without creating", {}, [http("PATCH", "/notes/missing-folder/missing.md", { content: "x" })])
cases << sc("stale PATCH after rename / delete cannot recreate",
            { "old.md" => "o", "gone.md" => "g", "folder/n.md" => "n" },
            [http("POST", "/notes/old.md/rename", { new_path: "new.md" }), http("PATCH", "/notes/old.md", { content: "stale" }),
             http("DELETE", "/notes/gone.md"), http("PATCH", "/notes/gone.md", { content: "stale" }),
             op("rename", old_path: "folder", new_path: "moved"), http("PATCH", "/notes/folder/n.md", { content: "stale" })])
cases << sc("DELETE /notes/:path", { "d.md" => "x" }, [http("DELETE", "/notes/d.md")])
cases << sc("DELETE /notes/:path revision match / stale",
            { "a.md" => "loaded", "b.md" => "changed" },
            [http("DELETE", "/notes/a.md", { expected_revision: NotesOps.sha("loaded") }), http("DELETE", "/notes/b.md", { expected_revision: NotesOps.sha("loaded") })])
cases << sc("DELETE /notes/:path missing", {}, [http("DELETE", "/notes/missing.md")])
cases << sc("GET /notes/:path JSON show and revision", { "n.md" => "a\r\nb" }, [http("GET", "/notes/n.md"), http("GET", "/notes/n"), http("GET", "/notes/missing.md")])
cases << sc("POST /folders/:path", {}, [http("POST", "/folders/new_folder"), http("POST", "/folders/parent/child")])
cases << sc("POST /folders/:path existing -> 422", { "existing_folder/" => nil }, [http("POST", "/folders/existing_folder")])
cases << sc("POST /folders/:path traversal / dotted name", {}, [http("POST", "/folders/..%2Fx"), http("POST", "/folders/v1.2")])
cases << sc("DELETE /folders/:path empty / non-empty / missing", { "empty/" => nil, "full/n.md" => "n" },
            [http("DELETE", "/folders/empty"), http("DELETE", "/folders/full"), http("DELETE", "/folders/missing")])

Golden.write("notes_crud", Golden.fixture(
  source: "app/services/notes_service.rb, app/models/note.rb, app/models/folder.rb, app/controllers/notes_controller.rb, app/controllers/folders_controller.rb",
  generator: NotesFS.gen_path("notes_crud.rb"),
  notes: "Each case runs input.ops sequentially in one fresh sandbox (<ROOT> = notes root realpath, <OUTSIDE> = sibling dir). " \
         "Service ops return true or {error,message}; model ops return {ok, errors (full_messages, en locale), path}; http ops run the real Rails app " \
         "(RAILS_ENV=test, Accept: application/json) and return {status, body} or {status:500, unhandled_exception}. expected_revision_of:X means " \
         "expected_revision = SHA256(X). tree_after: full snapshot (hidden files incl. .fed and lock files included; symlinks as {symlink}). " \
         "'.fed' content beginning with the template generated by Config is shown as '<FED_TEMPLATE>' + rest (template text in fixture key fed_template); <TMP> = parent of <ROOT>; random temp-file hex in error messages -> <HEX>. " \
         "in input trees '<FED_TEMPLATE>' prefix is expanded the same way. Time is frozen with travel_to(#{NotesFS::PINNED_NOW.iso8601(6)}) (Hugo dates, recent opened_at); TZ=UTC. " \
         "stat_mode results for newly created files depend on process umask (generated with umask #{format('%04o', File.umask)}). list_tree mtimes of files written during ops are masked to <TIME>.",
  fed_template: NotesFS.fed_template, cases: cases))
