mod app;
mod components;
mod config;
mod file_icons;
mod source;
mod theme;
mod translation;

#[cfg(all(feature = "desktop", not(feature = "native")))]
use dioxus::desktop::{Config, WindowBuilder};

#[cfg(all(feature = "desktop", not(feature = "native")))]
use dioxus::desktop::tao::dpi::{PhysicalPosition, PhysicalSize};

#[cfg(all(feature = "desktop", not(feature = "native"), target_os = "windows"))]
use dioxus::desktop::tao::platform::windows::WindowBuilderExtWindows;

#[cfg(all(feature = "desktop", not(feature = "native")))]
fn main() {
    config::SessionConfig::initialize();
    let session = config::SessionConfig::startup();
    let mut window = WindowBuilder::new()
        .with_title("Lattice — 源码阅读")
        .with_decorations(false)
        .with_min_inner_size(dioxus::desktop::LogicalSize::new(1180.0, 760.0));
    window = if let Some(geometry) = session.window
        && geometry.width >= 640
        && geometry.height >= 480
    {
        window
            .with_inner_size(PhysicalSize::new(geometry.width, geometry.height))
            .with_position(PhysicalPosition::new(geometry.x, geometry.y))
            .with_maximized(geometry.maximized)
    } else {
        window.with_inner_size(dioxus::desktop::LogicalSize::new(1520.0, 940.0))
    };
    #[cfg(target_os = "windows")]
    let window = window.with_undecorated_shadow(true);

    let config = Config::new()
        .with_window(window)
        .with_menu(None)
        .with_background_color((20, 23, 27, 255));

    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(app::App);
}

#[cfg(feature = "native")]
fn main() {
    config::SessionConfig::initialize();
    let window = dioxus_native::WindowAttributes::default()
        .with_title("Lattice — 源码阅读（Native）")
        .with_inner_size(dioxus_native::LogicalSize::new(1520.0, 940.0))
        .with_min_inner_size(dioxus_native::LogicalSize::new(1180.0, 760.0));
    let config = dioxus_native::Config::new().with_window_attributes(window);

    dioxus_native::launch_cfg(app::App, Vec::new(), vec![Box::new(config)]);
}

#[cfg(all(feature = "web", not(any(feature = "desktop", feature = "native"))))]
fn main() {
    config::SessionConfig::initialize();
    dioxus::launch(app::App);
}
