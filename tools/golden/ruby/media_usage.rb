# Golden fixtures: MediaUsageService (reverse index of images/ videos/ references in notes)
#
#   cd ~/Projects/FrankMD && GOLDEN_TMP=$SCRATCH/notes/misc mise exec ruby@3.4.8 -- \
#     bundle exec rails runner /mnt/data/Projects/FrankMDFast/tools/golden/ruby/media_usage.rb
require_relative "../lib/golden"

cases = []
BT = "\x60"

# Fake config with the app's default extension lists (Config::SCHEMA defaults).
class DefaultExtConfig
  def upload_extensions(key)
    Config::SCHEMA.fetch(key)[:default].split(",").map { |e| e.strip.downcase }.reject(&:blank?)
  end
end

# Strings that are not valid UTF-8 are encoded as {"invalid_utf8_base64": ...}
def safe(v)
  case v
  when String then v.dup.force_encoding("UTF-8").valid_encoding? ? v : { "invalid_utf8_base64" => [v.b].pack("m0") }
  when Array then v.map { |x| safe(x) }
  when Hash then v.to_h { |k, x| [k.is_a?(String) && !k.valid_encoding? ? "invalid_utf8:" + [k.b].pack("m0") : k, safe(x)] }
  else v
  end
end

def build(root, tree)
  tree.each do |rel, v|
    if v.is_a?(Hash) && v["base64"]
      File.binwrite(File.join(root, rel), v["base64"].unpack1("m0"))
    elsif v.is_a?(Hash) && v["symlink"]
      path = File.join(root, rel)
      FileUtils.mkdir_p(File.dirname(path))
      File.symlink(v["symlink"].sub("<OUTSIDE>", "#{root}_outside"), path)
    else
      Golden.build_tree(root, { rel => v })
    end
  end
end

def run_index(name, tree, outside: {})
  root = Golden.mktmp("mu")
  out = "#{root}_outside"
  FileUtils.mkdir_p(out)
  Golden.build_tree(out, outside)
  build(root, tree)
  index = Golden.capture { MediaUsageService.new(base_path: root).build_index }
  lib = Golden.capture { MediaLibraryService.new(base_path: root, config: DefaultExtConfig.new).list }
  library_paths = lib.is_a?(Hash) && !lib["error"] ? (lib[:images] + lib[:videos]).map { |i| i[:path] }.sort : lib
  expected = { "index" => Golden.normalize(index, root) }
  if index.is_a?(Hash) && !index["error"]
    expected["usage_counts"] = index.transform_values(&:length)
    expected["library_paths"] = library_paths
    expected["unreferenced_library_paths"] = library_paths.is_a?(Array) ? library_paths - index.keys : library_paths
  end
  { "name" => "build_index #{name}", "fn" => "build_index",
    "input" => { "tree" => tree, "outside_tree" => outside }, "expected" => expected }
ensure
  FileUtils.rm_rf(root) if root
  FileUtils.rm_rf(out) if out
end

# ---------------------------------------------------------------- whole-tree cases (from tests)
cases << run_index("markdown images, image links, html video sources by distinct note", {
  "projects/guides/guide.md" => <<~MD,
    ![photo](../../images/my%20cat.png)
    [![same photo](../../images/my%20cat.png)](https://example.test/photo)
    ![diagram](../../images/diagram%20%28draft%29.png)
    <video controls><source src="../../videos/clips/clip%20one.mp4" type="video/mp4"></video>
    [download video](../../videos/clips/clip%20one.mp4)
  MD
  "index.md" => "![photo](images/my%20cat.png)"
})
cases << run_index("normalizes destinations and html source urls", {
  "docs/media.md" => <<~MD
    ![cover](../images/cover%20photo.png)
    [open image](/images/cover%20photo.png#full-size)
    <img src=/notes/images/inline%20photo.png>
    [trailer](../videos/trailer.mp4)
    <video src="../videos/trailer.mp4"></video>
  MD
})
cases << run_index("unescapes markdown punctuation while retaining containment", {
  "references.md" => <<~'MD'
    ![diagram](images/diagram\(draft\).png)
    ![hash](images/hash\#name.png)
    ![question](images/question\?name.png)
    ![reference][diagram]
    [diagram]: images/diagram\(draft\).png
    ![outside](images/\.\./../../outside.png)
  MD
})
cases << run_index("full, collapsed, shortcut references with normalized labels", {
  "references.md" => <<~MD
    ![cover][  PHOTO   IMAGE ]
    ![video][]
    [shortcut]
    [ photo image ]: images/photo%20one.png
    [video]: videos/clip.mp4
    [shortcut]: images/shortcut.png
    [unused]: images/unused.png
  MD
})
cases << run_index("unused reference definition", { "unused.md" => "[photo]: images/photo.png\n" })
cases << run_index("duplicate reference uses once per note", {
  "duplicates.md" => <<~MD
    ![first][photo]
    [second][  PHOTO ]
    ![photo][]
    [photo]
    [photo]: images/photo.png
  MD
})
cases << run_index("plain text, unrelated url substrings, code", {
  "examples.md" => <<~MD
    Plain mention: images/mentioned.png
    [external](https://example.test/notes/images/external.png)
    [near match](notimages/near.png)
    #{BT}![inline](images/inline-code.png)#{BT}

    #{BT * 3}markdown
    ![fenced](images/fenced-code.png)
    <source src="videos/fenced-code.mp4">
    ![fenced reference][fenced]
    [fenced]: images/fenced-reference.png
    #{BT * 3}

    <!-- <img src="images/commented-out.png"> -->
  MD
})
cases << run_index("note-relative destinations, rejects outside root", {
  "nested/page.md" => <<~MD
    ![photo](../images/photo.png)
    ![outside](../../images/outside.png)
    ![wrong root](../notes/images/wrong-root.png)
  MD
})
cases << run_index("hidden notes and symlinked markdown ignored", {
  ".hidden/hidden.md" => "![hidden](images/hidden.png)",
  "visible.md" => "![visible](images/visible.png)",
  "linked.md" => { "symlink" => "<OUTSIDE>/outside.md" }
}, outside: { "outside.md" => "![outside](images/outside.png)" })

# ---------------------------------------------------------------- extra whole-tree cases
cases << run_index("empty root", {})
cases << run_index("media files present but unreferenced", {
  "images/a.png" => "png", "images/sub/b.jpg" => "jpg", "videos/c.mp4" => "mp4", "images/x.svg" => "svg",
  "note.md" => "no refs here"
})
cases << run_index("mixed corpus with library", {
  "images/used.png" => "p", "images/unused.png" => "p", "videos/v.webm" => "v", "images/My Cat.PNG" => "p",
  "a.md" => "![u](images/used.png) ![m](images/My%20Cat.PNG)",
  "b/c.md" => "<video src='../videos/v.webm'></video> ![u](../images/used.png)",
  "d.markdown" => "![ignored ext](images/unused.png)", "e.MD" => "![upper ext](images/unused.png)",
  "f.txt" => "![txt](images/unused.png)"
})
cases << run_index("symlinked directory of notes skipped", {
  "real/n.md" => "![r](../images/r.png)", "alias" => { "symlink" => "real" }
})
cases << run_index("hidden directories at depth and dotfiles", {
  "a/.git/x.md" => "![g](../../images/git.png)", "a/.note.md" => "![d](../images/dot.png)", "a/ok.md" => "![o](../images/ok.png)"
})
cases << run_index("unicode, spaces, encoded names", {
  "日本語/ノート.md" => "![猫](../images/%E7%8C%AB.png)\n![raw](../images/猫.png)\n![space](../images/a b.png)\n![<angle>](<../images/a b.png>)",
  "שלום.md" => "![h](images/שלום.png) ![e](images/e%CC%81.png) ![nfc](images/%C3%A9.png)",
  "emoji 👨‍👩‍👧.md" => "![f](images/👨‍👩‍👧.png)"
})
cases << run_index("query strings and fragments", {
  "q.md" => "![a](images/a.png?raw=1)\n![b](images/b.png#frag)\n![c](images/c.png?x#y)\n<img src=\"images/d.png?w=10\">\n[r]\n\n[r]: images/e.png?v=2\n"
})
cases << run_index("crlf and lone cr", {
  "crlf.md" => "![a](images/crlf.png)\r\n```\r\n![b](images/in-fence.png)\r\n```\r\n![c](images/after.png)\r\n",
  "cr.md" => "![a](images/cr1.png)\r![b](images/cr2.png)\r"
})
cases << run_index("deeply nested relative paths", {
  "a/b/c/d/e.md" => "![x](../../../../images/deep.png)\n![y](../../../../../images/too-far.png)\n![z](./../../../../videos/./v.mp4)"
})
cases << run_index("invalid utf8 note is scrubbed", { "bad.md" => { "base64" => ["\xFF\xFE![x](images/ok.png)\xC3".b].pack("m0") } })

# ---------------------------------------------------------------- canonical_media_path (private)
svc = MediaUsageService.new(base_path: "/nonexistent-golden-root")
REFS = [
  "images/a.png", "videos/a.mp4", "images", "images/", "videos/", "images/sub/x.png", "/images/a.png",
  "/notes/images/a.png", "/notes/notes/images/a.png", "notes/images/a.png", "//images/a.png", "///images/a.png",
  "http://x/images/a.png", "https://example.test/notes/images/a.png", "mailto:a@b", "data:image/png;base64,AAA",
  "file:///images/a.png", "C:/images/a.png", "c:images/a.png", "x+y.z-w:foo", "1abc:images/a.png",
  "images/a.png?x=1", "images/a.png#frag", "images/a\\?b.png", "images/a\\#b.png", "images/a\\\\?b.png",
  "images/a%20b.png", "images/a%2Fb.png", "images/a%5Cb.png", "images/a%2e%2e/x.png", "images/%2e%2e/x.png",
  "images/%2E/x.png", "images/%zz.png", "images/%E7%8C%AB.png", "images/%FF.png", "images/%00.png",
  "images/./a.png", "images/../images/a.png", "images/../videos/a.mp4", "images/../a.png", "../images/a.png",
  "./images/a.png", "images//a.png", "images/a.png/", "  images/a.png  ", "\timages/a.png",
  "images/a&amp;b.png", "images/&lt;x&gt;.png", "images/&#x41;.png", "images/\\(x\\).png", "images/\\_x\\_.png",
  "images/\\a.png", "IMAGES/a.png", "Images/a.png", "imagesX/a.png", "media/a.png", "", " ", "?", "#",
  "images/日本.png", "images/e\u0301.png", "images/a b.png", "images/a\u00a0b.png", "\\images\\a.png",
  "images/a\0b.png", "/", "/notes", "/notes/", "/notes/images", "videos/clips/clip%20one.mp4"
].freeze
NOTE_PATHS = ["note.md", "docs/note.md", "a/b/note.md", "images/note.md"].freeze
REFS.each do |ref|
  NOTE_PATHS.each do |np|
    next if np != "note.md" && !(ref.start_with?("..", "./", "images/..", "images/.", "/") || ref == "images/a.png")
    cases << { "name" => "canonical_media_path #{ref.inspect} from #{np}", "fn" => "canonical_media_path",
               "input" => { "reference" => ref, "note_path" => np },
               "expected" => safe(Golden.capture { svc.send(:canonical_media_path, ref, np) }) }
  end
end

# ---------------------------------------------------------------- references_in (private) raw extraction
CONTENTS = [
  "![a](x.png)", "[a](x.png)", "![a]( x.png )", "![a](<x y.png>)", "![a](x.png \"title\")", "![a](x.png 'title')",
  "\\![a](x.png)", "!\\[a](x.png)", "[a\\]b](x.png)", "![a](x\\)y.png)", "![a](x(y).png)", "![](empty-alt.png)",
  "![a]()", "![a](<>)", "![a]\n(x.png)", "![a](\nx.png)", "[[nested]](x.png)", "![a](x.png)![b](y.png)",
  "<img src=\"a.png\">", "<img src='a.png'>", "<img src=a.png>", "<IMG SRC=\"A.png\">", "<img alt=x src=\"a.png\">",
  "<img\nsrc=\"multi.png\">", "<video src=\"v.mp4\" controls>", "<audio src=\"a.mp3\">", "<source src=\"s.webm\">",
  "<track src=\"t.vtt\">", "<embed src=\"e.swf\">", "<object src=\"o.bin\">", "<iframe src=\"i.html\">",
  "<img data-src=\"lazy.png\">", "<img srcset=\"a.png 1x\">", "<img src = \"spaced.png\">", "<imgx src=\"no.png\">",
  "[r]: a.png\n![x][r]", "[r]: <a b.png>\n![x][r]", "   [r]: indented3.png\n[r]", "    [r]: indented4.png\n[r]",
  "[R]: upper.png\n[r]", "[a &amp; b]: amp.png\n[a & b]", "[a\\*b]: esc.png\n[a*b]", "[r]: first.png\n[r]: second.png\n[r]",
  "[r]: a.png \"title\"\n[r]", "[r]:a.png\n[r]", "[r]\n[r]: after.png", "[r] (paren)\n[r]: p.png", "[r](inline.png)\n[r]: def.png",
  "[r][]\n[r]: c.png", "![r][ ]\n[r]: s.png", "[text][missing]", "`[r]`\n[r]: code.png",
  "``` \n![a](fenced.png)\n```", "~~~\n![a](tilde.png)\n~~~", "````\n```\n![a](inner.png)\n````\n![b](after.png)",
  "```\n![a](unclosed.png)", "   ```\n![a](indented-fence.png)\n   ```", "    ```\n![a](code-indent.png)",
  "``` x\n![a](a.png)\n~~~\n![b](b.png)\n```\n![c](c.png)", "`` ![a](dbl.png) ``", "`unclosed ![a](x.png)",
  "<!-- ![a](c.png) -->![b](d.png)", "<!--\n![a](multi.png)\n-->", "<!-- unclosed ![a](x.png)",
  "\\`![a](escaped-tick.png)\\`", "text ![a](a.png) `code` ![b](b.png)"
].freeze
CONTENTS.each_with_index do |content, i|
  cases << { "name" => format("references_in %02d %s", i, content.inspect[0, 50]), "fn" => "references_in",
             "input" => { "content" => content },
             "expected" => Golden.capture { svc.send(:references_in, content) } }
end
["", "a\nb", "```\nx\n```\ny", "`a` b `c`", "<!-- x -->y", "a\r\n```\r\nb\r\n```\r\nc"].each do |content|
  cases << { "name" => "markdown_without_code #{content.inspect}", "fn" => "markdown_without_code",
             "input" => { "content" => content }, "expected" => svc.send(:markdown_without_code, content) }
end
["Photo", "  PHOTO   IMAGE ", "a\\*b", "a &amp; b", "&lt;x&gt;", "Ünïcödé", "ẞ", "İ", "tab\tsep", "nbsp\u00a0x", "", "\\\\"].each do |label|
  cases << { "name" => "normalize_reference_label #{label.inspect}", "fn" => "normalize_reference_label",
             "input" => { "label" => label }, "expected" => svc.send(:normalize_reference_label, label) }
end
["a.png", "a.png?x", "a.png#y", "a\\?b", "a\\\\?b", "a\\#b#c", "?start", "", "\\", "a\\\\\\?b?c"].each do |path|
  cases << { "name" => "strip_unescaped_url_suffix #{path.inspect}", "fn" => "strip_unescaped_url_suffix",
             "input" => { "path" => path }, "expected" => svc.send(:strip_unescaped_url_suffix, path) }
end

cases << { "name" => "constants", "fn" => "constants", "input" => {},
           "expected" => { "MEDIA_ROOTS" => MediaUsageService::MEDIA_ROOTS, "MAX_CACHED_ROOTS" => MediaUsageService::MAX_CACHED_ROOTS,
                           "MARKDOWN_DESTINATION" => MediaUsageService::MARKDOWN_DESTINATION.source,
                           "REFERENCE_DEFINITION" => MediaUsageService::REFERENCE_DEFINITION.source,
                           "REFERENCE_USE" => MediaUsageService::REFERENCE_USE.source,
                           "HTML_SOURCE" => MediaUsageService::HTML_SOURCE.source } }

Golden.write("media_usage", Golden.fixture(
  source: "app/services/media_usage_service.rb", generator: "tools/golden/ruby/media_usage.rb", cases: cases,
  notes: "build_index: tree materialized at <ROOT>; symlink targets '<OUTSIDE>/x' point into a sibling dir built from outside_tree. " \
         "usage_counts = index.transform_values(&:length) (LibraryController#usage JSON). library_paths = MediaLibraryService#list " \
         "image+video paths with DEFAULT image/video upload extensions; unreferenced_library_paths = library_paths - index keys (derived). " \
         "Private helpers called via send; regex sources recorded under constants (Ruby Onigmo syntax)."
))
