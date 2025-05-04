//! Utilities for graphical user interfaces

use {
    dark_light::Mode::{
        Dark,
        Light,
    },
    iced::Theme,
};
#[cfg(target_os = "linux")] use gio::prelude::*;

/// A function which can be used in [`iced::Application::theme`] or [`iced::Daemon::theme`] and returns the built-in light or dark theme based on system preferences.
///
/// Compared to iced's `auto-detect-theme` feature, this function adds compatibility with GNOME.
pub fn theme() -> Theme {
    //TODO automatically update on system theme change (https://github.com/fenhl/wheel/issues/1)
    #[cfg(target_os = "linux")] {
        let settings = gio::Settings::new("org.gnome.desktop.interface");
        if settings.settings_schema().map_or(false, |schema| schema.has_key("color-scheme")) {
            match settings.string("color-scheme").as_str() {
                "prefer-light" => return Theme::Light,
                "prefer-dark" => return Theme::Dark,
                _ => {}
            }
        }
    }
    match dark_light::detect() {
        Ok(Dark) => Theme::Dark,
        Ok(Light) => Theme::Light,
        Ok(dark_light::Mode::Unspecified) => {
            #[cfg(debug_assertions)] { eprintln!("got unspecified system theme") }
            Theme::Light
        }
        #[cfg_attr(not(debug_assertions), allow(unused))] Err(e) => {
            #[cfg(debug_assertions)] { eprintln!("error determining system theme: {e} ({e:?})") }
            Theme::Light
        }
    }
}
