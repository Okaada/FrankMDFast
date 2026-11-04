//! Extra tests (not ports) for the Omarchy 4.5 path:
//! `~/.local/state/omarchy/current/theme/colors.toml` with the
//! `omarchy-theme-color` cascade and the docs/research/omarchy-4.5.md §2.6
//! mapping. FrankMD itself reads the dead `~/.config/omarchy/current`
//! (see golden_theme.rs for that legacy contract).
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_testkit::golden;
use frankmd_theme::omarchy::load_current;
use frankmd_theme::{COLOR_KEYS, Mode, OmarchyPalette, OmarchyPaths, Rgb, contrast_ratio, from_omarchy, legacy};
use std::path::Path;

fn stock_themes() -> Vec<(String, String)> {
    golden("omarchy_theme")
        .cases
        .iter()
        .filter(|c| c.name.starts_with("theme "))
        .map(|c| (c.input["theme"].as_str().unwrap().to_owned(), c.input["colors_toml"].as_str().unwrap().to_owned()))
        .collect()
}

fn mode_key(toml: &str) -> Mode {
    if toml.contains("mode = \"light\"") { Mode::Light } else { Mode::Dark }
}

// new: omarchy 4.5 — the state dir, not ~/.config/omarchy/current
#[test]
fn paths_point_at_the_local_state_dir() {
    let p = OmarchyPaths::from_home(Path::new("/home/u"));
    assert_eq!(p.current, Path::new("/home/u/.local/state/omarchy/current"));
    assert_eq!(p.colors_toml(), Path::new("/home/u/.local/state/omarchy/current/theme/colors.toml"));
    assert_eq!(p.light_mode_file(), Path::new("/home/u/.local/state/omarchy/current/theme/light.mode"));
    assert_eq!(p.theme_name(), Path::new("/home/u/.local/state/omarchy/current/theme.name"));
    assert_eq!(p.generation(), Path::new("/home/u/.local/state/omarchy/current/theme.generation"));
    assert_ne!(p.current, legacy::legacy_current_dir(Path::new("/home/u")));
}

// new: omarchy 4.5 — all 23 stock colors.toml files map to a complete, readable token set
#[test]
fn stock_themes_map_to_complete_readable_tokens() {
    let mut failures = Vec::new();
    for (name, toml) in stock_themes() {
        let p = match OmarchyPalette::parse(&toml, false) {
            Ok(p) => p,
            Err(e) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        };
        if p.mode != mode_key(&toml) {
            failures.push(format!("{name}: palette mode {:?}", p.mode));
        }
        let t = from_omarchy(&p);
        if t.mode != p.mode {
            failures.push(format!("{name}: token mode {:?}", t.mode));
        }
        for k in COLOR_KEYS {
            if t.get(k).is_none() {
                failures.push(format!("{name}: missing {k}"));
            }
        }
        if t.colors.len() != 38 {
            failures.push(format!("{name}: {} tokens", t.colors.len()));
        }
        failures.extend(syntax_contrast_failures(|k| t.get(k).unwrap()).into_iter().map(|f| format!("{name}: {f}")));
        let tp = contrast_ratio(t.get("text-primary").unwrap(), t.get("code-bg").unwrap());
        if tp < 4.5 {
            failures.push(format!("{name}: text-primary contrast {tp:.2}"));
        }
        let at = contrast_ratio(t.get("accent-text").unwrap(), t.get("accent").unwrap());
        if at < 4.5 {
            failures.push(format!("{name}: accent-text on accent {at:.2}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// new: omarchy 4.5 — §2.6 direct (non-contrast-corrected) mappings
#[test]
fn stock_themes_use_the_proposed_direct_mapping() {
    let direct = [
        ("bg-primary", "background"),
        ("bg-secondary", "dark_background"),
        ("bg-tertiary", "lighter_background"),
        ("border", "muted"),
        ("accent", "accent"),
        ("selection", "selection"),
        ("scrollbar", "muted"),
        ("scrollbar-hover", "dark_foreground"),
        ("success", "green"),
        ("warning", "yellow"),
        ("error", "red"),
        ("code-bg", "dark_background"),
        ("heading-1", "magenta"),
        ("heading-2", "blue"),
        ("heading-3", "cyan"),
        ("folder-icon", "accent"),
        ("config-icon", "yellow"),
    ];
    let mut failures = Vec::new();
    for (name, toml) in stock_themes() {
        let p = OmarchyPalette::parse(&toml, false).unwrap();
        let t = from_omarchy(&p);
        for (token, source) in direct {
            if t.get(token) != p.get(source) {
                failures.push(format!("{name}: {token} = {:?}, {source} = {:?}", t.get(token), p.get(source)));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// new: omarchy 4.5 — syntax roles keep their source color unless it fails contrast
#[test]
fn syntax_roles_keep_source_colors_when_already_readable() {
    let sources = [
        ("syntax-keyword", "magenta"),
        ("syntax-function", "yellow"),
        ("syntax-type", "cyan"),
        ("syntax-string", "green"),
        ("syntax-number", "orange"),
        ("syntax-constant", "bright_cyan"),
        ("syntax-identifier", "foreground"),
        ("syntax-property", "bright_blue"),
        ("syntax-operator", "foreground"),
        ("syntax-comment", "muted"),
        ("syntax-punctuation", "dark_foreground"),
        ("syntax-invalid", "red"),
    ];
    let mut failures = Vec::new();
    for (name, toml) in stock_themes() {
        let p = OmarchyPalette::parse(&toml, false).unwrap();
        let t = from_omarchy(&p);
        let code_bg = t.get("code-bg").unwrap();
        for (token, source) in sources {
            let src = p.get(source).unwrap();
            let got = t.get(token).unwrap();
            if contrast_ratio(src, code_bg) >= 4.51 && got != src {
                failures.push(format!("{name}: {token} changed {src:?} → {got:?} although readable"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// new: omarchy 4.5 — non-color values (gradients) and uppercase hex are tolerated
#[test]
fn tolerates_gradient_strings_and_uppercase_hex() {
    let toml = "mode = \"dark\"\naccent = \"#82FB9C\"\nbackground = \"#0B0C16\"\nforeground = \"#ddf7ff\"\n\
                hyprland_active_border = \"rgba(26a269ee) rgba(2ec27eee) 45deg\"\n";
    let p = OmarchyPalette::parse(toml, false).unwrap();
    assert_eq!(p.get("accent"), Some(rgb("#82fb9c")));
    assert_eq!(p.get("background"), Some(rgb("#0b0c16")));
    assert_eq!(p.get("hyprland_active_border"), None);
}

// new: omarchy 4.5 — legacy short names map to canonical; canonical wins
#[test]
fn legacy_short_names_alias_to_canonical_keys() {
    let p = OmarchyPalette::parse("bg = \"#101010\"\nfg = \"#e0e0e0\"\ndark_fg = \"#808080\"\n", false).unwrap();
    assert_eq!(p.get("background"), Some(rgb("#101010")));
    assert_eq!(p.get("foreground"), Some(rgb("#e0e0e0")));
    assert_eq!(p.get("dark_foreground"), Some(rgb("#808080")));
    let both = OmarchyPalette::parse("bg = \"#101010\"\nbackground = \"#202020\"\n", false).unwrap();
    assert_eq!(both.get("background"), Some(rgb("#202020")));
    // Short names are always exposed as reverse aliases.
    assert_eq!(both.get("bg"), Some(rgb("#202020")));
}

// new: omarchy 4.5 — ANSI color0..color15 fallbacks
#[test]
fn ansi_colors_fill_missing_named_colors() {
    let ansi: String = (0..16).map(|i| format!("color{i} = \"#{:02x}{:02x}{:02x}\"\n", i * 10, i * 10, i * 10)).collect();
    let p = OmarchyPalette::parse(&ansi, false).unwrap();
    let c = |i: u8| Rgb(i * 10, i * 10, i * 10);
    assert_eq!(p.get("background"), Some(c(0)));
    assert_eq!(p.get("foreground"), Some(c(7)));
    for (i, name) in ["red", "green", "yellow", "blue", "magenta", "cyan"].iter().enumerate() {
        assert_eq!(p.get(name), Some(c(i as u8 + 1)), "{name}");
        assert_eq!(p.get(&format!("bright_{name}")), Some(c(i as u8 + 9)), "bright_{name}");
    }
    assert_eq!(p.get("bright_foreground"), Some(c(15)));
    assert_eq!(p.get("dark_foreground"), Some(c(8)));
    assert_eq!(p.get("muted"), Some(c(8)));
    assert_eq!(p.get("selection"), Some(c(8)));
}

// new: omarchy 4.5 — derived keys of resolve_theme_colors
#[test]
fn derived_keys_follow_the_cascade() {
    let toml = "background = \"#204060\"\nforeground = \"#e0e0e0\"\nyellow = \"#c0a000\"\nred = \"#c00000\"\n\
                purple = \"#8000ff\"\nselection_background = \"#333333\"\n";
    let p = OmarchyPalette::parse(toml, false).unwrap();
    let black = Rgb(0, 0, 0);
    let white = Rgb(255, 255, 255);
    let bg = rgb("#204060");
    assert_eq!(p.get("orange"), p.get("yellow"));
    assert_eq!(p.get("brown"), Some(rgb("#c0a000").mix(black, 0.5)));
    assert_eq!(p.get("dark_background"), Some(bg.mix(black, 0.25)));
    assert_eq!(p.get("darker_background"), Some(bg.mix(black, 0.5)));
    assert_eq!(p.get("bright_red"), Some(rgb("#c00000").mix(white, 0.2)));
    assert_eq!(p.get("magenta"), Some(rgb("#8000ff")));
    assert_eq!(p.get("selection"), Some(rgb("#333333")));
    assert_eq!(p.get("selection_background"), Some(rgb("#333333")));
    assert_eq!(p.get("cursor"), p.get("bright_foreground"));
    assert_eq!(p.get("selection_foreground"), p.get("bright_foreground"));
    assert_eq!(p.get("lighter_background"), Some(bg));
}

// new: omarchy 4.5 — mode precedence: mode > theme_type > light.mode > luminance
#[test]
fn mode_resolution_precedence() {
    let dark_bg = "background = \"#101010\"\n";
    let light_bg = "background = \"#f0f0f0\"\n";
    assert_eq!(OmarchyPalette::parse(&format!("mode = \"light\"\ntheme_type = \"dark\"\n{dark_bg}"), false).unwrap().mode, Mode::Light);
    assert_eq!(OmarchyPalette::parse(&format!("theme_type = \"light\"\n{dark_bg}"), false).unwrap().mode, Mode::Light);
    assert_eq!(OmarchyPalette::parse(&format!("mode = \"dark\"\n{light_bg}"), true).unwrap().mode, Mode::Dark);
    assert_eq!(OmarchyPalette::parse(dark_bg, true).unwrap().mode, Mode::Light);
    assert_eq!(OmarchyPalette::parse(light_bg, false).unwrap().mode, Mode::Light);
    assert_eq!(OmarchyPalette::parse(dark_bg, false).unwrap().mode, Mode::Dark);
    // R+G+B > 382 is light: #7f8080 = 383, #7f7f80 = 382.
    assert_eq!(OmarchyPalette::parse("background = \"#7f8080\"\n", false).unwrap().mode, Mode::Light);
    assert_eq!(OmarchyPalette::parse("background = \"#7f7f80\"\n", false).unwrap().mode, Mode::Dark);
}

// new: omarchy 4.5 — mix matches omarchy-theme-color's rounding
#[test]
fn mix_is_linear_srgb_with_rounding() {
    let black = Rgb(0, 0, 0);
    let white = Rgb(255, 255, 255);
    assert_eq!(black.mix(white, 0.5), Rgb(128, 128, 128));
    assert_eq!(black.mix(white, 0.0), black);
    assert_eq!(black.mix(white, 1.0), white);
    assert_eq!(rgb("#c00000").mix(white, 0.2), Rgb(205, 51, 51));
}

// new: omarchy 4.5 — load_current: not installed / installed / mid-swap
#[test]
fn load_current_handles_missing_present_and_mid_swap() {
    let h = OmarchyHome::new();
    let paths = OmarchyPaths::from_home(h.home());
    assert!(load_current(&paths).unwrap().is_none(), "no Omarchy → None");

    let (_, toml) = stock_themes().into_iter().find(|(n, _)| n == "tokyo-night").unwrap();
    h.write(".local/state/omarchy/current/theme/colors.toml", &toml);
    h.write(".local/state/omarchy/current/theme.name", "tokyo-night\n");
    let (name, palette) = load_current(&paths).unwrap().unwrap();
    assert_eq!(name, "tokyo-night");
    assert_eq!(palette, OmarchyPalette::parse(&toml, false).unwrap());

    // `rm -rf current/theme` happened, `mv next-theme theme` not yet.
    let h2 = OmarchyHome::new();
    h2.mkdir(".local/state/omarchy/current");
    h2.write(".local/state/omarchy/current/theme.name", "tokyo-night\n");
    assert!(load_current(&OmarchyPaths::from_home(h2.home())).is_err(), "mid-swap must not look like 'not installed'");
}

// new: omarchy 4.5 — light.mode file next to colors.toml
#[test]
fn load_current_honours_light_mode_file() {
    let h = OmarchyHome::new();
    h.write(".local/state/omarchy/current/theme/colors.toml", "background = \"#101010\"\nforeground = \"#eeeeee\"\n");
    h.write(".local/state/omarchy/current/theme/light.mode", "");
    let (_, p) = load_current(&OmarchyPaths::from_home(h.home())).unwrap().unwrap();
    assert_eq!(p.mode, Mode::Light);
}

// new: omarchy 4.5 — malformed colors.toml is an error, not a silent default theme
#[test]
fn malformed_colors_toml_is_an_error() {
    let h = OmarchyHome::new();
    h.write(".local/state/omarchy/current/theme/colors.toml", "background = \"#101010\nforeground = [\n");
    assert!(load_current(&OmarchyPaths::from_home(h.home())).is_err());
}
