# Golden fixtures: NotesService#search_content (+ GET /notes/search).
require_relative "notes_fs_common"

cases = []
M = NotesFS::BASE_MTIME
def f(c, m) = { "content" => c, "mtime" => m }

def search_case(cases, name, tree, query, opts = {}, note: nil, outside: {})
  NotesFS.sandbox(tree, outside: outside) do |root, _|
    yield root if block_given?
    svc = NotesService.new(base_path: root)
    kw = opts.transform_keys(&:to_sym)
    t0 = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    res = NotesFS.capture { NotesFS.json(svc.search_content(query, **kw)) }
    c = { "name" => name, "fn" => "NotesService#search_content",
          "input" => { "tree" => tree, "outside" => outside, "query" => query, "options" => opts },
          "expected" => NotesFS.norm(res, root) }
    c["notes"] = note if note
    cases << c
  end
end

BASIC = { "note1.md" => f("Hello world\nThis is a test", M), "note2.md" => f("Another file\nWith different content", M - 100) }.freeze
search_case(cases, "blank query", { "note.md" => "Some content" }, "")
search_case(cases, "nil query", { "note.md" => "Some content" }, nil)
search_case(cases, "whitespace-only query is blank", { "note.md" => "a b" }, "   ")
search_case(cases, "tab query is blank", { "note.md" => "a\tb" }, "\t")
search_case(cases, "NBSP query is blank (ActiveSupport blank?)", { "note.md" => "a b" }, " ")
search_case(cases, "newline query is blank", { "note.md" => "a\nb" }, "\n")
search_case(cases, "finds text", BASIC, "world")
search_case(cases, "case insensitive", { "note.md" => "Hello World" }, "hello world")
search_case(cases, "case insensitive uppercase query", { "note.md" => "hello world" }, "HELLO")
search_case(cases, "regex pattern", { "note.md" => "foo123bar\nfoo456bar\nhello world" }, "foo\\d+bar")
search_case(cases, "regex alternation", { "n.md" => "cat\ndog\nbird" }, "cat|bird")
search_case(cases, "regex anchors", { "n.md" => "start mid\nmid start\nstart" }, "^start$")
search_case(cases, "regex dot matches every line incl. empty? (no)", { "n.md" => "a\n\nb\n" }, ".")
search_case(cases, "empty-line regex", { "n.md" => "a\n\nb\n\n" }, "^$")
search_case(cases, "lookahead regex", { "n.md" => "foobar\nfoobaz" }, "foo(?=baz)")
search_case(cases, "inline flags regex", { "n.md" => "ABC\nabc" }, "(?-i)abc")
search_case(cases, "word boundary", { "n.md" => "cat\ncatalog\nthe cat sat" }, "\\bcat\\b")
search_case(cases, "invalid regex brackets escaped", { "note.md" => "test [brackets]" }, "[brackets")
search_case(cases, "invalid regex lone paren escaped", { "n.md" => "f(x) = 1\nnone" }, "f(")
search_case(cases, "invalid regex star escaped", { "n.md" => "a*b\nab" }, "*")
search_case(cases, "invalid regex trailing backslash escaped", { "n.md" => "path\\to\nother" }, "\\")
search_case(cases, "valid regex metachar dot used literally by user", { "n.md" => "v1.2\nv1x2" }, "v1.2")
search_case(cases, "question mark (valid? no -> escaped)", { "n.md" => "what?\nwhat" }, "?")
search_case(cases, "plus sign alone (invalid -> escaped)", { "n.md" => "a+b\nab" }, "+")
search_case(cases, "brace quantifier literal", { "n.md" => "a{2}\naa" }, "a{2}")
search_case(cases, "regex timeout pattern returns no matches for that file",
            { "n.md" => "#{'a' * 40}!" }, "(a|a)+$", note: "Catastrophic backtracking hits the 0.5s Regexp timeout; search_file rescues and returns [] for the file.")
search_case(cases, "unicode case folding (ção/ÇÃO)", { "n.md" => "AÇÃO\nação\nacao" }, "ação")
search_case(cases, "accents are not folded", { "n.md" => "café\ncafe\nCAFÉ" }, "cafe")
search_case(cases, "combining vs precomposed not equivalent", { "n.md" => "étude\nétude" }, "étude")
search_case(cases, "German sharp s / ẞ", { "n.md" => "straße\nSTRASSE\nSTRAẞE" }, "straße")
search_case(cases, "CJK", { "n.md" => "日本語のテキスト\n中文\n한국어" }, "日本")
search_case(cases, "Hebrew RTL", { "n.md" => "שלום עולם\nhello" }, "שלום")
search_case(cases, "emoji ZWJ sequence", { "n.md" => "family 👨‍👩‍👧 here\n👨 alone" }, "👨‍👩‍👧")
search_case(cases, "emoji partial (single codepoint inside ZWJ)", { "n.md" => "family 👨‍👩‍👧 here\nnone" }, "👩")
search_case(cases, "Turkish dotted I", { "n.md" => "İstanbul\nistanbul\nISTANBUL" }, "istanbul")
search_case(cases, "context lines (default 3)", { "note.md" => "Line 1\nLine 2\nLine 3 MATCH\nLine 4\nLine 5" }, "MATCH", { "context_lines" => 2 })
search_case(cases, "context clipped at file start/end", { "n.md" => "MATCH first\nmid\nMATCH last" }, "MATCH")
search_case(cases, "context_lines 0", { "n.md" => "a\nMATCH\nb" }, "MATCH", { "context_lines" => 0 })
search_case(cases, "context_lines 10 larger than file", { "n.md" => "a\nMATCH\nb" }, "MATCH", { "context_lines" => 10 })
search_case(cases, "context_lines negative", { "n.md" => "a\nb\nMATCH\nc\nd" }, "MATCH", { "context_lines" => -1 })
search_case(cases, "overlapping contexts for adjacent matches", { "n.md" => "x\nhit1\nhit2\ny" }, "hit")
search_case(cases, "CRLF lines are chomped", { "n.md" => "first\r\nsecond hit\r\nthird\r\n" }, "hit")
search_case(cases, "lone CR is not a line break", { "n.md" => "first\rsecond hit\rthird" }, "hit")
search_case(cases, "trailing blank lines", { "n.md" => "hit\n\n\n" }, "hit", { "context_lines" => 5 })
search_case(cases, "no trailing newline", { "n.md" => "a\nhit" }, "hit")
search_case(cases, "tabs and NBSP preserved in match_text", { "n.md" => "\tindented hit nbsp  " }, "hit")
search_case(cases, "BOM is part of first line", { "n.md" => "﻿hit at start" }, "^hit")
search_case(cases, "BOM line matched by content", { "n.md" => "﻿hit at start" }, "hit")
search_case(cases, "max_results respected within one file", { "n.md" => (1..10).map { |i| "findme #{i}" }.join("\n") }, "findme", { "max_results" => 3 })
search_case(cases, "max_results 0", { "n.md" => "findme" }, "findme", { "max_results" => 0 })
search_case(cases, "default max_results 50 with 60 matches", { "n.md" => (1..60).map { |i| "findme #{i}" }.join("\n") }, "findme", { "context_lines" => 0 })
search_case(cases, "max_results across files (which files are visited first follows readdir order)",
            (0...5).to_h { |i| ["note#{i}.md", f("findme", M - i * 10)] }, "findme", { "max_results" => 3 },
            note: "Traversal order is Dir.children (readdir) order and stops at max_results; the SET of files returned is filesystem-dependent. " \
                  "Only the count (3) and the mtime-desc ordering of what is returned are contractual.")
search_case(cases, "results sorted by file mtime desc", { "old.md" => f("hit old", M - 1000), "new.md" => f("hit new", M), "mid.md" => f("hit mid", M - 500) }, "hit")
search_case(cases, "multiple matches in same file keep line order", { "a.md" => f("hit 1\nno\nhit 3\nhit 4", M), "b.md" => f("hit b", M + 5) }, "hit")
search_case(cases, "nested folders", { "folder/nested.md" => "find this text", "a/b/c/d.md" => "find deep" }, "find")
search_case(cases, "hidden files/dirs skipped", { ".hidden.md" => "findme", ".dir/x.md" => "findme", "v.md" => "findme" }, "findme")
search_case(cases, "non-md and .fed skipped", { "a.txt" => "findme", ".fed" => "findme = 1", "b.MD" => "findme", "c.markdown" => "findme", "d.md" => "findme" }, "findme")
search_case(cases, "root images/ folder IS searched", { "images/notes.md" => "findme", "videos/x.md" => "findme" }, "findme")
search_case(cases, "symlinked dir and file skipped", { "real.md" => "findme", "alias.md" => { "symlink" => "real.md" }, "ext" => { "symlink" => "<OUTSIDE>" } },
            "findme", outside: { "external.md" => "findme" })
search_case(cases, "invalid UTF-8 scrubbed (good file still found)", { "bad.md" => { "base64" => ["hello \xFF\xFE world\n".b].pack("m0") }, "good.md" => "hello world" }, "world")
search_case(cases, "invalid UTF-8 scrubbed and matched", { "bad.md" => { "base64" => ["target_word \xFF here\n".b].pack("m0") } }, "target_word")
search_case(cases, "file over 5MB skipped", { "small.md" => f("findme", M), "large.md" => f("findme", M - 1) }, "findme") do |root|
  File.truncate(File.join(root, "large.md"), NotesService::MAX_SEARCH_FILE_BYTES + 1) # NUL padded
  File.utime(M - 1, M - 1, File.join(root, "large.md"))
end
search_case(cases, "file exactly 5MB searched (findme + newline padding to 5*1024*1024 bytes)", { "edge.md" => f("findme", M) }, "findme", { "context_lines" => 0 }) do |root|
  File.binwrite(File.join(root, "edge.md"), "findme\n" + "\n" * (NotesService::MAX_SEARCH_FILE_BYTES - 7))
  File.utime(M, M, File.join(root, "edge.md"))
end
search_case(cases, "name/path for unicode file names", { "日本語/ノート.md" => "hit", "with space.md" => "hit 2", "x.y.md" => "hit 3" }, "hit")
search_case(cases, "very long line", { "n.md" => "#{'x' * 5000}needle#{'y' * 5000}" }, "needle")
search_case(cases, "match_text is full line", { "n.md" => "prefix needle suffix" }, "needle")
search_case(cases, "query with surrounding spaces is used as-is", { "n.md" => "a needle b\nneedle" }, " needle ")
search_case(cases, "empty file", { "e.md" => "" }, "x")
search_case(cases, "mtime serialization (fractional and pre-epoch)", { "a.md" => f("hit", M + 0.75), "b.md" => f("hit", -86_400) }, "hit")
search_case(cases, "no results", BASIC, "nonexistent_xyz")

# HTTP endpoint: max_results 20, context 3; JSON and HTML partial
HT = (0...25).to_h { |i| ["n%02d.md" % i, f("findme #{i}", M - i)] }.merge("one.md" => f("single <b>hit</b> & \"q\"", M + 100)).freeze
[["findme", "application/json"], ["<b>hit</b>", "application/json"], ["<b>hit</b>", "text/html"], ["", "application/json"]].each do |q, accept|
  NotesFS.sandbox(HT) do |root, _|
    Golden.with_env("NOTES_PATH" => root) do
      r = NotesFS.http(:get, "/notes/search", params: { q: q }, headers: { "Accept" => accept })
      if q == "findme" && r["body"].is_a?(Array)
        r = r.merge("count" => r["body"].size)
        r["body"] = "<omitted: readdir-order dependent subset>"
      end
      cases << { "name" => "GET /notes/search q=#{q.inspect} accept=#{accept}", "fn" => "http",
                 "input" => { "tree" => HT.keys.size > 10 ? "26 files n00..n24.md ('findme i', mtime #{M}-i) + one.md" : HT, "query" => q, "accept" => accept },
                 "expected" => r }
    end
  end
end

Golden.write("search", Golden.fixture(
  source: "app/services/notes_service.rb#search_content/search_file, app/controllers/notes_controller.rb#search",
  generator: NotesFS.gen_path("search.rb"),
  notes: "expected = JSON of the results array as rendered by `render json:` (keys line_number, match_text, context[{line_number,content,is_match}], path, name, mtime). " \
         "mtime is a Ruby Time serialized by ActiveSupport (TZ=UTC; ISO8601 with milliseconds). File mtimes pinned via File.utime ({content,mtime} or #{M}-100*i in spec order). " \
         "Query compiled as Regexp(query, IGNORECASE, timeout 0.5s); RegexpError -> Regexp.escape(query). Blank (ActiveSupport blank?, incl. NBSP) -> []. " \
         "Lines = File#readlines(chomp: true).map(&:scrub) (\\n and \\r\\n separators; lone \\r is not a separator). Traversal stops once max_results reached " \
         "(readdir order, unsorted), final sort by -mtime.to_i (Ruby sort_by; ties keep no guaranteed order).",
  cases: cases))
