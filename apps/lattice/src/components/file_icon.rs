use dioxus::prelude::*;

use crate::file_icons;

#[component]
pub fn FileTypeIcon(
    path: String,
    #[props(default = false)] light: bool,
    #[props(default = 15.0)] size: f32,
) -> Element {
    let svg = file_icons::file_icon_svg(&path, light);
    let style = format!("width: {size}px; height: {size}px;");
    rsx! {
        span {
            class: "file-type-icon",
            style: "{style}",
            aria_hidden: "true",
            dangerous_inner_html: "{svg}",
        }
    }
}

#[component]
pub fn FolderTypeIcon(
    name: String,
    #[props(default = false)] expanded: bool,
    #[props(default = false)] light: bool,
    #[props(default = 15.0)] size: f32,
) -> Element {
    let svg = file_icons::folder_icon_svg(&name, expanded, light);
    let style = format!("width: {size}px; height: {size}px;");
    rsx! {
        span {
            class: "file-type-icon is-folder",
            style: "{style}",
            aria_hidden: "true",
            dangerous_inner_html: "{svg}",
        }
    }
}
