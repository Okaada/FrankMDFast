//! Embedded assets for the app: FrankMD's own brand/icons layered over
//! gpui-kit's bundled Lucide set.
//!
//! - `brand/icon.svg` — the FrankMD logo (about/empty states; the window and
//!   `.desktop` icon are installed from `assets/brand/` by packaging, since
//!   Wayland takes the icon from the desktop entry matched by app_id).
//! - `icons/ph/<name>.svg` — synthesized on demand from FrankMD's curated
//!   Phosphor set (`assets/icons/phosphor.json`, MIT): `[name, path, viewBox,
//!   keywords]` → `<svg viewBox=..><path fill="currentColor" d=../></svg>`.
//!   Used by the icon picker (FrankMD feature) and, through
//!   `gpui_kit::component::set_icon_resolver`, for UI chrome icons that have a
//!   Phosphor equivalent. Icons without one fall back to Lucide.
//! - everything else → `gpui_kit::assets::Assets` (Lucide).

use gpui_kit::{AssetSource, Result, SharedString};
use std::borrow::Cow;

const BRAND_ICON_SVG: &[u8] = include_bytes!("../../../assets/brand/icon.svg");
/// FrankMD's curated Phosphor icon table.
pub(crate) const PHOSPHOR_JSON: &str = include_str!("../../../assets/icons/phosphor.json");

/// Layered asset source: FrankMD assets first, then gpui-kit's.
pub(crate) struct FrankAssets;

impl AssetSource for FrankAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path == "brand/icon.svg" {
            return Ok(Some(Cow::Borrowed(BRAND_ICON_SVG)));
        }
        if let Some(name) = path
            .strip_prefix("icons/ph/")
            .and_then(|p| p.strip_suffix(".svg"))
        {
            return Ok(phosphor_svg(name).map(|s| Cow::Owned(s.into_bytes())));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        gpui_kit::assets::Assets.list(path)
    }
}

/// Build an SVG document for Phosphor icon `name` (parsed table cached once).
fn phosphor_svg(name: &str) -> Option<String> {
    // Scaffold: implemented with a LazyLock<HashMap<name, (path, viewBox)>>
    // over PHOSPHOR_JSON when the icon picker lands.
    let _ = (name, PHOSPHOR_JSON);
    None
}
