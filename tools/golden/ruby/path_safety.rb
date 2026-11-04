# Golden fixtures: PathSafety + path normalization/validation in Note/Folder/NotesService.
require_relative "notes_fs_common"

cases = []
LEX_BASE = "/data/notes" # does not exist -> purely lexical (no realpath check)

PATHS = [
  "note.md", "a/b.md", "../../etc/passwd", "/etc/passwd", "/data/notes-backup/secret.md",
  "/data/notesX", "notes..v2.md", nil, "", ".", "./", "..", "../", "a/../b.md", "a/../../b.md",
  "../notes/x.md", "../notes", "../notes-backup/x.md", "/data/notes/x.md", "/data/notes", "/data/notes/",
  "/data/notes/../notes/x.md", "/data/notes/../secret", "%2e%2e/%2e%2e/etc/passwd", "..%2fetc%2fpasswd",
  "%2E%2E", "..\\..\\etc\\passwd", "a\\b.md", "\\etc\\passwd", "foo\0bar.md", "\0", "~/x.md", "~", "~root",
  "a/./b.md", "a//b.md", "a///b.md", "trailing/", "trailing/.", "trailing//", "dir/..", "dir/../",
  "\u2026/x", "\uFF0E\uFF0E/x", "\u2025/x", "a/b/../../../x", "./../x", "./a.md", "a/b/../../c/./d.md",
  "日本語/ノート.md", "שלום.md", "👨‍👩‍👧.md", "e\u0301.md", "\u00e9.md", " .md", " ", "  leading.md",
  "trailing.md ", "\t", "\u00a0", "tab\tname.md", "new\nline.md", "cr\rname.md", ".hidden", ".fed",
  ".frankmd-config.lock", ".frankmd-filesystem.lock", "....", ".../x", "x/...", "x/....", "...",
  "/", "//etc/passwd", "///data/notes/x", "//data/notes/x", "C:\\Windows\\system32", "C:/Windows",
  "file:///etc/passwd", "a/" * 150 + "deep.md", "x" * 300 + ".md", "NOTE.MD", "Note.Md",
  "-leading-dash.md", "trailing-dash-.md", "a:b.md", "q\"uote's.md", "*?.md", "<>|.md",
  123, :sym
].freeze

def show(v) = v.is_a?(Pathname) ? v.to_s : v
def label(p) = p.nil? ? "nil" : p.inspect[0, 60]

# 1. lexical containment against several base spellings
[LEX_BASE, "/data/notes/", "/data/notes/./", "/data//notes", "/data/x/../notes"].each do |base|
  PATHS.each do |p|
    next if base != LEX_BASE && !(p.nil? || p.is_a?(String) && p.length < 40)
    cases << { "name" => "contain lexical base=#{base} path=#{label(p)}", "fn" => "PathSafety.contain",
               "input" => { "base" => base, "path" => p.is_a?(Symbol) ? { "symbol" => p.to_s } : p, "base_exists" => false },
               "expected" => NotesFS.capture { show(PathSafety.contain(base, p)) } }
  end
end
["/", "relative/base", "notes", ".", ""].each do |base|
  ["a.md", "../a.md", "/a.md", "..", "", "x/../../y"].each do |p|
    cases << { "name" => "contain lexical odd base=#{base.inspect} path=#{p.inspect}", "fn" => "PathSafety.contain",
               "input" => { "base" => base, "path" => p, "base_exists" => false },
               "expected" => NotesFS.capture { show(PathSafety.contain(base, p)) } }
  end
end

# 2. within? (separator anchored)
[["/data/notes", "/data/notes"], ["/data/notes/a", "/data/notes"], ["/data/notes-backup", "/data/notes"],
 ["/data/notesX", "/data/notes"], ["/data", "/data/notes"], ["/data/notes/", "/data/notes"],
 ["/", "/"], ["/a", "/"], ["/data/notes/a", "/data/notes/"]].each do |path, base|
  cases << { "name" => "within? #{path} in #{base}", "fn" => "PathSafety.within?",
             "input" => { "path" => path, "base" => base, "as" => "Pathname" },
             "expected" => PathSafety.within?(Pathname.new(path), Pathname.new(base)) }
end

# 3. real filesystem base with symlinks
TREE = {
  "ok.md" => "x", "sub/n.md" => "n", "Sub2/N.md" => "N", "empty/" => nil,
  "link_out_file.md" => { "symlink" => "<OUTSIDE>/secret.md" },
  "escape" => { "symlink" => "<OUTSIDE>" },
  "escape_rel" => { "symlink" => "../outside" },
  "inner" => { "symlink" => "sub" },
  "inner_abs" => { "symlink" => "<ROOT>/sub" },
  "inner_file.md" => { "symlink" => "ok.md" },
  "broken" => { "symlink" => "/nonexistent/x" },
  "broken_in" => { "symlink" => "missing_target" },
  "loop" => { "symlink" => "loop" },
  "dotdot_link" => { "symlink" => ".." },
  "self_link" => { "symlink" => "." },
  "sub/up" => { "symlink" => ".." },
  "sub/up2" => { "symlink" => "../.." },
  "日本語/ノート.md" => "j"
}.freeze
OUTSIDE = { "secret.md" => "s", "dir/x.md" => "x" }.freeze
FS_PATHS = [
  "ok.md", "sub/n.md", "sub", "missing.md", "new/dir/leaf.md", "link_out_file.md", "escape", "escape/secret.md",
  "escape/new.md", "escape/dir/x.md", "escape_rel/secret.md", "inner/n.md", "inner/new.md", "inner_abs/n.md",
  "inner_file.md", "broken", "broken/x.md", "broken_in", "broken_in/x.md", "loop", "loop/x.md",
  "dotdot_link", "dotdot_link/outside/secret.md", "dotdot_link/notes/ok.md", "self_link/ok.md", "sub/up/ok.md",
  "sub/up2/outside/secret.md", "sub/up2", "SUB/n.md", "sub2/N.md", "Sub2/n.md", "OK.md", "../outside/secret.md",
  "<ROOT>/ok.md", "<OUTSIDE>/secret.md", "", ".", "..", "日本語/ノート.md", "日本語/新.md", "foo\0bar.md",
  "empty", "empty/", "empty/.", "trailing/", "ok.md/", "ok.md/x", ".fed", ".frankmd-config.lock"
].freeze

NotesFS.sandbox(TREE, outside: OUTSIDE) do |root, out|
  FS_PATHS.each do |p|
    path = NotesFS.subst(p, root, out)
    cases << { "name" => "contain fs path=#{p.inspect}", "fn" => "PathSafety.contain",
               "input" => { "base" => "<ROOT>", "path" => p, "base_exists" => true, "tree" => TREE, "outside" => OUTSIDE },
               "expected" => NotesFS.norm(NotesFS.capture { show(PathSafety.contain(root, path)) }, root) }
  end
  svc = NotesService.new(base_path: root)
  FS_PATHS.each do |p|
    path = NotesFS.subst(p, root, out)
    [true, false].each do |must|
      cases << { "name" => "NotesService#safe_path must_exist=#{must} path=#{p.inspect}", "fn" => "NotesService#safe_path",
                 "input" => { "path" => p, "must_exist" => must, "tree" => TREE, "outside" => OUTSIDE },
                 "expected" => NotesFS.norm(NotesFS.capture { show(svc.send(:safe_path, path, must_exist: must)) }, root) }
    end
    %w[exists? file? directory?].each do |m|
      cases << { "name" => "NotesService##{m} path=#{p.inspect}", "fn" => "NotesService##{m}",
                 "input" => { "path" => p, "tree" => TREE, "outside" => OUTSIDE },
                 "expected" => NotesFS.norm(NotesFS.capture { svc.public_send(m, path) }, root) }
    end
  end
end

# 4. Note / Folder path helpers & validation
NOTE_PATHS = ["note", "note.md", "folder/note", "folder/note.md", ".fed", "sub/.fed", "x.fed", ".fed.md", "", nil,
              "  ", "note.MD", "note.markdown", "note.txt", "a.b", "dir/", "trailing.md/", "../x", "a/../b",
              "notes..v2", "日本語", "שלום", "👨‍👩‍👧", "e\u0301", ".hidden", "folder/.md", ".md", "md", "x.md.md",
              "\u00a0", "a\\b", "/abs/path"].freeze
NOTE_PATHS.each do |p|
  cases << { "name" => "Note.normalize_path #{p.inspect}", "fn" => "Note.normalize_path", "input" => p,
             "expected" => NotesFS.capture { Note.normalize_path(p) } }
  note = Note.new(path: p)
  cases << { "name" => "Note attrs #{p.inspect}", "fn" => "Note#name/directory/valid?", "input" => p,
             "expected" => { "name" => note.name, "directory" => note.directory, "valid" => note.valid?,
                             "errors" => note.errors.full_messages, "to_param" => note.to_param } }
  folder = Folder.new(path: p)
  cases << { "name" => "Folder attrs #{p.inspect}", "fn" => "Folder#name/parent_path/valid?", "input" => p,
             "expected" => { "name" => folder.name, "parent_path" => folder.parent_path, "valid" => folder.valid?,
                             "errors" => folder.errors.full_messages } }
end

# 5. HTTP guard for the root config file / lock files (NotesController#reject_config_file, serve_asset)
GUARD = [
  [:get, "/notes/.fed"], [:get, "/notes/./.fed"], [:get, "/notes/%2Efed"], [:get, "/notes/sub/../.fed"],
  [:get, "/notes/sub/.fed"], [:get, "/notes/.frankmd-config.lock"], [:get, "/notes/.frankmd-filesystem.lock"],
  [:get, "/notes/..%2F..%2Fetc%2Fpasswd"], [:get, "/notes/%2E%2E/secret.txt"], [:get, "/notes/escape/secret.md"],
  [:get, "/notes/img.png"], [:get, "/notes/missing.png"], [:get, "/notes/ok"], [:get, "/notes/ok.md"],
  [:patch, "/notes/.fed", { content: "x" }], [:post, "/notes/.fed", { content: "x" }], [:delete, "/notes/.fed"],
  [:post, "/notes/ok.md/rename", { new_path: ".fed" }], [:post, "/notes/ok.md/rename", { new_path: "./.fed" }],
  [:get, "/?file=.fed"], [:patch, "/notes/.frankmd-config.lock", { content: "x" }]
].freeze
GUARD_TREE = { "ok.md" => "ok", "img.png" => "PNG", "sub/.fed" => "theme = x\n",
               "escape" => { "symlink" => "<OUTSIDE>" } }.freeze
GUARD.each do |method, url, params|
  NotesFS.sandbox(GUARD_TREE, outside: OUTSIDE) do |root, _out|
    File.write(File.join(root, ".fed"), "theme = dark\n")
    Golden.with_env("NOTES_PATH" => root) do
      r = NotesFS.http(method, url, params: params)
      r["body"] = "<HTML>" if r["body"].is_a?(String) && r["body"].include?("<html")
      cases << { "name" => "http guard #{method.upcase} #{url} #{params&.to_json}", "fn" => "http",
                 "input" => { "tree" => GUARD_TREE.merge(".fed" => "theme = dark\n"), "outside" => OUTSIDE,
                              "method" => method.to_s.upcase, "url" => url, "params" => params },
                 "expected" => NotesFS.norm(r.merge("tree_after" => NotesFS.snap(root)), root) }
    end
  end
end

Golden.write("path_safety", Golden.fixture(
  source: "app/services/path_safety.rb, app/services/notes_service.rb#safe_path, app/models/note.rb, app/models/folder.rb, app/controllers/notes_controller.rb#reject_config_file",
  generator: NotesFS.gen_path("path_safety.rb"),
  notes: "PathSafety.contain returns a cleaned Pathname (rendered as string) or nil. Lexical cases use a base that does not exist " \
         "(no realpath check, as in path_safety_test). fs cases use a real temp tree; <ROOT> = realpath of notes root, <OUTSIDE> = sibling dir " \
         "'outside' (symlink targets in tree spec use these placeholders). Errors captured as {error,message}. Symbol paths are given as {symbol:..}. " \
         "http cases run the real Rails app (RAILS_ENV=test) through ActionDispatch::Integration::Session with Accept: application/json; " \
         "HTML bodies replaced by <HTML>; binary file bodies kept as string. A bare GET creates .fed / lock files as a side effect (see tree_after).",
  fed_template: NotesFS.fed_template, cases: cases))
