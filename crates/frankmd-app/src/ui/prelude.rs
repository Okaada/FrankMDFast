//! Imports shared by the shell's view modules (they all render from the same
//! view models and the same gpui-kit components).

pub(crate) use std::rc::Rc;

pub(crate) use frankmd_config::{Key, Value};
pub(crate) use frankmd_core::RelPath;
pub(crate) use frankmd_workspace::banners::{BannerKind, BannerRole, BannerStack};
pub(crate) use frankmd_workspace::focus::FocusRegion;
pub(crate) use frankmd_workspace::layout::{PreviewMode, SidebarMode, WidthClass};
pub(crate) use frankmd_workspace::overlay::{OverlayKind, PaletteMode};
pub(crate) use frankmd_workspace::status::SaveState;
pub(crate) use frankmd_workspace::Command;
pub(crate) use gpui_kit::component::button::{Button, ButtonVariants as _};
pub(crate) use gpui_kit::component::input::{Input, InputState};
pub(crate) use gpui_kit::component::theme::Theme;
pub(crate) use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
pub(crate) use gpui_kit::component::{Disableable as _, Selectable as _, StyledExt as _};
pub(crate) use gpui_kit::prelude::FluentBuilder as _;

pub(crate) use super::shell::{HelpTab, Shell};
pub(crate) use super::{actions, i18n};
