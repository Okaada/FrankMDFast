//! Port of `test/services/omarchy_theme_service_test.rb` (2 tests, A), against
//! the faithful legacy port (`frankmd_theme::legacy`). The Ruby test stubs
//! `File.exist?/read`; here the pure `theme_data_from` takes the contents.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use frankmd_theme::legacy::{ThemeData, theme_data_from};

fn alacritty_colors(background: &str, foreground: &str, low_contrast: &str) -> String {
    format!(
        "[colors.primary]\nbackground = \"{background}\"\nforeground = \"{foreground}\"\ndim_foreground = \"{low_contrast}\"\n\
         [colors.normal]\nblack = \"{low_contrast}\"\nred = \"#ff5555\"\ngreen = \"#00ff00\"\nyellow = \"#ffff00\"\n\
         blue = \"#0000ff\"\nmagenta = \"#ff00ff\"\ncyan = \"#00ffff\"\nwhite = \"#ffffff\"\n\
         [colors.bright]\nblue = \"#8080ff\"\ncyan = \"#80ffff\"\ngreen = \"#80ff80\"\nyellow = \"#ffff80\"\n"
    )
}

fn var<'a>(d: &'a ThemeData, name: &str) -> &'a str {
    d.variables
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
        .unwrap_or_else(|| panic!("missing {name}"))
}

fn assert_syntax_contrast(d: &ThemeData) {
    let failures = syntax_contrast_failures(|k| rgb(var(d, &format!("--theme-{k}"))));
    assert!(
        failures.is_empty(),
        "must meet 4.5:1 contrast: {failures:?}"
    );
}

// port: test/services/omarchy_theme_service_test.rb:10 "dark Omarchy palettes derive colors from ANSI and meet code contrast"
#[test]
fn dark_omarchy_palettes_derive_colors_from_ansi_and_meet_code_contrast() {
    let data = theme_data_from(
        "Test Theme\n",
        &alacritty_colors("#141414", "#f0f0f0", "#202020"),
    );
    assert!(data.is_dark);
    assert_eq!(var(&data, "--theme-syntax-keyword"), "#ff00ff");
    assert_syntax_contrast(&data);
    assert_ne!(var(&data, "--theme-syntax-comment"), "#202020");
}

// port: test/services/omarchy_theme_service_test.rb:19 "light Omarchy palettes adjust low-contrast colors and meet code contrast"
#[test]
fn light_omarchy_palettes_adjust_low_contrast_colors_and_meet_code_contrast() {
    let data = theme_data_from(
        "Test Theme\n",
        &alacritty_colors("#eeeeee", "#222222", "#dddddd"),
    );
    assert!(!data.is_dark);
    assert_syntax_contrast(&data);
    assert_ne!(var(&data, "--theme-syntax-comment"), "#dddddd");
}
