//! Request validation of `AiController` (`POST /ai/fix_grammar`, `POST /ai/generate_image`).

use frankmd_core::path::{NOTE_EXT, normalize_note_path};

use crate::service::is_blank;
use crate::{AiService, GeneratedImage, GrammarFix, ReferenceImages};

/// `POST /ai/fix_grammar` / `POST /ai/generate_image` request errors that the
/// controller raised *before* calling the service. Each carries the i18n key.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AiRequestError {
    /// 400 (`errors.no_file_provided`, `errors.note_is_empty`, `errors.no_prompt_provided`).
    #[error("bad request: {0}")]
    BadRequest(&'static str),
    /// 404 (`errors.note_not_found`).
    #[error("not found: {0}")]
    NotFound(&'static str),
    /// 403 `forbidden` (the message `reject_config_file` used): the path is not
    /// a markdown note (`.fed`, dotfiles, non-`.md`, absolute or `..` paths).
    /// **Security decision:** the original sent `.fed` — secrets included — to
    /// the LLM; only note content may leave the machine.
    #[error("forbidden")]
    Forbidden,
    /// 422: the service's error.
    #[error(transparent)]
    Service(#[from] crate::AiError),
}

/// Reads a note's current bytes from disk (the original fixed the file *on
/// disk*, not the editor buffer). `None` = not found.
///
/// Implementations must resolve `path` through the vault's symlink-aware
/// containment (`frankmd-fs`), so a note symlinked to a file outside the
/// vault (or to `.fed`) is not read.
pub trait NoteSource: Send + Sync {
    /// Read `path` (vault-relative, already normalized to a `.md` note path).
    fn read_note(&self, path: &str) -> Option<String>;
}

/// `POST /ai/fix_grammar` response body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarResponse {
    /// The note text sent.
    pub original: String,
    /// The fix.
    pub fix: GrammarFix,
}

/// Whether a normalized vault-relative path names a markdown note that may be
/// sent to an AI provider: relative, ending in `.md`, and with no empty,
/// hidden (`.`-prefixed, which covers `.fed`, `.` and `..`) or backslash/NUL
/// containing component.
pub fn is_ai_note_path(path: &str) -> bool {
    path.ends_with(NOTE_EXT)
        && !path.contains(['\\', '\0'])
        && path
            .split('/')
            .all(|segment| !segment.is_empty() && !segment.starts_with('.'))
}

/// **[COMPAT]** `AiController#fix_grammar`: blank path → 400, non-note path →
/// 403 (security), missing note → 404, blank content → 400, then
/// [`AiService::fix_grammar`] (errors → 422). All validation happens before
/// any network call.
pub async fn fix_grammar_note(
    service: &AiService,
    notes: &dyn NoteSource,
    path: &str,
) -> Result<GrammarResponse, AiRequestError> {
    if is_blank(path) {
        return Err(AiRequestError::BadRequest("errors.no_file_provided"));
    }
    let path = normalize_note_path(path);
    if !is_ai_note_path(&path) {
        tracing::warn!("grammar check refused for a non-note path");
        return Err(AiRequestError::Forbidden);
    }
    let text = notes
        .read_note(&path)
        .ok_or(AiRequestError::NotFound("errors.note_not_found"))?;
    if is_blank(&text) {
        return Err(AiRequestError::BadRequest("errors.note_is_empty"));
    }
    let fix = service.fix_grammar(Some(&text)).await?;
    Ok(GrammarResponse {
        original: text,
        fix,
    })
}

/// **[COMPAT]** `AiController#generate_image`: blank prompt → 400, then the
/// service. A blank `reference_image_path` counts as none (`.presence`).
pub async fn generate_image_request(
    service: &AiService,
    prompt: &str,
    reference_image_path: Option<&str>,
    refs: &dyn ReferenceImages,
) -> Result<GeneratedImage, AiRequestError> {
    if is_blank(prompt) {
        return Err(AiRequestError::BadRequest("errors.no_prompt_provided"));
    }
    let reference = reference_image_path.filter(|p| !p.is_empty());
    Ok(service
        .generate_image(Some(prompt), reference, refs)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_markdown_notes_qualify() {
        for ok in ["a.md", "dir/sub/note.md", "images/x.md"] {
            assert!(is_ai_note_path(ok), "{ok}");
        }
        for bad in [
            ".fed",
            "dir/.fed",
            ".hidden.md",
            ".git/x.md",
            "../x.md",
            "/etc/x.md",
            "a//b.md",
            "a\\b.md",
            "x.txt",
            "a/./b.md",
        ] {
            assert!(!is_ai_note_path(bad), "{bad}");
        }
    }
}
