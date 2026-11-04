//! Port of `test/i18n_vim_keys_test.rb` (7 locales × 18 keys = 126 generated tests,
//! one Rust test per generated Ruby test). Each key must be Rails-`present?` in
//! `assets/locales/<locale>.yml` (unchanged copy of `config/locales/`) under the
//! `<locale>` root: the vim-mode UI keys.

use frankmd_i18n::{Locale, key_present, locale_tree};

fn assert_defines(code: &str, key: &str) {
    let locale = Locale::from_code(code).expect("shipped locale");
    let tree = locale_tree(locale).expect("locale yaml parses");
    assert!(key_present(&tree, key), "config/locales/{code}.yml is missing {key}");
}

macro_rules! defines {
    ($name:ident, $locale:literal, $key:literal) => {
        // port: test/i18n_vim_keys_test.rb:34 "#{locale} defines #{key}"
        #[test]
        fn $name() {
            assert_defines($locale, $key);
        }
    };
}

defines!(en_defines_settings_vim, "en", "settings.vim");
defines!(en_defines_dialogs_help_tab_vim, "en", "dialogs.help.tab_vim");
defines!(en_defines_dialogs_help_vim_intro, "en", "dialogs.help.vim.intro");
defines!(en_defines_dialogs_help_vim_modes_title, "en", "dialogs.help.vim.modes_title");
defines!(en_defines_dialogs_help_vim_mode_normal, "en", "dialogs.help.vim.mode_normal");
defines!(en_defines_dialogs_help_vim_mode_insert, "en", "dialogs.help.vim.mode_insert");
defines!(en_defines_dialogs_help_vim_mode_visual, "en", "dialogs.help.vim.mode_visual");
defines!(en_defines_dialogs_help_vim_commands_title, "en", "dialogs.help.vim.commands_title");
defines!(en_defines_dialogs_help_vim_col_command, "en", "dialogs.help.vim.col_command");
defines!(en_defines_dialogs_help_vim_col_action, "en", "dialogs.help.vim.col_action");
defines!(en_defines_dialogs_help_vim_cmd_write, "en", "dialogs.help.vim.cmd_write");
defines!(en_defines_dialogs_help_vim_cmd_quit, "en", "dialogs.help.vim.cmd_quit");
defines!(en_defines_dialogs_help_vim_cmd_wq, "en", "dialogs.help.vim.cmd_wq");
defines!(en_defines_dialogs_help_vim_cmd_edit, "en", "dialogs.help.vim.cmd_edit");
defines!(en_defines_dialogs_help_vim_cmd_explore, "en", "dialogs.help.vim.cmd_explore");
defines!(en_defines_dialogs_help_vim_cmd_next, "en", "dialogs.help.vim.cmd_next");
defines!(en_defines_dialogs_help_vim_cmd_prev, "en", "dialogs.help.vim.cmd_prev");
defines!(en_defines_dialogs_help_vim_cmd_help, "en", "dialogs.help.vim.cmd_help");
defines!(es_defines_settings_vim, "es", "settings.vim");
defines!(es_defines_dialogs_help_tab_vim, "es", "dialogs.help.tab_vim");
defines!(es_defines_dialogs_help_vim_intro, "es", "dialogs.help.vim.intro");
defines!(es_defines_dialogs_help_vim_modes_title, "es", "dialogs.help.vim.modes_title");
defines!(es_defines_dialogs_help_vim_mode_normal, "es", "dialogs.help.vim.mode_normal");
defines!(es_defines_dialogs_help_vim_mode_insert, "es", "dialogs.help.vim.mode_insert");
defines!(es_defines_dialogs_help_vim_mode_visual, "es", "dialogs.help.vim.mode_visual");
defines!(es_defines_dialogs_help_vim_commands_title, "es", "dialogs.help.vim.commands_title");
defines!(es_defines_dialogs_help_vim_col_command, "es", "dialogs.help.vim.col_command");
defines!(es_defines_dialogs_help_vim_col_action, "es", "dialogs.help.vim.col_action");
defines!(es_defines_dialogs_help_vim_cmd_write, "es", "dialogs.help.vim.cmd_write");
defines!(es_defines_dialogs_help_vim_cmd_quit, "es", "dialogs.help.vim.cmd_quit");
defines!(es_defines_dialogs_help_vim_cmd_wq, "es", "dialogs.help.vim.cmd_wq");
defines!(es_defines_dialogs_help_vim_cmd_edit, "es", "dialogs.help.vim.cmd_edit");
defines!(es_defines_dialogs_help_vim_cmd_explore, "es", "dialogs.help.vim.cmd_explore");
defines!(es_defines_dialogs_help_vim_cmd_next, "es", "dialogs.help.vim.cmd_next");
defines!(es_defines_dialogs_help_vim_cmd_prev, "es", "dialogs.help.vim.cmd_prev");
defines!(es_defines_dialogs_help_vim_cmd_help, "es", "dialogs.help.vim.cmd_help");
defines!(he_defines_settings_vim, "he", "settings.vim");
defines!(he_defines_dialogs_help_tab_vim, "he", "dialogs.help.tab_vim");
defines!(he_defines_dialogs_help_vim_intro, "he", "dialogs.help.vim.intro");
defines!(he_defines_dialogs_help_vim_modes_title, "he", "dialogs.help.vim.modes_title");
defines!(he_defines_dialogs_help_vim_mode_normal, "he", "dialogs.help.vim.mode_normal");
defines!(he_defines_dialogs_help_vim_mode_insert, "he", "dialogs.help.vim.mode_insert");
defines!(he_defines_dialogs_help_vim_mode_visual, "he", "dialogs.help.vim.mode_visual");
defines!(he_defines_dialogs_help_vim_commands_title, "he", "dialogs.help.vim.commands_title");
defines!(he_defines_dialogs_help_vim_col_command, "he", "dialogs.help.vim.col_command");
defines!(he_defines_dialogs_help_vim_col_action, "he", "dialogs.help.vim.col_action");
defines!(he_defines_dialogs_help_vim_cmd_write, "he", "dialogs.help.vim.cmd_write");
defines!(he_defines_dialogs_help_vim_cmd_quit, "he", "dialogs.help.vim.cmd_quit");
defines!(he_defines_dialogs_help_vim_cmd_wq, "he", "dialogs.help.vim.cmd_wq");
defines!(he_defines_dialogs_help_vim_cmd_edit, "he", "dialogs.help.vim.cmd_edit");
defines!(he_defines_dialogs_help_vim_cmd_explore, "he", "dialogs.help.vim.cmd_explore");
defines!(he_defines_dialogs_help_vim_cmd_next, "he", "dialogs.help.vim.cmd_next");
defines!(he_defines_dialogs_help_vim_cmd_prev, "he", "dialogs.help.vim.cmd_prev");
defines!(he_defines_dialogs_help_vim_cmd_help, "he", "dialogs.help.vim.cmd_help");
defines!(ja_defines_settings_vim, "ja", "settings.vim");
defines!(ja_defines_dialogs_help_tab_vim, "ja", "dialogs.help.tab_vim");
defines!(ja_defines_dialogs_help_vim_intro, "ja", "dialogs.help.vim.intro");
defines!(ja_defines_dialogs_help_vim_modes_title, "ja", "dialogs.help.vim.modes_title");
defines!(ja_defines_dialogs_help_vim_mode_normal, "ja", "dialogs.help.vim.mode_normal");
defines!(ja_defines_dialogs_help_vim_mode_insert, "ja", "dialogs.help.vim.mode_insert");
defines!(ja_defines_dialogs_help_vim_mode_visual, "ja", "dialogs.help.vim.mode_visual");
defines!(ja_defines_dialogs_help_vim_commands_title, "ja", "dialogs.help.vim.commands_title");
defines!(ja_defines_dialogs_help_vim_col_command, "ja", "dialogs.help.vim.col_command");
defines!(ja_defines_dialogs_help_vim_col_action, "ja", "dialogs.help.vim.col_action");
defines!(ja_defines_dialogs_help_vim_cmd_write, "ja", "dialogs.help.vim.cmd_write");
defines!(ja_defines_dialogs_help_vim_cmd_quit, "ja", "dialogs.help.vim.cmd_quit");
defines!(ja_defines_dialogs_help_vim_cmd_wq, "ja", "dialogs.help.vim.cmd_wq");
defines!(ja_defines_dialogs_help_vim_cmd_edit, "ja", "dialogs.help.vim.cmd_edit");
defines!(ja_defines_dialogs_help_vim_cmd_explore, "ja", "dialogs.help.vim.cmd_explore");
defines!(ja_defines_dialogs_help_vim_cmd_next, "ja", "dialogs.help.vim.cmd_next");
defines!(ja_defines_dialogs_help_vim_cmd_prev, "ja", "dialogs.help.vim.cmd_prev");
defines!(ja_defines_dialogs_help_vim_cmd_help, "ja", "dialogs.help.vim.cmd_help");
defines!(ko_defines_settings_vim, "ko", "settings.vim");
defines!(ko_defines_dialogs_help_tab_vim, "ko", "dialogs.help.tab_vim");
defines!(ko_defines_dialogs_help_vim_intro, "ko", "dialogs.help.vim.intro");
defines!(ko_defines_dialogs_help_vim_modes_title, "ko", "dialogs.help.vim.modes_title");
defines!(ko_defines_dialogs_help_vim_mode_normal, "ko", "dialogs.help.vim.mode_normal");
defines!(ko_defines_dialogs_help_vim_mode_insert, "ko", "dialogs.help.vim.mode_insert");
defines!(ko_defines_dialogs_help_vim_mode_visual, "ko", "dialogs.help.vim.mode_visual");
defines!(ko_defines_dialogs_help_vim_commands_title, "ko", "dialogs.help.vim.commands_title");
defines!(ko_defines_dialogs_help_vim_col_command, "ko", "dialogs.help.vim.col_command");
defines!(ko_defines_dialogs_help_vim_col_action, "ko", "dialogs.help.vim.col_action");
defines!(ko_defines_dialogs_help_vim_cmd_write, "ko", "dialogs.help.vim.cmd_write");
defines!(ko_defines_dialogs_help_vim_cmd_quit, "ko", "dialogs.help.vim.cmd_quit");
defines!(ko_defines_dialogs_help_vim_cmd_wq, "ko", "dialogs.help.vim.cmd_wq");
defines!(ko_defines_dialogs_help_vim_cmd_edit, "ko", "dialogs.help.vim.cmd_edit");
defines!(ko_defines_dialogs_help_vim_cmd_explore, "ko", "dialogs.help.vim.cmd_explore");
defines!(ko_defines_dialogs_help_vim_cmd_next, "ko", "dialogs.help.vim.cmd_next");
defines!(ko_defines_dialogs_help_vim_cmd_prev, "ko", "dialogs.help.vim.cmd_prev");
defines!(ko_defines_dialogs_help_vim_cmd_help, "ko", "dialogs.help.vim.cmd_help");
defines!(pt_br_defines_settings_vim, "pt-BR", "settings.vim");
defines!(pt_br_defines_dialogs_help_tab_vim, "pt-BR", "dialogs.help.tab_vim");
defines!(pt_br_defines_dialogs_help_vim_intro, "pt-BR", "dialogs.help.vim.intro");
defines!(pt_br_defines_dialogs_help_vim_modes_title, "pt-BR", "dialogs.help.vim.modes_title");
defines!(pt_br_defines_dialogs_help_vim_mode_normal, "pt-BR", "dialogs.help.vim.mode_normal");
defines!(pt_br_defines_dialogs_help_vim_mode_insert, "pt-BR", "dialogs.help.vim.mode_insert");
defines!(pt_br_defines_dialogs_help_vim_mode_visual, "pt-BR", "dialogs.help.vim.mode_visual");
defines!(pt_br_defines_dialogs_help_vim_commands_title, "pt-BR", "dialogs.help.vim.commands_title");
defines!(pt_br_defines_dialogs_help_vim_col_command, "pt-BR", "dialogs.help.vim.col_command");
defines!(pt_br_defines_dialogs_help_vim_col_action, "pt-BR", "dialogs.help.vim.col_action");
defines!(pt_br_defines_dialogs_help_vim_cmd_write, "pt-BR", "dialogs.help.vim.cmd_write");
defines!(pt_br_defines_dialogs_help_vim_cmd_quit, "pt-BR", "dialogs.help.vim.cmd_quit");
defines!(pt_br_defines_dialogs_help_vim_cmd_wq, "pt-BR", "dialogs.help.vim.cmd_wq");
defines!(pt_br_defines_dialogs_help_vim_cmd_edit, "pt-BR", "dialogs.help.vim.cmd_edit");
defines!(pt_br_defines_dialogs_help_vim_cmd_explore, "pt-BR", "dialogs.help.vim.cmd_explore");
defines!(pt_br_defines_dialogs_help_vim_cmd_next, "pt-BR", "dialogs.help.vim.cmd_next");
defines!(pt_br_defines_dialogs_help_vim_cmd_prev, "pt-BR", "dialogs.help.vim.cmd_prev");
defines!(pt_br_defines_dialogs_help_vim_cmd_help, "pt-BR", "dialogs.help.vim.cmd_help");
defines!(pt_pt_defines_settings_vim, "pt-PT", "settings.vim");
defines!(pt_pt_defines_dialogs_help_tab_vim, "pt-PT", "dialogs.help.tab_vim");
defines!(pt_pt_defines_dialogs_help_vim_intro, "pt-PT", "dialogs.help.vim.intro");
defines!(pt_pt_defines_dialogs_help_vim_modes_title, "pt-PT", "dialogs.help.vim.modes_title");
defines!(pt_pt_defines_dialogs_help_vim_mode_normal, "pt-PT", "dialogs.help.vim.mode_normal");
defines!(pt_pt_defines_dialogs_help_vim_mode_insert, "pt-PT", "dialogs.help.vim.mode_insert");
defines!(pt_pt_defines_dialogs_help_vim_mode_visual, "pt-PT", "dialogs.help.vim.mode_visual");
defines!(pt_pt_defines_dialogs_help_vim_commands_title, "pt-PT", "dialogs.help.vim.commands_title");
defines!(pt_pt_defines_dialogs_help_vim_col_command, "pt-PT", "dialogs.help.vim.col_command");
defines!(pt_pt_defines_dialogs_help_vim_col_action, "pt-PT", "dialogs.help.vim.col_action");
defines!(pt_pt_defines_dialogs_help_vim_cmd_write, "pt-PT", "dialogs.help.vim.cmd_write");
defines!(pt_pt_defines_dialogs_help_vim_cmd_quit, "pt-PT", "dialogs.help.vim.cmd_quit");
defines!(pt_pt_defines_dialogs_help_vim_cmd_wq, "pt-PT", "dialogs.help.vim.cmd_wq");
defines!(pt_pt_defines_dialogs_help_vim_cmd_edit, "pt-PT", "dialogs.help.vim.cmd_edit");
defines!(pt_pt_defines_dialogs_help_vim_cmd_explore, "pt-PT", "dialogs.help.vim.cmd_explore");
defines!(pt_pt_defines_dialogs_help_vim_cmd_next, "pt-PT", "dialogs.help.vim.cmd_next");
defines!(pt_pt_defines_dialogs_help_vim_cmd_prev, "pt-PT", "dialogs.help.vim.cmd_prev");
defines!(pt_pt_defines_dialogs_help_vim_cmd_help, "pt-PT", "dialogs.help.vim.cmd_help");
