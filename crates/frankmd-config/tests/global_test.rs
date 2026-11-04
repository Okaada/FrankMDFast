//! The global `config.toml` layer: `.fed > global > ENV > default`.
#![allow(clippy::unwrap_used, clippy::disallowed_methods)]

mod common;

use common::*;
use frankmd_config::dirs::AppDirs;
use frankmd_config::{
    ConfigError, ConfigStore, EnvSnapshot, Key, Provenance, ResolvedConfig, Value,
    global_config_path, load_global, write_global,
};
use secrecy::ExposeSecret;
use std::path::PathBuf;

fn gpath(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("frankmd").join("config.toml")
}

fn rc(fed: Option<&str>, global: Vec<(Key, Option<Value>)>, e: EnvSnapshot) -> ResolvedConfig {
    ResolvedConfig::new(fed, e).with_global(global)
}

#[test]
fn path_is_config_dir_join_file() {
    let dirs = AppDirs {
        config: PathBuf::from("/c/frankmd"),
        state: PathBuf::from("/s"),
        cache: PathBuf::from("/k"),
    };
    assert_eq!(
        global_config_path(&dirs),
        PathBuf::from("/c/frankmd/config.toml")
    );
}

#[test]
fn precedence_fed_global_env_default() {
    let e = env(&[("FRANKMD_LOCALE", "es")]);
    let g = vec![(Key::Locale, s("pt-BR"))];

    let r = rc(Some("locale = fr\n"), g.clone(), e.clone()).get(Key::Locale);
    assert_eq!((r.value, r.source), (s("fr"), Provenance::Fed));

    let r = rc(Some("theme = x\n"), g.clone(), e.clone()).get(Key::Locale);
    assert_eq!((r.value, r.source), (s("pt-BR"), Provenance::Global));

    let r = rc(None, vec![], e).get(Key::Locale);
    assert_eq!(
        (r.value, r.source),
        (s("es"), Provenance::Env("FRANKMD_LOCALE"))
    );

    let r = rc(None, vec![], EnvSnapshot::default()).get(Key::Locale);
    assert_eq!((r.value, r.source), (s("en"), Provenance::Default));
    assert_eq!(Provenance::Global.as_str(), "global");
}

#[test]
fn global_beats_env_even_when_env_set() {
    let r = rc(
        None,
        vec![(Key::Locale, s("de"))],
        env(&[("FRANKMD_LOCALE", "es")]),
    )
    .get(Key::Locale);
    assert_eq!(r.source, Provenance::Global);
}

#[test]
fn empty_global_value_blocks_env_and_default() {
    let r = rc(
        None,
        vec![(Key::Locale, None)],
        env(&[("FRANKMD_LOCALE", "es")]),
    )
    .get(Key::Locale);
    assert_eq!((r.value, r.source), (None, Provenance::Global));
}

#[test]
fn empty_fed_value_blocks_global() {
    let r = rc(
        Some("locale =\n"),
        vec![(Key::Locale, s("de"))],
        EnvSnapshot::default(),
    )
    .get(Key::Locale);
    assert_eq!((r.value, r.source), (None, Provenance::Fed));
}

#[test]
fn in_file_predicates() {
    let c = rc(
        Some("theme = x\n"),
        vec![(Key::Locale, s("de"))],
        EnvSnapshot::default(),
    );
    assert!(c.in_vault_file(Key::Theme) && !c.in_global_file(Key::Theme));
    assert!(c.in_global_file(Key::Locale) && !c.in_vault_file(Key::Locale));
    assert!(!c.in_vault_file(Key::VimMode) && !c.in_global_file(Key::VimMode));
}

#[test]
fn global_ai_credentials_enable_file_only_mode() {
    let e = env(&[
        ("OPENAI_API_KEY", "sk-env"),
        ("ANTHROPIC_API_KEY", "ak-env"),
    ]);
    let c = rc(
        None,
        vec![(Key::AnthropicApiKey, s("ak-global"))],
        e.clone(),
    );
    assert!(c.ai_configured_in_file());
    // ENV openai key is ignored in file-only mode.
    assert_eq!(c.get_ai(Key::OpenaiApiKey).source, Provenance::Default);
    assert_eq!(c.get_ai(Key::AnthropicApiKey).source, Provenance::Global);
    assert_eq!(c.ai_providers_available().len(), 1);
    // .fed still wins over global inside file-only mode.
    let c = rc(
        Some("anthropic_api_key = ak-fed\n"),
        vec![(Key::AnthropicApiKey, s("ak-global"))],
        e,
    );
    assert_eq!(c.get_ai(Key::AnthropicApiKey).source, Provenance::Fed);
}

#[test]
fn image_generation_key_uses_global_between_fed_and_env() {
    let e = env(&[("OPENROUTER_API_KEY", "env")]);
    let c = rc(None, vec![(Key::OpenrouterApiKey, s("glob"))], e.clone());
    assert_eq!(c.image_generation_key().unwrap().expose_secret(), "glob");
    let c = rc(
        Some("openrouter_api_key = fed\n"),
        vec![(Key::OpenrouterApiKey, s("glob"))],
        e,
    );
    assert_eq!(c.image_generation_key().unwrap().expose_secret(), "fed");
}

#[test]
fn write_read_round_trip_keeps_other_keys_and_removes_on_none() {
    let d = tempfile::tempdir().unwrap();
    let p = gpath(&d);
    write_global(
        &p,
        &[
            (Key::Locale, s("de")),
            (Key::EditorFontSize, Some(Value::Int(18))),
            (Key::VimMode, Some(Value::Bool(true))),
        ],
    )
    .unwrap();
    write_global(&p, &[(Key::Theme, s("dark")), (Key::Locale, None)]).unwrap();
    let (vals, warns) = load_global(&p);
    assert!(warns.is_empty(), "{warns:?}");
    let get = |k: Key| vals.iter().find(|(x, _)| *x == k).map(|(_, v)| v.clone());
    assert_eq!(get(Key::Locale), None);
    assert_eq!(get(Key::Theme), Some(s("dark")));
    assert_eq!(get(Key::EditorFontSize), Some(Some(Value::Int(18))));
    assert_eq!(get(Key::VimMode), Some(Some(Value::Bool(true))));
}

#[test]
fn invalid_json_string_for_custom_theme_warns() {
    let d = tempfile::tempdir().unwrap();
    let p = gpath(&d);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "custom_theme = \"{not json\"\ntheme = \"x\"\n").unwrap();
    let (vals, warns) = load_global(&p);
    assert_eq!(vals, vec![(Key::Theme, s("x"))]);
    assert_eq!(warns.len(), 1);
    assert!(warns[0].contains("custom_theme"));
}

#[cfg(unix)]
#[test]
fn file_mode_is_0600_and_parent_created() {
    use std::os::unix::fs::PermissionsExt;
    let d = tempfile::tempdir().unwrap();
    let p = gpath(&d);
    assert!(!p.parent().unwrap().exists());
    write_global(&p, &[(Key::Locale, s("de"))]).unwrap();
    assert_eq!(
        std::fs::metadata(&p).unwrap().permissions().mode() & 0o777,
        0o600
    );
    write_global(&p, &[(Key::Locale, s("fr"))]).unwrap();
    assert_eq!(
        std::fs::metadata(&p).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn missing_file_is_empty_without_warning() {
    let d = tempfile::tempdir().unwrap();
    let (v, w) = load_global(&gpath(&d));
    assert!(v.is_empty() && w.is_empty());
}

#[test]
fn unknown_and_mistyped_keys_warn_and_are_skipped() {
    let d = tempfile::tempdir().unwrap();
    let p = gpath(&d);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(
        &p,
        "bogus = 1\neditor_font_size = \"big\"\nvim_mode = 3\nlocale = 5\nrecent_files = \"[]\"\nauth_token = \"t\"\nshow_stats = true\n",
    )
    .unwrap();
    let (vals, warns) = load_global(&p);
    assert_eq!(vals, vec![(Key::ShowStats, Some(Value::Bool(true)))]);
    assert_eq!(warns.len(), 6, "{warns:?}");
    for name in [
        "bogus",
        "editor_font_size",
        "vim_mode",
        "locale",
        "recent_files",
        "auth_token",
    ] {
        assert!(warns.iter().any(|w| w.contains(name)), "{name}: {warns:?}");
    }
}

#[test]
fn malformed_toml_warns_once() {
    let d = tempfile::tempdir().unwrap();
    let p = gpath(&d);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "this is = = not toml").unwrap();
    let (v, w) = load_global(&p);
    assert!(v.is_empty());
    assert_eq!(w.len(), 1);
}

#[test]
fn recent_files_and_auth_token_are_refused_on_write() {
    let d = tempfile::tempdir().unwrap();
    let p = gpath(&d);
    for k in [Key::RecentFiles, Key::AuthToken] {
        let err = write_global(&p, &[(Key::Locale, s("de")), (k, s("x"))]).unwrap_err();
        assert!(
            matches!(err, ConfigError::GlobalKeyNotAllowed { key } if key == k),
            "{err:?}"
        );
    }
    assert!(!p.exists(), "nothing written when any change is refused");
}

#[test]
fn secrets_are_masked_in_debug_and_entries() {
    let c = rc(
        None,
        vec![
            (Key::AnthropicApiKey, s("sk-super-secret")),
            (Key::Locale, s("de")),
        ],
        EnvSnapshot::default(),
    );
    let dbg = format!("{c:?}");
    assert!(!dbg.contains("sk-super-secret"), "{dbg}");
    assert!(dbg.contains("****"));
    let e = c
        .entries()
        .into_iter()
        .find(|e| e.key == Key::AnthropicApiKey)
        .unwrap();
    assert_eq!(e.source, Provenance::Global);
    assert_eq!(e.value, serde_json::json!("****"));
    let l = c
        .entries()
        .into_iter()
        .find(|e| e.key == Key::Locale)
        .unwrap();
    assert_eq!(
        (l.source, l.value),
        (Provenance::Global, serde_json::json!("de"))
    );
    let masked = c.all_settings_masked();
    assert!(
        masked
            .iter()
            .any(|(k, v)| k == "anthropic_api_key_configured" && *v == serde_json::json!(true))
    );
    assert!(!format!("{masked:?}").contains("sk-super-secret"));
}

#[test]
fn store_update_global_refreshes_current_and_leaves_fed_alone() {
    let v = TestVault::new();
    let d = tempfile::tempdir().unwrap();
    let mut store = v.open_empty_env().with_global(gpath(&d));
    assert_eq!(store.global_path(), Some(gpath(&d).as_path()));
    let fed_before = v.read_fed();
    let rc = store.update_global(&[(Key::Locale, s("de"))]).unwrap();
    assert_eq!(rc.get(Key::Locale).source, Provenance::Global);
    assert_eq!(store.current().get(Key::Locale).value, s("de"));
    assert_eq!(v.read_fed(), fed_before);
    // .fed overrides afterwards, and a .fed write keeps the global layer.
    let rc = store.update(&[(Key::Locale, s("fr"))]).unwrap();
    assert_eq!(rc.get(Key::Locale).source, Provenance::Fed);
    assert!(rc.in_global_file(Key::Locale) && rc.in_vault_file(Key::Locale));
    let rc = store.update_global(&[(Key::Locale, None)]).unwrap();
    assert!(!rc.in_global_file(Key::Locale));
}

#[test]
fn store_update_global_without_path_errors() {
    let v = TestVault::new();
    let mut store: ConfigStore = v.open_empty_env();
    assert!(store.global_path().is_none());
    assert!(matches!(
        store.update_global(&[(Key::Locale, s("de"))]),
        Err(ConfigError::NoGlobalConfig)
    ));
}

#[test]
fn store_reload_picks_up_external_global_edit() {
    let v = TestVault::new();
    let d = tempfile::tempdir().unwrap();
    let p = gpath(&d);
    let mut store = v.open_empty_env().with_global(p.clone());
    assert_eq!(store.current().get(Key::Locale).source, Provenance::Default);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "locale = \"it\"\nbogus = 1\n").unwrap();
    let rc = store.reload().unwrap();
    assert_eq!(rc.get(Key::Locale).value, s("it"));
    assert!(rc.warnings.iter().any(|w| w.contains("bogus")));
}

#[test]
fn no_global_attached_behaves_as_before() {
    let v = TestVault::with_fed("theme = nord\neditor_font_size = 20\n");
    let store = v.open(env(&[("FRANKMD_LOCALE", "es"), ("OPENAI_API_KEY", "k")]));
    let rc = store.current();
    assert!(rc.global_values().is_empty());
    assert_eq!(rc.get(Key::Theme).source, Provenance::Fed);
    assert_eq!(rc.get(Key::EditorFontSize).value, Some(Value::Int(20)));
    assert_eq!(
        rc.get(Key::Locale).source,
        Provenance::Env("FRANKMD_LOCALE")
    );
    assert_eq!(rc.get(Key::VimMode).source, Provenance::Default);
    assert!(!rc.in_global_file(Key::Theme));
    assert!(rc.entries().iter().all(|e| e.source != Provenance::Global));
}
