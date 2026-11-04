//! Port of `test/javascript/lib/theme_syntax_palette.test.js` (20 runtime
//! tests: 19 ported = 1 static + 18 generated per theme; 1 skipped).
//! The theme CSS files are `assets/themes/builtin.json` here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;

// port: test/javascript/lib/theme_syntax_palette.test.js:36 "covers all built-in themes"
#[test]
fn covers_all_built_in_themes() {
    let mut ids = frankmd_theme::builtin_ids();
    ids.sort_unstable();
    assert_eq!(
        ids,
        [
            "catppuccin", "catppuccin-latte", "dark", "ethereal", "everforest", "flexoki-light", "gruvbox",
            "hackerman", "kanagawa", "light", "matte-black", "nord", "osaka-jade", "ristretto", "rose-pine",
            "solarized-dark", "solarized-light", "tokyo-night",
        ]
    );
}

fn defines_every_syntax_role_with_readable_contrast(id: &str) {
    let tokens = frankmd_theme::builtin(id).unwrap_or_else(|| panic!("{id} must exist"));
    let failures = syntax_contrast_failures(|k| tokens.get(k).unwrap_or_else(|| panic!("{id} must define --theme-{k}")));
    assert!(failures.is_empty(), "{id}: {failures:?}");
}

macro_rules! per_theme {
    ($($fn_name:ident => $id:literal),* $(,)?) => {$(
        // port: test/javascript/lib/theme_syntax_palette.test.js:41 "${file} defines every syntax role with readable contrast"
        #[test]
        fn $fn_name() {
            defines_every_syntax_role_with_readable_contrast($id);
        }
    )*};
}

per_theme! {
    catppuccin_latte_defines_every_syntax_role => "catppuccin-latte",
    catppuccin_defines_every_syntax_role => "catppuccin",
    dark_defines_every_syntax_role => "dark",
    ethereal_defines_every_syntax_role => "ethereal",
    everforest_defines_every_syntax_role => "everforest",
    flexoki_light_defines_every_syntax_role => "flexoki-light",
    gruvbox_defines_every_syntax_role => "gruvbox",
    hackerman_defines_every_syntax_role => "hackerman",
    kanagawa_defines_every_syntax_role => "kanagawa",
    light_defines_every_syntax_role => "light",
    matte_black_defines_every_syntax_role => "matte-black",
    nord_defines_every_syntax_role => "nord",
    osaka_jade_defines_every_syntax_role => "osaka-jade",
    ristretto_defines_every_syntax_role => "ristretto",
    rose_pine_defines_every_syntax_role => "rose-pine",
    solarized_dark_defines_every_syntax_role => "solarized-dark",
    solarized_light_defines_every_syntax_role => "solarized-light",
    tokyo_night_defines_every_syntax_role => "tokyo-night",
}

// skip: test/javascript/lib/theme_syntax_palette.test.js:59 "renders syntax roles without borrowing heading or accent colors"
//       Asserts on app/assets/tailwind/components/prose.css (preview CSS); the native preview's
//       code-highlight → token mapping belongs to frankmd-markdown / frankmd-app.
