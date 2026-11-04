# Golden fixtures: HugoService (slugify, generate_blog_post, render_template,
# update_frontmatter_slug, template_source, notes_root) + HUGO_PATH_STYLE env.
#
#   cd ~/Projects/FrankMD && GOLDEN_TMP=$SCRATCH/notes/misc mise exec ruby@3.4.8 -- \
#     bundle exec rails runner /mnt/data/Projects/FrankMDFast/tools/golden/ruby/hugo.rb
require_relative "../lib/golden"
require "active_support/testing/time_helpers"

include ActiveSupport::Testing::TimeHelpers

cases = []
CLEAN_ENV = { "HUGO_PATH_STYLE" => nil, "FRANKMD_LOCALE" => nil }.freeze

def enc(v) = v.is_a?(Symbol) ? { "symbol" => v.to_s } : v

# ---------------------------------------------------------------- slugify
SLUG_INPUTS = [
  # from hugo_service_test.rb
  "Hello World", "hello world", "café açaí", "hello! @world#", "hello---world", "--hello-world--",
  "Vibe Code: Fiz um Editor de Markdown do zero com Claude Code (FrankMD)",
  "My Blog Post", "Test Post", "My Post", "Config Test", 'Say "Hello" World', "Custom Post", "No Template",
  # Portuguese / Spanish / French / German
  "Ação e reação", "ÇÃO ÇÃO", "Coração Valente", "Pão de Açúcar", "Olá, Mundo!", "Avó e avô",
  "Mañana será otro día", "¿Qué pasa?", "¡Hola!", "Niño Español", "PEÑA",
  "Crème brûlée à la française", "Ça va? Où êtes-vous?", "L'été dernier", "Noël", "œuvre cœur Œuvre",
  "Grüße aus München", "Straße", "STRASSE", "STRAẞE", "Äpfel Öl Übel", "Fußgängerübergänge",
  # Polish / Czech / Turkish / Vietnamese / Nordic
  "Zażółć gęślą jaźń", "ŁÓDŹ", "Łódź", "Kraków", "Wrocław", "Gdańsk", "Szczęście",
  "Příliš žluťoučký kůň úpěl ďábelské ódy", "ČEŠTINA", "Dvořák", "Brno ů",
  "İstanbul", "Istanbul", "ılık şişe çiğ öğün", "Türkçe Ğğ Şş", "IĞDIR",
  "Tiếng Việt", "Phở bò", "Đà Nẵng", "đường", "Hà Nội", "Nguyễn",
  "Smørrebrød", "ØRESUND", "Æble æ", "Ærø", "Ålesund", "Þórsmörk", "þ Þ ð Ð", "Reykjavík", "Ísland",
  "ß", "ẞ", "ø", "æ", "þ", "Æ", "Ø", "Þ", "ÿ Ÿ ý Ý",
  # other scripts
  "日本語", "日本語のタイトル", "中文标题 test", "한국어 제목", "Привет мир", "Ελληνικά", "שלום עולם", "مرحبا بالعالم",
  "שלום abc", "abc 日本 def", "हिन्दी", "ไทย",
  # emoji
  "Hello 👋 World", "👨‍👩‍👧", "🎉🎉🎉", "rocket🚀ship", "flag 🇧🇷 brazil", "❤️ love", "1️⃣ one",
  # combining / special whitespace
  "é", "Café", "ño", "Å", "nbsp here", "tab\there", "new\nline", "cr\rline",
  "crlf\r\nline", "zero​width", "soft­hyphen", "em—dash", "en–dash", "ellipsis…",
  "fullＡwidth", "ﬁ ligature", "ǅ titlecase", "Ⅻ roman", "ⓐ circled", "² superscript", "½ half",
  # punctuation / spacing / numbers
  "", " ", "   ", "\t", "-", "---", "!!!", "@#$%^&*()", "...", "___", "a_b_c", "a.b.c", "a/b/c", "a\\b\\c",
  "multiple    spaces", "  leading and trailing  ", "-leading-dash", "trailing-dash-", "a--b", "a - b",
  "a -- b -- c", "dots.in.title.", "C++ & C#", "Node.js v20", "100%", "50/50", "2026-02-01", "1.2.3",
  "123", "007 James Bond", "Top 10: Lists!", "Q&A", "it's", "don't stop", "\"quoted\"", "'single'",
  "(parens)", "[brackets]", "{braces}", "<html>", "a=b", "x+y", "k:v", "semi;colon", "comma,separated",
  "ALLCAPS", "camelCaseTitle", "snake_case_title", "kebab-case-title", "MiXeD 123 CaSe",
  "x" * 300, ("Ação " * 60).strip, "a" + "-" * 100 + "b", "🎉" * 50,
  "../../etc/passwd", "~/.ssh", "%2e%2e", "nul\0byte",
  # non-strings
  nil, 123
].freeze

SLUG_INPUTS.each_with_index do |text, i|
  cases << { "name" => format("slugify %03d %s", i, text.inspect[0, 50]), "fn" => "slugify",
             "input" => { "text" => text },
             "expected" => Golden.capture { HugoService.slugify(text) } }
end
cases << { "name" => "slugify every ACCENT_MAP key (lower + upper)", "fn" => "slugify",
           "input" => { "text" => HugoService::ACCENT_MAP.keys.join(" ") + " " + HugoService::ACCENT_MAP.keys.join(" ").upcase },
           "expected" => HugoService.slugify(HugoService::ACCENT_MAP.keys.join(" ") + " " + HugoService::ACCENT_MAP.keys.join(" ").upcase) }
HugoService::ACCENT_MAP.each_key do |k|
  [k, k.upcase].uniq.each do |c|
    cases << { "name" => "slugify accent #{c.inspect}", "fn" => "slugify", "input" => { "text" => "x#{c}y" },
               "expected" => HugoService.slugify("x#{c}y") }
  end
end

# ---------------------------------------------------------------- render_template
RENDER = [
  [HugoService::DEFAULT_TEMPLATE, "My Post", "my-post", "2026-02-01T10:30:00+00:00"],
  [HugoService::DEFAULT_TEMPLATE, 'Say "Hello" World', "say-hello-world", "2026-02-01T10:30:00+00:00"],
  [HugoService::DEFAULT_TEMPLATE, 'back\\slash \\1 \\0 \\& \\\\', "back-slash", "D"],
  [HugoService::DEFAULT_TEMPLATE, "", "", ""],
  [HugoService::DEFAULT_TEMPLATE, "{{slug}} in title", "slug-in-title", "D"],
  [HugoService::DEFAULT_TEMPLATE, "{{date}} {{title}}", "date-title", "THE-DATE"],
  ["{{title}}{{title}}{{slug}}{{slug}}{{date}}{{date}}", "T", "s", "d"],
  ["{{ title }} {{TITLE}} {title} {{title}", "T", "s", "d"],
  ["no placeholders", "T", "s", "d"],
  ["", "T", "s", "d"],
  ["title: '{{title}}'", "it's", "its", "d"],
  ["title: \"{{title}}\"\r\nslug: {{slug}}\r\n", "CRLF \"q\"", "crlf-q", "d"],
  ["{{title}}", "日本語 \"引用\" 👨‍👩‍👧", "", "d"],
  ["{{title}}", "already \\\"escaped\\\"", "x", "d"],
  ["{{title}}", "multi\nline\ntitle", "multi-line-title", "d"],
  ["{{slug}}", "T", "\\1\\2", "d"],
  ["{{date}}", "T", "s", "\\0"]
].freeze
RENDER.each_with_index do |(tpl, title, slug, date), i|
  cases << { "name" => format("render_template %02d %s", i, title.inspect[0, 40]), "fn" => "render_template",
             "input" => { "template" => tpl, "title" => title, "slug" => slug, "date" => date },
             "expected" => Golden.capture { HugoService.render_template(tpl, title: title, slug: slug, date: date) } }
end

# ---------------------------------------------------------------- update_frontmatter_slug
UFS = [
  # from tests
  ["---\ntitle: \"Test\"\nslug: \"old-slug\"\ndate: 2026-02-01\n---\n\nContent\n", "new-slug"],
  ["Just some regular content", "new-slug"],
  ["---\ntitle: \"Test\"\ndate: 2026-02-01\n---\n\nContent\n", "new-slug"],
  ["---\ntitle: \"Test\"\nslug: \"same-slug\"\n---\n\nContent\n", "same-slug"],
  ["---\ntitle: \"Test\"\nslug: old-slug\n---\n\nContent\n", "new-slug"],
  ["---\ntitle: \"Test\"\nslug: 'old-slug'\n---\n\nContent\n", "new-slug"],
  # edge
  ["", "x"], ["---", "x"], ["---\n", "x"], ["---\nslug: a\n", "x"], ["---\nslug: a\n---", "x"],
  ["---\nslug: a\n---\n", "x"], ["---\nslug: a\n---\nbody\n---\nmore\n", "x"],
  ["---\nslug: same\n---\n", "same"], ["---\nslug: 'same'\n---\n", "same"], ["---\nslug: same\n---\n", "\"same\""],
  ["---\nslug:\n---\n", "x"], ["---\nslug: \n---\n", "x"], ["---\nslug: \"\"\n---\n", "x"], ["---\nslug: ''\n---\n", "x"],
  ["---\nslug: \"\"\n---\n", ""], ["---\nslug: a\n---\n", ""],
  ["---\nslug: \"with spaces inside\"\n---\n", "x"], ["---\nslug: with spaces inside\n---\n", "x"],
  ["---\nslug: a # comment\n---\n", "x"], ["---\nslug: \"a\" # comment\n---\n", "x"],
  ["---\n  slug: indented\n---\n", "x"], ["---\nmyslug: a\n---\n", "x"], ["---\nslugs: [a]\n---\n", "x"],
  ["---\nslug:a\n---\n", "x"], ["---\nslug:\t\"tab\"\n---\n", "x"], ["---\nslug: a\nslug: b\n---\n", "x"],
  ["---\ntitle: t\n---\nslug: in-body\n", "x"], ["---\nslug: a\n---\nslug: in-body\n", "x"],
  ["---\r\ntitle: t\r\nslug: \"crlf\"\r\n---\r\n\r\nBody\r\n", "x"],
  ["---\r\nslug: crlf\r\n---\r\n", "x"], ["---\rslug: cr\r---\r", "x"],
  ["+++\ntitle = \"t\"\nslug = \"toml\"\n+++\n\nBody\n", "x"],
  ["+++\nslug: \"weird\"\n+++\n", "x"],
  ["----\nslug: a\n----\n", "x"], ["---  \nslug: a\n---  \nbody", "x"], ["--- \nslug: a\n---\t\nbody", "x"],
  ["---yaml\nslug: a\n---\n", "x"], [" ---\nslug: a\n---\n", "x"], ["\n---\nslug: a\n---\n", "x"],
  ["﻿---\nslug: a\n---\n", "x"],
  ["---\nslug: a\n---\n", "new \"quoted\" slug"], ["---\nslug: a\n---\n", "back\\1slash"],
  ["---\nslug: a\n---\n", "日本語"], ["---\nslug: 日本語\n---\n", "x"], ["---\nslug: \"emoji-👨‍👩‍👧\"\n---\n", "x"],
  ["---\ntitle: \"slug: inline\"\n---\n", "x"], ["---\ntags:\n  - slug: nested\n---\n", "x"],
  ["---\nslug: \"unterminated\n---\n", "x"], ["---\nslug: 'mixed\"\n---\n", "x"],
  ["---\nslug: \"a\"\"b\"\n---\n", "x"],
  ["---\n---\n", "x"], ["------\n", "x"], ["---\nslug: a\n---", "a"],
  [nil, "x"]
].freeze
UFS.each_with_index do |(content, new_slug), i|
  cases << { "name" => format("update_frontmatter_slug %02d %s", i, content.inspect[0, 40]), "fn" => "update_frontmatter_slug",
             "input" => { "content" => content, "new_slug" => new_slug },
             "expected" => Golden.capture { HugoService.update_frontmatter_slug(content, new_slug) } }
end

# ---------------------------------------------------------------- template_source / notes_root / generate_blog_post
CUSTOM_TPL = "---\ntitle: \"{{title}}\"\nslug: \"{{slug}}\"\ndate: {{date}}\ndraft: false\nauthor: \"Jane\"\ntags:\n  - uncategorized\n---\n"

def with_root(tree)
  root = Golden.mktmp("hugo")
  Golden.build_tree(root, tree.reject { |_, v| v.is_a?(Hash) && v["base64"] })
  tree.each { |rel, v| File.binwrite(File.join(root, rel), v["base64"].unpack1("m0")) if v.is_a?(Hash) && v["base64"] }
  Golden.with_env(CLEAN_ENV.merge("NOTES_PATH" => root)) { yield root }
ensure
  FileUtils.rm_rf(root) if root
end

TS_TREES = {
  "no template" => {},
  "custom .hugo_template.md" => { ".hugo_template.md" => CUSTOM_TPL },
  "empty .hugo_template.md" => { ".hugo_template.md" => "" },
  "template is a directory" => { ".hugo_template.md/" => nil },
  "template symlink to file in root" => { "real.md" => "LINKED {{title}}", ".hugo_template.md" => { "symlink" => "real.md" } },
  "dangling template symlink" => { ".hugo_template.md" => { "symlink" => "missing.md" } },
  "visible hugo_template.md (no dot) ignored" => { "hugo_template.md" => "NODOT {{title}}" },
  "uppercase .HUGO_TEMPLATE.md ignored" => { ".HUGO_TEMPLATE.md" => "UPPER {{title}}" },
  ".hugo_template.toml ignored" => { ".hugo_template.toml" => "TOML" },
  "template in subdir ignored" => { "content/.hugo_template.md" => "SUB" },
  "crlf template" => { ".hugo_template.md" => "+++\r\ntitle = \"{{title}}\"\r\n+++\r\n" },
  "invalid utf8 template" => { ".hugo_template.md" => { "base64" => ["bad \xFF\xFE {{title}}".b].pack("m0") } }
}.freeze

TS_TREES.each do |label, tree|
  with_root(tree) do |root|
    cases << { "name" => "template_source #{label}", "fn" => "template_source",
               "input" => { "tree" => tree, "base_path" => nil, "env" => { "NOTES_PATH" => "<ROOT>" } },
               "expected" => Golden.normalize(Golden.capture { s = HugoService.template_source; s.valid_encoding? ? s : { "base64" => [s].pack("m0"), "encoding" => s.encoding.to_s } }, root) }
  end
end
with_root({}) do |root|
  other = Golden.mktmp("hugo_other")
  File.write(File.join(other, ".hugo_template.md"), "OTHER {{title}}")
  cases << { "name" => "template_source explicit base_path overrides NOTES_PATH", "fn" => "template_source",
             "input" => { "tree" => {}, "base_path" => "<OTHER>", "other_tree" => { ".hugo_template.md" => "OTHER {{title}}" } },
             "expected" => HugoService.template_source(base_path: other) }
  cases << { "name" => "notes_root explicit", "fn" => "notes_root", "input" => { "base_path" => "/x/y", "env" => { "NOTES_PATH" => "<ROOT>" } },
             "expected" => HugoService.send(:notes_root, "/x/y").to_s }
  cases << { "name" => "notes_root from NOTES_PATH", "fn" => "notes_root", "input" => { "base_path" => nil, "env" => { "NOTES_PATH" => "<ROOT>" } },
             "expected" => Golden.normalize(HugoService.send(:notes_root).to_s, root) }
  FileUtils.rm_rf(other)
end
Golden.with_env("NOTES_PATH" => nil) do
  cases << { "name" => "notes_root default when NOTES_PATH unset (Rails.root/notes)", "fn" => "notes_root",
             "input" => { "base_path" => nil, "env" => { "NOTES_PATH" => nil } },
             "expected" => HugoService.send(:notes_root).to_s.sub(Rails.root.to_s, "<RAILS_ROOT>") }
end
Golden.with_env("NOTES_PATH" => "") do
  cases << { "name" => "notes_root NOTES_PATH empty string", "fn" => "notes_root",
             "input" => { "base_path" => nil, "env" => { "NOTES_PATH" => "" } },
             "expected" => HugoService.send(:notes_root).to_s }
end

PIN = { "zone" => "UTC", "time" => "2026-02-01T10:30:00" }.freeze
ZONES = [["UTC", [2026, 2, 1, 10, 30, 0]], ["America/Sao_Paulo", [2026, 2, 1, 10, 30, 0]],
         ["Asia/Kolkata", [2026, 12, 31, 23, 59, 59]], ["Pacific/Chatham", [2026, 7, 4, 1, 2, 3]],
         ["America/New_York", [2026, 1, 9, 0, 0, 0]]].freeze

GEN = [
  # [title, kwargs, tree(.fed etc), env]
  ["My Blog Post", {}, {}, {}],
  ["Test Post", {}, {}, {}],
  ["My Post", { parent: "content/posts" }, {}, {}],
  ["My Blog Post", { path_style: "flat" }, {}, {}],
  ["My Post", { parent: "content/posts", path_style: "flat" }, {}, {}],
  ["My Post", { path_style: "dated" }, {}, {}],
  ["Config Test", {}, { ".fed" => "hugo_path_style = flat\n" }, {}],
  ["Config Test", {}, { ".fed" => "hugo_path_style = dated\n" }, {}],
  ["Config Test", {}, {}, { "HUGO_PATH_STYLE" => "flat" }],
  ["Config Test", {}, { ".fed" => "hugo_path_style = dated\n" }, { "HUGO_PATH_STYLE" => "flat" }],
  ["Config Test", {}, {}, { "HUGO_PATH_STYLE" => "" }],
  ["Config Test", {}, {}, { "HUGO_PATH_STYLE" => "FLAT" }],
  ["Config Test", {}, {}, { "HUGO_PATH_STYLE" => " flat " }],
  ["Config Test", {}, { ".fed" => "hugo_path_style = nested\n" }, {}],
  ["Config Test", {}, { ".fed" => "hugo_path_style =\n" }, {}],
  ["Config Test", { path_style: "dated" }, { ".fed" => "hugo_path_style = flat\n" }, {}],
  ["Config Test", { path_style: "" }, { ".fed" => "hugo_path_style = flat\n" }, {}],
  ["Config Test", { path_style: "weird" }, {}, {}],
  ['Say "Hello" World', {}, {}, {}],
  ["Custom Post", {}, { ".hugo_template.md" => CUSTOM_TPL }, {}],
  ['Say "Hi"', {}, { ".hugo_template.md" => "title: \"{{title}}\"\n" }, {}],
  ["No Template", {}, {}, {}],
  ["", {}, {}, {}],
  ["!!!", { path_style: "flat" }, {}, {}],
  ["日本語", {}, {}, {}],
  ["Ação Rápida", { parent: "blog" }, {}, {}],
  ["X", { parent: "" }, {}, {}],
  ["X", { parent: "   " }, {}, {}],
  ["X", { parent: "a/" }, {}, {}],
  ["X", { parent: "/abs" }, {}, {}],
  ["X", { parent: "../escape" }, {}, {}],
  ["back\\slash \\1", {}, {}, {}],
  ["tpl uses {{slug}}", {}, { ".hugo_template.md" => "{{title}}|{{slug}}|{{date}}" }, {}]
].freeze

GEN.each_with_index do |(title, kw, tree, env), i|
  with_root(tree) do |root|
    Golden.with_env(env) do
      Time.use_zone(PIN["zone"]) do
        travel_to(Time.zone.local(2026, 2, 1, 10, 30, 0)) do
          result = Golden.capture { HugoService.generate_blog_post(title, **kw) }
          cases << { "name" => format("generate_blog_post %02d %s %s", i, title.inspect[0, 30], kw.inspect),
                     "fn" => "generate_blog_post",
                     "input" => { "title" => title, "parent" => kw[:parent], "path_style" => kw[:path_style],
                                  "kwargs_given" => kw.keys.map(&:to_s), "tree" => tree, "env" => env,
                                  "now" => PIN },
                     "expected" => { "result" => Golden.normalize(result, root), "tree_after" => Golden.snapshot(root) } }
        end
      end
    end
  end
end

ZONES.each do |zone, parts|
  with_root({}) do |root|
    Time.use_zone(zone) do
      travel_to(Time.zone.local(*parts)) do
        cases << { "name" => "generate_blog_post time zone #{zone}", "fn" => "generate_blog_post",
                   "input" => { "title" => "Zoned", "parent" => nil, "path_style" => nil, "kwargs_given" => [], "tree" => {}, "env" => {},
                                "now" => { "zone" => zone, "time" => format("%04d-%02d-%02dT%02d:%02d:%02d", *parts) } },
                   "expected" => { "result" => Golden.normalize(HugoService.generate_blog_post("Zoned"), root), "tree_after" => Golden.snapshot(root) } }
      end
    end
  end
end

cases << { "name" => "constants", "fn" => "constants", "input" => {},
           "expected" => { "TEMPLATE_FILE" => HugoService::TEMPLATE_FILE, "DEFAULT_TEMPLATE" => HugoService::DEFAULT_TEMPLATE,
                           "ACCENT_MAP" => HugoService::ACCENT_MAP,
                           "config_schema_hugo_path_style" => Config::SCHEMA["hugo_path_style"].transform_keys(&:to_s).transform_values { |v| v.is_a?(Symbol) ? v.to_s : v } } }

Golden.write("hugo", Golden.fixture(
  source: "app/services/hugo_service.rb", generator: "tools/golden/ruby/hugo.rb", cases: cases,
  notes: "Clock pinned with travel_to (input.now = {zone, local time}; Time.current in that zone). " \
         "NOTES_PATH points at a temp dir (<ROOT>) holding input.tree; HUGO_PATH_STYLE/FRANKMD_LOCALE cleared unless in input.env. " \
         "generate_blog_post result keys are symbols in Ruby (path, content); tree_after shows the call writes nothing. " \
         "kwargs_given lists which keyword args were passed (parent/path_style null otherwise). " \
         "notes_root default with NOTES_PATH unset uses <RAILS_ROOT>/notes. Non-UTF8 template_source returned as {base64, encoding}."
))
