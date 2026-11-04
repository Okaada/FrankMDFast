# Golden fixtures: OmarchyThemeService (alacritty.toml -> CSS variables)
#
# The service hard-codes ~/.config/omarchy/current/{theme.name,theme/alacritty.toml} as constants computed at
# load time. We NEVER touch the real ~/.config: the constants are re-pointed at a temp dir under $GOLDEN_TMP.
#
# Real themes: ~/Projects/omarchy/themes/<name>/ ship only colors.toml today; omarchy renders
# current/theme/alacritty.toml from default/themed/alacritty.toml.tpl using `omarchy-theme-color --all`.
# We reproduce that render (simple {{ key }} substitution, which is all that template uses) and embed both
# colors.toml and the rendered alacritty.toml so the Rust test needs no omarchy checkout.
#
#   cd ~/Projects/FrankMD && GOLDEN_TMP=$SCRATCH/notes/misc mise exec ruby@3.4.8 -- \
#     bundle exec rails runner /mnt/data/Projects/FrankMDFast/tools/golden/ruby/omarchy_theme.rb
require_relative "../lib/golden"
require "open3"

OMARCHY = File.expand_path(ENV.fetch("OMARCHY", "~/Projects/omarchy"))
cases = []
TMP = Golden.mktmp("omarchy")
FAKE_HOME_CONFIG = File.join(TMP, "home", ".config", "omarchy", "current")

S = OmarchyThemeService
%i[OMARCHY_DIR THEME_NAME_FILE ALACRITTY_FILE].each { |c| S.send(:remove_const, c) }
S.const_set(:OMARCHY_DIR, FAKE_HOME_CONFIG)
S.const_set(:THEME_NAME_FILE, File.join(FAKE_HOME_CONFIG, "theme.name"))
S.const_set(:ALACRITTY_FILE, File.join(FAKE_HOME_CONFIG, "theme", "alacritty.toml"))

def reset_current
  FileUtils.rm_rf(File.join(TMP, "home"))
  FileUtils.rm_rf(File.join(TMP, "themes"))
end

# files: { "theme.name" => str|nil|{"symlink"=>..}, "theme/alacritty.toml" => str, ... } relative to current/
def run_case(files, extra: {})
  reset_current
  FileUtils.mkdir_p(FAKE_HOME_CONFIG)
  files.each do |rel, v|
    next if v.nil?
    path = File.join(FAKE_HOME_CONFIG, rel)
    if rel.end_with?("/")
      FileUtils.mkdir_p(path)
    elsif v.is_a?(Hash) && v["symlink"]
      FileUtils.mkdir_p(File.dirname(path))
      File.symlink(v["symlink"].sub("<THEMES>", File.join(TMP, "themes")), path)
    elsif v.is_a?(Hash) && v["mode"]
      FileUtils.mkdir_p(File.dirname(path))
      File.write(path, v["content"])
      File.chmod(v["mode"], path)
    else
      FileUtils.mkdir_p(File.dirname(path))
      File.binwrite(path, v)
    end
  end
  extra.each do |rel, v|
    path = File.join(TMP, "themes", rel)
    FileUtils.mkdir_p(File.dirname(path))
    File.binwrite(path, v)
  end
  avail = S.available?
  data = Golden.capture { S.theme_data }
  { "available" => avail, "theme_data" => Golden.normalize(data, TMP) }
end

def render_alacritty(colors_file)
  out, err, st = Open3.capture3("bash", File.join(OMARCHY, "bin/omarchy-theme-color"), "--file", colors_file, "--all",
                                unsetenv_others: false)
  raise "omarchy-theme-color failed: #{err}" unless st.success?
  vals = out.lines.to_h { |l| l.chomp.split("\t", 2) }
  tpl = File.read(File.join(OMARCHY, "default/themed/alacritty.toml.tpl"))
  tpl.gsub(/\{\{\s*([A-Za-z0-9_]+)\s*\}\}/) { vals.fetch(Regexp.last_match(1)) { Regexp.last_match(0) } }
end

omarchy_sha = `git -C #{OMARCHY} rev-parse HEAD 2>/dev/null`.strip

# ---------------------------------------------------------------- every real theme
Dir.children(File.join(OMARCHY, "themes")).sort.each do |name|
  dir = File.join(OMARCHY, "themes", name)
  colors_file = File.join(dir, "colors.toml")
  next unless File.file?(colors_file)
  alacritty = File.exist?(File.join(dir, "alacritty.toml")) ? File.read(File.join(dir, "alacritty.toml")) : render_alacritty(colors_file)
  theme_name = "#{name}\n" # omarchy-theme-set: echo "$THEME_NAME" > theme.name
  files = { "theme.name" => theme_name, "theme/alacritty.toml" => alacritty }
  cases << { "name" => "theme #{name}", "fn" => "theme_data",
             "input" => { "theme" => name, "colors_toml" => File.read(colors_file), "files" => files },
             "expected" => run_case(files) }
end
REAL_COUNT = cases.size

# ---------------------------------------------------------------- tests + edge cases
def test_toml(background:, foreground:, low_contrast:)
  <<~TOML
    [colors.primary]
    background = "#{background}"
    foreground = "#{foreground}"
    dim_foreground = "#{low_contrast}"
    [colors.normal]
    black = "#{low_contrast}"
    red = "#ff5555"
    green = "#00ff00"
    yellow = "#ffff00"
    blue = "#0000ff"
    magenta = "#ff00ff"
    cyan = "#00ffff"
    white = "#ffffff"
    [colors.bright]
    blue = "#8080ff"
    cyan = "#80ffff"
    green = "#80ff80"
    yellow = "#ffff80"
  TOML
end
FULL = test_toml(background: "#141414", foreground: "#f0f0f0", low_contrast: "#202020")
EDGE = {
  "test: dark palette" => { "theme.name" => "Test Theme\n", "theme/alacritty.toml" => FULL },
  "test: light palette" => { "theme.name" => "Test Theme\n", "theme/alacritty.toml" => test_toml(background: "#eeeeee", foreground: "#222222", low_contrast: "#dddddd") },
  "missing theme.name" => { "theme/alacritty.toml" => FULL },
  "missing alacritty.toml" => { "theme.name" => "x\n" },
  "missing both" => {},
  "empty alacritty.toml" => { "theme.name" => "Empty\n", "theme/alacritty.toml" => "" },
  "empty theme.name" => { "theme.name" => "", "theme/alacritty.toml" => FULL },
  "theme.name whitespace/no newline/crlf" => { "theme.name" => "  My Theme \r\n\r\n", "theme/alacritty.toml" => FULL },
  "theme.name multi-line" => { "theme.name" => "first\nsecond\n", "theme/alacritty.toml" => FULL },
  "theme.name unicode" => { "theme.name" => "日本語 テーマ 🎨\n", "theme/alacritty.toml" => FULL },
  "theme.name is a directory" => { "theme.name/" => true, "theme/alacritty.toml" => FULL },
  "alacritty.toml is a directory" => { "theme.name" => "x\n", "theme/alacritty.toml/" => true },
  "unreadable alacritty.toml (EACCES)" => { "theme.name" => "x\n", "theme/alacritty.toml" => { "mode" => 0o000, "content" => FULL } },
  "dangling theme symlink" => { "theme.name" => "x\n", "theme" => { "symlink" => "<THEMES>/missing" } },
  "uppercase hex and single quotes" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\nbackground = '#ABCDEF'\nforeground = \"#FfFfFf\"\n" },
  "unquoted values" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\nbackground = #101010\nforeground=#eeeeee\n" },
  "0x hex not accepted" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\nbackground = \"0x101010\"\nforeground = \"#eeeeee\"\n" },
  "short / long / bad hex" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\nbackground = \"#fff\"\nforeground = \"#eeeeeeff\"\n[colors.normal]\nred = \"#gggggg\"\nblue = \"#12345\"\ngreen = \"#00ff00\"\n" },
  "inline comments and trailing spaces" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\nbackground = \"#202020\" # bg\nforeground = \"#e0e0e0\"   \n  [colors.normal]  \n  red = \"#ff0000\"\n# blue = \"#0000ff\"\n" },
  "section with comment / spaces in header" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary] # c\nbackground = \"#202020\"\n[ colors.normal ]\nred = \"#ff0000\"\n[colors.bright]\nred = \"#ff8888\"\n" },
  "keys outside sections ignored" => { "theme.name" => "x\n", "theme/alacritty.toml" => "background = \"#ffffff\"\n[window]\nbackground = \"#ffffff\"\n[colors.primary]\nforeground = \"#111111\"\n" },
  "nested sections like colors.search.matches ignored" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\nbackground = \"#000000\"\n[colors.search.matches]\nbackground = \"#ffff00\"\nforeground = \"#ff0000\"\n" },
  "unknown colors.* section" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.dim]\nred = \"#ff0000\"\n[colors.vi_mode_cursor]\ncursor = \"#123456\"\n" },
  "cursor and selection sections" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\nbackground = \"#1e1e2e\"\nforeground = \"#cdd6f4\"\n[colors.cursor]\ntext = \"#1e1e2e\"\ncursor = \"#f5e0dc\"\n[colors.selection]\ntext = \"#cdd6f4\"\nbackground = \"#45475a\"\n" },
  "duplicate keys last wins" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\nbackground = \"#000000\"\nbackground = \"#ffffff\"\n[colors.primary]\nforeground = \"#000000\"\n" },
  "crlf line endings" => { "theme.name" => "crlf\r\n", "theme/alacritty.toml" => FULL.gsub("\n", "\r\n") },
  "lone cr line endings" => { "theme.name" => "cr\r", "theme/alacritty.toml" => FULL.gsub("\n", "\r") },
  "tabs around =" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\n\tbackground\t=\t\"#303030\"\t\n" },
  "mismatched quotes" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\nbackground = \"#303030'\nforeground = '#eeeeee\n" },
  "dashed key not matched by \\w" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\ndim-foreground = \"#777777\"\nbackground = \"#000000\"\n" },
  "pure black background" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\nbackground = \"#000000\"\nforeground = \"#000000\"\n" },
  "pure white background" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\nbackground = \"#ffffff\"\nforeground = \"#ffffff\"\n" },
  "mid-grey luminance boundary #808080" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\nbackground = \"#808080\"\n" },
  "mid-grey #7f7f7f" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\nbackground = \"#7f7f7f\"\n" },
  "lighten clamps at white (#f8f8f8 dark? no)" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\nbackground = \"#0a0a0a\"\n[colors.normal]\nblack = \"#fafafa\"\n" },
  "only foreground (bg defaults)" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\nforeground = \"#00ff00\"\n" },
  "only normal colors" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.normal]\nred = \"#cc0000\"\ngreen = \"#00cc00\"\nyellow = \"#cccc00\"\nblue = \"#0000cc\"\nmagenta = \"#cc00cc\"\ncyan = \"#00cccc\"\nwhite = \"#cccccc\"\nblack = \"#333333\"\n" },
  "only bright colors" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.bright]\nred = \"#ff6666\"\ngreen = \"#66ff66\"\nyellow = \"#ffff66\"\nblue = \"#6666ff\"\ncyan = \"#66ffff\"\n" },
  "theme dir symlinked into a themes dir" => { "theme.name" => "linked\n", "theme" => { "symlink" => "<THEMES>/linked" } },
  "theme.name symlink" => { "theme.name" => { "symlink" => "<THEMES>/name.txt" }, "theme/alacritty.toml" => FULL },
  "invalid utf8 in alacritty.toml" => { "theme.name" => "x\n", "theme/alacritty.toml" => "[colors.primary]\n# \xFF\xFE\nbackground = \"#222222\"\n".b }
}
EXTRA = { "theme dir symlinked into a themes dir" => { "linked/alacritty.toml" => FULL },
          "theme.name symlink" => { "name.txt" => "Via Symlink\n" } }

EDGE.each do |label, files|
  extra = EXTRA.fetch(label, {})
  enc_files = files.transform_values do |v|
    v.is_a?(String) && !v.dup.force_encoding("UTF-8").valid_encoding? ? { "base64" => [v.b].pack("m0") } : (v == true ? nil : v)
  end
  cases << { "name" => "edge #{label}", "fn" => "theme_data", "input" => { "files" => enc_files, "themes_dir_files" => extra },
             "expected" => run_case(files.transform_values { |v| v == true ? "" : v }.reject { |k, _| k.end_with?("/") }
                                         .merge(files.select { |k, _| k.end_with?("/") }), extra: extra) }
end

# ---------------------------------------------------------------- private helper units
HEX = ["#000000", "#ffffff", "#808080", "#7f7f7f", "#1e1e2e", "#eff1f5", "#ff0000", "#00ff00", "#0000ff", "#ABCDEF", "#121212", "#f8f8f8", "#050505"]
HEX.each do |h|
  cases << { "name" => "dark_background? #{h}", "fn" => "dark_background?", "input" => { "hex" => h }, "expected" => S.send(:dark_background?, h) }
  cases << { "name" => "relative_luminance #{h}", "fn" => "relative_luminance", "input" => { "hex" => h }, "expected" => S.send(:relative_luminance, h) }
  [3, 7, 10, 12, 15].each do |p|
    cases << { "name" => "lighten #{h} #{p}", "fn" => "lighten", "input" => { "hex" => h, "percent" => p }, "expected" => S.send(:lighten, h, p) }
    cases << { "name" => "darken #{h} #{p}", "fn" => "darken", "input" => { "hex" => h, "percent" => p }, "expected" => S.send(:darken, h, p) }
  end
end
cases << { "name" => "dark_background? nil", "fn" => "dark_background?", "input" => { "hex" => nil }, "expected" => S.send(:dark_background?, nil) }
[["#ffffff", "#000000"], ["#f0f0f0", "#141414"], ["#cdd6f4", "#1e1e2e"], ["#ff0000", "#00ff00"], ["#808080", "#808080"]].each do |a, b|
  [0, 0.08, 0.25, 0.45, 0.5, 0.65, 1].each do |f|
    cases << { "name" => "blend #{a} #{b} #{f}", "fn" => "blend", "input" => { "fg" => a, "bg" => b, "factor" => f }, "expected" => S.send(:blend, a, b, f) }
  end
  cases << { "name" => "contrast_ratio #{a} #{b}", "fn" => "contrast_ratio", "input" => { "fg" => a, "bg" => b }, "expected" => S.send(:contrast_ratio, a, b) }
end
[["#202020", "#1b1b1b"], ["#dddddd", "#dcdcdc"], ["#ff0000", "#1e1e2e"], ["#0000ff", "#000000"], ["#ffff00", "#ffffff"],
 ["#808080", "#808080"], ["#777777", "#777777"], ["#f0f0f0", "#262626"], ["#585b70", "#313244"], ["#ff00ff", "#ff00ff"]].each do |fg, bg|
  cases << { "name" => "ensure_contrast #{fg} on #{bg}", "fn" => "ensure_contrast", "input" => { "fg" => fg, "bg" => bg }, "expected" => S.send(:ensure_contrast, fg, bg) }
end
[[0, 0, 0], [255, 255, 255], [-5, 300, 127.5], [12.4, 12.5, 12.6], [254.5, 0.49, 1.5]].each do |r, g, b|
  cases << { "name" => "rgb_to_hex #{[r, g, b].inspect}", "fn" => "rgb_to_hex", "input" => { "r" => r, "g" => g, "b" => b }, "expected" => S.send(:rgb_to_hex, r, g, b) }
end
["#abcdef", "abcdef", "#ABCDEF", "#fff", "#zzzzzz"].each do |h|
  cases << { "name" => "hex_to_rgb #{h}", "fn" => "hex_to_rgb", "input" => { "hex" => h }, "expected" => Golden.capture { S.send(:hex_to_rgb, h) } }
end
cases << { "name" => "parse_alacritty test toml", "fn" => "parse_alacritty", "input" => { "content" => FULL },
           "expected" => S.send(:parse_alacritty, FULL).transform_keys(&:to_s) }
cases << { "name" => "map_variables empty colors (all defaults)", "fn" => "map_variables", "input" => { "colors" => {} },
           "expected" => S.send(:map_variables, {}) }
cases << { "name" => "constants (path layout relative to HOME)", "fn" => "constants", "input" => {},
           "expected" => { "OMARCHY_DIR" => "~/.config/omarchy/current", "THEME_NAME_FILE" => "~/.config/omarchy/current/theme.name",
                           "ALACRITTY_FILE" => "~/.config/omarchy/current/theme/alacritty.toml",
                           "note" => "File.expand_path('~/.config/omarchy/current') evaluated once at class load (HOME at boot)" } }

FileUtils.rm_rf(TMP)
Golden.write("omarchy_theme", Golden.fixture(
  source: "app/services/omarchy_theme_service.rb", generator: "tools/golden/ruby/omarchy_theme.rb", cases: cases,
  omarchy_sha: omarchy_sha, real_theme_count: REAL_COUNT,
  notes: "theme_data cases: input.files are paths relative to <HOME>/.config/omarchy/current (the service constants were re-pointed " \
         "at a temp dir; real ~/.config never touched). 'theme <name>' cases: colors_toml is the omarchy theme's colors.toml " \
         "(omarchy repo at omarchy_sha) and files['theme/alacritty.toml'] is what omarchy renders from default/themed/alacritty.toml.tpl " \
         "via `omarchy-theme-color --all` (NOTE current omarchy writes to ~/.local/state/omarchy/current, not ~/.config/omarchy/current). " \
         "Symlink values {symlink: '<THEMES>/x'} point into a sibling temp themes dir built from input.themes_dir_files. " \
         "files value null = directory at that path ('/'-suffixed key). {mode: 0} = file chmod 000 (EACCES unless running as root). " \
         "expected.available = OmarchyThemeService.available?; theme_data keys are symbols in Ruby (theme_name, variables, is_dark). " \
         "Float results (relative_luminance, contrast_ratio) are Ruby Float -> JSON shortest round-trip repr."
))
