use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconKind {
    Search,
    Palette,
    ChevronDown,
    ChevronRight,
    Check,
    FileCode,
    Copy,
    ArrowLeft,
    ArrowRight,
    Lightbulb,
    Clock,
    Lock,
    Library,
    Code,
    Help,
    Minimize,
    Maximize,
    Close,
}

#[component]
pub fn Icon(kind: IconKind, #[props(default = 16.0)] size: f32) -> Element {
    let paths: &[&str] = match kind {
        IconKind::Search => &["M21 21l-4.35-4.35", "M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14Z"],
        IconKind::Palette => &[
            "M12 22a10 10 0 1 1 10-10c0 2-1 3-3 3h-2a2 2 0 0 0-1.4 3.4c.8.8.4 3.6-3.6 3.6Z",
            "M7.5 10.5h.01",
            "M10.5 6.5h.01",
            "M15 7.5h.01",
        ],
        IconKind::ChevronDown => &["m6 9 6 6 6-6"],
        IconKind::ChevronRight => &["m9 18 6-6-6-6"],
        IconKind::Check => &["m5 12 4 4L19 6"],
        IconKind::FileCode => &[
            "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8Z",
            "M14 2v6h6",
            "m10 13-2 2 2 2",
            "m14 13 2 2-2 2",
        ],
        IconKind::Copy => &["M8 8h12v12H8Z", "M16 8V4H4v12h4"],
        IconKind::ArrowLeft => &["M19 12H5", "m11 18-6-6 6-6"],
        IconKind::ArrowRight => &["M5 12h14", "m13 6 6 6-6 6"],
        IconKind::Lightbulb => &[
            "M9 18h6",
            "M10 22h4",
            "M8.2 14.5A7 7 0 1 1 15.8 14.5c-.7.6-.8 1.2-.8 2.5H9c0-1.3-.1-1.9-.8-2.5Z",
        ],
        IconKind::Clock => &["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Z", "M12 7v5l3 2"],
        IconKind::Lock => &["M6 10h12v11H6Z", "M8 10V7a4 4 0 0 1 8 0v3"],
        IconKind::Library => &["M4 3h4v18H4Z", "M10 3h4v18h-4Z", "m16 4 4-1 3 17-4 1Z"],
        IconKind::Code => &["m8 6-6 6 6 6", "m16 6 6 6-6 6", "m14 4-4 16"],
        IconKind::Help => &[
            "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Z",
            "M9.1 9a3 3 0 0 1 5.8 1c0 2-3 2-3 4",
            "M12 17h.01",
        ],
        IconKind::Minimize => &["M5 12h14"],
        IconKind::Maximize => &["M6 6h12v12H6Z"],
        IconKind::Close => &["M6 6l12 12", "M18 6 6 18"],
    };

    rsx! {
        svg {
            class: "icon",
            width: "{size}",
            height: "{size}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.65",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            for path in paths {
                path { d: *path }
            }
        }
    }
}

#[component]
pub fn IconButton(kind: IconKind, label: &'static str, #[props(default)] active: bool) -> Element {
    let class = if active {
        "icon-button is-active"
    } else {
        "icon-button"
    };

    rsx! {
        button {
            r#type: "button",
            class,
            title: label,
            aria_label: label,
            Icon { kind, size: 15.0 }
        }
    }
}
