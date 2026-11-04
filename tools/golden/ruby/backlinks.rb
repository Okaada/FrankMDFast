# Golden fixtures: BacklinksService#find_backlinks (+ GET /notes/*path/backlinks).
require_relative "notes_fs_common"

cases = []
# Results are emitted in Dir.children (readdir) order per directory, which is filesystem
# dependent. We sort results by [path, line_number] and record that the original order is
# unspecified (see notes).
def sorted(res) = res.is_a?(Array) ? res.sort_by { |r| [r["path"], r["line_number"]] } : res

def bl_case(cases, name, tree, target, opts = {})
  NotesFS.sandbox(tree) do |root, _|
    svc = BacklinksService.new(base_path: root)
    res = NotesFS.capture { NotesFS.json(svc.find_backlinks(target, **opts.transform_keys(&:to_sym))) }
    cases << { "name" => name, "fn" => "BacklinksService#find_backlinks",
               "input" => { "tree" => tree, "target" => target, "options" => opts },
               "expected" => NotesFS.norm(sorted(res), root) }
  end
end

bl_case(cases, "no backlinks", { "target.md" => "# Target note", "other.md" => "# Unrelated note" }, "target.md")
bl_case(cases, "wikilink by name", { "target.md" => "# Target note", "source.md" => "Links to [[target]] here" }, "target.md")
bl_case(cases, "wikilink with .md extension", { "target.md" => "# Target", "source.md" => "Links to [[target.md]] here" }, "target.md")
bl_case(cases, "wikilink case-insensitive", { "Target.md" => "# Target", "source.md" => "Links to [[target]] here" }, "Target.md")
bl_case(cases, "wikilink by full path", { "projects/myproject.md" => "# My Project", "index.md" => "See [[projects/myproject]] for details" }, "projects/myproject.md")
bl_case(cases, "standard markdown link", { "target.md" => "# Target", "source.md" => "Check [this link](target.md) out" }, "target.md")
bl_case(cases, "markdown link with path", { "docs/guide.md" => "# Guide", "readme.md" => "See [the guide](docs/guide.md)" }, "docs/guide.md")
bl_case(cases, "skips the target file itself", { "self.md" => "I link to [[self]] in my own content" }, "self.md")
bl_case(cases, "multiple files", { "target.md" => "# Target", "source1.md" => "See [[target]]", "source2.md" => "Also [[target]]", "source3.md" => "No link here" }, "target.md")
bl_case(cases, "multiple matches in same file", { "target.md" => "# Target", "source.md" => "First [[target]] and\nlater [[target]] again" }, "target.md")
bl_case(cases, "context lines option", { "target.md" => "# Target", "source.md" => "Line 1\nLine 2\nLink to [[target]] here\nLine 4\nLine 5" }, "target.md", { "context_lines" => 1 })
bl_case(cases, "default context 3 clipped", { "s.md" => "a\nb\nc\nd\n[[t]]\ne\nf\ng\nh" }, "t.md")
bl_case(cases, "traversal rejected", {}, "../etc/passwd")
bl_case(cases, "traversal rejected nested", {}, "folder/../../secret.md")
bl_case(cases, "double dots anywhere rejected (even in names)", { "s.md" => "[[notes..v2]]" }, "notes..v2.md")
# Aliases / headings / embeds (pattern requires exact [[name]] or [[name.md]] closing)
bl_case(cases, "alias [[a|b]] NOT matched", { "s.md" => "See [[target|Alias]]" }, "target.md")
bl_case(cases, "heading [[a#h]] NOT matched", { "s.md" => "See [[target#Heading]]" }, "target.md")
bl_case(cases, "block ref [[a^id]] NOT matched", { "s.md" => "See [[target^abc]]" }, "target.md")
bl_case(cases, "embed ![[a]] matched", { "s.md" => "Embed ![[target]]" }, "target.md")
bl_case(cases, "spaces inside brackets NOT matched", { "s.md" => "[[ target ]]" }, "target.md")
bl_case(cases, "wikilink with path when target given by name only", { "s.md" => "[[folder/target]]" }, "target.md")
bl_case(cases, "wikilink by name for nested target", { "folder/target.md" => "t", "s.md" => "[[target]]" }, "folder/target.md")
bl_case(cases, "wikilink by name matches same-named notes in other folders too", { "a/target.md" => "a", "b/target.md" => "b", "s.md" => "[[target]]" }, "a/target.md")
bl_case(cases, "wikilink with uppercase .MD", { "s.md" => "[[target.MD]]" }, "target.md")
bl_case(cases, "target without .md extension", { "target.md" => "t", "s.md" => "[[target]]" }, "target")
bl_case(cases, "target without extension skips itself", { "target.md" => "[[target]]" }, "target")
bl_case(cases, "target with other extension", { "s.md" => "[[image.png]] and [x](image.png)" }, "image.png")
# Markdown links
bl_case(cases, "markdown link without extension", { "s.md" => "[t](target)" }, "target.md")
bl_case(cases, "markdown link with ./ and ../ prefixes (substring match)", { "s.md" => "[a](./target.md)\n[b](../target.md)\n[c](/target.md)" }, "target.md")
bl_case(cases, "markdown link with anchor", { "s.md" => "[t](target.md#section)" }, "target.md")
bl_case(cases, "markdown link substring false positive", { "s.md" => "[x](mytarget.md)\n[y](target.mdx)\n[z](target-other.md)" }, "target.md")
bl_case(cases, "markdown link with empty text", { "s.md" => "[](target.md)" }, "target.md")
bl_case(cases, "image link matched", { "s.md" => "![alt](target.md)" }, "target.md")
bl_case(cases, "markdown link with URL-encoded space NOT matched", { "my note.md" => "x", "s.md" => "[a](my%20note.md)\n[b](my note.md)\n[c](<my note.md>)" }, "my note.md")
bl_case(cases, "markdown link with title", { "s.md" => "[t](target.md \"Title\")" }, "target.md")
bl_case(cases, "markdown link case-insensitive", { "s.md" => "[t](TARGET.MD)" }, "target.md")
bl_case(cases, "reference-style link NOT matched", { "s.md" => "[t][ref]\n\n[ref]: target.md" }, "target.md")
bl_case(cases, "bare mention NOT matched", { "s.md" => "talk about target.md and target" }, "target.md")
bl_case(cases, "nested path markdown link", { "s.md" => "[g](docs/sub/guide.md)\n[h](sub/guide.md)" }, "docs/sub/guide.md")
# Code blocks are not excluded
bl_case(cases, "links inside fenced and inline code still match", { "s.md" => "```\n[[target]]\n```\n`[[target]]`\n    [t](target.md)" }, "target.md")
# Regex metacharacters and unicode in names
bl_case(cases, "regex metacharacters in name are escaped", { "s.md" => "[[a+b (draft)]]\n[[aab (draft)]]\n[x](a+b (draft).md)" }, "a+b (draft).md")
bl_case(cases, "brackets in name", { "s.md" => "[[note [1]]]\n[[note [1].md]]" }, "note [1].md")
bl_case(cases, "unicode names (accents/CJK/Hebrew/emoji)", { "s.md" => "[[café]]\n[[CAFÉ]]\n[[cafe]]\n[[日本語]]\n[[שלום]]\n[[👨‍👩‍👧]]" }, "café.md")
bl_case(cases, "CJK target", { "s.md" => "[[日本語]]\n[x](日本語.md)" }, "日本語.md")
bl_case(cases, "Hebrew target", { "s.md" => "[[שלום]]" }, "שלום.md")
bl_case(cases, "emoji ZWJ target", { "s.md" => "[[👨‍👩‍👧]]\n[[👨]]" }, "👨‍👩‍👧.md")
bl_case(cases, "combining vs precomposed not equivalent", { "s.md" => "[[étude]]\n[[étude]]" }, "étude.md")
bl_case(cases, "spaces in name", { "s.md" => "[[My Note]]\n[[my note]]\n[[My  Note]]" }, "My Note.md")
# File filtering
bl_case(cases, "hidden files and dirs skipped, non-md skipped", { ".h.md" => "[[t]]", ".d/x.md" => "[[t]]", "a.txt" => "[[t]]", "b.MD" => "[[t]]", "v.md" => "[[t]]" }, "t.md")
bl_case(cases, "symlinked files and dirs ARE followed (unlike search)", { "real.md" => "[[t]]", "alias.md" => { "symlink" => "real.md" }, "ext" => { "symlink" => "<OUTSIDE>" } }, "t.md")
bl_case(cases, "CRLF lines chomped", { "s.md" => "a\r\n[[t]]\r\nb\r\n" }, "t.md")
bl_case(cases, "lone CR is not a line break", { "s.md" => "a\r[[t]]\rb" }, "t.md")
bl_case(cases, "empty target", { "s.md" => "[[]]\n[x]()\n[[.md]]" }, "")
bl_case(cases, "target '.md'", { "s.md" => "[[]]\n[[.md]]\n[x]()" }, ".md")
bl_case(cases, "invalid UTF-8 source file raises", { "bad.md" => { "base64" => ["[[t]] \xFF".b].pack("m0") } }, "t.md")

# outside spec for the symlink case needs content: rebuild that case with outside tree
cases.reject! { |c| c["name"].start_with?("symlinked files and dirs") }
NotesFS.sandbox({ "real.md" => "[[t]]", "alias.md" => { "symlink" => "real.md" }, "ext" => { "symlink" => "<OUTSIDE>" } }, outside: { "external.md" => "[[t]]" }) do |root, _|
  res = NotesFS.capture { NotesFS.json(BacklinksService.new(base_path: root).find_backlinks("t.md")) }
  cases << { "name" => "symlinked files and dirs ARE followed (unlike search)", "fn" => "BacklinksService#find_backlinks",
             "input" => { "tree" => { "real.md" => "[[t]]", "alias.md" => { "symlink" => "real.md" }, "ext" => { "symlink" => "<OUTSIDE>" } },
                          "outside" => { "external.md" => "[[t]]" }, "target" => "t.md", "options" => {} },
             "expected" => NotesFS.norm(sorted(res), root) }
end

# HTTP endpoint (normalizes path with .md)
BT = { "target.md" => "# T", "s.md" => "[[target]]", "n/s2.md" => "[x](target.md)" }.freeze
["/notes/target.md/backlinks", "/notes/target/backlinks", "/notes/none/backlinks", "/notes/..%2Fx/backlinks"].each do |url|
  NotesFS.sandbox(BT) do |root, _|
    Golden.with_env("NOTES_PATH" => root) do
      r = NotesFS.http(:get, url)
      r["body"]["backlinks"] = sorted(r["body"]["backlinks"]) if r["body"].is_a?(Hash) && r["body"]["backlinks"]
      cases << { "name" => "GET #{url}", "fn" => "http", "input" => { "tree" => BT, "url" => url }, "expected" => r }
    end
  end
end

Golden.write("backlinks", Golden.fixture(
  source: "app/services/backlinks_service.rb, app/controllers/notes_controller.rb#backlinks",
  generator: NotesFS.gen_path("backlinks.rb"),
  notes: "Results JSON: [{line_number, match_text, context[{line_number,content,is_match}], path, name}]. The original returns results in filesystem " \
         "readdir order (Dir.children, recursive, unsorted) with per-file line order; fixtures SORT results by [path, line_number] because that order " \
         "is not deterministic. Patterns (all /i): /\\[\\[NAME(\\.md)?\\]\\]/, /\\[\\[PATH_NO_EXT(\\.md)?\\]\\]/, /\\[[^\\]]*\\]\\([^)]*PATH_NO_EXT(\\.md)?[^)]*\\)/ " \
         "with NAME = basename minus .md, Regexp.escape'd. Any '..' in target -> ArgumentError 'Invalid path'. No code-block exclusion, no alias/heading support. " \
         "Unlike search/tree, symlinks are followed and invalid UTF-8 is not scrubbed (raises).",
  cases: cases))
