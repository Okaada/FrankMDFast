//! Phosphor icon subset (`assets/icons/phosphor.json`, port of lib/icon_data.js).
//! Backs the icon picker and the `:ph-<name>:` inline-icon markdown syntax.
//! Golden: `js_icon_data.json`.

use std::collections::HashMap;

/// `{source, license, format: "[name, svgPathData, viewBox, keywords]", data: [...]}`.
pub const PHOSPHOR_JSON: &str = include_str!("../../../../assets/icons/phosphor.json");

/// The viewBox every icon in the subset uses.
pub const VIEW_BOX: &str = "0 0 256 256";

/// One `ICON_DATA` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon {
    /// Icon name without the `ph-` prefix (`rocket`, `git-branch`).
    pub name: String,
    /// SVG path data (`d` attribute).
    pub path: String,
    /// SVG viewBox (always [`VIEW_BOX`] today).
    pub view_box: String,
    /// Space-separated search keywords.
    pub keywords: String,
}

/// What `:ph-<name>:` renders: **[COMPAT]** `getIconMap()` value `{path, viewBox}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconGlyph {
    /// SVG path data.
    pub path: &'static str,
    /// SVG viewBox.
    pub view_box: &'static str,
}

/// All icons in source order (**[COMPAT]** `ICON_DATA`), parsed once from
/// [`PHOSPHOR_JSON`] and cached.
pub fn icons() -> &'static [Icon] {
    todo!("parse PHOSPHOR_JSON once (OnceLock); needs serde_json as a normal dependency")
}

/// `name -> glyph` (**[COMPAT]** `getIconMap()`): last duplicate wins; built
/// once, every call returns the same map.
pub fn icon_map() -> &'static HashMap<&'static str, IconGlyph> {
    todo!()
}

/// Resolve an icon name (no `ph-` prefix, no colons).
pub fn lookup(name: &str) -> Option<IconGlyph> {
    todo!()
}
