# frozen_string_literal: true

# Golden fixtures for the .fed configuration (app/models/config.rb), plus
# EgressPolicy and CustomTheme. Run:
#   cd ~/Projects/FrankMD && GOLDEN_TMP=$SCRATCH/notes/config \
#     mise exec ruby@3.4.8 -- bundle exec rails runner /mnt/data/Projects/FrankMDFast/tools/golden/ruby/config.rb
#
# Produces tests/fixtures/golden/{fed_template.fed, config_template.json,
# config_parse.json, config_env.json, config_writeback.json,
# egress_policy.json, custom_theme.json}.
require_relative "../lib/golden"
require "base64"
require "stringio"

GEN = "tools/golden/ruby/config.rb"
ENV_KEYS = Config::SCHEMA.values.map { |s| s[:env] }.compact.uniq.freeze
OTHER_ENV = %w[NOTES_PATH XDG_PICTURES_DIR].freeze
SCRUB = (ENV_KEYS + OTHER_ENV).to_h { |k| [ k, nil ] }.freeze
# The images initializer may have populated IMAGES_PATH at boot; start clean.
SCRUB.each_key { |k| ENV.delete(k) }

# ---------------------------------------------------------------- helpers

def with_clean_env(env = {}, &blk)
  Golden.with_env(SCRUB.merge(env || {}), &blk)
end

# Capture Rails.logger output (info and above) emitted inside the block.
def with_log
  io = StringIO.new
  old = Rails.logger
  logger = ActiveSupport::Logger.new(io)
  logger.level = :info
  logger.formatter = proc { |sev, _t, _p, msg| "#{sev}: #{msg}\n" }
  Rails.logger = logger
  result = yield
  # keep only "SEV: message" lines; drop backtrace lines (absolute source paths)
  lines = io.string.lines.map(&:chomp).select { |l| l.match?(/\A(DEBUG|INFO|WARN|ERROR|FATAL): (?!\/)/) }
                         .map { |l| l.gsub(/\.fed\.[0-9a-f]{12}\.tmp/, ".fed.<HEX>.tmp") }
  [ result, lines ]
ensure
  Rails.logger = old
end

def mode_of(path)
  format("%o", File.lstat(path).mode & 0o777)
rescue SystemCallError
  nil
end

def modes(root)
  Dir.glob("**/*", File::FNM_DOTMATCH, base: root).sort.each_with_object({}) do |rel, h|
    next if %w[. ..].include?(File.basename(rel))
    path = File.join(root, rel)
    next if File.symlink?(path)
    h[rel] = mode_of(path)
  end
end

def read_fed(dir)
  path = File.join(dir, ".fed")
  return nil unless File.file?(path)
  data = File.binread(path)
  utf = data.dup.force_encoding("UTF-8")
  utf.valid_encoding? ? utf : { "base64" => [ data ].pack("m0") }
end

def write_fed_input(dir, input)
  if input.key?("fed")
    File.binwrite(File.join(dir, ".fed"), input["fed"])
  elsif input.key?("fed_base64")
    File.binwrite(File.join(dir, ".fed"), Base64.strict_decode64(input["fed_base64"]))
  end
  Golden.build_tree(dir, input["tree"]) if input["tree"]
end

def file_values(cfg)
  cfg.values.to_h { |k, v| [ k, v ] }
end

def resolved(cfg)
  Config::SCHEMA.keys.to_h { |k| [ k, cfg.get(k) ] }
end

AI_KEYS = %w[ai_provider ai_model ollama_api_base ollama_model openrouter_api_key openrouter_model
             anthropic_api_key anthropic_model gemini_api_key gemini_model openai_api_key openai_model].freeze

def full_snapshot(cfg)
  logs = LogsController.new
  {
    "file_values" => file_values(cfg),
    "resolved" => resolved(cfg),
    "sources" => Config::SCHEMA.to_h { |k, s| [ k, logs.send(:config_source, cfg, k, s) ] },
    "get_ai" => AI_KEYS.to_h { |k| [ k, cfg.get_ai(k) ] },
    "ai_configured_in_file" => cfg.ai_configured_in_file?,
    "ai_providers_available" => cfg.ai_providers_available,
    "effective_ai_provider" => cfg.effective_ai_provider,
    "effective_ai_model" => cfg.effective_ai_model,
    "features" => %w[s3_upload youtube_search google_search local_images ai bogus].to_h { |f| [ f, cfg.feature_available?(f) ] },
    "all_settings_masked" => cfg.all_settings,
    "upload_extensions" => {
      "image_upload_extensions" => cfg.upload_extensions("image_upload_extensions"),
      "video_upload_extensions" => cfg.upload_extensions("video_upload_extensions")
    },
    "get_unknown_key" => cfg.get("bogus")
  }
end

def norm(obj, root) = Golden.normalize(obj, root)

$template_text = nil

# ---------------------------------------------------------------- template

def gen_template
  cases = []
  # canonical template
  dir = Golden.mktmp("tpl")
  with_clean_env { Config.new(base_path: dir) }
  $template_text = File.read(File.join(dir, ".fed"))
  File.write(File.join(Golden::OUT_DIR, "fed_template.fed"), $template_text)
  warn "wrote fed_template.fed (#{$template_text.bytesize} bytes)"
  FileUtils.rm_rf(dir)

  all_env = ENV_KEYS.to_h { |k| [ k, "env-#{k.downcase}" ] }
  scenarios = [
    { "name" => "fresh empty dir, no env", "env" => {}, "tree" => {} },
    { "name" => "fresh empty dir, every schema ENV var set (template is static)", "env" => all_env, "tree" => {} },
    { "name" => "fresh dir with ENV booleans/locale set", "env" => { "FRANKMD_LOCALE" => "pt-BR", "AWS_S3_FORCE_PATH_STYLE" => "true", "IMAGES_PATH" => "/pics" }, "tree" => {} },
    { "name" => "dir with existing notes but no .fed", "env" => {}, "tree" => { "a.md" => "# A\n", "sub/b.md" => "b", ".hidden/" => nil } },
    { "name" => "nested non-existent base path is created", "env" => {}, "tree" => {}, "subpath" => "nested/deeply/path" },
    { "name" => "base_path nil uses NOTES_PATH env", "env" => {}, "tree" => {}, "via_notes_path" => true },
    { "name" => "NOTES_PATH set to empty string resolves relative to cwd", "env" => {}, "tree" => {}, "empty_notes_path" => true },
    { "name" => ".fed is a directory (no template written)", "env" => {}, "tree" => { ".fed/" => nil } },
    { "name" => ".fed is a dangling symlink (replaced by regular file)", "env" => {}, "tree" => { ".fed" => { "symlink" => "missing-target.fed" } } },
    { "name" => ".fed is a symlink to an existing file outside (read through; upgrade rewrites as regular file)", "env" => {}, "tree" => { "real/config.txt" => "theme = dark\n", ".fed" => { "symlink" => "real/config.txt" } } },
    { "name" => "read-only notes dir (cannot create .fed or lock)", "env" => {}, "tree" => {}, "readonly" => true },
    { "name" => "lock file path is a symlink (NOFOLLOW -> ELOOP)", "env" => {}, "tree" => { "elsewhere.lock" => "", ".frankmd-config.lock" => { "symlink" => "elsewhere.lock" } } }
  ]
  scenarios.each do |sc|
    root = Golden.mktmp("tpl")
    Golden.build_tree(root, sc["tree"])
    base = sc["subpath"] ? File.join(root, sc["subpath"]) : root
    File.chmod(0o555, root) if sc["readonly"]
    env = sc["env"].dup
    cfg, log = with_log do
      if sc["via_notes_path"]
        with_clean_env(env.merge("NOTES_PATH" => base)) { c = Config.new; [ c, c.config_file_path.to_s, resolved(c) ] }
      elsif sc["empty_notes_path"]
        Dir.chdir(root) { with_clean_env(env.merge("NOTES_PATH" => "")) { c = Config.new; [ c, c.config_file_path.to_s, resolved(c) ] } }
      else
        with_clean_env(env) { c = Config.new(base_path: base); [ c, c.config_file_path.to_s, resolved(c) ] }
      end
    end
    File.chmod(0o755, root) if sc["readonly"]
    c, cfg_path, res = cfg
    fed = read_fed(base)
    cases << norm({
      "name" => sc["name"], "fn" => "Config.new (template creation)",
      "input" => { "env" => env, "tree" => sc["tree"], "base_path" => sc["subpath"] ? "<ROOT>/#{sc["subpath"]}" : (sc["via_notes_path"] ? nil : (sc["empty_notes_path"] ? "" : "<ROOT>")),
                   "notes_path_env" => sc["via_notes_path"] ? "<ROOT>" : (sc["empty_notes_path"] ? "" : nil),
                   "cwd" => sc["empty_notes_path"] ? "<ROOT>" : nil, "readonly_root" => !!sc["readonly"] },
      "expected" => {
        "config_file_path" => cfg_path,
        "fed_content" => fed,
        "fed_equals_template" => fed == $template_text,
        "file_values" => file_values(c),
        "resolved_sample" => res.slice("theme", "locale", "editor_font", "images_path", "aws_s3_force_path_style"),
        "tree_after" => Golden.snapshot(root).transform_values { |v| v == $template_text ? "<TEMPLATE>" : v },
        "modes" => modes(root),
        "log" => log
      }
    }, root)
    FileUtils.rm_rf(root)
  end

  ai_section = Config::TEMPLATE_SECTIONS.find { |s| s[:marker] == "# AI/LLM" }[:lines].join("\n")
  upgrades = {
    "old config without AI section (from test)" => "# FrankMD Configuration\ntheme = gruvbox\neditor_font_size = 16\n\n# Local Images\n# images_path = /path/to/images\n",
    "custom comments preserved (from test)" => "# My custom header comment\ntheme = tokyo-night\n\n# My custom comment about images\nimages_path = /my/images\n",
    "AI marker present (from test)" => "# FrankMD Configuration\ntheme = dark\n\n# AI/LLM (for grammar checking)\nopenai_api_key = sk-existing\n",
    "empty file" => "",
    "only newline" => "\n",
    "only blank lines" => "\n\n\n",
    "no trailing newline" => "theme = dark",
    "trailing blank line already" => "theme = dark\n\n",
    "trailing whitespace-only last line" => "theme = dark\n   ",
    "CRLF file without AI" => "theme = dark\r\nvim_mode = true\r\n",
    "CRLF file with AI key" => "theme = dark\r\nai_model = x\r\n",
    "commented ai_model placeholder suppresses upgrade" => "theme = dark\n# ai_model = placeholder\n",
    "commented without space #ai_provider" => "#ai_provider=auto\n",
    "double hash ## ai_model does not count" => "## ai_model = x\n",
    "indented ai key counts" => "   ai_model = x\n",
    "indented comment '  # ai_model' does not count" => "  # ai_model = x\n",
    "uppercase AI key counts" => "OPENAI_API_KEY = sk\n",
    "image_generation_model is not an AI marker" => "image_generation_model = foo\n",
    "openai_model counts" => "openai_model = gpt\n",
    "marker text anywhere in line counts" => "theme = dark # AI/LLM\n",
    "ai_model without = does not count" => "# ai_model\n",
    "ai key prefix ai_models does not count" => "ai_models = x\n",
    "full template is unchanged" => :template,
    "template minus AI section gets it back" => :template_no_ai,
    "lone CR line endings" => "theme = dark\rvim_mode = true\r",
    "unicode content" => "# Configuração 日本語 שלום 👨‍👩‍👧\ntheme = ção\n"
  }
  upgrades.each do |name, content|
    content = $template_text if content == :template
    if content == :template_no_ai
      content = $template_text.sub("\n" + ai_section + "\n", "\n")
    end
    root = Golden.mktmp("upg")
    File.binwrite(File.join(root, ".fed"), content)
    File.chmod(0o644, File.join(root, ".fed"))
    (c, log) = with_log { with_clean_env { Config.new(base_path: root) } }
    after = read_fed(root)
    cases << norm({
      "name" => "upgrade: #{name}", "fn" => "upgrade_config_file (via Config.new)",
      "input" => { "fed" => content, "fed_mode" => "644" },
      "expected" => {
        "fed_after" => after,
        "changed" => after != content,
        "appended" => (after != content && after.is_a?(String) && after.start_with?(content.lines.map(&:chomp).join("\n"))) ? after.delete_prefix(content.lines.map(&:chomp).join("\n")) : nil,
        "fed_mode_after" => mode_of(File.join(root, ".fed")),
        "file_values" => file_values(c),
        "log" => log
      }
    }, root)
    FileUtils.rm_rf(root)
  end

  Golden.write("config_template", Golden.fixture(
    source: "app/models/config.rb#create_template_config/upgrade_config_file",
    generator: GEN,
    notes: "Template content is static (TEMPLATE_SECTIONS joined by \\n plus trailing \\n); see fed_template.fed. tree_after values equal to the template are replaced by \"<TEMPLATE>\". " \
           "Config.new always creates <base>/.frankmd-config.lock (mode 600, empty). .fed written atomically via .fed.<hex>.tmp + rename, mode 600. " \
           "'appended' = text appended to the chomp-joined original lines (upgrade rewrites lines joined by \\n, so CRLF becomes LF). log = Rails.logger lines (info+).",
    template_sections: Config::TEMPLATE_SECTIONS.map { |s| { "marker" => s[:marker], "lines" => s[:lines] } },
    cases: cases
  ))
end

# ---------------------------------------------------------------- parse

def parse_cases
  bom = "\uFEFF"
  long = "x" * 5000
  theme_json = JSON.generate({ "version" => 1, "name" => "Mine", "base" => "dark", "colors" => { "accent" => "#AABBCC" } })
  c = []
  add = ->(name, fed, env = {}) { c << { "name" => name, "fed" => fed, "env" => env } }
  # from tests
  add.("reads values (test)", "theme = gruvbox\neditor_font = fira-code\neditor_font_size = 16\n")
  add.("reads booleans (test)", "typewriter_mode = true\nsidebar_visible = false\n")
  add.("quoted values (test)", "theme = \"tokyo-night\"\neditor_font = 'jetbrains-mono'\n")
  add.("ignores comments (test)", "# This is a comment\ntheme = dark\n# editor_font = should-be-ignored\n")
  add.("ignores unknown keys (test)", "unknown_key = some_value\ntheme = light\n")
  add.("vim_mode (test)", "vim_mode = true\n")
  add.("cursor_recenter_on_click (test)", "cursor_recenter_on_click = true\n")
  add.("scroll_sync false (test)", "scroll_sync = false\n")
  add.("view_mode single (test)", "view_mode = single\n")
  add.("empty file (test)", "")
  add.("malformed lines (test)", "this is not valid\n= no key\ntheme = dark\neditor_font_size not an int\n")
  c << { "name" => "binary garbage (test)", "fed_base64" => [ "\x00\xFF\xFE\x00theme = dark\n".b ].pack("m0"), "env" => {} }
  add.("integer without trailing newline (test)", "editor_font_size = 20")
  %w[true 1 yes on TRUE Yes].each { |v| add.("bool true literal #{v} (test)", "typewriter_mode = #{v}") }
  %w[false 0 no off FALSE anything].each { |v| add.("bool false literal #{v} (test)", "typewriter_mode = #{v}") }
  add.("ai_provider only (test)", "ai_provider = anthropic")
  add.("malformed custom_theme keeps others (test)", "theme = nord\ncustom_theme = {broken\neditor_font_size = 18\n")
  # whitespace / line endings
  add.("CRLF line endings", "theme = dark\r\neditor_font_size = 18\r\nvim_mode = true\r\n")
  add.("lone CR line endings (single logical line)", "theme = dark\rvim_mode = true\r")
  add.("mixed LF/CRLF/CR", "theme = dark\r\nvim_mode = true\rshow_stats = true\neditor_width = 100")
  add.("blank and whitespace-only lines", "\n   \n\t\ntheme = dark\n\n\n")
  add.("no spaces around =", "theme=dark\neditor_font_size=18")
  add.("many spaces and tabs around =", "theme   =     dark\neditor_font\t=\thack\n\t  editor_font_size  \t=\t 22 \t\n")
  add.("leading indentation", "    theme = dark\n\tvim_mode = yes\n")
  add.("trailing whitespace in value", "theme = dark   \t\n")
  add.("NBSP around value (not stripped)", "theme =\u00A0dark\u00A0\n")
  add.("NBSP before key", "\u00A0theme = dark\n")
  add.("vertical tab / form feed around", "\v\ftheme = dark\f\v\n")
  add.("NUL at end of line", "theme = dark\u0000\n")
  add.("NUL at start of line", "\u0000theme = dark\n")
  add.("BOM at start of file", "#{bom}theme = dark\nvim_mode = true\n")
  add.("BOM at start of value", "theme = #{bom}dark\n")
  # quotes
  add.("double quoted with spaces inside", "theme = \"  spaced out  \"\n")
  add.("single quoted with spaces inside", "theme = '  spaced  '\n")
  add.("unbalanced leading double quote", "theme = \"dark\n")
  add.("unbalanced trailing double quote", "theme = dark\"\n")
  add.("mismatched quotes \"x'", "theme = \"dark'\n")
  add.("single double-quote char", "theme = \"\n")
  add.("single single-quote char", "theme = '\n")
  add.("empty double quotes", "theme = \"\"\n")
  add.("empty single quotes", "theme = ''\n")
  add.("nested quotes '\"x\"' stripped twice", "theme = '\"dark\"'\n")
  add.("nested quotes \"'x'\" stripped twice", "theme = \"'dark'\"\n")
  add.("triple double quotes", "theme = \"\"\"dark\"\"\"\n")
  add.("quotes inside value", "editor_font = my \"quoted\" font\n")
  add.("quoted integer", "editor_font_size = \"16\"\n")
  add.("quoted boolean", "vim_mode = 'true'\n")
  add.("quoted with inner whitespace integer", "editor_font_size = \" 16 \"\n")
  # = in values, inline #
  add.("= inside value", "theme = a=b=c\n")
  add.("= with spaces inside value", "ollama_api_base = http://h/?a = b\n")
  add.("double == ", "theme == dark\n")
  add.("inline # comment is part of value", "theme = dark # my theme\n")
  add.("hash without space", "theme = dark#x\n")
  add.("value starting with #", "theme = #dark\n")
  add.("quoted value containing #", "theme = \"dark # not comment\"\n")
  add.("hash then key (commented)", "#theme = dark\n")
  add.("semicolon comment not supported", "; theme = dark\ntheme = light ; c\n")
  # keys
  add.("duplicate keys - last wins", "theme = dark\ntheme = light\ntheme = nord\n")
  add.("duplicate keys where last is invalid json", "recent_files = [\"a.md\"]\nrecent_files = [broken\n")
  add.("duplicate key last empty", "theme = dark\ntheme =\n")
  add.("uppercase key", "THEME = dark\nEditor_Font_Size = 19\n")
  add.("dash in key not matched", "editor-font = hack\n")
  add.("dotted key not matched", "editor.font = hack\n")
  add.("key with space not matched", "editor font = hack\n")
  add.("unicode key not matched", "thème = dark\n")
  add.("key only no =", "theme\n")
  add.("colon separator not supported", "theme: dark\n")
  add.("INI section header ignored", "[ui]\ntheme = dark\n")
  add.("export prefix not supported", "export theme=dark\n")
  add.("all unknown keys", "foo = 1\nbar = 2\nnotes_path = /x\n")
  add.("numeric key", "123 = x\n")
  # empty values
  add.("empty string value", "theme =\n")
  add.("empty value with spaces", "theme =    \n")
  add.("empty boolean (nil, not default)", "sidebar_visible =\nscroll_sync = \n")
  add.("empty integer (nil, not default)", "editor_font_size =\n")
  add.("empty json recent_files", "recent_files =\n")
  add.("empty custom_theme", "custom_theme =\n")
  add.("empty string overrides ENV", "youtube_api_key =\n", { "YOUTUBE_API_KEY" => "env-key" })
  add.("empty quoted string overrides ENV", "locale = \"\"\n", { "FRANKMD_LOCALE" => "pt-BR" })
  # integers
  { "negative" => "-5", "plus sign" => "+7", "trailing junk" => "12px", "leading junk" => "px12", "float" => "16.9",
    "exponent" => "1e3", "hex" => "0x10", "octal-looking" => "010", "underscore" => "1_000", "double underscore" => "1__0",
    "spaces inside" => "1 4", "huge" => "999999999999999999999999", "words" => "abc", "fullwidth digits" => "１６",
    "leading zeros" => "0007", "minus zero" => "-0", "only sign" => "-", "binary" => "0b101" }.each do |label, v|
    add.("integer #{label}: #{v}", "editor_font_size = #{v}\npreview_zoom = #{v}\n")
  end
  # booleans
  %w[t y YES On oN enabled 2 -1 true. 1.0 ｔｒｕｅ].each { |v| add.("boolean literal #{v}", "sidebar_visible = #{v}\nvim_mode = #{v}\n") }
  add.("boolean with surrounding quotes and spaces", "vim_mode = \" true\"\n")
  # enums / strings (no validation on parse)
  add.("view_mode invalid enum accepted as string", "view_mode = bogus\nhugo_path_style = weird\nai_provider = nonexistent\n")
  add.("media_storage_provider invalid accepted on parse", "media_storage_provider = ftp\n")
  add.("media_storage_provider s3 accepted on parse without creds", "media_storage_provider = s3\n")
  add.("theme custom", "theme = custom\n")
  # json
  add.("recent_files valid array", "recent_files = [\"a.md\",\"dir/b.md\",\"ção/日本語.md\"]\n")
  add.("recent_files object", "recent_files = {\"a\":1}\n")
  add.("recent_files scalar number", "recent_files = 42\n")
  add.("recent_files quoted string (quotes stripped -> invalid)", "recent_files = \"abc\"\n")
  add.("recent_files invalid json ignored", "recent_files = [a, b]\n")
  add.("recent_files null", "recent_files = null\n")
  add.("recent_files single-quoted json", "recent_files = '[\"x\"]'\n")
  add.("recent_files with unicode escapes", "recent_files = [\"\\u00e7\\u00e3o\",\"\\ud83d\\ude00\"]\n")
  # custom theme
  add.("custom_theme valid", "custom_theme = #{theme_json}\n")
  add.("custom_theme invalid color ignored", "custom_theme = #{JSON.generate({ "version" => 1, "name" => "x", "base" => "dark", "colors" => { "accent" => "red" } })}\n")
  add.("custom_theme null", "custom_theme = null\n")
  add.("custom_theme wrong version", "custom_theme = #{JSON.generate({ "version" => 2, "name" => "x", "base" => "dark", "colors" => {} })}\n")
  add.("custom_theme too large", "custom_theme = #{JSON.generate({ "version" => 1, "name" => "x", "base" => "dark", "colors" => {}, "pad" => "y" * 17000 })}\n")
  # unicode & long
  add.("unicode values", "theme = ção\neditor_font = 日本語\nlocale = שלום\nimages_path = /home/ü/👨‍👩‍👧\nai_model = e\u0301\n")
  add.("very long value", "theme = #{long}\n")
  add.("path with spaces unquoted", "images_path = /home/me/My Pictures\n")
  add.("path traversal accepted as string", "images_path = ../../etc\n")
  add.("tilde path not expanded", "images_path = ~/Pictures\n")
  add.("value with backslashes", "images_path = C:\\Users\\me\\Pictures\n")
  add.("value with escaped newline literal", "theme = dark\\nvim_mode = true\n")
  add.("all keys set", Config::SCHEMA.keys.map { |k|
    v = case Config::SCHEMA[k][:type]
        when :json then "[\"r.md\"]"
        when :custom_theme then theme_json
        when :integer then "7"
        when :boolean then "yes"
        else "v-#{k}"
        end
    "#{k} = #{v}"
  }.join("\n") + "\n")
  add.("full template file (all commented)", :template)
  c
end

def gen_parse
  cases = parse_cases.map do |pc|
    root = Golden.mktmp("parse")
    input = { "env" => pc["env"] }
    if pc["fed"] == :template
      pc = pc.merge("fed" => $template_text)
    end
    if pc["fed_base64"]
      input["fed_base64"] = pc["fed_base64"]
    else
      input["fed"] = pc["fed"]
    end
    write_fed_input(root, input)
    (cfg, log) = with_log { with_clean_env(pc["env"]) { c = Config.new(base_path: root); [ file_values(c), resolved(c) ] } }
    out = norm({
      "name" => pc["name"], "fn" => "Config.new + get",
      "input" => input,
      "expected" => {
        "file_values" => cfg[0],
        "resolved" => cfg[1],
        "fed_after_load" => (f = read_fed(root)) == input["fed"] ? "<UNCHANGED>" : f,
        "log" => log
      }
    }, root)
    FileUtils.rm_rf(root)
    out
  end
  Golden.write("config_parse", Golden.fixture(
    source: "app/models/config.rb#parse_config/cast_value/get",
    generator: GEN,
    notes: "Each case writes input.fed (or fed_base64) to <ROOT>/.fed, scrubs every schema ENV var + NOTES_PATH + XDG_PICTURES_DIR, applies input.env, then Config.new(base_path: <ROOT>). " \
           "file_values = @values (keys explicitly present in file, in insertion order; may be null). resolved = get(key) for every SCHEMA key (file > ENV(present) > default). " \
           "fed_after_load: \"<UNCHANGED>\" or the file after load (Config.new appends the AI/LLM template section when no AI marker is present). log = Rails.logger info+ lines.",
    cases: cases
  ))
end

# ---------------------------------------------------------------- env

def gen_env
  cases = []
  keys = Config::SCHEMA.map do |k, s|
    { "key" => k, "type" => s[:type].to_s, "default" => s[:default], "env" => s[:env],
      "sensitive" => Config::SENSITIVE_KEYS.include?(k), "ui" => Config::UI_KEYS.include?(k),
      "ai_credential" => Config::AI_CREDENTIAL_KEYS.include?(k) }
  end
  other_env = [
    { "var" => "NOTES_PATH", "where" => "app/models/config.rb, app/controllers/*, app/services/*", "default" => "Rails.root/notes (Dockerfile: /rails/notes)", "use" => "notes root; .fed lives at $NOTES_PATH/.fed" },
    { "var" => "XDG_PICTURES_DIR", "where" => "config/initializers/images.rb", "default" => nil, "use" => "at boot, if IMAGES_PATH unset: IMAGES_PATH = XDG_PICTURES_DIR || (~/Pictures if dir exists)" },
    { "var" => "HOME", "where" => "config/initializers/images.rb (File.expand_path('~/Pictures'))", "default" => nil, "use" => "~/Pictures fallback for IMAGES_PATH" },
    { "var" => "FRANKMD_ALLOWED_HOSTS", "where" => "config/environments/production.rb", "default" => "", "use" => "comma-separated host allowlist; split(','), strip, reject blank; empty => all hosts allowed and forgery origin check off" },
    { "var" => "PORT", "where" => "config/puma.rb", "default" => "3000", "use" => "bind tcp://0.0.0.0:$PORT" },
    { "var" => "RAILS_MAX_THREADS", "where" => "config/puma.rb", "default" => "3", "use" => "puma threads" },
    { "var" => "PIDFILE", "where" => "config/puma.rb", "default" => nil, "use" => "puma pidfile when set" },
    { "var" => "RAILS_LOG_LEVEL", "where" => "config/environments/production.rb", "default" => "info", "use" => "log level" },
    { "var" => "CI", "where" => "config/environments/test.rb", "default" => nil, "use" => "eager_load in tests" },
    { "var" => "PATH", "where" => "app/services/images_service.rb", "default" => "", "use" => "executable lookup (split on File::PATH_SEPARATOR)" },
    { "var" => "BUNDLE_GEMFILE", "where" => "config/boot.rb, bin/kamal", "default" => "../Gemfile", "use" => "bundler" },
    { "var" => "OPENROUTER_API_KEY", "where" => "app/services/ai_service.rb#openrouter_key_for_images", "default" => nil, "use" => "image generation: file value (@values) || ENV, bypassing get_ai file-only mode" },
    { "var" => "SECRET_KEY_BASE", "where" => "bin/docker-entrypoint", "default" => "random hex 64 when empty", "use" => "rails secret" },
    { "var" => "THRUSTER_TARGET_PORT / EXTERNAL_PORT", "where" => "bin/docker-entrypoint", "default" => "3000", "use" => "puma port behind thruster; 80/443 coerced to 3000" },
    { "var" => "RAILS_ENV", "where" => "Dockerfile", "default" => "production (docker)", "use" => "rails env" }
  ]

  theme_json = JSON.generate({ "version" => 1, "name" => "Mine", "base" => "dark", "colors" => { "accent" => "#AABBCC" } })
  scen = []
  s = ->(name, fed, env) { scen << [ name, fed, env ] }
  # Per-env-var precedence matrix
  Config::SCHEMA.each do |k, sc|
    next unless sc[:env]
    ev = sc[:type] == :boolean ? "true" : "env-#{k}"
    fv = sc[:type] == :boolean ? "false" : "file-#{k}"
    s.("#{sc[:env]}: default only", "# ai_model = placeholder\n", {})
    s.("#{sc[:env]}: env only", "# ai_model = placeholder\n", { sc[:env] => ev })
    s.("#{sc[:env]}: file only", "# ai_model = placeholder\n#{k} = #{fv}\n", {})
    s.("#{sc[:env]}: file beats env", "# ai_model = placeholder\n#{k} = #{fv}\n", { sc[:env] => ev })
    s.("#{sc[:env]}: env empty string -> default", "# ai_model = placeholder\n", { sc[:env] => "" })
    s.("#{sc[:env]}: env whitespace -> default (blank)", "# ai_model = placeholder\n", { sc[:env] => " \t " })
    s.("#{sc[:env]}: env with surrounding spaces kept raw", "# ai_model = placeholder\n", { sc[:env] => "  #{ev}  " })
    s.("#{sc[:env]}: empty file value beats env", "# ai_model = placeholder\n#{k} =\n", { sc[:env] => ev })
    s.("#{sc[:env]}: commented file value does not count", "# ai_model = placeholder\n# #{k} = #{fv}\n", { sc[:env] => ev })
  end
  # removed duplicates of the shared default-only scenario
  scen.uniq! { |n, f, e| [ f, e ] }
  # boolean env casting
  %w[true TRUE 1 yes on false 0 no off anything].each do |v|
    s.("AWS_S3_FORCE_PATH_STYLE=#{v}", "", { "AWS_S3_FORCE_PATH_STYLE" => v })
  end
  # upload extension normalization (test)
  s.("VIDEO_UPLOAD_EXTENSIONS normalization (test)", "", { "VIDEO_UPLOAD_EXTENSIONS" => " .MP4 , .MOV " })
  s.("IMAGE_UPLOAD_EXTENSIONS with empty items", "", { "IMAGE_UPLOAD_EXTENSIONS" => ",, .PNG,,JPG , ,.Webp," })
  s.("image_upload_extensions file value", "image_upload_extensions = .HEIC,.Ávif\n", { "IMAGE_UPLOAD_EXTENSIONS" => ".png" })
  # AI scenarios (from tests)
  s.("AI: nothing configured", "", {})
  s.("AI: openai+anthropic env (test)", "", { "OPENAI_API_KEY" => "sk-test", "ANTHROPIC_API_KEY" => "sk-ant-test" })
  s.("AI: priority order all env (test)", "", { "OLLAMA_API_BASE" => "http://localhost:11434", "OPENROUTER_API_KEY" => "sk-or-test", "ANTHROPIC_API_KEY" => "sk-ant-test", "OPENAI_API_KEY" => "sk-test" })
  s.("AI: explicit provider available (test)", "ai_provider = openai", { "OLLAMA_API_BASE" => "http://localhost:11434", "OPENAI_API_KEY" => "sk-test" })
  s.("AI: explicit provider unavailable falls back (test)", "ai_provider = openai", { "OLLAMA_API_BASE" => "http://localhost:11434" })
  s.("AI: provider-specific default model (test)", "", { "ANTHROPIC_API_KEY" => "sk-ant-test" })
  s.("AI: global ai_model override (test)", "ai_model = gpt-4-turbo", { "OPENAI_API_KEY" => "sk-test" })
  s.("AI: provider-specific model in file (test)", "anthropic_model = claude-3-opus-20240229", { "ANTHROPIC_API_KEY" => "sk-ant-test" })
  s.("AI: gemini only (test)", "", { "GEMINI_API_KEY" => "gemini-test" })
  s.("AI: credential in file -> file-only mode (test)", "anthropic_api_key = sk-ant-test", {})
  s.("AI: only ai_provider in file is not file mode (test)", "ai_provider = anthropic", {})
  s.("AI: file credential ignores env credentials (test)", "anthropic_api_key = sk-file-anthropic", { "OPENAI_API_KEY" => "sk-env-openai", "OPENROUTER_API_KEY" => "sk-env-openrouter" })
  s.("AI: env used when no credential in file (test)", "theme = dark", { "OPENAI_API_KEY" => "sk-env-openai" })
  s.("AI: file mode ignores env model and provider", "ollama_api_base = http://h:11434", { "AI_PROVIDER" => "openai", "AI_MODEL" => "env-model", "OLLAMA_MODEL" => "env-ollama", "OPENAI_API_KEY" => "sk" })
  s.("AI: empty credential in file still triggers file-only mode", "openai_api_key =", { "OPENAI_API_KEY" => "sk-env", "ANTHROPIC_API_KEY" => "sk-ant" })
  s.("AI: AI_PROVIDER env selects gemini", "", { "AI_PROVIDER" => "gemini", "GEMINI_API_KEY" => "g", "OPENAI_API_KEY" => "o" })
  s.("AI: ai_provider auto with ollama+gemini", "ai_provider = auto", { "OLLAMA_API_BASE" => "http://x", "GEMINI_API_KEY" => "g" })
  s.("AI: ai_provider uppercase not matched", "ai_provider = OpenAI", { "OPENAI_API_KEY" => "o", "ANTHROPIC_API_KEY" => "a" })
  s.("AI: ai_model empty in file falls to provider model", "ai_model =", { "OPENAI_API_KEY" => "o", "AI_MODEL" => "env-model" })
  s.("AI: AI_MODEL env global override", "", { "OPENROUTER_API_KEY" => "or", "AI_MODEL" => "env-model" })
  s.("AI: openrouter model env", "", { "OPENROUTER_API_KEY" => "or", "OPENROUTER_MODEL" => "meta/llama" })
  s.("AI: provider model empty in file -> nil", "openai_api_key = sk\nopenai_model =", {})
  # features
  s.("S3: creds in file (test)", "aws_access_key_id = key\naws_secret_access_key = secret\naws_s3_bucket = bucket\n", {})
  s.("S3: creds in env", "", { "AWS_ACCESS_KEY_ID" => "k", "AWS_SECRET_ACCESS_KEY" => "s", "AWS_S3_BUCKET" => "b" })
  s.("S3: missing bucket", "", { "AWS_ACCESS_KEY_ID" => "k", "AWS_SECRET_ACCESS_KEY" => "s" })
  { "http endpoint" => "http://localhost:9000", "https endpoint" => "https://s3.example.com", "ftp endpoint" => "ftp://x",
    "endpoint with userinfo" => "http://u:p@minio:9000", "endpoint without scheme" => "minio:9000", "endpoint invalid uri" => "http://exa mple",
    "endpoint host only no scheme" => "localhost", "endpoint empty host" => "http://", "endpoint uppercase scheme" => "HTTP://minio" }.each do |label, ep|
    s.("S3: #{label}", "", { "AWS_ACCESS_KEY_ID" => "k", "AWS_SECRET_ACCESS_KEY" => "s", "AWS_S3_BUCKET" => "b", "AWS_S3_ENDPOINT" => ep })
  end
  s.("S3: media default local even with creds (test)", "", { "AWS_ACCESS_KEY_ID" => "key", "AWS_SECRET_ACCESS_KEY" => "secret", "AWS_S3_BUCKET" => "bucket" })
  s.("YouTube in file (test)", "youtube_api_key = test-key\n", {})
  s.("YouTube env (test)", "", { "YOUTUBE_API_KEY" => "test-youtube-key" })
  s.("YouTube file beats env (test)", "youtube_api_key = file-key\n", { "YOUTUBE_API_KEY" => "env-key" })
  s.("Google needs both (test)", "google_api_key = api-key\ngoogle_cse_id = cse-id\n", {})
  s.("Google only key", "google_api_key = api-key\n", {})
  s.("local_images via IMAGES_PATH env", "", { "IMAGES_PATH" => "/pics" })
  s.("local_images via file", "images_path = /my/images\n", {})
  s.("auth token env", "", { "FRANKMD_AUTH_TOKEN" => "a" * 40 })
  s.("auth token file beats env", "auth_token = #{"b" * 32}\n", { "FRANKMD_AUTH_TOKEN" => "a" * 40 })
  s.("locale env", "", { "FRANKMD_LOCALE" => "pt-BR" })
  s.("custom theme and recent files have no env", "custom_theme = #{theme_json}\nrecent_files = [\"a.md\"]\n", {})
  s.("all env vars set, empty file", "", ENV_KEYS.to_h { |k| [ k, k == "AWS_S3_FORCE_PATH_STYLE" ? "yes" : "env-#{k.downcase}" ] })
  s.("all env vars set, template file", :template, ENV_KEYS.to_h { |k| [ k, k == "AWS_S3_FORCE_PATH_STYLE" ? "yes" : "env-#{k.downcase}" ] })

  scen.each do |name, fed, env|
    fed = $template_text if fed == :template
    root = Golden.mktmp("env")
    File.binwrite(File.join(root, ".fed"), fed)
    snap = with_clean_env(env) { full_snapshot(Config.new(base_path: root)) }
    cases << norm({ "name" => name, "fn" => "precedence", "input" => { "fed" => fed, "env" => env }, "expected" => snap }, root)
    FileUtils.rm_rf(root)
  end

  # NOTES_PATH resolution
  root = Golden.mktmp("env")
  r = with_clean_env("NOTES_PATH" => root) { c = Config.new; { "base_path" => c.base_path.to_s, "config_file_path" => c.config_file_path.to_s } }
  cases << norm({ "name" => "NOTES_PATH env sets base path when base_path nil", "fn" => "Config.new", "input" => { "env" => { "NOTES_PATH" => "<ROOT>" } }, "expected" => r }, root)
  r = with_clean_env("NOTES_PATH" => "/should/not/be/used") { c = Config.new(base_path: root); { "base_path" => c.base_path.to_s } }
  cases << norm({ "name" => "explicit base_path beats NOTES_PATH", "fn" => "Config.new", "input" => { "env" => { "NOTES_PATH" => "/should/not/be/used" }, "base_path" => "<ROOT>" }, "expected" => r }, root)
  r = with_clean_env("NOTES_PATH" => "#{root}/trailing/") { c = Config.new; { "base_path" => c.base_path.to_s, "config_file_path" => c.config_file_path.to_s } }
  cases << norm({ "name" => "NOTES_PATH with trailing slash", "fn" => "Config.new", "input" => { "env" => { "NOTES_PATH" => "<ROOT>/trailing/" } }, "expected" => r }, root)
  FileUtils.rm_rf(root)

  # images initializer: IMAGES_PATH fallback from XDG_PICTURES_DIR / ~/Pictures
  init = Rails.root.join("config/initializers/images.rb").to_s
  [
    [ "IMAGES_PATH already set wins", { "IMAGES_PATH" => "/set", "XDG_PICTURES_DIR" => "/xdg" }, true ],
    [ "XDG_PICTURES_DIR used when IMAGES_PATH unset", { "XDG_PICTURES_DIR" => "/xdg" }, true ],
    [ "~/Pictures used when it exists", {}, true ],
    [ "nothing set when ~/Pictures missing", {}, false ],
    [ "IMAGES_PATH empty string counts as set", { "IMAGES_PATH" => "", "XDG_PICTURES_DIR" => "/xdg" }, true ],
    [ "XDG_PICTURES_DIR empty string is used verbatim", { "XDG_PICTURES_DIR" => "" }, true ]
  ].each do |name, env, pictures|
    home = Golden.mktmp("home")
    FileUtils.mkdir_p(File.join(home, "Pictures")) if pictures
    res = with_clean_env(env.merge("HOME" => home)) { load init; ENV.key?("IMAGES_PATH") ? ENV["IMAGES_PATH"] : nil }
    cases << norm({ "name" => "images initializer: #{name}", "fn" => "config/initializers/images.rb",
                    "input" => { "env" => env.merge("HOME" => "<ROOT>"), "home_has_Pictures" => pictures },
                    "expected" => { "IMAGES_PATH" => res } }, home)
    FileUtils.rm_rf(home)
  end

  # FRANKMD_ALLOWED_HOSTS parsing expression (same expression as production.rb)
  [ "", " ", "a.com", "a.com,b.com", " a.com , ,b.com ,", ",,,", "notes.lan,.example.com,192.168.1.5" ].each do |v|
    res = v.split(",").map(&:strip).reject(&:blank?)
    cases << { "name" => "FRANKMD_ALLOWED_HOSTS=#{v.inspect}", "fn" => "production.rb allowed_hosts", "input" => { "env" => { "FRANKMD_ALLOWED_HOSTS" => v } },
               "expected" => { "allowed_hosts" => res, "all_hosts_allowed" => res.empty? } }
  end

  # ai_service openrouter_key_for_images
  [
    [ "file key", "openrouter_api_key = file-or\n", { "OPENROUTER_API_KEY" => "env-or" } ],
    [ "env key bypasses file-only mode", "anthropic_api_key = a\n", { "OPENROUTER_API_KEY" => "env-or" } ],
    [ "empty file key (nil) falls back to env", "openrouter_api_key =\n", { "OPENROUTER_API_KEY" => "env-or" } ],
    [ "none", "", {} ]
  ].each do |name, fed, env|
    root = Golden.mktmp("env")
    File.write(File.join(root, ".fed"), fed)
    res = with_clean_env(env) do
      cfg = Config.new(base_path: root)
      v = cfg.instance_variable_get(:@values)&.dig("openrouter_api_key") || ENV["OPENROUTER_API_KEY"]
      { "openrouter_key_for_images" => v, "get_ai_openrouter_api_key" => cfg.get_ai("openrouter_api_key") }
    end
    cases << { "name" => "openrouter_key_for_images: #{name}", "fn" => "AiService.openrouter_key_for_images (expression)", "input" => { "fed" => fed, "env" => env }, "expected" => res }
    FileUtils.rm_rf(root)
  end

  Golden.write("config_env", Golden.fixture(
    source: "app/models/config.rb#get/get_ai/feature_available?/effective_ai_* + config/initializers/images.rb",
    generator: GEN,
    notes: "Scrubs all schema ENV vars, NOTES_PATH and XDG_PICTURES_DIR, then applies input.env. ENV counts only when .present? (non-blank). ENV values are cast but not stripped. " \
           "sources computed with LogsController#config_source. all_settings_masked = all_settings (sensitive keys replaced by <key>_configured booleans). " \
           "Many precedence cases include '# ai_model = placeholder' only to keep the AI section from being appended (irrelevant to values). " \
           "FRANKMD_ALLOWED_HOSTS and openrouter_key_for_images cases evaluate the same expressions as the source (they are not reachable as standalone methods).",
    frankmd_version: FrankMD::VERSION,
    config_constants: {
      "CONFIG_FILE" => Config::CONFIG_FILE, "LOCK_FILE" => Config::LOCK_FILE, "CONFIG_VERSION" => Config::CONFIG_VERSION,
      "SENSITIVE_KEYS" => Config::SENSITIVE_KEYS, "UI_KEYS" => Config::UI_KEYS,
      "AI_PROVIDER_PRIORITY" => Config::AI_PROVIDER_PRIORITY, "AI_CREDENTIAL_KEYS" => Config::AI_CREDENTIAL_KEYS,
      "ai_providers_available_order" => %w[ollama openrouter anthropic gemini openai]
    },
    keys: keys,
    other_env: other_env,
    cases: cases
  ))
end

# ---------------------------------------------------------------- writeback

def apply_op(cfg, op, root)
  case op["op"]
  when "set" then cfg.set(op["key"], op["value"])
  when "update" then cfg.update(op["values"])
  when "modify_append"
    seen = nil
    r = cfg.modify(op["key"]) { |cur| seen = cur; Array(cur) + [ op["value"] ] }
    { "returned" => r, "block_saw" => seen }
  when "modify_replace"
    seen = nil
    r = cfg.modify(op["key"]) { |cur| seen = cur; op["value"] }
    { "returned" => r, "block_saw" => seen }
  when "external_write"
    File.binwrite(File.join(root, ".fed"), op["fed"])
    nil
  when "reload" then :reload
  when "generate_config_lines" then cfg.send(:generate_config_lines)
  when "format_value" then Golden.capture { cfg.send(:format_value, op["key"], op["value"]) }
  when "ui_settings" then cfg.ui_settings
  end
end

def gen_writeback
  theme = { "version" => 1, "name" => "My \"theme\"", "base" => "dark", "colors" => { "accent" => "#AABBCC" } }
  marker = "# ai_model = placeholder\n"
  s3 = { "AWS_ACCESS_KEY_ID" => "k", "AWS_SECRET_ACCESS_KEY" => "s", "AWS_S3_BUCKET" => "b" }
  sc = []
  w = ->(name, fed, ops, env: {}, tree: nil, readonly: false) { sc << { "name" => name, "fed" => fed, "ops" => ops, "env" => env, "tree" => tree, "readonly" => readonly } }
  set = ->(k, v) { { "op" => "set", "key" => k, "value" => v } }
  upd = ->(h) { { "op" => "update", "values" => h } }

  # from tests
  w.("set on fresh template uncomments theme line (test)", nil, [ set.("theme", "nord") ])
  w.("set on fresh template: every UI key", nil, [ upd.({ "theme" => "dark", "locale" => "pt-BR", "editor_font" => "hack", "editor_font_size" => 18, "preview_zoom" => 125,
                                                           "sidebar_visible" => false, "view_mode" => "single", "typewriter_mode" => true, "cursor_recenter_on_click" => true,
                                                           "vim_mode" => true, "scroll_sync" => false, "editor_indent" => 0, "editor_line_numbers" => 1, "show_stats" => true,
                                                           "editor_width" => 100, "media_storage_provider" => "local" }) ])
  w.("set on fresh template: every non-UI key", nil, [ upd.(Config::SCHEMA.keys.reject { |k| Config::UI_KEYS.include?(k) || %w[recent_files].include?(k) }.to_h { |k|
    [ k, Config::SCHEMA[k][:type] == :boolean ? true : "v-#{k}" ] }) ])
  w.("update multiple values (test)", nil, [ upd.({ "theme" => "dark", "editor_font_size" => 18 }) ])
  w.("set same key thrice does not duplicate (test)", nil, [ set.("theme", "dark"), set.("theme", "light"), set.("theme", "gruvbox") ])
  w.("preserves user structure (test)", "editor_font = hack\ntheme = dark\neditor_font_size = 16\n#{marker}", [ set.("theme", "gruvbox") ])
  w.("does not re-add removed values (test)", "theme = dark\neditor_font = hack\neditor_font_size = 18\n#{marker}",
     [ { "op" => "external_write", "fed" => "theme = dark\neditor_font_size = 18\n#{marker}" }, { "op" => "reload" }, set.("theme", "nord") ])
  w.("stale instance: set after external removal only touches its key", "theme = dark\neditor_font = hack\n#{marker}",
     [ { "op" => "external_write", "fed" => "theme = dark\n#{marker}" }, set.("theme", "nord") ])
  w.("only modifies target key (test)", "# My custom comment\ntheme = dark\n\n# Another section\neditor_font = fira-code\neditor_font_size = 20\n", [ set.("editor_font_size", 22) ])
  w.("appends new key at end (test)", "theme = dark\n#{marker}", [ set.("editor_font", "hack") ])
  w.("rejects quote/newline injection (test)", "theme = dark\n", [ upd.({ "theme" => "light\"\nimages_path = /outside" }) ])
  w.("unsafe quote in font rejected (test)", "theme = nord\ncustom_theme = {broken\neditor_font_size = 18\n", [ set.("editor_font", "unsafe\"font") ])
  w.("update preserves 0600 (test)", "theme = light\n", [ upd.({ "theme" => "dark" }) ])
  w.("update adds font to existing (controller test)", "theme = tokyo-night\n", [ upd.({ "editor_font" => "fira-code" }) ])
  w.("partial updates (controller test)", nil, [ upd.({ "theme" => "dark" }), { "op" => "reload" }, upd.({ "editor_font" => "hack" }) ])
  w.("locale pt-BR (controller test)", nil, [ upd.({ "locale" => "pt-BR" }) ])
  w.("booleans via update (controller test)", nil, [ upd.({ "typewriter_mode" => true, "sidebar_visible" => false, "cursor_recenter_on_click" => true, "show_stats" => true }) ])
  w.("custom theme + theme atomic (test)", nil, [ upd.({ "theme" => "custom", "custom_theme" => JSON.generate(theme) }), { "op" => "reload" }, { "op" => "ui_settings" } ])
  w.("invalid palette rejects whole update (test)", nil, [ set.("theme", "nord"), upd.({ "theme" => "custom", "custom_theme" => JSON.generate(theme.merge("colors" => { "accent" => "url(https://example.com)" })) }) ])
  w.("media: unknown provider rejected (test)", nil, [ set.("media_storage_provider", "unknown") ])
  w.("media: s3 without creds rejects whole update (test)", nil, [ upd.({ "media_storage_provider" => "s3", "theme" => "dark" }) ])
  w.("media: s3 with env creds persists (test)", nil, [ set.("media_storage_provider", "s3"), { "op" => "reload" }, set.("media_storage_provider", "local") ], env: s3)
  w.("media: s3 with creds in file", "aws_access_key_id = k\naws_secret_access_key = s\naws_s3_bucket = b\n#{marker}", [ set.("media_storage_provider", "s3") ])
  w.("media: S3 with invalid endpoint rejected", nil, [ set.("media_storage_provider", "s3") ], env: s3.merge("AWS_S3_ENDPOINT" => "ftp://x"))
  w.("media: uppercase LOCAL rejected", nil, [ set.("media_storage_provider", "LOCAL") ])
  w.("media: nil provider rejected", nil, [ set.("media_storage_provider", nil) ])
  w.("show_stats persists (test)", nil, [ set.("show_stats", true), { "op" => "reload" } ])
  # string formatting
  w.("string with space gets quoted", marker, [ set.("editor_font", "my font") ])
  w.("string with = gets quoted", marker, [ set.("ollama_api_base", "http://h/?a=b") ])
  w.("string with leading/trailing spaces quoted and roundtrips", marker, [ set.("theme", "  dark  "), { "op" => "reload" } ])
  w.("string with tab is not quoted", marker, [ set.("theme", "a\tb"), { "op" => "reload" } ])
  w.("string with # not quoted (roundtrips)", marker, [ set.("theme", "dark#1"), { "op" => "reload" } ])
  w.("string with single quotes around does not roundtrip", marker, [ set.("theme", "'dark'"), { "op" => "reload" } ])
  w.("string with CR rejected", marker, [ set.("theme", "a\rb") ])
  w.("string with LF rejected", marker, [ set.("theme", "a\nb") ])
  w.("string with double quote rejected", marker, [ set.("theme", "a\"b") ])
  w.("string with NBSP not quoted", marker, [ set.("theme", "a\u00A0b"), { "op" => "reload" } ])
  w.("unicode string", marker, [ set.("theme", "ção 日本語 שלום 👨‍👩‍👧"), { "op" => "reload" } ])
  w.("empty string writes 'key = ' and value becomes nil", marker, [ set.("theme", ""), { "op" => "reload" } ])
  w.("whitespace-only string -> nil", marker, [ set.("theme", "   "), { "op" => "reload" } ])
  w.("nil string", "theme = dark\n#{marker}", [ set.("theme", nil), { "op" => "reload" } ])
  w.("integer given to string key", marker, [ set.("theme", 42) ])
  w.("very long string", marker, [ set.("theme", "y" * 3000), { "op" => "reload" } ])
  # integers / booleans
  w.("integer from string", marker, [ set.("editor_font_size", "18") ])
  w.("integer from junk string -> 0", marker, [ set.("editor_font_size", "abc") ])
  w.("integer from float", marker, [ set.("editor_font_size", 18.7) ])
  w.("integer from '12px'", marker, [ set.("editor_font_size", "12px") ])
  w.("integer from negative", marker, [ set.("editor_indent", -3) ])
  w.("integer from true (NoMethodError?)", marker, [ set.("editor_font_size", true) ])
  w.("integer nil", "editor_font_size = 16\n#{marker}", [ set.("editor_font_size", nil), { "op" => "reload" } ])
  w.("boolean from 'yes'", marker, [ set.("vim_mode", "yes") ])
  w.("boolean from 'false'", marker, [ set.("vim_mode", "false") ])
  w.("boolean from 0", marker, [ set.("vim_mode", 0) ])
  w.("boolean from 1", marker, [ set.("vim_mode", 1) ])
  w.("boolean from 'garbage'", marker, [ set.("vim_mode", "garbage") ])
  w.("boolean nil writes false but instance holds nil", marker, [ set.("vim_mode", nil), { "op" => "reload" } ])
  w.("boolean empty string", marker, [ set.("sidebar_visible", ""), { "op" => "reload" } ])
  # json
  w.("recent_files array", marker, [ set.("recent_files", [ "a.md", "ção/日本語.md", "x\"y.md" ]), { "op" => "reload" } ])
  w.("recent_files JSON string", marker, [ set.("recent_files", "[\"a.md\"]") ])
  w.("recent_files invalid JSON string", marker, [ set.("recent_files", "[broken") ])
  w.("recent_files empty array", marker, [ set.("recent_files", []), { "op" => "reload" } ])
  w.("recent_files with newline in entry (JSON-escaped)", marker, [ set.("recent_files", [ "a\nb.md" ]), { "op" => "reload" } ])
  w.("recent_files html chars", marker, [ set.("recent_files", [ "</script>&<>.md", "\u2028" ]) ])
  w.("modify append recent_files", "recent_files = [\"a.md\"]\n#{marker}", [ { "op" => "modify_append", "key" => "recent_files", "value" => "b.md" } ])
  w.("modify reads fresh disk state", "recent_files = [\"a.md\"]\n#{marker}",
     [ { "op" => "external_write", "fed" => "recent_files = [\"z.md\"]\n#{marker}" }, { "op" => "modify_append", "key" => "recent_files", "value" => "b.md" } ])
  w.("modify replace theme", "theme = dark\n#{marker}", [ { "op" => "modify_replace", "key" => "theme", "value" => "nord" } ])
  w.("modify unknown key", marker, [ { "op" => "modify_replace", "key" => "bogus", "value" => "x" } ])
  # custom theme
  w.("custom_theme hash value", marker, [ set.("custom_theme", theme), { "op" => "reload" } ])
  w.("custom_theme nil", "custom_theme = #{JSON.generate(theme)}\n#{marker}", [ set.("custom_theme", nil), { "op" => "reload" } ])
  w.("custom_theme invalid", marker, [ set.("custom_theme", { "version" => 1 }) ])
  # unknown / empty
  w.("set unknown key returns false, no write", "theme = dark\n", [ set.("bogus", "x") ])
  w.("update with only unknown keys returns true, no write", "theme = dark\n#{marker}", [ upd.({ "bogus" => "x" }) ])
  w.("update with empty hash", "theme = dark\n#{marker}", [ upd.({}) ])
  w.("update mix of known and unknown", "theme = dark\n#{marker}", [ upd.({ "bogus" => "x", "theme" => "nord" }) ])
  w.("uppercase key in update is skipped (case-sensitive schema)", "theme = dark\n#{marker}", [ upd.({ "THEME" => "nord" }) ])
  # file structure quirks
  w.("uppercase key line in file is replaced", "THEME = dark\n#{marker}", [ set.("theme", "nord") ])
  w.("duplicate key lines are all replaced", "theme = dark\neditor_font = x\ntheme = light\n#{marker}", [ set.("theme", "nord") ])
  w.("commented key before real key -> both written", "# theme = dark\ntheme = light\n#{marker}", [ set.("theme", "nord") ])
  w.("two commented key lines -> only first replaced", "# theme = a\n# theme = b\n#{marker}", [ set.("theme", "nord") ])
  w.("commented key without space", "#theme=old\n#{marker}", [ set.("theme", "nord") ])
  w.("indented commented key", "   #   theme = old\n#{marker}", [ set.("theme", "nord") ])
  w.("prose comment matching key=pattern is replaced", "# theme = the colours I like\n#{marker}", [ set.("theme", "nord") ])
  w.("indented real key line replaced without indentation", "    theme = old   \n#{marker}", [ set.("theme", "nord") ])
  w.("unrelated lines preserved verbatim (spacing, quotes)", "editor_font   =   'fira'  \n  vim_mode=yes\ngarbage line here\n[section]\n#{marker}", [ set.("theme", "nord") ])
  w.("whitespace-only lines normalized to empty", "theme = dark\n   \n\t\n#{marker}", [ set.("theme", "nord") ])
  w.("CRLF file converted to LF", "theme = dark\r\n# comment\r\neditor_font = x\r\n#{marker}", [ set.("theme", "nord") ])
  w.("lone CR file", "theme = dark\rvim_mode = true\r#{marker}", [ set.("vim_mode", false) ])
  w.("no trailing newline gains one", "theme = dark\n# ai_model = p", [ set.("theme", "nord") ])
  w.("multiple appends keep op order", marker, [ upd.({ "vim_mode" => true, "theme" => "nord", "editor_width" => 90 }) ])
  w.("BOM first line preserved and not matched", "\uFEFFtheme = dark\n#{marker}", [ set.("theme", "nord") ])
  w.("key line with inline comment fully replaced", "theme = dark # mine\n#{marker}", [ set.("theme", "nord") ])
  w.("file without AI section is upgraded first", "theme = dark\n", [ set.("theme", "nord") ])
  w.("empty file", "", [ set.("theme", "nord") ])
  # filesystem
  w.(".fed symlink is replaced by regular file", nil, [ set.("theme", "nord") ], tree: { "real.fed" => "theme = dark\n#{marker}", ".fed" => { "symlink" => "real.fed" } })
  w.(".fed is a directory", nil, [ set.("theme", "nord") ], tree: { ".fed/" => nil })
  w.("lock file is a symlink", "theme = dark\n#{marker}", [ set.("theme", "nord") ], tree: { "x.lock" => "", ".frankmd-config.lock" => { "symlink" => "x.lock" } })
  w.("read-only notes dir", "theme = dark\n#{marker}", [ set.("theme", "nord") ], readonly: true)
  # private helpers
  w.("generate_config_lines (unused private) with file values", "theme = dark\neditor_font_size = 18\nvim_mode = yes\nimages_path = /x\n#{marker}", [ { "op" => "generate_config_lines" } ])
  [ [ "theme", "a b" ], [ "theme", "a=b" ], [ "theme", "a\"b" ], [ "vim_mode", nil ], [ "vim_mode", "x" ], [ "editor_font_size", nil ], [ "recent_files", nil ],
    [ "media_storage_provider", "local" ], [ "media_storage_provider", "s3" ], [ "media_storage_provider", "x" ], [ "custom_theme", nil ] ].each do |k, v|
    w.("format_value #{k}=#{v.inspect}", marker, [ { "op" => "format_value", "key" => k, "value" => v } ])
  end

  cases = sc.map do |c|
    root = Golden.mktmp("wb")
    File.binwrite(File.join(root, ".fed"), c["fed"]) if c["fed"]
    Golden.build_tree(root, c["tree"]) if c["tree"]
    File.chmod(0o600, File.join(root, ".fed")) if File.file?(File.join(root, ".fed")) && !File.symlink?(File.join(root, ".fed"))
    results = nil
    log = nil
    fed_after_load = nil
    instance_values = nil
    with_clean_env(c["env"]) do
      (results, log) = with_log do
        cfg = Config.new(base_path: root)
        fed_after_load = read_fed(root)
        File.chmod(0o555, root) if c["readonly"]
        rs = c["ops"].map do |op|
          r = Golden.capture { apply_op(cfg, op, root) }
          if r == :reload
            cfg = Config.new(base_path: root)
            r = { "reloaded_file_values" => file_values(cfg) }
          end
          { "op" => op["op"], "result" => r, "instance_values" => file_values(cfg), "fed" => read_fed(root) }
        end
        File.chmod(0o755, root) if c["readonly"]
        rs
      end
    end
    reread = with_clean_env(c["env"]) { file_values(Config.new(base_path: root)) rescue nil }
    out = norm({
      "name" => c["name"], "fn" => c["ops"].map { |o| o["op"] }.uniq.join("+"),
      "input" => { "fed" => c["fed"], "tree" => c["tree"], "env" => c["env"], "readonly_root" => c["readonly"], "ops" => c["ops"] },
      "expected" => {
        "fed_after_load" => fed_after_load == c["fed"] ? "<UNCHANGED>" : (fed_after_load == $template_text ? "<TEMPLATE>" : fed_after_load),
        "steps" => results,
        "reread_file_values" => reread,
        "tree_after" => Golden.snapshot(root),
        "modes" => modes(root),
        "log" => log
      }
    }, root)
    FileUtils.chmod_R(0o755, root) rescue nil
    FileUtils.rm_rf(root)
    out
  end
  Golden.write("config_writeback", Golden.fixture(
    source: "app/models/config.rb#set/update/modify/save_keys_without_lock/format_value",
    generator: GEN,
    notes: "input.fed null => no .fed initially (template is generated by Config.new; fed_after_load \"<TEMPLATE>\"). .fed chmod 600 before load. Ops run on one Config instance; 'reload' replaces it with Config.new. " \
           "steps[i].fed = .fed content after the op; instance_values = @values after the op. set/update return false on failure (reason only in log). " \
           "'external_write' overwrites .fed behind the instance's back. reread_file_values = fresh Config.new @values at the end. Ops 'format_value'/'generate_config_lines' call private methods.",
    cases: cases
  ))
end

# ---------------------------------------------------------------- egress

def gen_egress
  cases = []
  ips = %w[
    169.254.169.254 127.0.0.1 10.0.0.5 172.16.0.1 192.168.1.10 169.254.1.1 ::1 fc00::1 fe80::1
    0.0.0.0 100.64.0.1 192.0.2.1 198.18.0.1 203.0.113.1 240.0.0.1 224.0.0.1 ff02::1 ::ffff:127.0.0.1
    93.184.216.34 2606:2800:220:1:248:1893:25c8:1946
    0.0.0.1 0.255.255.255 1.0.0.0 1.1.1.1 8.8.8.8 9.255.255.255 10.0.0.0 10.255.255.255 11.0.0.0
    100.63.255.255 100.64.0.0 100.127.255.255 100.128.0.0 126.255.255.255 127.0.0.0 127.255.255.255 128.0.0.0
    169.253.255.255 169.254.0.0 169.254.255.255 169.255.0.0 172.15.255.255 172.16.0.0 172.31.255.255 172.32.0.0
    192.0.0.0 192.0.0.255 192.0.1.0 192.0.2.0 192.0.2.255 192.0.3.0 192.88.99.1 192.167.255.255 192.168.0.0
    192.168.255.255 192.169.0.0 198.17.255.255 198.18.0.0 198.19.255.255 198.20.0.0 198.51.99.255 198.51.100.0
    198.51.100.255 198.51.101.0 203.0.112.255 203.0.113.0 203.0.113.255 203.0.114.0 223.255.255.255 224.0.0.0
    239.255.255.255 240.0.0.0 255.255.255.254 255.255.255.255
    :: ::2 ::127.0.0.1 ::ffff:0.0.0.0 ::ffff:10.0.0.1 ::ffff:8.8.8.8 ::ffff:169.254.169.254 ::ffff:7f00:1 ::ffff:100.64.0.1 ::ffff:224.0.0.1
    64:ff9b::7f00:1 64:ff9b::808:808 2002:7f00:1:: 2001:db8::1 2001::1 2606:4700:4700::1111 2a00:1450:4001:82a::200e
    fc00:: fd00::1 fdff:ffff:ffff:ffff:ffff:ffff:ffff:ffff fe00::1 fe7f::1 fe80:: febf:ffff:ffff:ffff:ffff:ffff:ffff:ffff fec0::1
    feff::1 ff00:: ff00::1 ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff 100:: 100::1 100::ffff:ffff:ffff:ffff 100:0:0:1:: 0100::0001
    FE80::ABCD 2001:DB8::A ::FFFF:127.0.0.1 fe80::1%eth0 fe80::1%25eth0
    10.0.0.0/8 8.8.8.0/24 0.0.0.0/0 ::/0
    999.1.1.1 1.2.3 01.2.3.4 1.2.3.4.5 localhost example.com 127.1 0x7f.0.0.1 2130706433 [::1] :::1
  ] + [ "", " 127.0.0.1", "127.0.0.1 " ]
  ips.each do |ip|
    exp = Golden.capture do
      addr = IPAddr.new(ip)
      { "normalized" => addr.to_s, "family" => addr.ipv4? ? "ipv4" : "ipv6", "blocked" => EgressPolicy.blocked_ip?(addr) }
    end
    cases << { "name" => "blocked_ip? #{ip.inspect}", "fn" => "blocked_ip?", "input" => { "ip" => ip }, "expected" => exp }
  end

  stub = lambda do |addrs, &blk|
    calls = []
    orig = EgressPolicy.method(:resolve)
    EgressPolicy.define_singleton_method(:resolve) { |host| calls << host; addrs.map { |a| IPAddr.new(a) } }
    begin
      [ blk.call, calls ]
    ensure
      EgressPolicy.define_singleton_method(:resolve, orig)
    end
  end
  summarize = lambda do |r|
    return r if r.is_a?(Hash)
    uri, ip = r
    { "uri" => uri.to_s, "scheme" => uri.scheme, "host" => uri.host, "hostname" => uri.hostname, "port" => uri.port, "path" => uri.path,
      "query" => uri.query, "userinfo" => uri.userinfo, "fragment" => uri.fragment, "ip" => ip }
  end
  pub = [ "93.184.216.34" ]
  stubbed = [
    [ "file:///etc/passwd", pub ], [ "http://", pub ], [ "https://does-not-exist.invalid/x.png", [] ],
    [ "http://metadata.example.com/latest/meta-data/", [ "169.254.169.254" ] ],
    [ "https://split-horizon.example.com/x.png", [ "93.184.216.34", "127.0.0.1" ] ],
    [ "https://split-horizon2.example.com/x.png", [ "127.0.0.1", "93.184.216.34" ] ],
    [ "https://example.com/image.png", pub ],
    [ "https://example.com/image.png", [ "2606:2800:220:1:248:1893:25c8:1946", "93.184.216.34" ] ],
    [ "HTTPS://EXAMPLE.COM/Image.PNG", pub ], [ "Http://example.com", pub ],
    [ "ftp://example.com/x", pub ], [ "gopher://example.com/", pub ], [ "javascript:alert(1)", pub ], [ "data:image/png;base64,AAAA", pub ],
    [ "//example.com/x.png", pub ], [ "example.com/x.png", pub ], [ "/x.png", pub ], [ "", pub ], [ "   ", pub ], [ nil, pub ],
    [ "http:example.com", pub ], [ "http:/example.com", pub ], [ "http:///example.com", pub ],
    [ "https://example.com:8443/a?b=c#frag", pub ], [ "http://example.com:0/", pub ], [ "http://example.com:65536/", pub ], [ "http://example.com:abc/", pub ],
    [ "http://user:pass@example.com/", pub ], [ "http://example.com@evil.com/", [ "10.0.0.1" ] ],
    [ "http://exämple.com/", pub ], [ "http://xn--exmple-cua.com/", pub ], [ "http://例え.jp/", pub ], [ "http://example.com/ção.png", pub ],
    [ "http://example.com/a b.png", pub ], [ "http://example.com/%2e%2e/x", pub ], [ "http://exa mple.com/", pub ],
    [ "http://[::1]/", [ "::1" ] ], [ "http://[2606:2800:220:1:248:1893:25c8:1946]/", [ "2606:2800:220:1:248:1893:25c8:1946" ] ],
    [ "http://example.com./", pub ], [ "http://.example.com/", pub ], [ "http://-example-.com/", pub ], [ "http://example_underscore.com/", pub ],
    [ "http://localhost/", [ "127.0.0.1", "::1" ] ], [ "http://LOCALHOST./", [ "127.0.0.1" ] ],
    [ "https://example.com/\u0000", pub ], [ "https://example.com/\n", pub ], [ " https://example.com/", pub ], [ "https://example.com/ ", pub ],
    [ "http://a.com/x.png", [ "::ffff:127.0.0.1" ] ], [ "http://a.com/x.png", [ "fe80::1" ] ], [ "http://a.com/x.png", [ "100.64.1.1" ] ],
    [ "http://a.com/x.png", [ "0.0.0.0" ] ], [ "http://a.com/x.png", [ "239.1.1.1" ] ], [ "http://a.com/x.png", [ "2001:db8::1" ] ]
  ]
  stubbed.each do |url, addrs|
    r, calls = stub.call(addrs) { Golden.capture { EgressPolicy.checked_target(url) } }
    cases << { "name" => "checked_target #{url.inspect} -> stub #{addrs.inspect}", "fn" => "checked_target (resolve stubbed)",
               "input" => { "url" => url, "resolve_returns" => addrs }, "expected" => { "result" => summarize.call(r), "resolve_called_with" => calls } }
  end

  # real resolution, numeric hosts only (no DNS); localhost reads /etc/hosts
  real = %w[http://127.0.0.1/ http://127.1/ http://0x7f.0.0.1/ http://0x7f000001/ http://2130706433/ http://017700000001/ http://0177.0.0.1/
            http://0/ http://0.0.0.0/ http://10.1/ http://169.254.169.254/latest http://8.8.8.8/ http://1.1.1.1:8080/x
            http://[::1]/ http://[::ffff:127.0.0.1]/ http://[2606:4700:4700::1111]/ http://[fe80::1]/ http://localhost/ http://[::]/
            http://255.255.255.255/]
  real.each do |url|
    r = Golden.capture { EgressPolicy.checked_target(url) }
    cases << { "name" => "checked_target real resolve #{url}", "fn" => "checked_target (real resolve)", "input" => { "url" => url }, "expected" => { "result" => summarize.call(r) } }
  end
  %w[127.0.0.1 127.1 0x7f.0.0.1 0x7f000001 2130706433 017700000001 0177.0.0.1 0 10.1 8.8.8.8 ::1 [::1] ::ffff:127.0.0.1 fe80::1 localhost].each do |h|
    r = Golden.capture { EgressPolicy.resolve(h).map(&:to_s) }
    cases << { "name" => "resolve #{h}", "fn" => "resolve (real)", "input" => { "host" => h }, "expected" => r }
  end
  Golden.write("egress_policy", Golden.fixture(
    source: "app/services/egress_policy.rb",
    generator: GEN,
    notes: "blocked_ip?: input string parsed with IPAddr.new (errors are IPAddr parse errors, i.e. the string is not an address); expected.normalized = IPAddr#to_s. " \
           "checked_target (resolve stubbed): EgressPolicy.resolve replaced to return the given addresses and record the host it was called with (uri.host, IPv6 keeps brackets). " \
           "Errors: EgressPolicy::BlockedError with exact messages. 'real resolve' cases use the system resolver with numeric hosts only (glibc inet_aton forms like 127.1/0x7f.0.0.1/2130706433 resolve) plus localhost (/etc/hosts: machine dependent order).",
    allowed_schemes: EgressPolicy::ALLOWED_SCHEMES,
    extra_blocked: EgressPolicy::EXTRA_BLOCKED.map { |r| "#{r}/#{r.prefix}" },
    cases: cases
  ))
end

# ---------------------------------------------------------------- custom theme

def gen_custom_theme
  base = { "version" => 1, "name" => " My colors ", "base" => "nord", "colors" => { "accent" => "#AABBCC" } }
  cases = []
  add = ->(name, raw, json: true) do
    input = json ? { "raw_json" => raw.is_a?(String) ? raw : JSON.generate(raw) } : { "raw" => raw }
    arg = json ? input["raw_json"] : raw
    cases << { "name" => name, "fn" => "CustomTheme.parse", "input" => input, "expected" => Golden.capture { CustomTheme.parse(arg) } }
  end
  add.("normalizes bounded palette (test)", base)
  add.("null JSON -> nil (test)", "null")
  [ "red", "#123", "url(https://example.com)", "#aabbcc;display:none", "var(--secret)" ].each { |v| add.("unsafe color #{v} (test)", base.merge("colors" => { "accent" => v })) }
  [ [ "unknown color key (test)", { "colors" => { "font-family" => "#aabbcc" } } ], [ "base omarchy (test)", { "base" => "omarchy" } ], [ "version 2 (test)", { "version" => 2 } ],
    [ "empty name (test)", { "name" => "" } ], [ "61-char name (test)", { "name" => "a" * 61 } ], [ "newline in name (test)", { "name" => "bad\nname" } ], [ "extra css key (test)", { "css" => "body{}" } ] ].each do |n, ch|
    add.(n, base.merge(ch))
  end
  add.("oversized whitespace string (test)", " " * (CustomTheme::MAX_BYTES + 1))
  add.("exactly MAX_BYTES whitespace (passes size, fails JSON)", " " * CustomTheme::MAX_BYTES)
  add.("controller test theme with quotes in name", { "version" => 1, "name" => "My \"theme\"", "base" => "dark", "colors" => { "accent" => "#AABBCC" } })
  # colors
  add.("all color keys", base.merge("colors" => CustomTheme::COLOR_KEYS.each_with_index.to_h { |k, i| [ k, format("#%06X", i * 99_991 % 0xFFFFFF) ] }))
  add.("empty colors", base.merge("colors" => {}))
  add.("colors not a hash", base.merge("colors" => [ "#aabbcc" ]))
  add.("colors missing", base.except("colors"))
  add.("colors null", base.merge("colors" => nil))
  [ "#aabbcc", "#AaBbCc", "#000000", "#ffffff", "aabbcc", "#aabbc", "#aabbccd", "#aabbccdd", "#gggggg", " #aabbcc", "#aabbcc ", "#aabbcc\n", "\n#aabbcc", "#ａａｂｂｃｃ", "##aabbc" ].each do |v|
    add.("color #{v.inspect}", base.merge("colors" => { "accent" => v }))
  end
  add.("color as integer", base.merge("colors" => { "accent" => 0xaabbcc }))
  add.("color null", base.merge("colors" => { "accent" => nil }))
  add.("color key uppercase", base.merge("colors" => { "ACCENT" => "#aabbcc" }))
  add.("color key order preserved", base.merge("colors" => { "text-primary" => "#111111", "accent" => "#222222", "bg-primary" => "#333333" }))
  # names
  add.("name exactly 60 chars", base.merge("name" => "n" * 60))
  add.("name 60 chars plus surrounding spaces", base.merge("name" => "  #{"n" * 60}  "))
  add.("name 60 multibyte chars", base.merge("name" => "ç" * 60))
  add.("name 61 multibyte chars", base.merge("name" => "日" * 61))
  add.("name emoji ZWJ", base.merge("name" => "👨‍👩‍👧 family"))
  add.("name combining", base.merge("name" => "Cafe\u0301"))
  add.("name RTL", base.merge("name" => "שלום"))
  add.("name only spaces", base.merge("name" => "   "))
  add.("name only NBSP (not stripped)", base.merge("name" => "\u00A0"))
  add.("name with tab", base.merge("name" => "a\tb"))
  add.("name with DEL", base.merge("name" => "a\u007Fb"))
  add.("name with C1 control U+0085", base.merge("name" => "a\u0085b"))
  add.("name with zero-width space", base.merge("name" => "a\u200Bb"))
  add.("name with leading tab stripped? (tab is cntrl)", base.merge("name" => "\tname"))
  add.("name trailing newline", base.merge("name" => "name\n"))
  add.("name integer", base.merge("name" => 5))
  add.("name missing", base.except("name"))
  add.("name with html", base.merge("name" => "<script>alert(1)</script>"))
  # base / version / format
  CustomTheme::BASE_THEMES.each { |b| add.("base #{b}", base.merge("base" => b)) }
  add.("base uppercase", base.merge("base" => "Dark"))
  add.("base missing", base.except("base"))
  add.("base custom", base.merge("base" => "custom"))
  add.("version float 1.0", "{\"version\":1.0,\"name\":\"x\",\"base\":\"dark\",\"colors\":{}}")
  add.("version string", base.merge("version" => "1"))
  add.("version missing", base.except("version"))
  add.("version true", base.merge("version" => true))
  add.("top-level array", "[]")
  add.("top-level string", "\"theme\"")
  add.("top-level number", "1")
  add.("top-level object empty", "{}")
  add.("malformed JSON", "{broken")
  add.("empty string", "")
  add.("JSON with duplicate keys (last wins)", "{\"version\":1,\"name\":\"a\",\"name\":\"b\",\"base\":\"dark\",\"colors\":{}}")
  add.("JSON with surrounding whitespace", "  {\"version\":1,\"name\":\"x\",\"base\":\"dark\",\"colors\":{}}  \n")
  add.("check order: bad version and bad name -> format error", base.merge("version" => 2, "name" => ""))
  add.("check order: bad name and bad base -> name error", base.merge("name" => "", "base" => "x"))
  add.("check order: bad base and bad colors -> base error", base.merge("base" => "x", "colors" => { "a" => 1 }))
  big = base.merge("name" => "big", "pad" => nil).except("pad")
  add.("large but under limit (padded whitespace)", JSON.generate(big) + " " * (CustomTheme::MAX_BYTES - JSON.generate(big).bytesize))
  add.("one byte over limit", JSON.generate(big) + " " * (CustomTheme::MAX_BYTES - JSON.generate(big).bytesize + 1))
  # non-string raws (Hash passed directly)
  add.("hash input (not size-checked)", base, json: false)
  add.("hash input large name list", base.merge("colors" => { "accent" => "#ABCDEF" }), json: false)
  add.("nil input", nil, json: false)
  add.("array input", [ 1 ], json: false)
  add.("integer input", 5, json: false)
  Golden.write("custom_theme", Golden.fixture(
    source: "app/services/custom_theme.rb#parse",
    generator: GEN,
    notes: "input.raw_json is passed as a String (JSON-parsed by parse); input.raw is passed as an already-decoded value (Hash etc., not size-checked). Errors: ArgumentError (validation) or JSON::ParserError (malformed JSON; message text from Ruby json gem).",
    version: CustomTheme::VERSION, max_bytes: CustomTheme::MAX_BYTES, base_themes: CustomTheme::BASE_THEMES, color_keys: CustomTheme::COLOR_KEYS,
    cases: cases
  ))
end

FileUtils.mkdir_p(Golden::OUT_DIR)
gen_template
gen_parse
gen_env
gen_writeback
gen_egress
gen_custom_theme
