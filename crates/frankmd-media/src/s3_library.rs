//! **[COMPAT]** port of `app/services/s3_media_library_service.rb`: the S3
//! Library (objects under `frankmd/` only).

use crate::MediaKind;
use crate::config::MediaConfig;
use crate::env::MediaEnv;
use crate::error::MediaError;
use crate::library::{LibraryListing, MediaItem, RenamedItem};
use crate::server_folder::extname;
use crate::upload_storage::{self, UploadKind};

/// `PREFIX` = `"#{S3_KEY_PREFIX}/"`.
pub const PREFIX: &str = "frankmd/";
/// Max key length in bytes (`valid_key?`).
pub const MAX_KEY_BYTES: usize = 1024;
/// `MAX_OBJECTS` guard while paginating (`"S3 Library contains too many objects"`);
/// s3_media_library_service.rb: `MAX_OBJECTS = 5_000`.
pub const MAX_OBJECTS: usize = 5_000;

/// S3 Library bound to one config snapshot.
#[derive(Debug)]
pub struct S3MediaLibrary {
    config: MediaConfig,
    client: crate::s3::S3Client,
}

impl S3MediaLibrary {
    /// `S3MediaLibraryService.new(config:)` (client from `env.s3_client`).
    pub fn new(env: &MediaEnv, config: &MediaConfig) -> Result<Self, MediaError> {
        let client = env.s3_client(config)?;
        Ok(Self {
            config: config.clone(),
            client,
        })
    }

    fn bucket(&self) -> &str {
        self.config.s3.bucket.as_deref().unwrap_or_default()
    }

    fn object_url(&self, key: &str) -> String {
        upload_storage::s3_url(
            &self.config,
            self.bucket(),
            &upload_storage::s3_region(&self.config),
            key,
        )
    }

    /// `media_for(key)`: the media kind of a valid managed key (image wins).
    fn media_for(&self, key: &str) -> Option<MediaKind> {
        if !valid_key(key) {
            return None;
        }
        let ext = extname(key).to_lowercase();
        if self.config.image_upload_extensions.contains(&ext) {
            Some(MediaKind::Image)
        } else if self.config.video_upload_extensions.contains(&ext) {
            Some(MediaKind::Video)
        } else {
            None
        }
    }

    /// `objects`: every page under [`PREFIX`].
    async fn objects(&self) -> Result<Vec<crate::s3::S3Object>, MediaError> {
        let mut results = Vec::new();
        let mut token: Option<String> = None;
        loop {
            let page = self
                .client
                .list_objects_v2(PREFIX, token.as_deref())
                .await?;
            results.extend(page.objects);
            if results.len() > MAX_OBJECTS {
                return Err(MediaError::Rejected(
                    "S3 Library contains too many objects".to_owned(),
                ));
            }
            if !page.truncated {
                break;
            }
            match page
                .next_continuation_token
                .filter(|t| !t.trim().is_empty())
            {
                Some(next) => token = Some(next),
                None => {
                    return Err(MediaError::Rejected(
                        "S3 Library pagination failed".to_owned(),
                    ));
                }
            }
        }
        Ok(results)
    }

    /// `list`: `ListObjectsV2(prefix: "frankmd/")` until not truncated; supported
    /// extensions only; items carry `provider: "s3"` and the `s3_url` URL.
    /// A truncated page without a token → `Rejected("S3 Library pagination failed")`.
    pub async fn list(&self) -> Result<LibraryListing, MediaError> {
        let mut listing = LibraryListing::default();
        for object in self.objects().await? {
            let Some(kind) = self.media_for(&object.key) else {
                continue;
            };
            let url = self.object_url(&object.key);
            let item = MediaItem {
                name: object.key.rsplit('/').next().unwrap_or_default().to_owned(),
                path: object.key.clone(),
                size: object.size,
                mtime: object.last_modified.clone().unwrap_or_default(),
                preview_url: url.clone(),
                kind,
                s3_url: Some(url),
            };
            match kind {
                MediaKind::Image => listing.images.push(item),
                MediaKind::Video => listing.videos.push(item),
            }
        }
        for items in [&mut listing.images, &mut listing.videos] {
            items.sort_by(|a, b| {
                let ta = a.mtime.parse::<jiff::Timestamp>().ok();
                let tb = b.mtime.parse::<jiff::Timestamp>().ok();
                tb.cmp(&ta).then_with(|| a.path.cmp(&b.path))
            });
        }
        Ok(listing)
    }

    /// `delete(key)`: `Ok(false)` without any request for foreign, traversal or
    /// unsupported keys; else `DeleteObject` → `Ok(true)`.
    pub async fn delete(&self, key: &str) -> Result<bool, MediaError> {
        if self.media_for(key).is_none() {
            return Ok(false);
        }
        self.client.delete_object(key).await?;
        Ok(true)
    }

    /// `rename(key, new_name)`: validate the name (basename, sanitized, same
    /// media kind, ≤ 1024-byte key), `HeadObject` the sibling (404 = free), then
    /// `CopyObject(copy_source: "<bucket>/<url-encoded old key>")`, then
    /// `DeleteObject(old)`. `Ok(None)` (no mutation) for invalid names or an
    /// existing target.
    pub async fn rename(
        &self,
        key: &str,
        new_name: &str,
    ) -> Result<Option<RenamedItem>, MediaError> {
        let Some(kind) = self.media_for(key) else {
            return Ok(None);
        };
        if !self.valid_new_name(new_name, kind) {
            return Ok(None);
        }
        // `File.join(File.dirname(key), new_name)`; managed keys always have a directory.
        let directory = key.rsplit_once('/').map_or(".", |(dir, _)| dir);
        let destination = format!("{directory}/{new_name}");
        if !valid_key(&destination)
            || destination == key
            || self.client.head_object(&destination).await?
        {
            return Ok(None);
        }
        self.client.copy_object(key, &destination).await?;
        self.client.delete_object(key).await?;
        Ok(Some(RenamedItem {
            name: new_name.to_owned(),
            url: self.object_url(&destination),
            path: destination,
        }))
    }

    /// `valid_new_name?`: a bare basename, already sanitized, with an
    /// extension of the same media kind.
    fn valid_new_name(&self, name: &str, kind: MediaKind) -> bool {
        let basename = name.rsplit('/').next().unwrap_or_default();
        if name.is_empty() || name != basename || name != upload_storage::sanitize_filename(name) {
            return false;
        }
        let kind = match kind {
            MediaKind::Image => UploadKind::Image,
            MediaKind::Video => UploadKind::Video,
        };
        upload_storage::validate_filename(&self.config, name, kind).is_ok()
    }
}

/// `valid_key?`: under [`PREFIX`], at most [`MAX_KEY_BYTES`], and every `/`
/// segment non-blank, not `.`/`..`, without `\` or NUL.
fn valid_key(key: &str) -> bool {
    if !key.starts_with(PREFIX) || key.len() > MAX_KEY_BYTES {
        return false;
    }
    let segments: Vec<&str> = key.split('/').collect();
    segments.len() > 1
        && segments.iter().all(|s| {
            !s.trim().is_empty()
                && *s != "."
                && *s != ".."
                && !s.contains('\\')
                && !s.contains('\0')
        })
}
