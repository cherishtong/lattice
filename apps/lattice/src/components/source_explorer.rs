use dioxus::prelude::*;

use crate::source::SourceSession;
use crate::theme::Theme;

use super::file_icon::{FileTypeIcon, FolderTypeIcon};
use super::icon::{Icon, IconKind};

#[component]
pub fn SourceExplorer(
    source: SourceSession,
    on_open_library: EventHandler<()>,
    theme: Signal<Theme>,
) -> Element {
    let light = theme.read().is_light();
    let mut filter = use_signal(String::new);
    let state = source.state.read();
    let needle = filter.read().trim().to_lowercase();
    let pack_name = state
        .pack
        .as_ref()
        .map_or("Source Pack", |pack| pack.name());
    let pack_version = state.pack.as_ref().map_or("—", |pack| pack.version());
    let pack_language = state
        .pack
        .as_ref()
        .map_or("C", |pack| pack.source_language());
    let implemented_language = state
        .pack
        .as_ref()
        .map_or("Python", |pack| pack.implemented_language());
    let (pack_mark, pack_mark_class) = match pack_name {
        "CPython" => ("Py", "python-mark"),
        "OpenJDK" => ("Jv", "java-mark"),
        "Rust" => ("Rs", "rust-mark"),
        "TypeScript" => ("Ts", "typescript-mark"),
        "SQLite" => ("Sq", "sqlite-mark"),
        "GCC" => ("Gc", "gcc-mark"),
        "Clang" => ("Cl", "clang-mark"),
        _ => ("C", "gcc-mark"),
    };
    let bootstrapping = state.tree.is_empty() && state.bootstrap_error.is_none();

    rsx! {
        aside { class: "learning-sidebar",
            button { r#type: "button", class: "source-pack", title: "切换源码包",
                onclick: move |_| on_open_library.call(()),
                span { class: "{pack_mark_class}", "{pack_mark}" }
                span { class: "source-pack-copy",
                    span { class: "source-pack-title",
                        strong { "{pack_name}" }
                        span { class: "version-chip", "v{pack_version}" }
                    }
                    span { class: "source-pack-subtitle", "{implemented_language} 实现 · {pack_language} 源码" }
                }
                Icon { kind: IconKind::ChevronDown, size: 14.0 }
            }

            div { class: "source-browser",
                div { class: "source-browser-heading",
                    span { "{state.file_count} 个文件 · 只读" }
                    if bootstrapping {
                        span { class: "source-loading", "读取中…" }
                    }
                }

                div { class: "source-filter",
                    Icon { kind: IconKind::Search, size: 12.0 }
                    input {
                        r#type: "search",
                        placeholder: "筛选文件，例如 listobject.c",
                        value: "{filter}",
                        oninput: move |event| filter.set(event.value()),
                    }
                }

                div { class: "source-tree", aria_label: "源码文件",
                    if state.tree.is_empty() && bootstrapping {
                        div { class: "source-empty", "正在扫描源码包…" }
                    } else if state.tree.is_empty() {
                        div { class: "source-empty",
                            strong { "未找到源码包" }
                            span { "{state.bootstrap_error.as_deref().unwrap_or(\"运行 scripts/fetch-cpython-pack.ps1\")}" }
                        }
                    } else {
                        for entry in state.tree.iter() {
                            if subtree_matches(entry, &needle) {
                                {render_source_entry(entry, 0, source, state.active_path(), &needle, light)}
                            }
                        }
                    }
                }
            }

            button { r#type: "button", class: "library-card",
                onclick: move |_| on_open_library.call(()),
                span { class: "library-card-icon",
                    Icon { kind: IconKind::Library, size: 17.0 }
                }
                span { class: "library-card-copy",
                    strong { "本地源码书库" }
                    span { "{pack_name} {pack_version} · 离线可用" }
                }
                Icon { kind: IconKind::ArrowRight, size: 14.0 }
            }
        }
    }
}

fn render_source_entry(
    entry: &lattice_content::SourceEntry,
    depth: u8,
    source: SourceSession,
    selected: Option<&str>,
    needle: &str,
    light: bool,
) -> Element {
    let force_expand = !needle.is_empty();
    let expanded = entry.is_dir && (force_expand || source.is_expanded(&entry.rel_path));
    let is_selected = !entry.is_dir && selected == Some(entry.rel_path.as_str());
    let class = if is_selected {
        "source-row is-selected"
    } else {
        "source-row"
    };
    let rel_path = entry.rel_path.clone();
    let is_dir = entry.is_dir;

    rsx! {
        button {
            r#type: "button",
            class,
            style: "padding-left: {8 + u32::from(depth) * 13}px",
            title: "{entry.rel_path}",
            onclick: move |_| {
                if is_dir {
                    source.toggle_directory(rel_path.clone());
                } else {
                    source.open(rel_path.clone());
                }
            },
            span { class: "source-chevron",
                if entry.is_dir {
                    if expanded {
                        Icon { kind: IconKind::ChevronDown, size: 11.0 }
                    } else {
                        Icon { kind: IconKind::ChevronRight, size: 11.0 }
                    }
                }
            }
            if entry.is_dir {
                FolderTypeIcon {
                    name: entry.name.clone(),
                    expanded,
                    light,
                    size: 14.0,
                }
            } else {
                FileTypeIcon { path: entry.rel_path.clone(), light, size: 14.0 }
            }
            span { class: "source-name", "{entry.name}" }
        }
        if expanded {
            for child in entry.children.iter() {
                if subtree_matches(child, needle) {
                    {render_source_entry(child, depth + 1, source, selected, needle, light)}
                }
            }
        }
    }
}

fn subtree_matches(entry: &lattice_content::SourceEntry, needle: &str) -> bool {
    if needle.is_empty() || entry.rel_path.to_lowercase().contains(needle) {
        return true;
    }
    entry
        .children
        .iter()
        .any(|child| subtree_matches(child, needle))
}
