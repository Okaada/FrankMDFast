# Golden fixtures: translations — locale list, flattened keys per locale, interpolation vars,
# keys required by coverage tests, and the exact JSON GET /translations serves per locale.
#
#   cd ~/Projects/FrankMD && GOLDEN_TMP=$SCRATCH/notes/misc mise exec ruby@3.4.8 -- \
#     bundle exec rails runner /mnt/data/Projects/FrankMDFast/tools/golden/ruby/translations.rb
require_relative "../lib/golden"
require "yaml"

OUT_SUB = File.join(Golden::OUT_DIR, "translations")
FileUtils.mkdir_p(OUT_SUB)
cases = []

LOCALES = Rails.application.config.i18n.available_locales.map(&:to_s)
LOCALE_FILES = Dir.children(Rails.root.join("config/locales")).sort

def flatten(h, prefix = nil, out = {})
  h.each do |k, v|
    path = [prefix, k.to_s].compact.join(".")
    v.is_a?(Hash) ? flatten(v, path, out) : out[path] = v
  end
  out
end

def interp_vars(v)
  case v
  when String then v.scan(/%\{(\w+)\}/).flatten.uniq.sort
  when Array then v.flat_map { |x| interp_vars(x) }.uniq.sort
  else []
  end
end

cases << { "name" => "locales", "fn" => "locales", "input" => {},
           "expected" => { "available_locales" => LOCALES, "default_locale" => I18n.default_locale.to_s,
                           "fallbacks_config" => Rails.application.config.i18n.fallbacks.inspect,
                           "locale_files" => LOCALE_FILES,
                           "i18n_fallbacks_chain" => LOCALES.to_h { |l| [l, I18n.fallbacks[l.to_sym].map(&:to_s)] } } }

raw = LOCALES.to_h { |l| [l, YAML.safe_load_file(Rails.root.join("config/locales/#{l}.yml")).fetch(l)] }
flat = raw.transform_values { |t| flatten(t) }
en_keys = flat["en"].keys

LOCALES.each do |l|
  leaves = flat[l]
  types = leaves.transform_values { |v| v.class.name }.reject { |_, t| t == "String" }
  File.write(File.join(OUT_SUB, "#{l}.flat.json"), JSON.pretty_generate(leaves.sort.to_h) + "\n")
  cases << { "name" => "flattened keys #{l}", "fn" => "flatten_leaves",
             "input" => { "locale" => l, "file" => "config/locales/#{l}.yml" },
             "expected" => { "leaf_count" => leaves.size, "keys" => leaves.keys.sort, "values_file" => "translations/#{l}.flat.json",
                             "non_string_leaves" => types,
                             "missing_vs_en" => (en_keys - leaves.keys).sort, "extra_vs_en" => (leaves.keys - en_keys).sort,
                             "blank_values" => leaves.select { |_, v| v.is_a?(String) && v.strip.empty? }.keys.sort } }
end

# interpolation variables per key per locale (only keys having any)
LOCALES.each do |l|
  vars = flat[l].transform_values { |v| interp_vars(v) }.reject { |_, v| v.empty? }.sort.to_h
  mismatch = flat[l].keys.select { |k| flat["en"].key?(k) && interp_vars(flat[l][k]) != interp_vars(flat["en"][k]) }.sort
  cases << { "name" => "interpolation variables #{l}", "fn" => "interpolation_vars", "input" => { "locale" => l },
             "expected" => { "vars" => vars, "keys_with_vars_differing_from_en" => mismatch } }
end

# translation_coverage_test logic (exact failure strings) incl. the synthetic test input
def coverage_failures(english, translations_by_locale)
  english_leaves = flatten(english)
  translations_by_locale.each_with_object([]) do |(locale, translations), failures|
    tl = flatten(translations)
    english_leaves.each_key do |path|
      unless tl.key?(path)
        failures << "#{locale}: missing key #{path}"
        next
      end
      value = tl[path]
      next if value.is_a?(String) && !value.strip.empty?
      failures << "#{locale}: blank or non-string value for #{path} (got #{value.class})"
    end
  end
end
cases << { "name" => "coverage failures synthetic (from translation_coverage_test)", "fn" => "translation_failures",
           "input" => { "english" => { "dialogs" => { "help" => { "find" => "Find" } }, "errors" => { "required" => "Required" } },
                        "translations" => { "pt-BR" => { "dialogs" => { "help" => {} }, "errors" => { "required" => "   " } },
                                            "ja" => { "dialogs" => { "help" => { "find" => false } }, "errors" => { "required" => "必須" } } } },
           "expected" => coverage_failures({ "dialogs" => { "help" => { "find" => "Find" } }, "errors" => { "required" => "Required" } },
                                           { "pt-BR" => { "dialogs" => { "help" => {} }, "errors" => { "required" => "   " } },
                                             "ja" => { "dialogs" => { "help" => { "find" => false } }, "errors" => { "required" => "必須" } } }) }
cases << { "name" => "coverage failures real locales", "fn" => "translation_failures",
           "input" => { "english" => "config/locales/en.yml", "locales" => (LOCALES - ["en"]).sort },
           "expected" => coverage_failures(raw["en"], (LOCALES - ["en"]).sort.to_h { |l| [l, raw[l]] }) }

# required key lists mined from test sources
def eval_array(src, const) = eval(src[/#{const} = ((?:%w)?\[.*?\])\.freeze/m, 1]) # rubocop:disable Security/Eval
{ "test/i18n_media_upload_keys_test.rb" => "REQUIRED_KEYS", "test/i18n_vim_keys_test.rb" => "REQUIRED_KEYS" }.each do |file, const|
  src = File.read(Rails.root.join(file))
  keys = eval_array(src, const)
  locales = eval_array(src, "LOCALES")
  present = locales.to_h { |l| [l, keys.to_h { |k| v = k.split(".").reduce(raw[l]) { |n, s| n.is_a?(Hash) ? n[s] : nil }; [k, v.present?] }] }
  cases << { "name" => "required keys #{file}", "fn" => "required_keys", "input" => { "test_file" => file, "locales" => locales, "keys" => keys },
             "expected" => { "present" => present, "all_present" => present.values.all? { |h| h.values.all? } } }
end
ctrl_src = File.read(Rails.root.join("test/controllers/translations_controller_test.rb"))
ctrl_lists = ctrl_src.scan(/required_(?:keys|paths) = %w\[(.*?)\]/m).map { |m| m.first.split }
sections = %w[common status errors]
ctrl_lists.each_with_index do |list, i|
  prefix = i < 3 ? sections[i] : nil
  cases << { "name" => "required keys translations_controller_test list #{i} #{prefix || 'dialog paths'}", "fn" => "required_keys",
             "input" => { "test_file" => "test/controllers/translations_controller_test.rb", "section" => prefix, "keys" => list },
             "expected" => LOCALES.to_h { |l| [l, list.to_h { |k| path = prefix ? [prefix, *k.split(".")] : k.split("."); v = path.reduce(raw[l]) { |n, s| n.is_a?(Hash) ? n[s] : nil }; [k, v] }] } }
end

# GET /translations served JSON (real request through the Rails stack)
root = Golden.mktmp("translations")
served_index = {}
Golden.with_env("NOTES_PATH" => root, "FRANKMD_LOCALE" => nil) do
  Rails.application.config.hosts.clear if Rails.application.config.respond_to?(:hosts)
  session = ActionDispatch::Integration::Session.new(Rails.application)
  session.host! "localhost"
  requests = LOCALES.map { |l| [l, { "FRANKMD_LOCALE" => l }, nil] } +
             [["invalid_locale", { "FRANKMD_LOCALE" => "invalid_locale" }, nil], ["unset", { "FRANKMD_LOCALE" => nil }, nil],
              ["param_ja", { "FRANKMD_LOCALE" => nil }, "ja"], ["param_invalid", { "FRANKMD_LOCALE" => "es" }, "xx"],
              ["lowercase_pt-br", { "FRANKMD_LOCALE" => "pt-br" }, nil], ["pt_BR_underscore", { "FRANKMD_LOCALE" => "pt_BR" }, nil],
              ["fed_es", { "FRANKMD_LOCALE" => "ja" }, nil, "locale = es\n"]]
  requests.each do |label, env, param, fed|
    FileUtils.rm_f(File.join(root, ".fed"))
    File.write(File.join(root, ".fed"), fed) if fed
    Golden.with_env(env) do
      I18n.locale = :en
      path = param ? "/translations?locale=#{param}" : "/translations"
      session.get path, headers: { "Accept" => "application/json", "User-Agent" => "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36" }
      body = session.response.body
      parsed = JSON.parse(body) rescue nil
      file = "translations/served_#{label}.json"
      File.write(File.join(Golden::OUT_DIR, file), JSON.pretty_generate(parsed || { "raw" => body }) + "\n")
      served_index[label] = parsed && parsed["locale"]
      cases << { "name" => "GET /translations #{label}", "fn" => "translations_controller#show",
                 "input" => { "env" => env, "query_locale" => param, "fed" => fed, "path" => path },
                 "expected" => { "status" => session.response.status, "content_type" => session.response.media_type,
                                 "locale" => parsed && parsed["locale"], "top_level_sections" => parsed && parsed["translations"]&.keys,
                                 "body_file" => file,
                                 "leaf_count" => parsed && flatten(parsed["translations"]).size,
                                 "en_fill_keys" => parsed && (flatten(parsed["translations"]).keys - flatten(raw.fetch(parsed["locale"], {}).slice(*parsed["translations"].keys)).keys).sort } }
    end
  end
end
FileUtils.rm_rf(root)
I18n.locale = :en

Golden.write("translations", Golden.fixture(
  source: "config/locales/*.yml + app/controllers/translations_controller.rb", generator: "tools/golden/ruby/translations.rb", cases: cases,
  notes: "Full flattened values per locale in translations/<locale>.flat.json (dotted path -> value). " \
         "Served JSON bodies (exact parsed body of GET /translations, real Rails request, NOTES_PATH=temp) in translations/served_<label>.json; " \
         "en_fill_keys = served leaves absent from the locale's own config/locales YAML: filled from en by deep merge OR coming from Rails gem locale files (activemodel errors.messages.*, errors.format, errors.connection_refused...). " \
         "Locale resolution order: ?locale= param > Config 'locale' (.fed > FRANKMD_LOCALE env > 'en'); invalid values keep previous I18n.locale (reset to en per request here). " \
         "Interpolation vars are %{name} tokens."
))
