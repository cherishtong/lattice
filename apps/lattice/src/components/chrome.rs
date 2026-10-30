use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use dioxus::html::Key;
use dioxus::prelude::*;
use lattice_content::{InstallControl, SourcePack};
use lattice_query::{IndexControl, SourceIndex};

use crate::source::{DiffKind, DiffSymbolStatus, SourceSession};
use crate::theme::Theme;

use super::icon::{Icon, IconKind};

#[derive(Clone)]
struct ImportTask {
    install: InstallControl,
    index: IndexControl,
}

impl ImportTask {
    fn new() -> Self {
        Self {
            install: InstallControl::new(),
            index: IndexControl::new(),
        }
    }

    fn cancel(&self) {
        self.install.cancel();
        self.index.cancel();
    }

    fn is_cancelled(&self) -> bool {
        self.install.is_cancelled() || self.index.is_cancelled()
    }
}

fn import_status(task: &ImportTask) -> String {
    let copied = task.install.copied_files();
    let total = task.install.total_files();
    if total > 0 && copied < total {
        return format!("正在复制源码包… {copied} / {total} 个文件");
    }
    let done = task.index.files_done();
    let index_total = task.index.files_total();
    if index_total > 0 {
        return format!("正在建立符号索引… {done} / {index_total} 个文件");
    }
    if total > 0 {
        "源码已复制，正在准备符号索引…".to_owned()
    } else {
        "正在准备导入…".to_owned()
    }
}

#[component]
pub fn TitleBar(
    theme: Signal<Theme>,
    source: SourceSession,
    mut search_open: Signal<bool>,
    mut library_open: Signal<bool>,
    mut help_open: Signal<bool>,
) -> Element {
    #[cfg(all(feature = "desktop", not(feature = "native")))]
    let window = dioxus::desktop::use_window();
    #[cfg(all(feature = "desktop", not(feature = "native")))]
    let drag_window = {
        let window = window.clone();
        move |_: MouseEvent| window.drag()
    };
    #[cfg(all(feature = "desktop", not(feature = "native")))]
    let toggle_window = {
        let window = window.clone();
        move |_: MouseEvent| window.toggle_maximized()
    };
    #[cfg(not(all(feature = "desktop", not(feature = "native"))))]
    let drag_window = move |_: MouseEvent| {};
    #[cfg(not(all(feature = "desktop", not(feature = "native"))))]
    let toggle_window = move |_: MouseEvent| {};
    let navigation = [
        (IconKind::Code, "源码", true),
        (IconKind::Search, "搜索", false),
    ];

    rsx! {
        header { class: "title-bar",
            div { class: "brand title-drag-region",
                onmousedown: drag_window,
                ondoubleclick: toggle_window,
                LatticeLogo {}
                div { class: "brand-copy",
                    span { class: "brand-name", "Lattice" }
                    span { class: "brand-tagline", "源码阅读" }
                }
            }

            nav { class: "primary-nav", aria_label: "主导航",
                for (kind, label, active) in navigation {
                    button {
                        r#type: "button",
                        class: if active { "nav-item is-active" } else { "nav-item" },
                        title: if kind == IconKind::Search { "搜索（Ctrl+K）" } else { "" },
                        onclick: move |_| {
                            if kind == IconKind::Search {
                                search_open.set(true);
                                library_open.set(false);
                                help_open.set(false);
                            }
                        },
                        Icon { kind, size: 15.0 }
                        span { "{label}" }
                    }
                }
            }

            button {
                r#type: "button",
                class: "global-search",
                title: "搜索源码包（Ctrl+K）",
                onclick: move |_| {
                    search_open.set(true);
                    library_open.set(false);
                    help_open.set(false);
                },
                Icon { kind: IconKind::Search, size: 15.0 }
                span { class: "search-placeholder", "搜索文件或符号…" }
            }

            div { class: "title-actions",
                span { class: "offline-badge",
                    span { class: "status-dot" }
                    "离线就绪"
                }
                ThemeSwitcher { theme }
                button {
                    r#type: "button",
                    class: "icon-button",
                    title: "快捷键帮助（?）",
                    aria_label: "快捷键帮助",
                    onclick: move |_| {
                        help_open.set(true);
                        search_open.set(false);
                        library_open.set(false);
                    },
                    Icon { kind: IconKind::Help, size: 15.0 }
                }
                button {
                    r#type: "button",
                    class: "icon-button",
                    title: "本地源码书库",
                    aria_label: "本地源码书库",
                    onclick: move |_| {
                        library_open.set(true);
                        search_open.set(false);
                        help_open.set(false);
                    },
                    Icon { kind: IconKind::Library, size: 15.0 }
                }
                WindowControls {}
            }

            if search_open() {
                GlobalSearch {
                    source,
                    on_close: move |_| search_open.set(false),
                }
            }
            if library_open() {
                LibraryPanel {
                    source,
                    on_close: move |_| library_open.set(false),
                }
            }
            if help_open() {
                ShortcutHelp { on_close: move |_| help_open.set(false) }
            }
        }
    }
}

#[cfg(all(feature = "desktop", not(feature = "native")))]
#[component]
fn WindowControls() -> Element {
    let window = dioxus::desktop::use_window();
    let minimize = {
        let window = window.clone();
        move |_| window.set_minimized(true)
    };
    let toggle = {
        let window = window.clone();
        move |_| window.toggle_maximized()
    };
    rsx! {
        div { class: "window-controls",
            button {
                r#type: "button",
                class: "window-control",
                title: "最小化",
                aria_label: "最小化",
                onclick: minimize,
                Icon { kind: IconKind::Minimize, size: 12.0 }
            }
            button {
                r#type: "button",
                class: "window-control",
                title: "最大化或还原",
                aria_label: "最大化或还原",
                onclick: toggle,
                Icon { kind: IconKind::Maximize, size: 11.0 }
            }
            button {
                r#type: "button",
                class: "window-control is-close",
                title: "关闭",
                aria_label: "关闭",
                onclick: move |_| window.close(),
                Icon { kind: IconKind::Close, size: 12.0 }
            }
        }
    }
}

#[cfg(not(all(feature = "desktop", not(feature = "native"))))]
#[component]
fn WindowControls() -> Element {
    rsx! {}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchFilter {
    All,
    Symbols,
    Files,
}

#[component]
fn GlobalSearch(source: SourceSession, on_close: EventHandler<()>) -> Element {
    let mut query = use_signal(String::new);
    let mut active = use_signal(|| 0usize);
    let mut filter_signal = use_signal(|| SearchFilter::All);
    let mut case_sensitive = use_signal(|| false);
    let mut path_filter = use_signal(String::new);
    let needle = query.read().clone();
    let path_needle = path_filter.read().clone();
    let case_sensitive_now = case_sensitive();
    let index = source.index.read();
    let results = index.as_ref().map_or_else(Default::default, |index| {
        index.search_filtered(&needle, 12, 8, case_sensitive_now, &path_needle)
    });
    let recent_searches = source.recent_searches.read().clone();
    let indexing = *source.index_loading.read();
    let index_error = source.index_error.read().clone();
    let filter = filter_signal();
    let show_symbols = matches!(filter, SearchFilter::All | SearchFilter::Symbols);
    let show_files = matches!(filter, SearchFilter::All | SearchFilter::Files);
    let mut targets: Vec<(String, u32)> = Vec::new();
    if show_symbols {
        targets.extend(
            results
                .symbols
                .iter()
                .map(|symbol| (symbol.path.clone(), symbol.start_line)),
        );
    }
    if show_files {
        targets.extend(results.files.iter().map(|file| (file.path.clone(), 1)));
    }
    let total = targets.len();
    let active_index = if total == 0 { 0 } else { active() % total };

    use_effect(move || {
        let _ = active();
        let _ = total;
        dioxus::document::eval(
            "requestAnimationFrame(() => {
                const el = document.querySelector('.search-result.is-active');
                if (el) el.scrollIntoView({ block: 'nearest' });
            });",
        );
    });

    rsx! {
        button {
            r#type: "button",
            class: "search-backdrop",
            aria_label: "关闭搜索",
            onclick: move |_| on_close.call(()),
        }
        section { class: "search-palette", role: "dialog", aria_label: "源码包搜索",
            div { class: "search-palette-head",
                Icon { kind: IconKind::Search, size: 17.0 }
                input {
                    r#type: "search",
                    class: "search-palette-input",
                    placeholder: "搜索文件、函数、宏、类型…",
                    value: "{needle}",
                    autofocus: true,
                    oninput: move |event| {
                        query.set(event.value());
                        active.set(0);
                    },
                    onkeydown: move |event| match event.key() {
                            Key::ArrowDown => {
                                if total > 0 {
                                    active.set((active_index + 1) % total);
                                    event.prevent_default();
                                }
                            }
                            Key::ArrowUp => {
                                if total > 0 {
                                    active.set((active_index + total - 1) % total);
                                    event.prevent_default();
                                }
                            }
                            Key::Enter => {
                                if let Some((path, line)) = targets.get(active_index).cloned() {
                                    event.prevent_default();
                                    source.remember_search(needle.clone());
                                    source.open_at(path, line);
                                    on_close.call(());
                                }
                            }
                            _ => {}
                        }
                }
                if indexing {
                    span { class: "search-index-status", "索引中…" }
                } else if let Some(index) = index.as_ref() {
                    span { class: "search-index-status",
                        "{index.symbol_count()} 符号"
                    }
                }
                button {
                    r#type: "button",
                    class: "search-close",
                    title: "关闭",
                    onclick: move |_| on_close.call(()),
                    "Esc"
                }
            }

            div { class: "search-filter-row",
                for (value, label) in [
                    (SearchFilter::All, "全部"),
                    (SearchFilter::Symbols, "符号"),
                    (SearchFilter::Files, "文件"),
                ] {
                    button {
                        r#type: "button",
                        class: if filter == value { "search-filter is-active" } else { "search-filter" },
                        onclick: move |_| {
                            filter_signal.set(value);
                            active.set(0);
                        },
                        "{label}"
                    }
                }
                button {
                    r#type: "button",
                    class: if case_sensitive_now { "search-filter is-active" } else { "search-filter" },
                    title: "区分大小写",
                    onclick: move |_| {
                        case_sensitive.set(!case_sensitive_now);
                        active.set(0);
                    },
                    "Aa"
                }
                input {
                    r#type: "search",
                    class: "search-path-filter",
                    placeholder: "路径包含…",
                    value: "{path_needle}",
                    oninput: move |event| {
                        path_filter.set(event.value());
                        active.set(0);
                    },
                }
            }

            div { class: "search-results",
                if let Some(error) = index_error {
                    div { class: "search-empty", "符号索引失败：{error}" }
                } else if needle.trim().is_empty() {
                    if recent_searches.is_empty() {
                        div { class: "search-empty",
                            strong { "在整个源码包中查找" }
                            span { "输入文件名、函数名、宏或类型名。结果会直接打开到对应位置。" }
                        }
                    } else {
                        div { class: "search-recent",
                            div { class: "search-group-title", "最近搜索" }
                            for entry in recent_searches.iter() {
                                button {
                                    r#type: "button",
                                    key: "{entry}",
                                    class: "search-recent-row",
                                    onclick: {
                                        let entry = entry.clone();
                                        move |_| {
                                            query.set(entry.clone());
                                            active.set(0);
                                        }
                                    },
                                    Icon { kind: IconKind::Clock, size: 11.0 }
                                    span { "{entry}" }
                                }
                            }
                        }
                    }
                } else if total == 0 {
                    div { class: "search-empty",
                        strong { "没有匹配结果" }
                        span { if indexing { "符号索引仍在建立，请稍后再试。" } else { "尝试更短的关键字或切换筛选条件。" } }
                    }
                } else {
                    if show_symbols && !results.symbols.is_empty() {
                        div { class: "search-group-title", "符号" }
                        for (index, symbol) in results.symbols.iter().enumerate() {
                            button {
                                r#type: "button",
                                key: "{symbol.path}-{symbol.start_line}-{symbol.name}",
                                class: if index == active_index { "search-result is-active" } else { "search-result" },
                                onclick: {
                                    let path = symbol.path.clone();
                                    let line = symbol.start_line;
                                    let remembered = needle.clone();
                                    move |_| {
                                        source.remember_search(remembered.clone());
                                        source.open_at(path.clone(), line);
                                        on_close.call(());
                                    }
                                },
                                span { class: "search-result-glyph", "{symbol.kind.glyph()}" }
                                span { class: "search-result-copy",
                                    strong { "{symbol.name}" }
                                    span { "{symbol.kind.label()} · {symbol.path}:{symbol.start_line}" }
                                }
                                Icon { kind: IconKind::ArrowRight, size: 12.0 }
                            }
                        }
                    }
                    if show_files && !results.files.is_empty() {
                        div { class: "search-group-title", "文件" }
                        for (index, file) in results.files.iter().enumerate() {
                            button {
                                r#type: "button",
                                key: "{file.path}",
                                class: if results.symbols.len() + index == active_index { "search-result is-active" } else { "search-result" },
                                onclick: {
                                    let path = file.path.clone();
                                    let remembered = needle.clone();
                                    move |_| {
                                        source.remember_search(remembered.clone());
                                        source.open_at(path.clone(), 1);
                                        on_close.call(());
                                    }
                                },
                                span { class: "search-result-glyph is-file", "F" }
                                span { class: "search-result-copy",
                                    strong { "{file.name}" }
                                    span { "{file.path}" }
                                }
                                Icon { kind: IconKind::ArrowRight, size: 12.0 }
                            }
                        }
                    }
                }
            }

            div { class: "search-palette-foot",
                span { "↑↓ 选择 · Enter 打开 · Esc 关闭" }
                span { "只读 · 离线" }
            }
        }
    }
}

#[component]
fn ShortcutHelp(on_close: EventHandler<()>) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "search-backdrop",
            aria_label: "关闭快捷键帮助",
            onclick: move |_| on_close.call(()),
        }
        section { class: "search-palette shortcut-help", role: "dialog", aria_label: "快捷键帮助",
            div { class: "search-palette-head",
                Icon { kind: IconKind::Help, size: 17.0 }
                div { class: "library-head-copy",
                    strong { "快捷键与操作" }
                    span { "按 ? 或 Ctrl+/ 随时打开" }
                }
                button {
                    r#type: "button",
                    class: "search-close",
                    title: "关闭",
                    onclick: move |_| on_close.call(()),
                    "Esc"
                }
            }

            div { class: "shortcut-help-body",
                for group in SHORTCUT_GROUPS {
                    section { class: "shortcut-group",
                        h3 { "{group.title}" }
                        for (label, keys) in group.items {
                            div { class: "shortcut-row",
                                span { class: "shortcut-label", "{label}" }
                                span { class: "shortcut-keys",
                                    for key in *keys {
                                        kbd { "{key}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "search-palette-foot",
                span { "窗口：拖动左上角品牌区 · 双击最大化" }
                span { "只读 · 离线" }
            }
        }
    }
}

struct ShortcutGroup {
    title: &'static str,
    items: &'static [(&'static str, &'static [&'static str])],
}

const SHORTCUT_GROUPS: &[ShortcutGroup] = &[
    ShortcutGroup {
        title: "搜索与跳转",
        items: &[
            ("全局搜索", &["Ctrl", "K"]),
            ("文件内查找", &["Ctrl", "F"]),
            ("选择搜索结果", &["↑", "↓"]),
            ("打开搜索结果", &["Enter"]),
            ("下一个匹配", &["Enter"]),
            ("上一个匹配", &["Shift", "Enter"]),
            ("折叠 / 展开代码", &["行号旁", "箭头"]),
            ("折叠 / 展开当前", &["Alt", "["]),
            ("折叠全部", &["Alt", "Shift", "["]),
            ("展开全部", &["Alt", "]"]),
            ("关闭面板", &["Esc"]),
        ],
    },
    ShortcutGroup {
        title: "标签与历史",
        items: &[
            ("下一个标签", &["Ctrl", "Tab"]),
            ("上一个标签", &["Ctrl", "Shift", "Tab"]),
            ("关闭当前标签", &["Ctrl", "W"]),
            ("后退", &["Alt", "←"]),
            ("前进", &["Alt", "→"]),
        ],
    },
    ShortcutGroup {
        title: "界面缩放",
        items: &[
            ("放大", &["Ctrl", "="]),
            ("缩小", &["Ctrl", "-"]),
            ("恢复 100%", &["Ctrl", "0"]),
            ("鼠标缩放", &["Ctrl", "滚轮"]),
            ("帮助", &["?"]),
            ("帮助（备用）", &["Ctrl", "/"]),
        ],
    },
    ShortcutGroup {
        title: "窗口",
        items: &[
            ("移动窗口", &["拖动品牌区"]),
            ("最大化 / 还原", &["双击品牌区"]),
            ("最小化 / 关闭", &["右上角按钮"]),
        ],
    },
];

#[component]
pub fn DiffPanel(source: SourceSession) -> Element {
    let diff = source.diff.read();
    let loading = *source.diff_loading.read();
    let error = source.diff_error.read().clone();
    if diff.is_none() && !loading && error.is_none() {
        return rsx! {};
    }

    rsx! {
        button {
            r#type: "button",
            class: "search-backdrop",
            aria_label: "关闭版本对比",
            onclick: move |_| source.close_diff(),
        }
        section { class: "search-palette diff-palette", role: "dialog", aria_label: "版本对比",
            div { class: "search-palette-head",
                Icon { kind: IconKind::Code, size: 17.0 }
                div { class: "library-head-copy",
                    strong { "版本对比" }
                    if let Some(diff) = diff.as_ref() {
                        span { "{diff.base_label}  →  {diff.other_label}" }
                    } else {
                        span { "比较同一文件在不同版本中的差异" }
                    }
                }
                button {
                    r#type: "button",
                    class: "search-close",
                    title: "关闭",
                    onclick: move |_| source.close_diff(),
                    "Esc"
                }
            }

            if loading {
                div { class: "code-state",
                    span { class: "source-spinner" }
                    strong { "正在生成差异…" }
                }
            } else if let Some(error) = error {
                div { class: "code-state is-error",
                    strong { "无法生成对比" }
                    span { "{error}" }
                }
            } else if let Some(diff) = diff.as_ref() {
                div { class: "diff-summary",
                    span { class: "diff-path", "{diff.path}" }
                    span { class: "diff-added", "+{diff.added}" }
                    span { class: "diff-removed", "−{diff.removed}" }
                    span { class: "diff-symbol-count", "{diff.symbols.len()} 个符号变化" }
                }
                if !diff.symbols.is_empty() {
                    div { class: "diff-symbols",
                        for symbol in diff.symbols.iter().take(80) {
                            span {
                                key: "{symbol.name}-{symbol.status:?}",
                                class: match symbol.status {
                                    DiffSymbolStatus::Added => "diff-symbol is-added",
                                    DiffSymbolStatus::Removed => "diff-symbol is-removed",
                                    DiffSymbolStatus::Changed => "diff-symbol is-changed",
                                },
                                title: "{symbol.kind.label()}",
                                span { class: "diff-symbol-glyph", "{symbol.kind.glyph()}" }
                                "{symbol.name}"
                            }
                        }
                        if diff.symbols.len() > 80 {
                            span { class: "diff-symbol-more", "还有 {diff.symbols.len() - 80} 个" }
                        }
                    }
                }
                div { class: "diff-scroll",
                    for (index, line) in diff.lines.iter().enumerate() {
                        div {
                            key: "diff-{index}",
                            class: match line.kind {
                                DiffKind::Same => "diff-line",
                                DiffKind::Added => "diff-line is-added",
                                DiffKind::Removed => "diff-line is-removed",
                            },
                            span { class: "diff-number", "{line.old_line.map_or_else(String::new, |line| line.to_string())}" }
                            span { class: "diff-number", "{line.new_line.map_or_else(String::new, |line| line.to_string())}" }
                            span { class: "diff-sign",
                                match line.kind {
                                    DiffKind::Added => "+",
                                    DiffKind::Removed => "−",
                                    DiffKind::Same => " ",
                                }
                            }
                            span { class: "diff-text", "{line.text}" }
                        }
                    }
                }
                div { class: "search-palette-foot",
                    span { if diff.truncated { "差异过大，仅显示前 4,000 行记录。" } else { "相同行不计数。" } }
                    span { "{diff.lines.len()} 条记录" }
                }
            }
        }
    }
}

#[component]
fn LibraryPanel(source: SourceSession, on_close: EventHandler<()>) -> Element {
    let busy = use_signal(|| false);
    let mut notice = use_signal(|| None::<String>);
    let mut confirm_uninstall = use_signal(|| None::<String>);
    let mut import_open = use_signal(|| false);
    let mut import_path = use_signal(String::new);
    let import_task = use_signal(|| None::<ImportTask>);
    let packs = source.installed_packs.read();
    let current_root = source
        .state
        .read()
        .pack
        .as_ref()
        .map(|pack| pack.root().to_path_buf());
    let active_name = source
        .state
        .read()
        .pack
        .as_ref()
        .map(|pack| pack.name().to_owned());
    let active_file = source.state.read().active_path().map(str::to_owned);
    let busy_now = *busy.read();
    let notice_now = notice.read().clone();
    let confirm_now = confirm_uninstall.read().clone();
    let import_open_now = *import_open.read();
    let import_path_now = import_path.read().clone();
    let importing_now = import_task.read().is_some();

    rsx! {
        button {
            r#type: "button",
            class: "search-backdrop",
            aria_label: "关闭源码书库",
            onclick: move |_| on_close.call(()),
        }
        section { class: "search-palette library-palette", role: "dialog", aria_label: "本地源码书库",
            div { class: "search-palette-head",
                Icon { kind: IconKind::Library, size: 17.0 }
                div { class: "library-head-copy",
                    strong { "本地源码书库" }
                    span { "已发现 {packs.len()} 个源码包" }
                }
                button {
                    r#type: "button",
                    class: "search-close",
                    title: "关闭",
                    onclick: move |_| on_close.call(()),
                    "Esc"
                }
            }

            div { class: "search-results",
                if packs.is_empty() {
                    div { class: "search-empty",
                        strong { "没有发现源码包" }
                        span { "点击下方“导入源码包”，或把目录放入 .content 后重启。" }
                    }
                } else {
                    for pack in packs.iter() {
                        {
                            let active = current_root.as_deref() == Some(pack.root());
                            let glyph = match pack.name() {
                                "CPython" => "Py",
                                "OpenJDK" => "Jv",
                                "Rust" => "Rs",
                                "TypeScript" => "Ts",
                                "SQLite" => "Sq",
                                "Nginx" => "Nx",
                                "Spring Boot" => "Sp",
                                "Vue" => "Vu",
                                "React" => "Re",
                                _ => "C",
                            };
                            let root_label = pack.root().display().to_string();
                            let pending_uninstall =
                                confirm_now.as_deref() == Some(root_label.as_str());
                            let comparable = !active
                                && active_name.as_deref() == Some(pack.name())
                                && active_file.is_some();
                            rsx! {
                                div {
                                    key: "{pack.root().display()}",
                                    class: if active { "library-pack-row is-active" } else { "library-pack-row" },
                                    span { class: "library-pack-glyph", "{glyph}" }
                                    div { class: "library-pack-copy",
                                        div { class: "library-pack-title",
                                            strong { "{pack.name()}" }
                                            span { class: "version-chip", "v{pack.version()}" }
                                            if active {
                                                span { class: "library-active-chip", "使用中" }
                                            }
                                        }
                                        span { class: "library-pack-meta",
                                            "{pack.implemented_language()} · {pack.source_language()} 源码"
                                        }
                                        span { class: "library-pack-path", "{pack.root().display()}" }
                                    }
                                    if active {
                                        span { class: "library-current-label", "当前" }
                                    } else {
                                        div { class: "library-actions",
                                            if comparable {
                                                button {
                                                    r#type: "button",
                                                    class: "library-compare",
                                                    disabled: busy_now,
                                                    onclick: {
                                                        let pack = pack.clone();
                                                        move |_| {
                                                            source.open_diff(pack.clone());
                                                            on_close.call(());
                                                        }
                                                    },
                                                    "对比"
                                                }
                                            }
                                            button {
                                                r#type: "button",
                                                class: if pending_uninstall { "library-uninstall is-confirming" } else { "library-uninstall" },
                                                disabled: busy_now,
                                                onclick: {
                                                    let pack = pack.clone();
                                                    let root_label = root_label.clone();
                                                    move |_| {
                                                        if pending_uninstall {
                                                            confirm_uninstall.set(None);
                                                            uninstall_source_pack(source, pack.clone(), busy, notice);
                                                        } else {
                                                            confirm_uninstall.set(Some(root_label.clone()));
                                                        }
                                                    }
                                                },
                                                if pending_uninstall { "确认卸载" } else { "卸载" }
                                            }
                                            button {
                                                r#type: "button",
                                                class: "library-switch",
                                                disabled: busy_now,
                                                onclick: {
                                                    let pack = pack.clone();
                                                    move |_| {
                                                        confirm_uninstall.set(None);
                                                        source.switch_pack(pack.clone());
                                                        on_close.call(());
                                                    }
                                                },
                                                "切换"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if import_open_now {
                div { class: "library-import-row",
                    input {
                        r#type: "text",
                        class: "library-import-input",
                        placeholder: "源码包目录或 ZIP，例如 E:\\sources\\rust-std.zip",
                        value: "{import_path_now}",
                        oninput: move |event| import_path.set(event.value()),
                    }
                    button {
                        r#type: "button",
                        class: "library-import-submit",
                        disabled: busy_now || import_path_now.trim().is_empty(),
                        onclick: move |_| {
                            import_source_pack(
                                source,
                                import_path_now.clone(),
                                busy,
                                notice,
                                import_open,
                                import_path,
                                import_task,
                            );
                        },
                        "安装"
                    }
                    button {
                        r#type: "button",
                        class: "library-import-cancel",
                        disabled: busy_now,
                        onclick: move |_| {
                            import_open.set(false);
                            import_path.set(String::new());
                            notice.set(None);
                        },
                        "取消"
                    }
                }
            }

            if let Some(message) = notice_now {
                div { class: "library-notice",
                    span { "{message}" }
                    if importing_now {
                        button {
                            r#type: "button",
                            class: "library-import-cancel",
                            onclick: move |_| {
                                if let Some(task) = import_task.read().as_ref() {
                                    task.cancel();
                                    notice.set(Some("正在取消导入…".to_owned()));
                                }
                            },
                            "取消导入"
                        }
                    }
                }
            }

            div { class: "search-palette-foot",
                button {
                    r#type: "button",
                    class: "library-import",
                    disabled: busy_now,
                    onclick: move |_| {
                        confirm_uninstall.set(None);
                        notice.set(None);
                        import_open.set(!import_open_now);
                    },
                    if busy_now { "正在导入…" } else { "导入源码包" }
                }
                span { "只读 · 离线" }
            }
        }
    }
}

fn import_source_pack(
    source: SourceSession,
    directory: String,
    mut busy: Signal<bool>,
    mut notice: Signal<Option<String>>,
    mut import_open: Signal<bool>,
    mut import_path: Signal<String>,
    mut import_task: Signal<Option<ImportTask>>,
) {
    let directory = directory.trim().trim_matches('"').to_owned();
    if directory.is_empty() {
        notice.set(Some("请输入源码包目录。".to_owned()));
        return;
    }
    busy.set(true);
    notice.set(Some("正在复制源码包…".to_owned()));
    let task = ImportTask::new();
    import_task.set(Some(task.clone()));

    let done = Arc::new(AtomicBool::new(false));
    let ticker_task = task.clone();
    let ticker_done = done.clone();
    dioxus::dioxus_core::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(180)).await;
            if ticker_done.load(Ordering::Relaxed) {
                break;
            }
            notice.set(Some(import_status(&ticker_task)));
        }
    });

    dioxus::dioxus_core::spawn_forever(async move {
        let install_control = task.install.clone();
        let is_zip = directory.to_ascii_lowercase().ends_with(".zip");
        let result = tokio::task::spawn_blocking(move || {
            if is_zip {
                SourcePack::install_zip_with(std::path::Path::new(&directory), &install_control)
            } else {
                SourcePack::install_from_with(std::path::Path::new(&directory), &install_control)
            }
        })
        .await;
        let message = match result {
            Ok(Ok(pack)) => {
                let name = pack.name().to_owned();
                source.refresh_installed_packs();
                import_open.set(false);
                import_path.set(String::new());
                if task.is_cancelled() {
                    done.store(true, Ordering::Relaxed);
                    import_task.set(None);
                    notice.set(Some(format!(
                        "已取消「{name}」的索引建立；源码包已保留，可在书库中重新导入。"
                    )));
                    busy.set(false);
                    return;
                }
                notice.set(Some(format!("已导入「{name}」，正在建立符号索引…")));
                let index_control = task.index.clone();
                let prepared = tokio::task::spawn_blocking(move || {
                    let tree = pack.scan()?;
                    SourceIndex::load_or_build_with_control(&pack, &tree, &index_control)
                })
                .await;
                if task.is_cancelled() {
                    format!("已取消「{name}」的符号索引；源码包已保留，可稍后重新建立。")
                } else {
                    match prepared {
                        Ok(Ok(Some(_))) => format!("已导入「{name}」，符号索引已就绪。"),
                        Ok(Ok(None)) => format!("已取消「{name}」的符号索引；源码包已保留。"),
                        Ok(Err(error)) => format!("已导入「{name}」，但符号索引建立失败：{error}"),
                        Err(error) => format!("已导入「{name}」，但符号索引任务失败：{error}"),
                    }
                }
            }
            Ok(Err(error)) if error == lattice_content::INSTALL_CANCELLED => {
                import_open.set(false);
                import_path.set(String::new());
                "已取消导入，未留下半成品源码包。".to_owned()
            }
            Ok(Err(error)) => format!("导入失败：{error}"),
            Err(error) => format!("导入失败：{error}"),
        };
        done.store(true, Ordering::Relaxed);
        import_task.set(None);
        notice.set(Some(message));
        busy.set(false);
    });
}

fn uninstall_source_pack(
    source: SourceSession,
    pack: SourcePack,
    mut busy: Signal<bool>,
    mut notice: Signal<Option<String>>,
) {
    busy.set(true);
    notice.set(None);
    let name = pack.name().to_owned();
    dioxus::dioxus_core::spawn_forever(async move {
        let result = tokio::task::spawn_blocking(move || pack.uninstall()).await;
        let message = match result {
            Ok(Ok(())) => {
                source.refresh_installed_packs();
                format!("已卸载「{name}」。")
            }
            Ok(Err(error)) => format!("卸载失败：{error}"),
            Err(error) => format!("卸载失败：{error}"),
        };
        notice.set(Some(message));
        busy.set(false);
    });
}

#[component]
fn ThemeSwitcher(theme: Signal<Theme>) -> Element {
    let mut open = use_signal(|| false);
    let current = *theme.read();

    rsx! {
        div { class: "theme-switcher",
            button {
                r#type: "button",
                class: if open() { "theme-trigger is-open" } else { "theme-trigger" },
                title: "选择界面主题",
                aria_label: "选择界面主题",
                onclick: move |_| open.toggle(),
                Icon { kind: IconKind::Palette, size: 14.0 }
                span { class: "theme-trigger-label", "{current.label()}" }
                Icon { kind: IconKind::ChevronDown, size: 11.0 }
            }

            if open() {
                button {
                    r#type: "button",
                    class: "theme-menu-backdrop",
                    aria_label: "关闭主题菜单",
                    onclick: move |_| open.set(false),
                }
                div { class: "theme-menu", role: "menu",
                    div { class: "theme-menu-heading",
                        strong { "外观主题" }
                        span { "即时切换" }
                    }
                    div { class: "theme-options",
                        for option in Theme::ALL {
                            button {
                                r#type: "button",
                                key: "{option.id()}",
                                class: if current == option { "theme-option is-active" } else { "theme-option" },
                                role: "menuitem",
                                onclick: move |_| {
                                    theme.set(option);
                                    open.set(false);
                                },
                                span { class: format!("theme-swatch theme-swatch-{}", option.id()),
                                    span { class: "swatch-sidebar" }
                                    span { class: "swatch-editor" }
                                    span { class: "swatch-accent" }
                                }
                                span { class: "theme-option-copy",
                                    strong { "{option.label()}" }
                                    span { "{option.description()}" }
                                }
                                span { class: "theme-check",
                                    if current == option {
                                        Icon { kind: IconKind::Check, size: 13.0 }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "theme-menu-footer",
                        span { class: "status-dot" }
                        "两种风格，各含暗色与纸白；颜色和语法均由主题令牌驱动"
                    }
                }
            }
        }
    }
}

#[component]
fn LatticeLogo() -> Element {
    rsx! {
        svg {
            class: "lattice-logo",
            width: "29",
            height: "29",
            view_box: "0 0 30 30",
            fill: "none",
            path {
                d: "M15 2.5 26 8.8v12.4L15 27.5 4 21.2V8.8Z",
                fill: "currentColor",
                fill_opacity: "0.13",
                stroke: "currentColor",
                stroke_width: "1.5",
                stroke_linejoin: "round",
            }
            path {
                d: "m4 8.8 11 6.3 11-6.3M15 15.1v12.4",
                stroke: "currentColor",
                stroke_width: "1.35",
                stroke_linejoin: "round",
            }
            path {
                d: "m9.5 12 5.5-3.1 5.5 3.1v6.2L15 21.3l-5.5-3.1Z",
                fill: "currentColor",
                fill_opacity: "0.28",
                stroke: "currentColor",
                stroke_width: "1.2",
                stroke_linejoin: "round",
            }
        }
    }
}

#[component]
pub fn StatusBar(source: SourceSession, zoom: Signal<f64>) -> Element {
    let state = source.state.read();
    let document = state.active_document();
    let line_count = document.map_or(0, |document| document.lines.len());
    let symbol_count = document.map_or(0, |document| document.symbols.len());
    let pack_name = state
        .pack
        .as_ref()
        .map_or("Source Pack", |pack| pack.name());
    let pack_version = state.pack.as_ref().map_or("—", |pack| pack.version());
    let source_language = state
        .pack
        .as_ref()
        .map_or("C", |pack| pack.source_language());
    let file_count = state.file_count;
    let zoom_percent = (*zoom.read() * 100.0).round() as i32;

    rsx! {
        footer { class: "status-bar",
            div { class: "status-group",
                span { class: "status-item is-success",
                    Icon { kind: IconKind::Check, size: 12.0 }
                    "源码已载入"
                }
                span { class: "status-separator" }
                span { class: "status-item", "{pack_name} {pack_version}" }
                span { class: "status-item",
                    Icon { kind: IconKind::Lock, size: 11.0 }
                    "只读"
                }
            }
            div { class: "status-group status-center",
                span { class: "status-item", "{file_count} 个文件" }
                span { class: "status-separator" }
                span { class: "status-item", "{symbol_count} 个符号" }
            }
            div { class: "status-group status-right",
                span { class: "status-item", title: "Ctrl + 滚轮 / Ctrl + = / Ctrl + - / Ctrl + 0",
                    "{zoom_percent}%"
                }
                span { class: "status-item", "{line_count} 行" }
                span { class: "status-item", "UTF-8" }
                span { class: "status-item is-language", "{source_language}" }
            }
        }
    }
}
