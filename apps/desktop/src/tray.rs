use std::fs::File;
use std::path::Path;

use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::model::AppLocale;

const OPEN_NEW_WINDOW: &str = "open-new-window";
const QUIT: &str = "quit";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayAction {
    OpenNewWindow,
    Quit,
}

impl TrayAction {
    pub fn from_menu_id(id: &str) -> Option<Self> {
        match id {
            OPEN_NEW_WINDOW => Some(Self::OpenNewWindow),
            QUIT => Some(Self::Quit),
            _ => None,
        }
    }
}

pub struct Tray {
    _icon: TrayIcon,
    open_new_window: MenuItem,
    quit: MenuItem,
}

impl Tray {
    pub fn create(icon_path: &Path, locale: Option<AppLocale>) -> Result<Self, String> {
        let labels = labels(locale);
        let menu = Menu::new();
        let open_new_window =
            MenuItem::with_id(OPEN_NEW_WINDOW, labels.open_new_window, true, None);
        let quit = MenuItem::with_id(QUIT, labels.quit, true, None);
        menu.append_items(&[&open_new_window, &PredefinedMenuItem::separator(), &quit])
            .map_err(|error| error.to_string())?;
        let icon = load_icon(icon_path)?;
        let icon = TrayIconBuilder::new()
            .with_id("dsh-desktop")
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .with_tooltip("DSH Desktop")
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            _icon: icon,
            open_new_window,
            quit,
        })
    }

    pub fn update_locale(&self, locale: Option<AppLocale>) {
        let labels = labels(locale);
        self.open_new_window.set_text(labels.open_new_window);
        self.quit.set_text(labels.quit);
    }
}

pub fn menu_action(event: &MenuEvent) -> Option<TrayAction> {
    TrayAction::from_menu_id(event.id().as_ref())
}

struct Labels {
    open_new_window: &'static str,
    quit: &'static str,
}

fn labels(locale: Option<AppLocale>) -> Labels {
    match locale {
        Some(AppLocale::EnUs) => Labels {
            open_new_window: "Open New Window",
            quit: "Quit",
        },
        Some(AppLocale::ZhCn) | None => Labels {
            open_new_window: "打开新窗口",
            quit: "退出",
        },
    }
}

fn load_icon(path: &Path) -> Result<Icon, String> {
    let decoder = png::Decoder::new(File::open(path).map_err(|error| error.to_string())?);
    let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut pixels)
        .map_err(|error| error.to_string())?;
    pixels.truncate(info.buffer_size());
    Icon::from_rgba(pixels, info.width, info.height).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_menu_actions() {
        assert_eq!(
            TrayAction::from_menu_id(OPEN_NEW_WINDOW),
            Some(TrayAction::OpenNewWindow)
        );
        assert_eq!(TrayAction::from_menu_id(QUIT), Some(TrayAction::Quit));
        assert_eq!(TrayAction::from_menu_id("unknown"), None);
    }
}
