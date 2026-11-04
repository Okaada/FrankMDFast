//! **[COMPAT]** port of `app/services/media_service.rb` (video uploads; never
//! resized). Golden: `media.json` `MediaService.*`.

use crate::env::MediaEnv;
use crate::error::MediaError;
use crate::param::Param;
use crate::pipeline::{UploadRequest, store_media};
use crate::upload_storage::{self as us, Upload};
use crate::{MediaKind, UploadOrigin};

/// `SUBDIR`.
pub const SUBDIR: &str = "videos";

/// `VIDEO_MIME_TYPES` (S3 `Content-Type` by key extension; else `application/octet-stream`).
pub const VIDEO_MIME_TYPES: [(&str, &str); 7] = [
    (".mp4", "video/mp4"),
    (".webm", "video/webm"),
    (".mkv", "video/x-matroska"),
    (".mov", "video/quicktime"),
    (".avi", "video/x-msvideo"),
    (".m4v", "video/x-m4v"),
    (".ogv", "video/ogg"),
];

/// `save_upload(file, upload_to_s3:, s3_prefix:)` → `videos/<dest_filename>` or the S3 URL.
/// No file → `Invalid("No file provided")`; provider/size/extension rejections as
/// [`crate::upload_storage`]; S3 collision → `Upstream("Video upload failed")`
/// (no provider detail); other S3 errors propagate.
pub async fn save_upload(
    env: &MediaEnv,
    file: Option<Upload>,
    upload_to_s3: &Param,
    s3_prefix: Option<&str>,
) -> Result<String, MediaError> {
    let Some(file) = file else {
        return Err(MediaError::Invalid("No file provided".to_owned()));
    };
    let config = env.settings.current();
    let provider = us::provider(&config, upload_to_s3)?;
    let mut req = UploadRequest::new(
        UploadOrigin::Drop,
        MediaKind::Video,
        file.original_filename,
        file.bytes,
    );
    req.s3_prefix = s3_prefix.map(str::to_owned);
    Ok(store_media(env, &config, provider, req)
        .await?
        .url()
        .to_owned())
}

/// S3 `Content-Type` for a video key (by its extension, case-insensitive).
pub fn content_type_for_key(key: &str) -> &'static str {
    let ext = us::ruby_extname(key).to_lowercase();
    VIDEO_MIME_TYPES
        .iter()
        .find(|(e, _)| *e == ext)
        .map_or("application/octet-stream", |(_, t)| t)
}
