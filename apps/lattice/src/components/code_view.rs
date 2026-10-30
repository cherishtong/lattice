use std::collections::{HashMap, HashSet};

use dioxus::html::Key;
use dioxus::prelude::*;

use crate::source::{
    SourceSession, SymbolSelection, is_foldable_symbol, scroll_to_row, scroll_to_top_row,
};
use crate::theme::Theme;
use crate::translation::TranslationSession;

use super::file_icon::FileTypeIcon;
use super::icon::{Icon, IconKind};

#[derive(Clone, Debug, PartialEq, Eq)]
struct SearchMatch {
    line: u32,
    start: usize,
    end: usize,
}

const CODE_ROW_HEIGHT: f64 = 22.0;
const CODE_OVERSCAN: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FoldControl {
    start_line: u32,
    end_line: u32,
    collapsed: bool,
}

fn is_foldable(kind: lattice_parser::SymbolKind) -> bool {
    is_foldable_symbol(kind)
}

/// 同一个起始行可能有多个符号，折叠范围取最大的那个。
fn fold_regions(symbols: &[lattice_parser::Symbol]) -> HashMap<u32, u32> {
    let mut regions = HashMap::new();
    for symbol in symbols {
        if !is_foldable(symbol.kind) || symbol.end_line <= symbol.start_line {
            continue;
        }
        let entry = regions.entry(symbol.start_line).or_insert(symbol.end_line);
        *entry = (*entry).max(symbol.end_line);
    }
    regions
}

fn visible_line_indices(
    line_count: usize,
    regions: &HashMap<u32, u32>,
    collapsed: &HashSet<u32>,
) -> Vec<usize> {
    let mut hidden = vec![false; line_count];
    for start in collapsed {
        let Some(end) = regions.get(start) else {
            continue;
        };
        let first = (*start as usize).min(line_count);
        let last = (*end as usize).min(line_count);
        for line in first + 1..=last {
            if line > 0 {
                hidden[line - 1] = true;
            }
        }
    }
    (0..line_count).filter(|index| !hidden[*index]).collect()
}

fn folds_containing(collapsed: &HashSet<u32>, regions: &HashMap<u32, u32>, line: u32) -> Vec<u32> {
    let mut containing: Vec<u32> = collapsed
        .iter()
        .copied()
        .filter(|start| *start < line && regions.get(start).is_some_and(|end| *end >= line))
        .collect();
    containing.sort_unstable();
    containing
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScrollResolution {
    Row(u32),
    Expanded,
    Missing,
    Waiting,
}

/// 把文档行号解析成折叠后的可见行号；目标被折叠时先展开。
fn resolve_visible_row(
    source: &SourceSession,
    mut folds: Signal<HashMap<String, HashSet<u32>>>,
    line: u32,
) -> ScrollResolution {
    let (path, regions, line_count) = {
        let state = source.state.read();
        let Some(document) = state.active_document() else {
            return ScrollResolution::Waiting;
        };
        let path = state
            .active_path()
            .unwrap_or(document.path.as_str())
            .to_owned();
        (path, fold_regions(&document.symbols), document.lines.len())
    };
    if line == 0 {
        return ScrollResolution::Missing;
    }
    if line_count == 0 {
        return ScrollResolution::Waiting;
    }
    let index = line as usize - 1;
    if index >= line_count {
        return ScrollResolution::Missing;
    }
    let collapsed = {
        let guard = folds.read();
        guard.get(&path).cloned().unwrap_or_default()
    };
    let visible = visible_line_indices(line_count, &regions, &collapsed);
    if !visible.contains(&index) {
        let containing = folds_containing(&collapsed, &regions, line);
        if containing.is_empty() {
            return ScrollResolution::Missing;
        }
        let mut guard = folds.write();
        let entry = guard.entry(path).or_default();
        for start in containing {
            entry.remove(&start);
        }
        return ScrollResolution::Expanded;
    }
    visible
        .iter()
        .position(|visible| *visible == index)
        .map_or(ScrollResolution::Missing, |row| {
            ScrollResolution::Row(row as u32 + 1)
        })
}

fn visible_line_range(
    scroll_top: f64,
    viewport_height: f64,
    total_lines: usize,
) -> std::ops::Range<usize> {
    if total_lines == 0 {
        return 0..0;
    }
    let first = ((scroll_top / CODE_ROW_HEIGHT).floor() as usize)
        .saturating_sub(CODE_OVERSCAN)
        .min(total_lines);
    let last = (((scroll_top + viewport_height) / CODE_ROW_HEIGHT).ceil() as usize + CODE_OVERSCAN)
        .min(total_lines);
    first..last.max(first)
}

#[derive(Clone, Copy, Debug)]
struct LineHit {
    start: usize,
    end: usize,
    active: bool,
}

struct RenderedToken {
    text: String,
    class: String,
    definition: Option<String>,
}

#[component]
pub fn CodeView(
    source: SourceSession,
    mut find_open: Signal<bool>,
    theme: Signal<Theme>,
    translation: TranslationSession,
) -> Element {
    let light = theme.read().is_light();
    let mut find_query = use_signal(String::new);
    let mut active_match = use_signal(|| 0usize);
    let mut scroll_top = use_signal(|| 0.0_f64);
    let mut viewport_height = use_signal(|| 700.0_f64);
    let folds = source.collapsed;

    use_effect(move || {
        let _ = source.state.read().active;
        dioxus::document::eval(
            "requestAnimationFrame(() => {
                const tab = document.querySelector('.editor-tab.is-active');
                const list = document.querySelector('.editor-tab-list');
                if (!tab || !list) return;
                const tabRect = tab.getBoundingClientRect();
                const listRect = list.getBoundingClientRect();
                if (tabRect.left < listRect.left) {
                    list.scrollLeft -= listRect.left - tabRect.left;
                } else if (tabRect.right > listRect.right) {
                    list.scrollLeft += tabRect.right - listRect.right;
                }
            });",
        );
    });

    let state = source.state.read();
    let active_tab = state.active_tab();
    let selection = active_tab.and_then(|tab| tab.selection);
    let document = active_tab.and_then(|tab| tab.document.as_ref());
    let query = find_query.read().clone();
    let matches = if find_open() && !query.is_empty() {
        document.map_or_else(Vec::new, |document| find_matches(document, &query))
    } else {
        Vec::new()
    };
    let match_count = matches.len();
    let active_index = if match_count == 0 {
        0
    } else {
        active_match() % match_count
    };
    let mut hits_by_line: HashMap<u32, Vec<LineHit>> = HashMap::new();
    for (index, found) in matches.iter().enumerate() {
        hits_by_line.entry(found.line).or_default().push(LineHit {
            start: found.start,
            end: found.end,
            active: index == active_index,
        });
    }
    let path = state.active_path().unwrap_or("选择一个源码文件");
    let file_name = path.rsplit('/').next().unwrap_or(path);
    let pack_name = state
        .pack
        .as_ref()
        .map_or("Source Pack", |pack| pack.name());
    let pack_version = state.pack.as_ref().map_or("—", |pack| pack.version());
    let line_count = document.map_or(0, |document| document.lines.len());
    let current_comment =
        document.and_then(|document| comment_block_at_line(document, *source.current_line.read()));
    let mut translate_comment = translation.clone();
    let symbol_count = document.map_or(0, |document| document.symbols.len());
    let language = document.map_or("源码", |document| document.language);
    let tab_loading = active_tab.is_some_and(|tab| tab.is_loading());
    let tab_error = active_tab.and_then(|tab| tab.error.as_deref());
    let from_cache = active_tab.is_some_and(|tab| tab.from_cache);
    let segments: Vec<&str> = path.split('/').collect();
    let last_segment = segments.len().saturating_sub(1);
    let current_path = path.to_owned();
    let regions = document.map_or_else(HashMap::new, |document| fold_regions(&document.symbols));
    let collapsed = {
        let guard = folds.read();
        guard.get(&current_path).cloned().unwrap_or_default()
    };
    let visible_lines = document.map_or_else(Vec::new, |document| {
        visible_line_indices(document.lines.len(), &regions, &collapsed)
    });
    let visible_count = visible_lines.len();
    let total_height = visible_count as f64 * CODE_ROW_HEIGHT;
    let visible = visible_line_range(scroll_top(), viewport_height(), visible_count);
    let first_visible = visible.start;
    let virtual_offset = first_visible as f64 * CODE_ROW_HEIGHT;

    let mut pending_scroll = source.pending_scroll;
    let mut pending_scroll_top = source.pending_scroll_top;
    use_effect(move || {
        let Some(line) = *pending_scroll_top.read() else {
            return;
        };
        match resolve_visible_row(&source, folds, line) {
            ScrollResolution::Row(row) => {
                scroll_top.set(((row.saturating_sub(1)) as f64 * CODE_ROW_HEIGHT).max(0.0));
                scroll_to_top_row(row);
                pending_scroll_top.set(None);
            }
            ScrollResolution::Missing => pending_scroll_top.set(None),
            ScrollResolution::Expanded | ScrollResolution::Waiting => {}
        }
    });
    use_effect(move || {
        let Some(line) = *pending_scroll.read() else {
            return;
        };
        match resolve_visible_row(&source, folds, line) {
            ScrollResolution::Row(row) => {
                scroll_to_row(row);
                pending_scroll.set(None);
            }
            ScrollResolution::Missing => pending_scroll.set(None),
            ScrollResolution::Expanded | ScrollResolution::Waiting => {}
        }
    });

    let toggle_path = current_path.clone();
    let toggle_fold = EventHandler::new(move |start_line: u32| {
        source.toggle_fold(toggle_path.clone(), start_line);
    });
    let scroll_lines = visible_lines.clone();

    rsx! {
        section { class: "reader-panel",
            header { class: "lesson-header",
                div { class: "lesson-header-copy",
                    span { class: "lesson-eyebrow", "{pack_name} {pack_version}  /  离线源码包" }
                    h1 { "{file_name}" }
                    p {
                        if line_count > 0 {
                            "真实源码 · {line_count} 行 · {symbol_count} 个符号"
                        } else if tab_loading
                            || (state.tabs.is_empty() && state.bootstrap_error.is_none())
                        {
                            "正在加载源码视图…"
                        } else {
                            "从左侧源码树选择一个文件"
                        }
                    }
                }
                div { class: "lesson-header-actions",
                    span { class: "reading-time",
                        Icon { kind: IconKind::FileCode, size: 13.0 }
                        "只读源码"
                    }
                }
            }

            div { class: "editor-tabs",
                div { class: "editor-tab-list", role: "tablist",
                    for (index, tab) in state.tabs.iter().enumerate() {
                        div {
                            key: "{tab.path}",
                            class: if state.active == Some(index) { "editor-tab is-active" } else { "editor-tab" },
                            role: "tab",
                            if tab.is_loading() {
                                span { class: "tab-spinner" }
                            } else {
                                FileTypeIcon { path: tab.path.clone(), light, size: 14.0 }
                            }
                            button {
                                r#type: "button",
                                class: "tab-label",
                                title: "{tab.path}",
                                onclick: move |_| source.activate_tab(index),
                                "{tab.title()}"
                            }
                            button {
                                r#type: "button",
                                class: "tab-close",
                                title: "关闭标签页",
                                onclick: move |_| source.close_tab(index),
                                "×"
                            }
                        }
                    }
                }
                div { class: "editor-tab-spacer" }
                button {
                    r#type: "button",
                    class: "history-button",
                    title: "后退",
                    disabled: !state.can_go_back(),
                    onclick: move |_| source.go_history(-1),
                    Icon { kind: IconKind::ArrowLeft, size: 14.0 }
                }
                button {
                    r#type: "button",
                    class: "history-button",
                    title: "前进",
                    disabled: !state.can_go_forward(),
                    onclick: move |_| source.go_history(1),
                    Icon { kind: IconKind::ArrowRight, size: 14.0 }
                }
            }

            div { class: "source-breadcrumb",
                div { class: "breadcrumb-items",
                    Icon { kind: IconKind::FileCode, size: 13.0 }
                    for (index, segment) in segments.iter().enumerate() {
                        if index > 0 {
                            Icon { kind: IconKind::ChevronRight, size: 11.0 }
                        }
                        if index == last_segment {
                            strong { "{segment}" }
                        } else {
                            span { "{segment}" }
                        }
                    }
                }
                div { class: "source-meta",
                    span { "离线" }
                    span { class: "meta-separator" }
                    Icon { kind: IconKind::Lock, size: 11.0 }
                    span { "只读源码" }
                }
            }

            div { class: "code-context-bar",
                div { class: "code-context-symbol",
                    Icon { kind: IconKind::Code, size: 13.0 }
                    span { "{language}" }
                    span { class: "context-divider", "/" }
                    span { class: "context-muted", "v{pack_version}" }
                    span { class: "context-divider", "/" }
                    span { class: "context-muted", "{line_count} 行" }
                }
                div { class: "code-context-actions",
                    if let Some(status) = source.lookup_status.read().as_deref() {
                        span { class: "lookup-status", "{status}" }
                    }
                    if line_count > 0 {
                        span { class: "load-mode",
                            if from_cache { "缓存渲染" } else { "实时解析" }
                        }
                    }
                    span { class: "guide-highlight-status",
                        Icon { kind: IconKind::Lightbulb, size: 12.0 }
                        "{symbol_count} 个符号 · 双击标识符查定义"
                    }
                    if !regions.is_empty() {
                        button {
                            r#type: "button",
                            class: "fold-action",
                            title: "折叠全部符号（Ctrl+Alt+[）",
                            onclick: {
                                let path = current_path.clone();
                                let starts: Vec<u32> = regions.keys().copied().collect();
                                move |_| source.collapse_folds(path.clone(), starts.clone())
                            },
                            "折叠全部"
                        }
                        button {
                            r#type: "button",
                            class: "fold-action",
                            title: "展开全部符号（Ctrl+Alt+]）",
                            onclick: {
                                let path = current_path.clone();
                                move |_| source.clear_folds(&path)
                            },
                            "展开全部"
                        }
                    }
                    button {
                        r#type: "button",
                        class: "find-toggle",
                        title: "翻译当前注释",
                        disabled: current_comment.is_none(),
                        onclick: {
                            let comment = current_comment.clone();
                            move |_| {
                                if let Some((text, start_line, end_line)) = comment.clone() {
                                    translate_comment.select_and_open(text, start_line, end_line);
                                }
                            }
                        },
                        Icon { kind: IconKind::Lightbulb, size: 12.0 }
                        "翻译注释"
                    }
                    button {
                        r#type: "button",
                        class: if find_open() { "find-toggle is-active" } else { "find-toggle" },
                        title: "搜索当前文件",
                        onclick: move |_| find_open.toggle(),
                        Icon { kind: IconKind::Search, size: 12.0 }
                        "查找"
                    }
                }
            }

            if find_open() {
                div { class: "find-bar",
                    Icon { kind: IconKind::Search, size: 13.0 }
                    input {
                        r#type: "search",
                        class: "find-input",
                        placeholder: "搜索当前文件…",
                        value: "{query}",
                        oninput: move |event| {
                            find_query.set(event.value());
                            active_match.set(0);
                        },
                        onkeydown: {
                            let nav_matches = matches.clone();
                            move |event| {
                                if event.key() != Key::Enter {
                                    return;
                                }
                                event.prevent_default();
                                let count = nav_matches.len();
                                if count == 0 {
                                    return;
                                }
                                let current = active_match() % count;
                                let next = if event.modifiers().shift() {
                                    if current == 0 { count - 1 } else { current - 1 }
                                } else {
                                    (current + 1) % count
                                };
                                active_match.set(next);
                                pending_scroll.set(Some(nav_matches[next].line));
                            }
                        },
                    }
                    span { class: "find-count",
                        if query.is_empty() {
                            "输入关键字"
                        } else if match_count == 0 {
                            "无结果"
                        } else {
                            "{active_index + 1} / {match_count}"
                        }
                    }
                    button {
                        r#type: "button",
                        class: "find-nav",
                        title: "上一个匹配",
                        onclick: {
                            let nav_matches = matches.clone();
                            move |_| {
                                let count = nav_matches.len();
                                if count == 0 {
                                    return;
                                }
                                let current = active_match() % count;
                                let previous = if current == 0 { count - 1 } else { current - 1 };
                                active_match.set(previous);
                                pending_scroll.set(Some(nav_matches[previous].line));
                            }
                        },
                        "↑"
                    }
                    button {
                        r#type: "button",
                        class: "find-nav",
                        title: "下一个匹配",
                        onclick: {
                            let nav_matches = matches.clone();
                            move |_| {
                                let count = nav_matches.len();
                                if count == 0 {
                                    return;
                                }
                                let current = active_match() % count;
                                let next = (current + 1) % count;
                                active_match.set(next);
                                pending_scroll.set(Some(nav_matches[next].line));
                            }
                        },
                        "↓"
                    }
                    button {
                        r#type: "button",
                        class: "find-close",
                        title: "关闭查找",
                        onclick: move |_| {
                            find_open.set(false);
                            active_match.set(0);
                        },
                        "×"
                    }
                }
            }

            div { class: "code-viewport", aria_label: "源码",
                if let Some(document) = document {
                    div {
                        class: "code-scroll",
                        onscroll: {
                            let scroll_lines = scroll_lines.clone();
                            move |event| {
                                let top = event.scroll_top();
                                scroll_top.set(top);
                                viewport_height.set(event.client_height() as f64);
                                if !scroll_lines.is_empty() {
                                    let row = ((top / CODE_ROW_HEIGHT).round() as usize)
                                        .min(scroll_lines.len() - 1);
                                    let line = scroll_lines[row] as u32 + 1;
                                    source.remember_scroll(line);
                                }
                            }
                        },
                        div {
                            class: "code-virtual-space",
                            style: "height: {total_height}px;",
                            div {
                                class: "code-virtual-window",
                                style: "top: {virtual_offset}px;",
                                for row in visible.clone() {
                                    if let Some(&line_index) = visible_lines.get(row) {
                                        {
                                            let line = &document.lines[line_index];
                                            let fold = regions.get(&line.number).map(|end_line| FoldControl {
                                                start_line: line.number,
                                                end_line: *end_line,
                                                collapsed: collapsed.contains(&line.number),
                                            });
                                            render_code_line(
                                                line,
                                                selection,
                                                hits_by_line.get(&line.number).map_or(&[][..], Vec::as_slice),
                                                source,
                                                fold,
                                                toggle_fold,
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if tab_loading
                    || (state.tabs.is_empty() && state.bootstrap_error.is_none())
                {
                    div { class: "code-state",
                        span { class: "source-spinner" }
                        strong { "正在打开源码…" }
                        span { "读取文件与预构建渲染缓存" }
                    }
                } else if let Some(error) = tab_error.or(state.bootstrap_error.as_deref()) {
                    div { class: "code-state is-error",
                        Icon { kind: IconKind::FileCode, size: 24.0 }
                        strong { "无法打开源码" }
                        span { "{error}" }
                    }
                } else {
                    div { class: "code-state",
                        Icon { kind: IconKind::FileCode, size: 24.0 }
                        strong { "选择一个源码文件" }
                        span { "从左侧源码树开始阅读真实实现。" }
                    }
                }
            }
        }
    }
}

fn render_code_line(
    line: &lattice_parser::Line,
    selection: Option<SymbolSelection>,
    hits: &[LineHit],
    source: SourceSession,
    fold: Option<FoldControl>,
    on_toggle_fold: EventHandler<u32>,
) -> Element {
    let mut class = String::from("code-line real-code-line");
    if fold.is_some_and(|fold| fold.collapsed) {
        class.push_str(" is-folded");
    }
    if let Some(selection) = selection
        && line.number >= selection.start_line
        && line.number <= selection.end_line
    {
        class.push_str(" is-selected");
        if line.number == selection.start_line {
            class.push_str(" is-symbol-start");
        }
        if line.number == selection.end_line {
            class.push_str(" is-symbol-end");
        }
    }
    let tokens = render_tokens(line, hits);
    let line_number = line.number;
    let folded_lines = fold.map_or(0, |fold| fold.end_line - fold.start_line);

    rsx! {
        div { class, "data-line": "{line.number}",
            span { class: "line-number",
                if let Some(fold) = fold {
                    button {
                        r#type: "button",
                        class: if fold.collapsed { "line-fold is-collapsed" } else { "line-fold" },
                        title: if fold.collapsed {
                            format!("展开 {} 行", fold.end_line - fold.start_line)
                        } else {
                            format!("折叠 {} 行", fold.end_line - fold.start_line)
                        },
                        onclick: move |_| on_toggle_fold.call(fold.start_line),
                        if fold.collapsed { "▸" } else { "▾" }
                    }
                }
                span { class: "line-number-text", "{line.number}" }
            }
            code { class: "line-content",
                for token in tokens {
                    if let Some(word) = token.definition {
                        span {
                            class: format!("{} token-def", token.class),
                            title: "双击查看 {word} 的定义",
                            ondoubleclick: move |_| {
                                source.open_definition_at(word.clone(), line_number)
                            },
                            "{token.text}"
                        }
                    } else {
                        span { class: token.class, "{token.text}" }
                    }
                }
                if let Some(fold) = fold
                    && fold.collapsed
                {
                    span { class: "fold-summary", "⋯ {folded_lines} 行已折叠" }
                }
            }
        }
    }
}

fn find_matches(document: &lattice_parser::Document, query: &str) -> Vec<SearchMatch> {
    let needle = query.to_ascii_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    for line in document.lines.iter() {
        let text: String = line.spans.iter().map(|span| span.text.as_str()).collect();
        let haystack = text.to_ascii_lowercase();
        let mut cursor = 0;
        while cursor < haystack.len() {
            let Some(relative) = haystack[cursor..].find(&needle) else {
                break;
            };
            let start = cursor + relative;
            let end = start + needle.len();
            matches.push(SearchMatch {
                line: line.number,
                start,
                end,
            });
            cursor = end.max(start + 1);
        }
    }
    matches
}

fn render_tokens(line: &lattice_parser::Line, hits: &[LineHit]) -> Vec<RenderedToken> {
    let mut tokens = Vec::new();
    let mut offset = 0;

    for span in line.spans.iter() {
        let span_start = offset;
        let span_end = offset + span.text.len();
        let mut cuts = vec![span_start, span_end];
        for hit in hits {
            if hit.start < span_end && hit.end > span_start {
                cuts.push(hit.start.max(span_start));
                cuts.push(hit.end.min(span_end));
            }
        }
        cuts.sort_unstable();
        cuts.dedup();

        for pair in cuts.windows(2) {
            let start = pair[0];
            let end = pair[1];
            if start >= end {
                continue;
            }
            let is_hit = hits.iter().any(|hit| hit.start < end && hit.end > start);
            let is_active = hits
                .iter()
                .any(|hit| hit.active && hit.start < end && hit.end > start);
            let mut class = format!("token-{}", span.class);
            if is_hit {
                class.push_str(" search-hit");
            }
            if is_active {
                class.push_str(" is-active");
            }
            let text = span.text[start - span_start..end - span_start].to_owned();
            let definition = if !is_hit
                && matches!(
                    span.class,
                    "function" | "macro" | "type" | "constant" | "property" | "plain"
                ) {
                identifier_candidate(&text)
            } else {
                None
            };
            tokens.push(RenderedToken {
                text,
                class,
                definition,
            });
        }

        offset = span_end;
    }

    tokens
}

fn identifier_candidate(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.len() < 2 {
        return None;
    }
    let mut characters = trimmed.chars();
    let first = characters.next()?;
    if first != '_' && !first.is_ascii_alphabetic() {
        return None;
    }
    if !characters.all(|character| character == '_' || character.is_ascii_alphanumeric()) {
        return None;
    }
    Some(trimmed.to_owned())
}

fn comment_block_at_line(
    document: &lattice_parser::Document,
    line: u32,
) -> Option<(String, u32, u32)> {
    let index = line.saturating_sub(1) as usize;
    if index >= document.lines.len() || !is_comment_line(&document.lines[index]) {
        return None;
    }
    let mut first = index;
    while first > 0 && is_comment_line(&document.lines[first - 1]) {
        first -= 1;
    }
    let mut last = index;
    while last + 1 < document.lines.len() && is_comment_line(&document.lines[last + 1]) {
        last += 1;
    }
    let text = document.lines[first..=last]
        .iter()
        .map(comment_line_text)
        .collect::<Vec<_>>()
        .join("\n");
    let text = text.trim().to_owned();
    if text.is_empty() {
        None
    } else {
        Some((
            text,
            document.lines[first].number,
            document.lines[last].number,
        ))
    }
}

fn is_comment_line(line: &lattice_parser::Line) -> bool {
    let mut has_comment = false;
    for span in &line.spans {
        if span.class == "comment" {
            has_comment = true;
        } else if !span.text.trim().is_empty() {
            return false;
        }
    }
    has_comment
}

fn comment_line_text(line: &lattice_parser::Line) -> String {
    let text: String = line
        .spans
        .iter()
        .filter(|span| span.class == "comment")
        .map(|span| span.text.as_str())
        .collect();
    text.trim()
        .trim_start_matches(['/', '*', '!', '<', ' '])
        .trim_end_matches(['/', '*', '>', ' '])
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_parser::{Document, Line, Span};

    fn document() -> Document {
        Document {
            path: "listobject.c".to_owned(),
            language: "C",
            lines: vec![Line {
                number: 1,
                spans: vec![
                    Span {
                        text: "static ".to_owned(),
                        class: "keyword",
                    },
                    Span {
                        text: "List_Append".to_owned(),
                        class: "function",
                    },
                    Span {
                        text: "(list);".to_owned(),
                        class: "plain",
                    },
                ],
            }],
            symbols: Vec::new(),
            relations: Vec::new(),
        }
    }

    #[test]
    fn finds_case_insensitive_ascii_matches() {
        let matches = find_matches(&document(), "list");
        assert_eq!(matches.len(), 2);
        assert_eq!((matches[0].start, matches[0].end), (7, 11));
        assert_eq!((matches[1].start, matches[1].end), (19, 23));
    }

    #[test]
    fn splits_tokens_around_search_hits() {
        let line = &document().lines[0];
        let tokens = render_tokens(
            line,
            &[LineHit {
                start: 7,
                end: 11,
                active: true,
            }],
        );
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[1].text, "List");
        assert!(tokens[1].class.contains("search-hit"));
        assert!(tokens[1].class.contains("is-active"));
    }

    #[test]
    fn extracts_adjacent_comment_block() {
        let mut document = document();
        document.lines = vec![
            Line {
                number: 1,
                spans: vec![Span {
                    text: "/// Return the list length.".to_owned(),
                    class: "comment",
                }],
            },
            Line {
                number: 2,
                spans: vec![Span {
                    text: "/// The list must not be empty.".to_owned(),
                    class: "comment",
                }],
            },
            Line {
                number: 3,
                spans: vec![Span {
                    text: "int size;".to_owned(),
                    class: "plain",
                }],
            },
        ];
        let (text, start, end) = comment_block_at_line(&document, 2).expect("comment block");
        assert_eq!(start, 1);
        assert_eq!(end, 2);
        assert!(text.contains("Return the list length."));
        assert!(text.contains("must not be empty"));
        assert!(comment_block_at_line(&document, 3).is_none());
    }

    #[test]
    fn virtual_range_covers_viewport_and_overscan() {
        let range = visible_line_range(0.0, 440.0, 2_000);
        assert_eq!(range.start, 0);
        assert_eq!(range.end, 32);

        let range = visible_line_range(4_400.0, 440.0, 2_000);
        assert_eq!(range.start, 188);
        assert_eq!(range.end, 232);

        let range = visible_line_range(43_000.0, 440.0, 2_000);
        assert_eq!(range, 1_942..1_987);
    }

    #[test]
    fn folds_hide_symbol_bodies_and_keep_the_header() {
        let symbols = vec![
            lattice_parser::Symbol {
                name: "parse".to_owned(),
                kind: lattice_parser::SymbolKind::Function,
                start_line: 10,
                end_line: 20,
                signature: String::new(),
            },
            lattice_parser::Symbol {
                name: "VALUE".to_owned(),
                kind: lattice_parser::SymbolKind::Constant,
                start_line: 25,
                end_line: 30,
                signature: String::new(),
            },
        ];
        let regions = fold_regions(&symbols);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions.get(&10), Some(&20));

        let collapsed = HashSet::from([10]);
        let visible = visible_line_indices(30, &regions, &collapsed);
        assert_eq!(visible.len(), 30 - 10);
        assert!(!visible.contains(&10));
        assert!(visible.contains(&9));
        assert!(visible.contains(&20));
    }

    #[test]
    fn finds_the_fold_that_hides_a_line() {
        let regions = HashMap::from([(10, 40), (20, 30)]);
        let collapsed = HashSet::from([10, 20]);
        assert_eq!(folds_containing(&collapsed, &regions, 25), vec![10, 20]);
        assert_eq!(folds_containing(&collapsed, &regions, 35), vec![10]);
        assert!(folds_containing(&collapsed, &regions, 45).is_empty());
    }
}
