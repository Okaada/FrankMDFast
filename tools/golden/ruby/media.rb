# Golden fixtures: pure-ish media helpers — UploadStorage, MediaService, ImagesService (local paths),
# MediaLibraryService, ImageLibraryService (managed). No network: S3 cases only exercise key/url/config logic.
#
#   cd ~/Projects/FrankMD && GOLDEN_TMP=$SCRATCH/notes/misc mise exec ruby@3.4.8 -- \
#     bundle exec rails runner /mnt/data/Projects/FrankMDFast/tools/golden/ruby/media.rb
require_relative "../lib/golden"
require "active_support/testing/time_helpers"
require "rack/test"
require "aws-sdk-s3"
require "base64"

include ActiveSupport::Testing::TimeHelpers

cases = []

# ---- determinism: SecureRandom.hex(n) -> first 2n chars of "0123456789abcdef" repeated
HEX_PATTERN = "0123456789abcdef"
SecureRandom.singleton_class.prepend(Module.new do
  def hex(n = 16) = (HEX_PATTERN * ((2 * n / 16) + 1))[0, 2 * n]
end)
PINNED = { "process_TZ" => "UTC", "rails_zone" => "UTC", "time" => "2026-01-01T12:00:00Z",
           "securerandom_hex" => "SecureRandom.hex(n) returns the first 2n chars of '0123456789abcdef' repeated (hex(4)='01234567', hex(8)='0123456789abcdef', hex(16)='0123456789abcdef0123456789abcdef')" }.freeze
ENV["TZ"] = "UTC"

ENV_CLEAR = %w[AWS_ACCESS_KEY_ID AWS_SECRET_ACCESS_KEY AWS_S3_BUCKET AWS_REGION AWS_S3_ENDPOINT AWS_S3_FORCE_PATH_STYLE
               IMAGES_PATH IMAGE_UPLOAD_EXTENSIONS VIDEO_UPLOAD_EXTENSIONS FRANKMD_LOCALE].to_h { |k| [k, nil] }.freeze

def enc(v)
  case v
  when String then v.dup.force_encoding("UTF-8").valid_encoding? ? v.dup.force_encoding("UTF-8") : { "base64" => [v.b].pack("m0") }
  when Array then v.map { |x| enc(x) }
  when Hash then v.to_h { |k, x| [k.to_s, enc(x)] }
  when Symbol then v.to_s
  else v
  end
end

# Runs block with NOTES_PATH=<ROOT> containing `fed` (.fed text or nil) + tree, clock pinned.
def env_case(fed: nil, tree: {}, env: {}, time: Time.utc(2026, 1, 1, 12, 0, 0))
  root = Golden.mktmp("media")
  Golden.build_tree(root, tree)
  File.write(File.join(root, ".fed"), fed) if fed
  Golden.with_env(ENV_CLEAR.merge("NOTES_PATH" => root).merge(env)) do
    travel_to(time) { yield root }
  end
ensure
  FileUtils.rm_rf(root) if root
end

def snapshot_wo_fed(root) = Golden.snapshot(root).reject { |k, _| k == ".fed" }

def add(cases, fn, name, input, expected)
  cases << { "name" => "#{fn} #{name}", "fn" => fn, "input" => enc(input), "expected" => enc(expected) }
end

NAMES = [
  "clip.mp4", "my cool clip!.mp4", "../../../etc/evil.mp4", "../../etc/pa ss!.mp4", "video.mp4", "photo.png",
  "../ev il.png", "a b c.JPG", "Ação Rápida.png", "日本語.png", "שלום.webm", "👨‍👩‍👧.gif", "e\u0301.png",
  "tab\tname.png", "nbsp\u00a0x.png", "new\nline.png", "back\\slash\\x.png", "C:\\Users\\x.png", ".hidden.png",
  "..png", "...", "", " ", "noext", "double.tar.gz", "UPPER.PNG", "dots...png", "-dash-.png", "under_score.png",
  "a" * 300 + ".png", "%2e%2e%2fx.png", "nul\0byte.png", "quote\"s'.png", "semi;colon.png", "<script>.png", nil
].freeze

# ---------------------------------------------------------------- dest_filename
NAMES.each do |n|
  env_case { add(cases, "UploadStorage.dest_filename", n.inspect[0, 50], { "original_filename" => n, "pinned" => PINNED }, Golden.capture { UploadStorage.dest_filename(n) }) }
end
env_case(env: { "TZ" => "America/Sao_Paulo" }, time: Time.utc(2026, 1, 1, 1, 30, 0)) do
  ENV["TZ"] = "America/Sao_Paulo"
  add(cases, "UploadStorage.dest_filename", "process TZ America/Sao_Paulo uses local Time.now",
      { "original_filename" => "x.png", "pinned" => PINNED.merge("process_TZ" => "America/Sao_Paulo", "time" => "2026-01-01T01:30:00Z") },
      UploadStorage.dest_filename("x.png"))
  ENV["TZ"] = "UTC"
end

# ---------------------------------------------------------------- sanitize_s3_key
SKEYS = ["blog/2026/hero", "../../etc/passwd", "/a//b/", "a/./b", "my folder/a b", "../..", nil, "", "/", "//", ".",
         "...", "a/../b", "a\\b", "日本/語", "a b/c d.png", "x/y.z/", " / ", "%2e%2e/x", "a\0b", 123, "trailing/.",
         "emoji/👨‍👩‍👧.png", "UPPER/Case.PNG", "-_.~/!@#"]
SKEYS.each { |k| add(cases, "UploadStorage.sanitize_s3_key", k.inspect, { "value" => k }, Golden.capture { UploadStorage.sanitize_s3_key(k) }) }

# ---------------------------------------------------------------- s3_key / unique_s3_key
S3K = [
  ["photo.png", nil, nil], ["../ev il.png", nil, nil], ["photo.png", "blog/2026/hero.png", "ignored"],
  ["photo.png", nil, "blog/2026"], ["photo.png", "../../etc/passwd", nil], ["photo.png", "../..", nil], ["photo.png", nil, "///"],
  ["movie.webm", nil, nil], ["photo.png", "hero", nil], ["photo.png", "hero.jpg", nil], ["photo", "hero", nil], ["photo", nil, nil],
  ["photo.png", "dir/", nil], ["photo.png", "a.b/c", nil], ["photo.PNG", "x.png", nil], ["archive.tar.gz", nil, nil],
  ["", nil, nil], [nil, nil, nil], ["日本語.png", nil, "日本"], ["photo.png", "", ""], ["photo.png", " ", nil],
  [".hidden", nil, nil], ["photo.png", ".hidden", nil], ["photo.png", "my key!.png", "p"], ["a.png", nil, "pre fix/sub dir"],
  ["photo.png", "x.", nil], ["photo.", nil, nil]
]
S3K.each do |fn, key, prefix|
  env_case do
    add(cases, "UploadStorage.s3_key", "#{fn.inspect} key=#{key.inspect} prefix=#{prefix.inspect}",
        { "original_filename" => fn, "custom_key" => key, "custom_prefix" => prefix, "pinned" => PINNED },
        Golden.capture { UploadStorage.s3_key(fn, custom_key: key, custom_prefix: prefix) })
  end
end
env_case(time: Time.utc(2026, 12, 31, 23, 59, 59)) do
  add(cases, "UploadStorage.s3_key", "year boundary (Time.current in Rails zone UTC)",
      { "original_filename" => "p.png", "custom_key" => nil, "custom_prefix" => nil, "pinned" => PINNED.merge("time" => "2026-12-31T23:59:59Z") },
      UploadStorage.s3_key("p.png"))
end
[["k.png", "f.jpg"], ["k", "f.jpg"], ["k", "f"], ["k", nil], ["a.b/c", "f.png"], ["k.tar.gz", "x"], [".png", "x"], ["k.", "f.png"]].each do |k, f|
  add(cases, "UploadStorage.unique_s3_key", "#{k.inspect} #{f.inspect}", { "key" => k, "original_filename" => f, "pinned" => PINNED },
      Golden.capture { UploadStorage.unique_s3_key(k, f) })
end

# ---------------------------------------------------------------- s3_url
S3URL = [
  [nil, ["mybucket", "us-west-2", "frankmd/2026/09/a b.png"]],
  ["aws_s3_endpoint = http://localhost:9000/\naws_s3_force_path_style = true\n", ["mybucket", "us-east-1", "frankmd/x.png"]],
  ["aws_s3_endpoint = https://s3.example.com\naws_s3_force_path_style = false\n", ["mybucket", "us-east-1", "k.png"]],
  ["aws_s3_endpoint = http://minio.local:9000\naws_s3_force_path_style = false\n", ["b", "us-east-1", "k.png"]],
  ["aws_s3_endpoint = https://s3.example.com:443\n", ["b", "r", "k.png"]],
  ["aws_s3_endpoint = http://s3.example.com:80/\n", ["b", "r", "k.png"]],
  ["aws_s3_endpoint = http://s3.example.com:443\n", ["b", "r", "k.png"]],
  ["aws_s3_endpoint = https://host/base/path/\naws_s3_force_path_style = true\n", ["b", "r", "k.png"]],
  ["aws_s3_endpoint = https://host/base/path\n", ["b", "r", "k.png"]],
  [nil, ["b", "r", "日本/語 #?&+%.png"]], [nil, ["b", "r", "a//b"]], [nil, ["b", "r", ""]], [nil, ["b", "r", "/lead"]],
  [nil, ["b", "r", "tilde~star*quote'paren().png"]]
]
S3URL.each do |fed, args|
  env_case(fed: fed) do
    add(cases, "UploadStorage.s3_url", "#{args.inspect} fed=#{fed.inspect}", { "fed" => fed, "bucket" => args[0], "region" => args[1], "key" => args[2] },
        Golden.capture { UploadStorage.s3_url(*args) })
  end
end
env_case(env: { "AWS_S3_ENDPOINT" => "http://envhost:9000", "AWS_S3_FORCE_PATH_STYLE" => "true" }) do
  add(cases, "UploadStorage.s3_url", "endpoint from ENV", { "env" => { "AWS_S3_ENDPOINT" => "http://envhost:9000", "AWS_S3_FORCE_PATH_STYLE" => "true" }, "bucket" => "b", "region" => "r", "key" => "k.png" },
      UploadStorage.s3_url("b", "r", "k.png"))
end

# ---------------------------------------------------------------- s3_region / s3_configured? / provider
[[nil, {}], ["aws_region = eu-west-1\n", {}], [nil, { "AWS_REGION" => "ap-south-1" }], ["aws_region =\n", {}]].each do |fed, env|
  env_case(fed: fed, env: env) { add(cases, "UploadStorage.s3_region", "fed=#{fed.inspect} env=#{env}", { "fed" => fed, "env" => env }, UploadStorage.s3_region) }
end
CREDS = "aws_access_key_id = k\naws_secret_access_key = s\naws_s3_bucket = b\n"
S3CONF = [nil, CREDS, "aws_access_key_id = k\naws_s3_bucket = b\n", CREDS + "aws_s3_endpoint = http://localhost:9000\n",
          CREDS + "aws_s3_endpoint = https://minio.example\n", CREDS + "aws_s3_endpoint = ftp://x\n",
          CREDS + "aws_s3_endpoint = http://user:pw@host\n", CREDS + "aws_s3_endpoint = not a uri\n",
          CREDS + "aws_s3_endpoint = http://\n", CREDS + "aws_s3_endpoint = localhost:9000\n", CREDS + "aws_s3_endpoint = HTTPS://UP.example\n",
          "aws_access_key_id = \"\"\naws_secret_access_key = s\naws_s3_bucket = b\n"]
S3CONF.each do |fed|
  env_case(fed: fed) do
    add(cases, "UploadStorage.s3_configured?", fed.inspect[0, 80], { "fed" => fed }, Golden.capture { UploadStorage.s3_configured?(Config.new) })
  end
end
PROVIDERS = [
  [nil, nil], [nil, false], [nil, true], [nil, "true"], [nil, "false"], [nil, "1"], [nil, "0"], [nil, ""], [nil, "yes"],
  ["media_storage_provider = s3\n", nil], ["media_storage_provider = s3\n" + CREDS, nil], ["media_storage_provider = s3\n" + CREDS, false],
  ["media_storage_provider = s3\n" + CREDS, true], ["media_storage_provider = invalid\n", nil], ["media_storage_provider = S3\n", nil],
  ["media_storage_provider = local\n", true], ["media_storage_provider =\n", nil]
]
PROVIDERS.each do |fed, legacy|
  env_case(fed: fed) do
    add(cases, "UploadStorage.provider", "fed=#{fed.inspect[0, 60]} legacy_s3=#{legacy.inspect}", { "fed" => fed, "legacy_s3" => legacy },
        Golden.capture { UploadStorage.provider(legacy_s3: legacy) })
  end
end
%w[InvalidAccessKeyId SignatureDoesNotMatch InvalidToken ExpiredToken TokenRefreshRequired AccessDenied NoSuchBucket
   InvalidBucketName AuthorizationHeaderMalformed SlowDown InternalError NoSuchKey PreconditionFailed RequestTimeout].each do |code|
  err = Aws::S3::Errors.error_class(code).new(nil, "msg")
  add(cases, "UploadStorage.invalid_s3_configuration_error?", "Aws::S3::Errors::#{code}", { "error_class" => "Aws::S3::Errors::#{code}", "code" => code },
      UploadStorage.invalid_s3_configuration_error?(err))
end
add(cases, "UploadStorage.invalid_s3_configuration_error?", "Aws::Errors::InvalidRegionError", { "error_class" => "Aws::Errors::InvalidRegionError" },
    UploadStorage.invalid_s3_configuration_error?(Aws::Errors::InvalidRegionError.new))
add(cases, "UploadStorage.invalid_s3_configuration_error?", "Timeout::Error", { "error_class" => "Timeout::Error" },
    UploadStorage.invalid_s3_configuration_error?(Timeout::Error.new))
add(cases, "UploadStorage.invalid_s3_configuration_error?", "Seahorse::Client::NetworkingError", { "error_class" => "Seahorse::Client::NetworkingError" },
    UploadStorage.invalid_s3_configuration_error?(Seahorse::Client::NetworkingError.new(StandardError.new("x"))))

# ---------------------------------------------------------------- validate_filename! / extension config
EXT_CONFIGS = [nil, "image_upload_extensions = .png,.jpg\nvideo_upload_extensions = .mp4,.webm\n",
               "image_upload_extensions = .JPG , .Png ,, ,.svg\n", "image_upload_extensions =\n", "image_upload_extensions = png,jpg\n"]
FILES = ["photo.png", "photo.PNG", "photo.jpeg", "photo.svg", "photo.SVG", "evil.html", "evil.htm", "photo.png.html", "photo.html.png",
         "README", "", ".png", "..png", "x.", "clip.mp4", "clip.MOV", "clip.webm", "notes.txt", "a.bmp", "a.webp", "a.gif", "a.tiff",
         "dir/x.png", "x.png ", "x.p ng", nil]
EXT_CONFIGS.each do |fed|
  env_case(fed: fed) do
    cfg = Config.new
    %w[image_upload_extensions video_upload_extensions].each do |key|
      add(cases, "Config#upload_extensions", "#{key} fed=#{fed.inspect}", { "fed" => fed, "key" => key }, cfg.upload_extensions(key))
    end
    FILES.each do |f|
      [["image_upload_extensions", "image"], ["video_upload_extensions", "video"]].each do |key, kind|
        next if fed && kind == "video" && !f.to_s.match?(/mp4|mov|webm|txt|README/)
        add(cases, "UploadStorage.validate_filename!", "#{f.inspect} #{kind} fed=#{fed.inspect[0, 40]}",
            { "fed" => fed, "filename" => f, "config_key" => key, "kind" => kind },
            Golden.capture { UploadStorage.validate_filename!(f, key, kind) })
      end
    end
  end
end
env_case(env: { "IMAGE_UPLOAD_EXTENSIONS" => ".png, .SVG" }) do
  add(cases, "UploadStorage.validate_filename!", "IMAGE_UPLOAD_EXTENSIONS env allows svg",
      { "env" => { "IMAGE_UPLOAD_EXTENSIONS" => ".png, .SVG" }, "filename" => "x.svg", "config_key" => "image_upload_extensions", "kind" => "image" },
      Golden.capture { UploadStorage.validate_filename!("x.svg", "image_upload_extensions", "image") })
end

# ---------------------------------------------------------------- size limits
MAX = UploadStorage::MAX_UPLOAD_BYTES
[nil, 0, 1, MAX - 1, MAX, MAX + 1, 10 * MAX, -1].each do |size|
  add(cases, "UploadStorage.enforce_bytes!", size.inspect, { "size" => size }, Golden.capture { UploadStorage.enforce_bytes!(size); "ok" })
end
SizeObj = Struct.new(:size)
[[SizeObj.new(MAX), "size=MAX"], [SizeObj.new(MAX + 1), "size=MAX+1"], [Object.new, "no size method"], [SizeObj.new(nil), "size nil"]].each do |obj, label|
  add(cases, "UploadStorage.enforce_size!", label, { "object" => label }, Golden.capture { UploadStorage.enforce_size!(obj); "ok" })
end
b64max = UploadStorage.max_base64_encoded_bytes
add(cases, "UploadStorage.max_base64_encoded_bytes", "value", {}, b64max)
[0, b64max - 1, b64max, b64max + 1].each do |size|
  add(cases, "UploadStorage.enforce_base64_size!", size.to_s, { "size" => size }, Golden.capture { UploadStorage.enforce_base64_size!(size); "ok" })
end

# ---------------------------------------------------------------- MediaService.save_upload (local only)
def upload(name, content = "fake video bytes", type = "video/mp4")
  Rack::Test::UploadedFile.new(StringIO.new(content), type, original_filename: name)
end
["clip.mp4", "notes.txt", "README", "my cool clip!.mp4", "../../../etc/evil.mp4", "movie.WEBM", "a.mkv", "日本.mov", "evil.html", "x.mp4.html", ""].each do |n|
  env_case do |root|
    r = Golden.capture { MediaService.save_upload(upload(n)) }
    add(cases, "MediaService.save_upload", "local #{n.inspect}", { "original_filename" => n, "content" => "fake video bytes", "fed" => nil, "pinned" => PINNED },
        { "result" => r, "tree_after" => snapshot_wo_fed(root) })
  end
end
env_case { |root| add(cases, "MediaService.save_upload", "nil file", { "original_filename" => nil }, { "result" => MediaService.save_upload(nil), "tree_after" => snapshot_wo_fed(root) }) }
env_case(fed: "media_storage_provider = s3\n") do |root|
  add(cases, "MediaService.save_upload", "s3 provider without credentials", { "original_filename" => "clip.mp4", "fed" => "media_storage_provider = s3\n" },
      { "result" => MediaService.save_upload(upload("clip.mp4")), "tree_after" => snapshot_wo_fed(root) })
end
env_case do |root|
  add(cases, "MediaService.save_upload", "legacy upload_to_s3=true conflicts with local", { "original_filename" => "clip.mp4", "upload_to_s3" => true },
      { "result" => MediaService.save_upload(upload("clip.mp4"), upload_to_s3: true), "tree_after" => snapshot_wo_fed(root) })
end
add(cases, "MediaService.VIDEO_MIME_TYPES", "constant", {}, MediaService::VIDEO_MIME_TYPES)

# ---------------------------------------------------------------- ImagesService local upload paths
["photo.png", "photo.svg", "evil.html", "a b!.JPG", "../../x.png", "README", "日本.gif", "x.webp", "x.bmp", "x.jpeg"].each do |n|
  env_case do |root|
    r = Golden.capture { ImagesService.upload_file(upload(n, "img", "image/png")) }
    add(cases, "ImagesService.upload_file", "local #{n.inspect}", { "original_filename" => n, "content" => "img", "resize" => nil, "pinned" => PINNED },
        { "result" => r, "tree_after" => snapshot_wo_fed(root) })
  end
end
PNG1 = Base64.strict_encode64("\x89PNG\r\n\x1a\nfake".b)
B64 = [
  [PNG1, nil, nil], [PNG1, "image/png", "custom.png"], [PNG1, "image/svg+xml", nil], [PNG1, "image/png", "evil.html"],
  [PNG1, "image/jpeg", "noext"], [PNG1, "image/gif; charset=binary", nil], [PNG1, "IMAGE/WEBP", nil], [PNG1, "text/html", nil],
  [PNG1, "image/png", "a b!.png"], [PNG1, "image/png", "../../up.png"], [PNG1, "", ""], [PNG1, "image/x-ms-bmp", nil],
  [PNG1, "image/png", "x.svg"], [PNG1, "image/png", "dots.in.name"], [PNG1, "image/png", "trailing."],
  ["not base64!!", nil, nil], ["", nil, nil], ["QQ", nil, nil], ["QQ==\n", nil, nil]
]
B64.each do |data, mime, fname|
  env_case do |root|
    r = Golden.capture { ImagesService.upload_base64_data(data, mime_type: mime, filename: fname) }
    add(cases, "ImagesService.upload_base64_data", "mime=#{mime.inspect} filename=#{fname.inspect} data=#{data[0, 12].inspect}",
        { "base64_data" => data, "mime_type" => mime, "filename" => fname, "pinned" => PINNED },
        { "result" => r, "tree_after" => snapshot_wo_fed(root) })
  end
end
%w[.jpg .JPEG .png .gif .webp .svg .bmp .tiff .html] .push("").each do |ext|
  add(cases, "ImagesService.content_type_for", ext.inspect, { "path" => "x#{ext}" }, ImagesService.send(:content_type_for, Pathname.new("x#{ext}")))
end
["image/jpeg", "image/png", "image/gif", "image/webp", "image/svg+xml", "image/bmp", "image/x-ms-bmp", "IMAGE/PNG",
 "image/png; charset=utf-8", " image/png ", "text/html", "", nil, "application/octet-stream", "image/avif"].each do |ct|
  add(cases, "ImagesService.extension_for_content_type", ct.inspect, { "content_type" => ct }, Golden.capture { ImagesService.send(:extension_for_content_type, ct) })
end
[0.25, "0.25", 0.33, "0.333", 0.5, "true", 0.67, "0.666", 0.75, 1, 1.0, "100%", "25%", 2, "abc", nil, "; rm -rf /", "50", 0.1].each do |r|
  add(cases, "ImagesService.imagemagick_resize_arg", r.inspect, { "ratio" => r }, ImagesService.send(:imagemagick_resize_arg, r))
end
[10, "5", 0, -3, 1000, "abc", nil, 2.7, "12abc"].each do |v|
  add(cases, "ImagesService.clamp_limit", v.inspect, { "value" => v }, ImagesService.send(:clamp_limit, v))
  add(cases, "ImagesService.clamp_offset", v.inspect, { "value" => v }, ImagesService.send(:clamp_offset, v))
end
add(cases, "ImagesService.constants", "constants", {}, {
  "SUPPORTED_EXTENSIONS" => ImagesService::SUPPORTED_EXTENSIONS, "MAX_RESULTS" => ImagesService::MAX_RESULTS,
  "MAX_EXTERNAL_IMAGE_BYTES" => ImagesService::MAX_EXTERNAL_IMAGE_BYTES, "MAX_EXTERNAL_IMAGE_REDIRECTS" => ImagesService::MAX_EXTERNAL_IMAGE_REDIRECTS,
  "MAX_EXTERNAL_IMAGE_SECONDS" => ImagesService::MAX_EXTERNAL_IMAGE_SECONDS,
  "UploadStorage" => { "SANITIZE_RE" => UploadStorage::SANITIZE_RE.source, "MAX_UPLOAD_BYTES" => UploadStorage::MAX_UPLOAD_BYTES,
                       "S3_KEY_PREFIX" => UploadStorage::S3_KEY_PREFIX, "DEFAULT_S3_REGION" => UploadStorage::DEFAULT_S3_REGION }
})

# ---------------------------------------------------------------- MediaLibraryService list / delete
MTIME = 1_767_268_800 # 2026-01-01T12:00:00Z
def m(content, offset = 0) = { "content" => content, "mtime" => MTIME + offset }
LIB_TREE = {
  "images/photo.png" => m("image", 10), "images/not-supported.svg" => m("svg"), "videos/clip.mp4" => m("video", 5),
  "videos/nested/clip.webm" => m("nested video", 5), "videos/readme.txt" => m("txt"), "images/B.PNG" => m("upper", 10),
  "images/a b #1.jpg" => m("spaces", 20), "images/日本.gif" => m("jp", 1), "images/.hidden.png" => m("hidden"),
  "images/.dir/x.png" => m("in hidden dir"), "images/sub/deep/x.jpeg" => m("deep", 3), "notes.png" => m("root png"),
  "attachments/photo.png" => m("attachment"), "images/linked.png" => { "symlink" => "../notes.png" },
  "images/external" => { "symlink" => "../attachments" }, "images/protected.png" => m("p", 2), "images/noext" => m("x"),
  "images/x.PnG" => m("mixed", 10)
}.freeze

env_case(tree: LIB_TREE) do |root|
  [nil, "image_upload_extensions = .png,.jpg\nvideo_upload_extensions = .mp4,.webm\n"].each do |fed|
    File.write(File.join(root, ".fed"), fed.to_s)
    list = MediaLibraryService.new(base_path: root, config: Config.new(base_path: root)).list
    add(cases, "MediaLibraryService#list", "fed=#{fed.inspect}", { "tree" => LIB_TREE, "fed" => fed, "mtime_note" => "mtime pinned via File.utime; process TZ=UTC" },
        Golden.normalize(list, root))
  end
end
env_case(tree: { "images" => { "symlink" => "real_images" }, "real_images/a.png" => m("a") }) do |root|
  add(cases, "MediaLibraryService#list", "images root itself a symlink",
      { "tree" => { "images" => { "symlink" => "real_images" }, "real_images/a.png" => m("a") }, "fed" => nil },
      MediaLibraryService.new(base_path: root, config: Config.new(base_path: root)).list)
end
env_case { |root| add(cases, "MediaLibraryService#list", "empty root", { "tree" => {}, "fed" => nil }, MediaLibraryService.new(base_path: root, config: Config.new(base_path: root)).list) }

DEL_PATHS = ["images/photo.png", "images/sub/deep/x.jpeg", "videos/nested/clip.webm", "images/../notes.png", "notes.png",
             "attachments/photo.png", "images/readme.md", "images/a\0b.png", "images/nested/a\0b.png", "images/\0", "images/linked.png",
             "images/external/photo.png", "images/missing.png", "images/", "images", "/images/photo.png", "images//photo.png",
             "images/./photo.png", "images/sub/../photo.png", "images\\photo.png", "videos/readme.txt", "images/not-supported.svg",
             "images/B.PNG", "IMAGES/photo.png", "images/sub", "images/x.PnG", "images/.hidden.png", "images/.dir/x.png", "", nil,
             "<ROOT>/images/photo.png", "images/photo.png/", " images/photo.png"]
DEL_PATHS.each do |p|
  env_case(tree: LIB_TREE) do |root|
    path = p&.sub("<ROOT>", root)
    svc = MediaLibraryService.new(base_path: root, config: Config.new(base_path: root))
    safe = Golden.capture { svc.send(:safe_media_file, path) }
    del = Golden.capture { svc.delete(path) }
    add(cases, "MediaLibraryService#delete", p.inspect, { "tree" => LIB_TREE, "fed" => nil, "path" => p },
        { "safe_media_file" => Golden.normalize(safe.is_a?(Pathname) ? safe.to_s : safe, root), "result" => del,
          "tree_after" => snapshot_wo_fed(root).transform_values { |v| v.is_a?(Hash) ? v : v } })
  end
end

# ---------------------------------------------------------------- ImageLibraryService (managed collection only)
[{}, { search: "PHOTO" }, { search: "日本" }, { limit: 2 }, { limit: 2, offset: 2 }, { limit: 0 }, { limit: 500 }, { offset: -5 },
 { limit: "abc", offset: "xyz" }, { offset: 100 }, { search: "" }, { collection: "bogus" }].each do |kw|
  env_case(tree: LIB_TREE) do |root|
    r = Golden.capture { ImageLibraryService.new.list(**kw) }
    add(cases, "ImageLibraryService#list", kw.inspect, { "tree" => LIB_TREE, "fed" => nil, "kwargs" => kw.transform_keys(&:to_s) },
        Golden.normalize(r, root))
  end
end

Golden.write("media", Golden.fixture(
  source: "app/services/{upload_storage,media_service,images_service,media_library_service,image_library_service}.rb",
  generator: "tools/golden/ruby/media.rb", cases: cases, pinned: PINNED,
  notes: "Clock pinned with travel_to (Time.now/Time.current = 2026-01-01T12:00:00Z unless input.pinned overrides); process ENV TZ=UTC " \
         "(dest_filename uses Time.now in the PROCESS local zone, s3_key uses Time.current in the Rails zone). SecureRandom.hex stubbed: see pinned. " \
         "NOTES_PATH=<ROOT> temp dir; input.fed is the .fed file content (nil = no file; Config.new then writes its template .fed, " \
         "which is excluded from tree_after). AWS_*/IMAGES_PATH/*_UPLOAD_EXTENSIONS env cleared unless input.env. " \
         "Uploads go through UploadStorage.with_temp_copy (Rails.root/tmp/uploads, cleaned). Non-UTF8 strings as {base64}. " \
         "Markdown snippet generation is not Ruby (client JS) and is not covered here. No S3/network calls are made."
))
