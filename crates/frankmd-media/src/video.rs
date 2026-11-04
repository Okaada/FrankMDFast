//! **[COMPAT]** port of `app/services/media_service.rb` (video uploads; never
//! resized). Golden: `media.json` `MediaService.*`.

use crate::env::MediaEnv;
use crate::error::MediaError;
use crate::param::Param;
use crate::upload_storage::Upload;

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
    todo!("backend.md §7.11")
}
