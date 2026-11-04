# Golden fixtures: NotesService#list_tree (tree JSON, sort order, filtering) and Folder#children.
require_relative "notes_fs_common"

cases = []
def tree_case(cases, name, tree, outside: {}, extra: {}, note: nil)
  NotesFS.sandbox(tree, outside: outside) do |root, out|
    yield(root, out) if block_given?
    svc = NotesService.new(base_path: root)
    result = NotesFS.capture { NotesFS.json(svc.list_tree) }
    c = { "name" => name, "fn" => "NotesService#list_tree",
          "input" => { "tree" => tree, "outside" => outside }.merge(extra),
          "expected" => { "tree" => NotesFS.norm(result, root), "tree_after" => NotesFS.snap(root) } }
    c["notes"] = note if note
    cases << c
  end
end

M = NotesFS::BASE_MTIME
def f(content, mtime) = { "content" => content, "mtime" => mtime }

tree_case(cases, "empty directory", {})
tree_case(cases, "two files (newest first)", { "note1.md" => "# Test\n\nContent", "note2.md" => "# Test\n\nContent" })
tree_case(cases, "files sorted by mtime desc regardless of name",
          { "a.md" => f("a", M - 500), "b.md" => f("b", M), "c.md" => f("c", M - 100), "z.md" => f("z", M + 50) })
tree_case(cases, "nested folder with child", { "folder1/" => nil, "folder1/nested.md" => "x" })
tree_case(cases, "root images/videos stay visible when they contain markdown",
          { "images/photo.png" => "media", "images/notes/guide.md" => "g", "videos/clip.mp4" => "media" })
tree_case(cases, "only root managed media folders are hidden",
          { "images/photo.png" => "media", "videos/clip.mp4" => "media", "images-old/note.md" => "n",
            "archive/images/note.md" => "n", "archive/videos/note.md" => "n" })
tree_case(cases, "empty root images/ and videos/ hidden; Images/ (case) kept",
          { "images/" => nil, "videos/" => nil, "Images/" => nil, "VIDEOS/x.png" => "p" })
tree_case(cases, "root images/ with only a hidden .md stays hidden (glob skips dotfiles)",
          { "images/.secret.md" => "s", "images/a.png" => "p" })
tree_case(cases, "root images/ with deeply nested md stays visible",
          { "images/a/b/c/d.md" => "d", "images/x.png" => "p" })
tree_case(cases, "root images/ containing a .MD (uppercase) file is hidden", { "images/UP.MD" => "u" })
tree_case(cases, "root images symlink is skipped", { "real/x.md" => "x", "images" => { "symlink" => "real" } })
tree_case(cases, "folders before files", { "zebra.md" => "z", "alpha/" => nil })
tree_case(cases, "folder order stable regardless of folder mtime", { "alpha/" => nil, "beta/" => nil, "note.md" => "n" }) do |root, _|
  File.utime(M + 999, M + 999, File.join(root, "beta"))
  File.utime(M - 999, M - 999, File.join(root, "alpha"))
end
tree_case(cases, "broken symlink skipped", { "real.md" => "r", "broken_link.md" => { "symlink" => "/nonexistent/target.md" } })
tree_case(cases, "symlinked directory (outside) skipped", { "a.md" => "a", "external" => { "symlink" => "<OUTSIDE>" } },
          outside: { "external.md" => "findme" })
tree_case(cases, "symlinked directory (inside) skipped", { "sub/n.md" => "n", "alias" => { "symlink" => "sub" } })
tree_case(cases, "symlinked md file (inside and outside) skipped",
          { "real.md" => "r", "alias.md" => { "symlink" => "real.md" }, "out.md" => { "symlink" => "<OUTSIDE>/o.md" } },
          outside: { "o.md" => "o" })
tree_case(cases, "hidden files and dirs ignored",
          { ".hidden.md" => "h", "visible.md" => "v", ".git/config" => "x", ".obsidian/a.md" => "a", "sub/.dot.md" => "d", "sub/ok.md" => "o" })
tree_case(cases, ".fed omitted at root and in subfolders",
          { ".fed" => "theme = dark", "note.md" => "n", "subfolder/.fed" => "theme = dark", "subfolder/note.md" => "n" })
tree_case(cases, "hidden temp/lock files ignored",
          { ".fed.abc123.tmp" => "", ".frankmd-filesystem.lock" => "", ".frankmd-config.lock" => "", ".n.md.0a1b2c.tmp" => "", "n.md" => "n" })
tree_case(cases, "non-markdown files excluded (extension is case-sensitive .md)",
          { "a.txt" => "t", "b.markdown" => "m", "c.MD" => "C", "d.Md" => "d", "e" => "noext", "f.md.bak" => "b",
            "g.png" => "p", "h.pdf" => "p", "i.md" => "i", ".md" => "dotmd", "j..md" => "jj", "k.md " => "space",
            "folder.md/inner.md" => "dir named .md", "empty.md" => "" })
tree_case(cases, "empty directories included with empty children",
          { "empty/" => nil, "outer/inner/" => nil, "outer/only_txt/x.txt" => "t" })
deep = (1..10).map { |i| "l#{i}" }.join("/")
tree_case(cases, "deeply nested 10 levels", { "#{deep}/leaf.md" => "leaf", "l1/top.md" => "top", "#{deep}/l11/" => nil })
tree_case(cases, "folder sort: case-insensitive by downcase, then byte order",
          { "beta/" => nil, "Alpha/" => nil, "charlie/" => nil, "Delta/" => nil, "_under/" => nil, "10/" => nil,
            "2/" => nil, "a b/" => nil, "a-b/" => nil, "a_b/" => nil, "ab/" => nil, "A.B/" => nil, "~tilde/" => nil, "x.md" => "x" })
tree_case(cases, "folder sort: unicode names (downcase is Unicode-aware, compare by bytes)",
          { "Éclair/" => nil, "zeta/" => nil, "eclair/" => nil, "Ångström/" => nil, "日本語/" => nil, "שלום/" => nil,
            "😀 emoji/" => nil, "👨‍👩‍👧 family/" => nil, "étude/" => nil, "étude2/" => nil, "ẞig/" => nil,
            "İstanbul/" => nil, "Ölçü/" => nil, "Zulu/" => nil, "ção/" => nil, "Ça va/" => nil, " nbsp/" => nil })
tree_case(cases, "unicode / CJK / Hebrew / emoji / spaces in file names",
          { "日本語/ノート.md" => "j", "שלום.md" => "h", "👨‍👩‍👧 family.md" => "e", "café com leite.md" => "c",
            "é.md" => "comb", "  spaced  .md" => "s", "tab\tname.md" => "t", "über/ünïcödé.md" => "u" })
tree_case(cases, "names with leading dashes, dots in middle, multiple extensions",
          { "-dash.md" => "d", "notes..v2.md" => "v", "a.b.c.md" => "abc", "x.md.md" => "xx", "-dir-/y.md" => "y" })
tree_case(cases, "mtime truncation (fractional), negative and future mtimes",
          { "frac.md" => f("f", M + 0.9), "neg.md" => f("n", -100), "future.md" => f("fu", 4_102_444_800),
            "zero.md" => f("z", 0) })
tree_case(cases, "files with equal mtime (tie: order follows Ruby sort_by on readdir order; not guaranteed)",
          { "tie_a.md" => f("a", M), "tie_b.md" => f("b", M), "newer.md" => f("n", M + 1) },
          note: "Tie order is implementation-defined (unstable sort over Dir.children order). Do not rely on it.")
tree_case(cases, "folder names equal after downcase (tie)",
          { "Same/a.md" => "a", "same/b.md" => "b", "SAME/" => nil },
          note: "Ties after downcase keep implementation-defined order (unstable sort_by over readdir).")
tree_case(cases, "non-UTF8-safe content does not matter for tree", { "bin.md" => { "base64" => ["\xFF\xFE\x00x".b].pack("m0") } })
tree_case(cases, "mixed realistic tree",
          { "Inbox/todo.md" => f("t", M - 10), "Inbox/ideas.md" => f("i", M - 5), "Projects/frankmd/spec.md" => f("s", M),
            "Projects/frankmd/assets/logo.png" => "p", "Projects/Alpha/readme.md" => f("r", M - 1),
            "journal.md" => f("j", M - 2), "README.md" => f("R", M - 3), "images/logo.png" => "p", ".fed" => "theme = dark\n",
            "archive/" => nil })

# Folder#children (uses list_tree, finds node by path)
CHILD_TREE = { "a/b/c.md" => "c", "a/d.md" => "d", "a/empty/" => nil, "x.md" => "x", "日本/語.md" => "j" }.freeze
["a", "a/b", "a/empty", "missing", "x.md", "", "a/", "日本"].each do |p|
  NotesFS.sandbox(CHILD_TREE) do |root, _|
    Golden.with_env("NOTES_PATH" => root) do
      cases << { "name" => "Folder#children #{p.inspect}", "fn" => "Folder#children",
                 "input" => { "tree" => CHILD_TREE, "path" => p },
                 "expected" => NotesFS.capture { NotesFS.json(Folder.new(path: p).children) } }
    end
  end
end

# Note.all == list_tree via NOTES_PATH, and Folder.find / exists?
NotesFS.sandbox(CHILD_TREE) do |root, _|
  Golden.with_env("NOTES_PATH" => root) do
    cases << { "name" => "Note.all uses NOTES_PATH", "fn" => "Note.all", "input" => { "tree" => CHILD_TREE },
               "expected" => NotesFS.json(Note.all) }
    ["a", "a/b", "x.md", "missing", "", nil, "../x"].each do |p|
      cases << { "name" => "Folder.find #{p.inspect}", "fn" => "Folder.find", "input" => { "tree" => CHILD_TREE, "path" => p },
                 "expected" => NotesFS.capture { fo = Folder.find(p); { "path" => fo.path, "name" => fo.name, "parent_path" => fo.parent_path, "persisted" => fo.persisted? } } }
    end
  end
end

# HTTP tree endpoint returns HTML partial; capture the HTML string (expanded/selected params)
HT = { "folder1/note.md" => f("n", M), "folder1/sub/deep.md" => f("d", M - 1), "top.md" => f("t", M - 2), "e/" => nil }.freeze
[nil, { expanded: "folder1", selected: "folder1/note.md" }, { expanded: "folder1,folder1/sub", selected: "top.md" }].each do |params|
  NotesFS.sandbox(HT) do |root, _|
    Golden.with_env("NOTES_PATH" => root) do
      r = NotesFS.http(:get, "/notes/tree", params: params, headers: { "Accept" => "text/html" })
      cases << { "name" => "GET /notes/tree #{params.to_json}", "fn" => "http", "input" => { "tree" => HT, "params" => params },
                 "expected" => r.merge("tree_after" => NotesFS.snap(root)) }
    end
  end
end

Golden.write("notes_tree", Golden.fixture(
  source: "app/services/notes_service.rb#list_tree/build_tree, app/models/folder.rb#children",
  generator: NotesFS.gen_path("notes_tree.rb"),
  notes: "Tree items: folder {name,path,type:'folder',children} / file {name (basename minus .md),path,type:'file',file_type:'markdown',mtime (Integer epoch seconds, truncated)}. " \
         "Order per directory: folders first sorted by basename.downcase (Ruby Unicode downcase, then byte-wise String <=>), then files by mtime DESC. " \
         "mtimes pinned via File.utime: explicit {content,mtime} entries use their mtime, others get #{M} - 100*i in spec order of files (i counts files only, " \
         "per tree spec incl. outside spec separately). Symlinks (files or dirs, inside or outside root, broken) are always skipped (lstat). " \
         "list_tree creates <ROOT>/.frankmd-filesystem.lock (visible in tree_after). Ties (equal mtime, equal downcased folder names) are not ordered deterministically by the original.",
  fed_template: NotesFS.fed_template, cases: cases))
