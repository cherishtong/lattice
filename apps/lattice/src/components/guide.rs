use std::collections::{HashMap, HashSet};

use dioxus::html::Key;
use dioxus::prelude::*;

use crate::source::{CallNode, CallTree, SourceSession};

use lattice_parser::{ReferenceKind, SymbolKind, TypeRelationKind};

use super::icon::{Icon, IconButton, IconKind};

fn outline_key(path: &str, start_line: u32) -> String {
    format!("{path}:{start_line}")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GuideTab {
    Outline,
    References,
    Relations,
    FileInfo,
}

#[component]
pub fn GuidePanel(source: SourceSession) -> Element {
    let mut tab = use_signal(|| GuideTab::Outline);
    let mut query = use_signal(String::new);
    let mut reference_page = use_signal(|| 160usize);
    let mut show_conditional = use_signal(|| false);
    let mut collapsed_outline = use_signal(HashSet::<String>::new);
    let mut outline_active = use_signal(|| 0usize);
    let mut outline_followed = use_signal(|| 0u32);
    let mut relations_key = use_signal(|| None::<(String, u32)>);
    let mut show_dependency_graph = use_signal(|| true);
    let state = source.state.read();
    let active_tab = state.active_tab();
    let selected_line =
        active_tab.and_then(|tab| tab.selection.map(|selection| selection.start_line));
    let selected_selection = active_tab.and_then(|tab| tab.selection);
    let document = active_tab.and_then(|tab| tab.document.as_ref());
    let pack_name = state
        .pack
        .as_ref()
        .map_or("Source Pack", |pack| pack.name());
    let pack_version = state.pack.as_ref().map_or("—", |pack| pack.version());
    let path = state.active_path().unwrap_or("未打开文件");
    let needle = query.read().trim().to_lowercase();
    let selected_name = active_tab.and_then(|tab| {
        let selection = tab.selection?;
        tab.document
            .as_ref()?
            .symbols
            .iter()
            .find(|symbol| symbol.start_line == selection.start_line)
            .map(|symbol| symbol.name.clone())
    });
    let selected_symbol = active_tab.and_then(|tab| {
        let selection = tab.selection?;
        tab.document
            .as_ref()?
            .symbols
            .iter()
            .find(|symbol| symbol.start_line == selection.start_line)
    });
    let type_relations = match (selected_name.as_deref(), selected_selection, active_tab) {
        (Some(name), Some(selection), Some(tab)) => {
            let index = source.index.read();
            index.as_ref().map(|index| {
                index.relations_for(
                    name,
                    &tab.path,
                    selection.start_line,
                    tab.document
                        .as_ref()
                        .map_or("C", |document| document.language),
                )
            })
        }
        _ => None,
    };
    let index_ready = source.index.read().is_some();
    let selected_kind = selected_symbol.map(|symbol| symbol.kind);
    let show_type_relations = selected_kind.is_some_and(is_type_symbol);
    let type_relations = type_relations.unwrap_or_default();
    let references = source.references.read();
    let references_loading = *source.references_loading.read();
    let references_error = source.references_error.read().clone();
    let references_progress = *source.references_progress.read();
    let show_cfg = show_conditional();
    let call_tree = source.call_tree.read();
    let visible_call_nodes = call_tree
        .as_ref()
        .map_or_else(Vec::new, |tree| flatten_call_tree(tree));
    let callees_loading = *source.callees_loading.read();
    let callees_error = source.callees_error.read().clone();
    let dependencies = source.dependencies.read();
    let reverse_dependencies = source.reverse_dependencies.read();
    let reverse_loading = *source.reverse_loading.read();
    let reverse_error = source.reverse_error.read().clone();
    let reverse_progress = *source.reverse_progress.read();
    let graph_deps: Vec<(String, String)> = dependencies.as_ref().map_or_else(Vec::new, |deps| {
        deps.iter()
            .filter_map(|dep| dep.path.clone().map(|path| (dep.name.clone(), path)))
            .collect()
    });
    let graph_center = path.rsplit('/').next().unwrap_or(path).to_owned();
    let callers: Vec<_> = references.as_ref().map_or_else(Vec::new, |results| {
        results
            .hits
            .iter()
            .filter(|hit| hit.kind == ReferenceKind::Call)
            .collect()
    });
    let relations_selection = selected_selection;
    let relations_name = selected_name.clone();
    use_effect(move || {
        if tab() != GuideTab::Relations {
            return;
        }
        let (Some(name), Some(selection)) = (relations_name.clone(), relations_selection) else {
            return;
        };
        let key = (name.clone(), selection.start_line);
        if relations_key() == Some(key.clone()) {
            return;
        }
        relations_key.set(Some(key));
        source.lookup_references(name.clone());
        source.lookup_callees(name, selection.start_line, selection.end_line);
        source.lookup_reverse_dependencies();
    });
    let outline_items = document.map_or_else(Vec::new, |document| {
        build_outline_items(&document.symbols, document.language, &needle)
    });
    let active_line = *source.current_line.read();
    let collapsed_snapshot = collapsed_outline.read().clone();
    let visible_outline: Vec<&OutlineItem> = outline_items
        .iter()
        .filter(|item| {
            !item
                .ancestors
                .iter()
                .any(|start| collapsed_snapshot.contains(&outline_key(path, *start)))
        })
        .collect();
    let keyboard_items: Vec<(u32, u32, bool)> = visible_outline
        .iter()
        .map(|item| (item.start_line, item.end_line, item.has_children))
        .collect();
    let outline_active_index = if visible_outline.is_empty() {
        0
    } else {
        outline_active().min(visible_outline.len() - 1)
    };
    let current_start = document.and_then(|document| {
        document
            .symbols
            .iter()
            .filter(|symbol| symbol.start_line <= active_line && symbol.end_line >= active_line)
            .min_by_key(|symbol| symbol.end_line.saturating_sub(symbol.start_line))
            .map(|symbol| symbol.start_line)
    });
    use_effect(move || {
        let _ = tab();
        if tab() != GuideTab::Outline {
            return;
        }
        let Some(start) = current_start else {
            return;
        };
        if outline_followed() == start {
            return;
        }
        outline_followed.set(start);
        let script = format!(
            "requestAnimationFrame(() => {{ const el = document.querySelector('[data-outline-start=\"{start}\"]'); if (el) el.scrollIntoView({{ block: 'nearest' }}); }});"
        );
        dioxus::document::eval(&script);
    });
    let filtered_references: Vec<_> = references.as_ref().map_or_else(Vec::new, |results| {
        results
            .hits
            .iter()
            .filter(|hit| show_cfg || !hit.conditional)
            .collect()
    });
    let filtered_total = filtered_references.len();
    let visible_count = reference_page().min(filtered_total);
    let visible_references = &filtered_references[..visible_count];
    let conditional_total = references.as_ref().map_or(0, |results| {
        results.hits.iter().filter(|hit| hit.conditional).count()
    });

    rsx! {
        aside { class: "guide-panel",
            div { class: "guide-tabs", role: "tablist",
                button {
                    r#type: "button",
                    class: if tab() == GuideTab::Outline { "guide-tab is-active" } else { "guide-tab" },
                    role: "tab",
                    onclick: move |_| tab.set(GuideTab::Outline),
                    "符号大纲"
                }
                button {
                    r#type: "button",
                    class: if tab() == GuideTab::References { "guide-tab is-active" } else { "guide-tab" },
                    role: "tab",
                    onclick: {
                        let selected_name = selected_name.clone();
                        move |_| {
                            tab.set(GuideTab::References);
                            reference_page.set(160);
                            if let Some(name) = selected_name.clone() {
                                source.lookup_references(name);
                            }
                        }
                    },
                    "引用"
                }
                button {
                    r#type: "button",
                    class: if tab() == GuideTab::Relations { "guide-tab is-active" } else { "guide-tab" },
                    role: "tab",
                    onclick: move |_| tab.set(GuideTab::Relations),
                    "调用链"
                }
                button {
                    r#type: "button",
                    class: if tab() == GuideTab::FileInfo { "guide-tab is-active" } else { "guide-tab" },
                    role: "tab",
                    onclick: move |_| tab.set(GuideTab::FileInfo),
                    "文件信息"
                }
            }

            if tab() == GuideTab::Outline {
                div { class: "outline-toolbar",
                    div { class: "guide-section-heading",
                        div {
                            span { class: "section-kicker", "当前文件" }
                            h2 { "源码符号" }
                        }
                        span { class: "step-progress",
                            "{document.map_or(0, |document| document.symbols.len())}"
                        }
                    }

                    input {
                        r#type: "search",
                        class: "outline-search",
                        placeholder: "筛选符号…",
                        value: "{query}",
                        oninput: move |event| query.set(event.value()),
                    }
                }

                div {
                    class: "outline-scroll",
                    tabindex: "0",
                    onkeydown: {
                        let keyboard_items = keyboard_items.clone();
                        let keyboard_path = path.to_owned();
                        move |event| {
                            let count = keyboard_items.len();
                            if count == 0 {
                                return;
                            }
                            let mut index = outline_active().min(count - 1);
                            match event.key() {
                                Key::ArrowDown => {
                                    index = (index + 1).min(count - 1);
                                    outline_active.set(index);
                                    event.prevent_default();
                                }
                                Key::ArrowUp => {
                                    index = index.saturating_sub(1);
                                    outline_active.set(index);
                                    event.prevent_default();
                                }
                                Key::Home => {
                                    index = 0;
                                    outline_active.set(index);
                                    event.prevent_default();
                                }
                                Key::End => {
                                    index = count - 1;
                                    outline_active.set(index);
                                    event.prevent_default();
                                }
                                Key::Enter => {
                                    let (start_line, end_line, _) = keyboard_items[index];
                                    source.select_symbol(start_line, end_line);
                                    event.prevent_default();
                                }
                                Key::ArrowLeft | Key::ArrowRight => {
                                    let (start_line, _, has_children) = keyboard_items[index];
                                    if has_children {
                                        let key = outline_key(&keyboard_path, start_line);
                                        let collapse = event.key() == Key::ArrowLeft;
                                        let mut guard = collapsed_outline.write();
                                        if collapse {
                                            guard.insert(key);
                                        } else {
                                            guard.remove(&key);
                                        }
                                        event.prevent_default();
                                    }
                                }
                                _ => {}
                            }
                            let script = format!(
                                "requestAnimationFrame(() => {{ const el = document.querySelector('[data-outline-index=\"{index}\"]'); if (el) el.scrollIntoView({{ block: 'nearest' }}); }});"
                            );
                            dioxus::document::eval(&script);
                        }
                    },
                    if let Some(document) = document {
                        div { class: "outline-list",
                            for (index, item) in visible_outline.iter().enumerate() {
                                {
                                    let mut row_class = String::from("outline-row");
                                    if item.depth > 0 {
                                        row_class.push_str(" is-child");
                                    }
                                    if is_container_kind(item.kind) {
                                        row_class.push_str(" is-container");
                                    }
                                    if selected_line == Some(item.start_line) {
                                        row_class.push_str(" is-selected");
                                    }
                                    if current_start == Some(item.start_line) {
                                        row_class.push_str(" is-current");
                                    }
                                    if index == outline_active_index {
                                        row_class.push_str(" is-active");
                                    }
                                    let collapsed = collapsed_snapshot
                                        .contains(&outline_key(path, item.start_line));
                                    let padding = format!("padding-left: {}px;", 6 + item.depth * 14);
                                    rsx! {
                                        div {
                                            key: "{item.name}-{item.start_line}-{item.depth}",
                                            class: "outline-entry",
                                            style: "{padding}",
                                            "data-outline-start": "{item.start_line}",
                                            "data-outline-index": "{index}",
                                            if item.has_children {
                                                button {
                                                    r#type: "button",
                                                    class: if collapsed { "outline-toggle is-collapsed" } else { "outline-toggle" },
                                                    title: if collapsed { "展开子项" } else { "折叠子项" },
                                                    tabindex: "-1",
                                                    onclick: {
                                                        let key = outline_key(path, item.start_line);
                                                        move |_| {
                                                            let mut guard = collapsed_outline.write();
                                                            if !guard.remove(&key) {
                                                                guard.insert(key.clone());
                                                            }
                                                        }
                                                    },
                                                    if collapsed { "▸" } else { "▾" }
                                                }
                                            }
                                            button {
                                                r#type: "button",
                                                class: "{row_class}",
                                                onclick: {
                                                    let line = item.start_line;
                                                    let end_line = item.end_line;
                                                    move |_| {
                                                        outline_active.set(index);
                                                        source.select_symbol(line, end_line);
                                                    }
                                                },
                                                span { class: "outline-glyph", "{item.kind.glyph()}" }
                                                span { class: "outline-copy",
                                                    strong { "{item.name}" }
                                                    span { "{item.kind.label()} · {item.start_line}–{item.end_line}" }
                                                }
                                                Icon { kind: IconKind::ArrowRight, size: 11.0 }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if document.symbols.is_empty() {
                            div { class: "outline-empty", "该文件暂无可识别的符号。" }
                        } else if visible_outline.is_empty() {
                            div { class: "outline-empty", "没有匹配当前筛选条件的符号。" }
                        }
                    } else {
                        div { class: "outline-empty", "打开源码文件后显示符号大纲。" }
                    }
                }
            } else if tab() == GuideTab::Relations {
                div { class: "guide-scroll",
                    section { class: "guide-section",
                        div { class: "guide-section-heading",
                            div {
                                span { class: "section-kicker", "调用链 · 直接关系" }
                                h2 { "{selected_name.as_deref().unwrap_or(\"未选择符号\")}" }
                            }
                            if selected_name.is_some() {
                                span { class: "step-progress", "{callers.len() + call_tree.as_ref().map_or(0, |tree| tree.nodes.len().saturating_sub(1)) + dependencies.as_ref().map_or(0, Vec::len) + reverse_dependencies.as_ref().map_or(0, Vec::len)}" }
                            }
                        }

                        if selected_name.is_none() {
                            div { class: "outline-empty", "先在符号大纲中选择一个符号，再查看调用链。" }
                        } else {
                            if show_type_relations {
                                div { class: "relation-section",
                                    div { class: "relation-heading",
                                        span { "继承 / 实现" }
                                        span { class: "relation-count", "{type_relations.bases.len()}" }
                                    }
                                    if !index_ready {
                                        div { class: "reference-loading",
                                            span { class: "source-spinner" }
                                            "正在建立符号索引，稍后显示跨文件关系。"
                                        }
                                    } else if type_relations.bases.is_empty() {
                                        div { class: "outline-empty", "没有检测到直接基类、接口或 Trait。" }
                                    } else {
                                        div { class: "reference-list",
                                            for (index, target) in type_relations.bases.iter().enumerate() {
                                                {
                                                    let resolved = target.path.clone().zip(target.line);
                                                    rsx! {
                                                        if let Some((path, line)) = resolved {
                                                            button {
                                                                r#type: "button",
                                                                key: "base-{index}-{target.name}",
                                                                class: "reference-row",
                                                                onclick: move |_| source.open_at(path.clone(), line),
                                                                span { class: "reference-line", "{line}" }
                                                                span { class: "reference-copy",
                                                                    span { class: "reference-preview", "{target.name}" }
                                                                    span { class: "reference-path", "{path}" }
                                                                }
                                                                span { class: "reference-kind", "{relation_kind_label(target.kind)}" }
                                                                Icon { kind: IconKind::ArrowRight, size: 11.0 }
                                                            }
                                                        } else {
                                                            div {
                                                                key: "base-{index}-{target.name}",
                                                                class: "reference-row is-unresolved",
                                                                span { class: "reference-line", "·" }
                                                                span { class: "reference-copy",
                                                                    span { class: "reference-preview", "{target.name}" }
                                                                    span { class: "reference-path", "未解析到源码位置" }
                                                                }
                                                                span { class: "reference-kind", "{relation_kind_label(target.kind)}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div { class: "relation-section",
                                    div { class: "relation-heading",
                                        span { "派生 / 实现者" }
                                        span { class: "relation-count", "{type_relations.derived.len()}" }
                                    }
                                    if !index_ready {
                                        div { class: "reference-loading",
                                            span { class: "source-spinner" }
                                            "正在建立符号索引，稍后显示跨文件关系。"
                                        }
                                    } else if type_relations.derived.is_empty() {
                                        div { class: "outline-empty", "没有检测到派生类型或实现者。" }
                                    } else {
                                        div { class: "reference-list",
                                            for (index, target) in type_relations.derived.iter().enumerate() {
                                                {
                                                    let resolved = target.path.clone().zip(target.line);
                                                    rsx! {
                                                        if let Some((path, line)) = resolved {
                                                            button {
                                                                r#type: "button",
                                                                key: "derived-{index}-{target.name}",
                                                                class: "reference-row",
                                                                onclick: move |_| source.open_at(path.clone(), line),
                                                                span { class: "reference-line", "{line}" }
                                                                span { class: "reference-copy",
                                                                    span { class: "reference-preview", "{target.name}" }
                                                                    span { class: "reference-path", "{path}" }
                                                                }
                                                                span { class: "reference-kind", "{relation_kind_label(target.kind)}" }
                                                                Icon { kind: IconKind::ArrowRight, size: 11.0 }
                                                            }
                                                        } else {
                                                            div {
                                                                key: "derived-{index}-{target.name}",
                                                                class: "reference-row is-unresolved",
                                                                span { class: "reference-line", "·" }
                                                                span { class: "reference-copy",
                                                                    span { class: "reference-preview", "{target.name}" }
                                                                    span { class: "reference-path", "未解析到源码位置" }
                                                                }
                                                                span { class: "reference-kind", "{relation_kind_label(target.kind)}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "relation-section",
                                div { class: "relation-heading",
                                    span { "调用者" }
                                    span { class: "relation-count", "{callers.len()}" }
                                }
                                if references_loading {
                                    div { class: "reference-loading",
                                        span { class: "source-spinner" }
                                        "正在查找调用者…"
                                    }
                                } else if let Some(error) = references_error.as_ref() {
                                    div { class: "outline-empty", "{error}" }
                                } else if callers.is_empty() {
                                    div { class: "outline-empty", "没有检测到直接调用者。" }
                                } else {
                                    div { class: "reference-list",
                                        for hit in callers.iter().take(80) {
                                            button {
                                                r#type: "button",
                                                key: "caller-{hit.path}-{hit.line}",
                                                class: "reference-row",
                                                onclick: {
                                                    let path = hit.path.clone();
                                                    let line = hit.line;
                                                    move |_| source.open_at(path.clone(), line)
                                                },
                                                span { class: "reference-line", "{hit.line}" }
                                                span { class: "reference-copy",
                                                    span { class: "reference-preview", "{hit.preview}" }
                                                    span { class: "reference-path", "{hit.path}" }
                                                }
                                                span { class: "reference-kind", "调用" }
                                                Icon { kind: IconKind::ArrowRight, size: 11.0 }
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "relation-section",
                                div { class: "relation-heading",
                                    span { "被调用 · 多层" }
                                    span { class: "relation-count",
                                        "{call_tree.as_ref().map_or(0, |tree| tree.nodes.len().saturating_sub(1))}"
                                    }
                                }
                                if callees_loading {
                                    div { class: "reference-loading",
                                        span { class: "source-spinner" }
                                        "正在解析当前符号的调用…"
                                    }
                                } else if let Some(error) = callees_error.as_ref() {
                                    div { class: "outline-empty", "{error}" }
                                } else if call_tree.is_some() {
                                    if visible_call_nodes.len() <= 1 {
                                        div { class: "outline-empty", "当前符号范围内没有检测到函数调用。" }
                                    } else {
                                        div { class: "call-tree",
                                            for node in visible_call_nodes.iter() {
                                                {
                                                    let padding = format!("padding-left: {}px;", 4 + node.depth * 14);
                                                    let resolved = node.path.clone().zip(node.def_line);
                                                    let expandable = node.depth > 0
                                                        && node.path.is_some()
                                                        && node.def_line.is_some();
                                                    let node_id = node.id;
                                                    rsx! {
                                                        div {
                                                            key: "call-node-{node.id}",
                                                            class: if node.depth == 0 { "call-node is-root" } else { "call-node" },
                                                            style: "{padding}",
                                                            if expandable {
                                                                button {
                                                                    r#type: "button",
                                                                    class: if node.expanded { "call-toggle" } else { "call-toggle is-collapsed" },
                                                                    title: if node.expanded { "折叠下层调用" } else { "展开下层调用" },
                                                                    onclick: move |_| source.toggle_call_node(node_id),
                                                                    if node.expanded { "▾" } else { "▸" }
                                                                }
                                                            } else {
                                                                span { class: "call-toggle-spacer" }
                                                            }
                                                            if let Some((path, target_line)) = resolved {
                                                                button {
                                                                    r#type: "button",
                                                                    class: "call-main",
                                                                    onclick: move |_| source.open_at(path.clone(), target_line),
                                                                    span { class: "call-name", "{node.name}" }
                                                                    span { class: "call-meta",
                                                                        "{path}:{target_line} · 调用行 {node.call_line}"
                                                                    }
                                                                }
                                                            } else {
                                                                div { class: "call-main is-unresolved",
                                                                    span { class: "call-name", "{node.name}()" }
                                                                    span { class: "call-meta", "未解析到定义 · 调用行 {node.call_line}" }
                                                                }
                                                            }
                                                            if node.loading {
                                                                span { class: "source-spinner call-spinner" }
                                                            }
                                                            if let Some(error) = node.error.as_ref() {
                                                                span { class: "call-error", "{error}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "relation-section",
                                div { class: "relation-heading",
                                    span { "模块依赖" }
                                    span { class: "relation-count", "{dependencies.as_ref().map_or(0, Vec::len)}" }
                                    if !graph_deps.is_empty() {
                                        button {
                                            r#type: "button",
                                            class: if show_dependency_graph() { "reference-filter is-active" } else { "reference-filter" },
                                            onclick: move |_| show_dependency_graph.set(!show_dependency_graph()),
                                            if show_dependency_graph() { "列表" } else { "关系图" }
                                        }
                                    }
                                }
                                if callees_loading {
                                    div { class: "reference-loading",
                                        span { class: "source-spinner" }
                                        "正在解析导入关系…"
                                    }
                                } else if let Some(deps) = dependencies.as_ref() {
                                    if deps.is_empty() {
                                        div { class: "outline-empty", "当前文件没有可解析的导入或包含关系。" }
                                    } else if show_dependency_graph() && !graph_deps.is_empty() {
                                        {
                                            let count = graph_deps.len().min(10);
                                            rsx! {
                                                svg {
                                                    class: "dependency-graph",
                                                    view_box: "0 0 520 340",
                                                    role: "img",
                                                    for (index, (_, dep_path)) in graph_deps.iter().take(count).enumerate() {
                                                        {
                                                            let angle = std::f64::consts::TAU * index as f64 / count.max(1) as f64
                                                                - std::f64::consts::FRAC_PI_2;
                                                            let x = 260.0 + 150.0 * angle.cos();
                                                            let y = 165.0 + 120.0 * angle.sin();
                                                            let x_text = format!("{x:.1}");
                                                            let y_text = format!("{y:.1}");
                                                            let label_y = format!("{:.1}", y + 21.0);
                                                            let label = dep_path
                                                                .rsplit('/')
                                                                .next()
                                                                .unwrap_or(dep_path)
                                                                .chars()
                                                                .take(18)
                                                                .collect::<String>();
                                                            let click_path = dep_path.clone();
                                                            rsx! {
                                                                g {
                                                                    key: "graph-{dep_path}",
                                                                    class: "dependency-node",
                                                                    onclick: move |_| source.open_at(click_path.clone(), 1),
                                                                    line {
                                                                        class: "dependency-line",
                                                                        x1: "260",
                                                                        y1: "165",
                                                                        x2: "{x_text}",
                                                                        y2: "{y_text}",
                                                                    }
                                                                    circle {
                                                                        class: "dependency-dot",
                                                                        cx: "{x_text}",
                                                                        cy: "{y_text}",
                                                                        r: "5",
                                                                    }
                                                                    text {
                                                                        class: "dependency-label",
                                                                        x: "{x_text}",
                                                                        y: "{label_y}",
                                                                        text_anchor: "middle",
                                                                        "{label}"
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    circle {
                                                        class: "dependency-center-dot",
                                                        cx: "260",
                                                        cy: "165",
                                                        r: "10",
                                                    }
                                                    text {
                                                        class: "dependency-center-label",
                                                        x: "260",
                                                        y: "190",
                                                        text_anchor: "middle",
                                                        "{graph_center}"
                                                    }
                                                }
                                                if graph_deps.len() > count {
                                                    div { class: "reference-note",
                                                        "关系图仅显示前 {count} 个直接依赖，完整列表见切换后的“列表”。"
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        div { class: "reference-list",
                                            for dep in deps.iter() {
                                                {
                                                    let resolved = dep.path.clone();
                                                    rsx! {
                                                        if let Some(path) = resolved {
                                                            button {
                                                                r#type: "button",
                                                                key: "dep-{dep.name}-{path}",
                                                                class: "reference-row",
                                                                onclick: move |_| source.open_at(path.clone(), 1),
                                                                span { class: "reference-line", "→" }
                                                                span { class: "reference-copy",
                                                                    span { class: "reference-preview", "{dep.name}" }
                                                                    span { class: "reference-path", "{path}" }
                                                                }
                                                                Icon { kind: IconKind::ArrowRight, size: 11.0 }
                                                            }
                                                        } else {
                                                            div {
                                                                key: "dep-{dep.name}",
                                                                class: "reference-row is-unresolved",
                                                                span { class: "reference-line", "→" }
                                                                span { class: "reference-copy",
                                                                    span { class: "reference-preview", "{dep.name}" }
                                                                    span { class: "reference-path", "外部或未在源码包中" }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "relation-section",
                                div { class: "relation-heading",
                                    span { "被依赖" }
                                    span { class: "relation-count",
                                        "{reverse_dependencies.as_ref().map_or(0, Vec::len)}"
                                    }
                                }
                                if reverse_loading {
                                    div { class: "reference-loading",
                                        span { class: "source-spinner" }
                                        span { class: "reference-loading-copy",
                                            "正在扫描哪些文件导入了当前文件…"
                                            if let Some((scanned, total)) = reverse_progress
                                                && total > 0
                                            {
                                                span { class: "reference-progress", "{scanned} / {total} 个文件" }
                                            }
                                        }
                                        button {
                                            r#type: "button",
                                            class: "reference-cancel",
                                            onclick: move |_| source.cancel_reverse_dependencies(),
                                            "取消"
                                        }
                                    }
                                } else if let Some(error) = reverse_error.as_ref() {
                                    div { class: "outline-empty", "{error}" }
                                } else if let Some(entries) = reverse_dependencies.as_ref() {
                                    if entries.is_empty() {
                                        div { class: "outline-empty", "没有发现导入当前文件的其他文件。" }
                                    } else {
                                        div { class: "reference-list",
                                            for entry in entries.iter() {
                                                button {
                                                    r#type: "button",
                                                    key: "reverse-{entry.path}-{entry.line}",
                                                    class: "reference-row",
                                                    onclick: {
                                                        let path = entry.path.clone();
                                                        let line = entry.line;
                                                        move |_| source.open_at(path.clone(), line)
                                                    },
                                                    span { class: "reference-line", "{entry.line}" }
                                                    span { class: "reference-copy",
                                                        span { class: "reference-preview", "{entry.path}" }
                                                        span { class: "reference-path", "{entry.hint}" }
                                                    }
                                                    Icon { kind: IconKind::ArrowRight, size: 11.0 }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "reference-note",
                                "调用者来自语义引用扫描；被调用项来自当前符号范围内的 tree-sitter 调用节点。点击可跳转到源码位置。"
                            }
                        }
                    }
                }
            } else if tab() == GuideTab::FileInfo {
                div { class: "guide-scroll",
                    section { class: "guide-section",
                        div { class: "guide-section-heading",
                            div {
                                span { class: "section-kicker", "Source Pack" }
                                h2 { "文件信息" }
                            }
                        }
                        dl { class: "file-facts",
                            div {
                                dt { "源码包" }
                                dd { "{pack_name}" }
                            }
                            div {
                                dt { "版本" }
                                dd { "{pack_version}" }
                            }
                            div {
                                dt { "路径" }
                                dd { class: "fact-path", "{path}" }
                            }
                            div {
                                dt { "语言" }
                                dd { "{document.map_or(\"—\", |document| document.language)}" }
                            }
                            div {
                                dt { "行数" }
                                dd { "{document.map_or(0, |document| document.lines.len())}" }
                            }
                            div {
                                dt { "符号" }
                                dd { "{document.map_or(0, |document| document.symbols.len())}" }
                            }
                            div {
                                dt { "文件总数" }
                                dd { "{state.file_count}" }
                            }
                            div {
                                dt { "读取模式" }
                                dd { "本地只读 · 离线" }
                            }
                        }
                    }
                }
            } else {
                div { class: "guide-scroll",
                    section { class: "guide-section",
                        div { class: "guide-section-heading",
                            div {
                                span { class: "section-kicker", "候选引用" }
                                h2 { "{selected_name.as_deref().unwrap_or(\"未选择符号\")}" }
                            }
                            if let Some(results) = references.as_ref() {
                                span { class: "step-progress", "{results.hits.len()}" }
                            }
                        }

                        if selected_name.is_none() {
                            div { class: "outline-empty", "先在符号大纲中选择一个符号，再查看它的引用。" }
                        } else if references_loading {
                            div { class: "reference-loading",
                                span { class: "source-spinner" }
                                span { class: "reference-loading-copy",
                                    "正在扫描候选引用…"
                                    if let Some((scanned, total, hits)) = references_progress
                                        && total > 0
                                    {
                                        span { class: "reference-progress",
                                            "{scanned} / {total} 个文件 · {hits} 条"
                                        }
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "reference-cancel",
                                    onclick: move |_| source.cancel_references(),
                                    "取消"
                                }
                            }
                        } else if let Some(error) = references_error {
                            div { class: "outline-empty", "{error}" }
                        } else if let Some(results) = references.as_ref() {
                            div { class: "reference-toolbar",
                                span { class: "reference-count",
                                    "{filtered_total} / {results.hits.len()} 条"
                                }
                                button {
                                    r#type: "button",
                                    class: if show_cfg { "reference-filter is-active" } else { "reference-filter" },
                                    disabled: conditional_total == 0,
                                    onclick: move |_| show_conditional.set(!show_cfg),
                                    if show_cfg { "隐藏 cfg 分支" } else { "显示 cfg 分支" }
                                }
                            }
                            if filtered_total == 0 {
                                div { class: "outline-empty",
                                    if results.hits.is_empty() {
                                        "没有找到候选引用。"
                                    } else {
                                        "当前符号的引用都位于条件编译分支中，可点击“显示 cfg 分支”查看。"
                                    }
                                }
                            } else {
                                div { class: "reference-list",
                                    for hit in visible_references.iter() {
                                        button {
                                            r#type: "button",
                                            key: "{hit.path}-{hit.line}",
                                            class: "reference-row",
                                            onclick: {
                                                let path = hit.path.clone();
                                                let line = hit.line;
                                                move |_| source.open_at(path.clone(), line)
                                            },
                                            span { class: "reference-line", "{hit.line}" }
                                            span { class: "reference-copy",
                                                span { class: "reference-preview", "{hit.preview}" }
                                                span { class: "reference-path", "{hit.path}" }
                                            }
                                            if show_cfg && hit.conditional {
                                                span { class: "reference-cfg", "cfg" }
                                            }
                                            span { class: "reference-kind", "{hit.kind.label()}" }
                                            Icon { kind: IconKind::ArrowRight, size: 11.0 }
                                        }
                                    }
                                }
                                if visible_count < filtered_total {
                                    div { class: "reference-pager",
                                        span { "已显示 {visible_count} / {filtered_total} 条" }
                                        button {
                                            r#type: "button",
                                            onclick: move |_| {
                                                reference_page.set(reference_page() + 160);
                                            },
                                            "加载更多"
                                        }
                                        button {
                                            r#type: "button",
                                            onclick: move |_| reference_page.set(filtered_total),
                                            "显示全部"
                                        }
                                    }
                                }
                                if results.truncated {
                                    div { class: "reference-note", "结果超过 20,000 条上限，仅保留前 20,000 条。" }
                                }
                                div { class: "reference-note",
                                    "已排除注释与字符串；条件编译分支默认隐藏，按定义、调用、类型、宏、字段排序。"
                                }
                            }
                        } else {
                            div { class: "outline-empty", "点击“引用”标签开始扫描当前符号。" }
                        }
                    }
                }
            }

            div { class: "guide-footer",
                IconButton { kind: IconKind::Copy, label: "复制当前路径" }
                span { "{path}" }
            }
        }
    }
}

#[derive(Clone, Debug)]
struct OutlineItem {
    name: String,
    kind: lattice_parser::SymbolKind,
    start_line: u32,
    end_line: u32,
    depth: usize,
    ancestors: Vec<u32>,
    has_children: bool,
}

fn is_container_kind(kind: lattice_parser::SymbolKind) -> bool {
    matches!(
        kind,
        lattice_parser::SymbolKind::Class
            | lattice_parser::SymbolKind::Interface
            | lattice_parser::SymbolKind::Enum
            | lattice_parser::SymbolKind::Record
            | lattice_parser::SymbolKind::Annotation
            | lattice_parser::SymbolKind::Struct
            | lattice_parser::SymbolKind::Union
            | lattice_parser::SymbolKind::Trait
            | lattice_parser::SymbolKind::Module
            | lattice_parser::SymbolKind::Impl
    )
}

fn build_outline_items(
    symbols: &[lattice_parser::Symbol],
    language: &str,
    needle: &str,
) -> Vec<OutlineItem> {
    let mut parents = vec![None; symbols.len()];
    if matches!(
        language,
        "Java" | "JavaScript" | "TypeScript" | "Rust" | "C++"
    ) {
        for (inner_index, inner) in symbols.iter().enumerate() {
            let mut best: Option<(usize, u32)> = None;
            for (outer_index, outer) in symbols.iter().enumerate() {
                if inner_index == outer_index || !is_container_kind(outer.kind) {
                    continue;
                }
                let contains =
                    outer.start_line <= inner.start_line && outer.end_line >= inner.end_line;
                let strictly_contains =
                    outer.start_line < inner.start_line || outer.end_line > inner.end_line;
                if !contains || !strictly_contains {
                    continue;
                }
                let span = outer.end_line.saturating_sub(outer.start_line);
                if best.is_none_or(|(_, best_span)| span < best_span) {
                    best = Some((outer_index, span));
                }
            }
            parents[inner_index] = best.map(|(outer_index, _)| outer_index);
        }
    }

    let mut visible = vec![needle.is_empty(); symbols.len()];
    if !needle.is_empty() {
        for (index, symbol) in symbols.iter().enumerate() {
            if !symbol.name.to_lowercase().contains(needle) {
                continue;
            }
            let mut current = Some(index);
            while let Some(node) = current {
                if visible[node] {
                    break;
                }
                visible[node] = true;
                current = parents[node];
            }
        }
    }

    symbols
        .iter()
        .enumerate()
        .filter(|(index, _)| visible[*index])
        .map(|(index, symbol)| {
            let mut depth = 0;
            let mut ancestors = Vec::new();
            let mut current = parents[index];
            while let Some(parent) = current {
                depth += 1;
                ancestors.push(symbols[parent].start_line);
                if depth >= 8 {
                    break;
                }
                current = parents[parent];
            }
            let has_children = visible
                .iter()
                .enumerate()
                .any(|(child, shown)| *shown && parents[child] == Some(index));
            OutlineItem {
                name: symbol.name.clone(),
                kind: symbol.kind,
                start_line: symbol.start_line,
                end_line: symbol.end_line,
                depth,
                ancestors,
                has_children,
            }
        })
        .collect()
}

fn flatten_call_tree(tree: &CallTree) -> Vec<&CallNode> {
    let Some(root) = tree.nodes.iter().find(|node| node.parent.is_none()) else {
        return Vec::new();
    };
    let mut children: HashMap<u64, Vec<&CallNode>> = HashMap::new();
    for node in &tree.nodes {
        if let Some(parent) = node.parent {
            children.entry(parent).or_default().push(node);
        }
    }
    for nodes in children.values_mut() {
        nodes.sort_by_key(|node| (node.call_line, node.name.clone()));
    }
    let mut visible = vec![root];
    if root.expanded {
        push_call_children(root.id, &children, &mut visible);
    }
    visible
}

fn push_call_children<'a>(
    parent: u64,
    children: &HashMap<u64, Vec<&'a CallNode>>,
    output: &mut Vec<&'a CallNode>,
) {
    let Some(nodes) = children.get(&parent) else {
        return;
    };
    for node in nodes {
        output.push(node);
        if node.expanded {
            push_call_children(node.id, children, output);
        }
    }
}

fn is_type_symbol(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Class
            | SymbolKind::Interface
            | SymbolKind::Struct
            | SymbolKind::Union
            | SymbolKind::Enum
            | SymbolKind::Type
            | SymbolKind::Record
            | SymbolKind::Trait
            | SymbolKind::Impl
    )
}

fn relation_kind_label(kind: TypeRelationKind) -> &'static str {
    match kind {
        TypeRelationKind::Extends => "继承",
        TypeRelationKind::Implements => "实现",
        TypeRelationKind::Base => "基类",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_parser::{Symbol, SymbolKind};

    fn symbol(name: &str, kind: SymbolKind, start_line: u32, end_line: u32) -> Symbol {
        Symbol {
            name: name.to_owned(),
            kind,
            start_line,
            end_line,
            signature: String::new(),
        }
    }

    #[test]
    fn flattens_call_tree_according_to_expansion() {
        let node = |id: u64, parent: Option<u64>, depth: usize, expanded: bool| CallNode {
            id,
            parent,
            name: format!("fn{id}"),
            call_line: id as u32,
            path: Some(format!("src/{id}.c")),
            def_line: Some(1),
            start_line: Some(1),
            end_line: Some(10),
            depth,
            expanded,
            loading: false,
            children_loaded: true,
            error: None,
        };
        let tree = CallTree {
            nodes: vec![
                node(0, None, 0, true),
                node(1, Some(0), 1, false),
                node(2, Some(1), 2, false),
            ],
            next_id: 3,
        };
        let visible: Vec<u64> = flatten_call_tree(&tree)
            .iter()
            .map(|node| node.id)
            .collect();
        assert_eq!(visible, vec![0, 1]);

        let mut expanded = tree;
        expanded.nodes[1].expanded = true;
        let visible: Vec<u64> = flatten_call_tree(&expanded)
            .iter()
            .map(|node| node.id)
            .collect();
        assert_eq!(visible, vec![0, 1, 2]);
    }

    #[test]
    fn groups_java_methods_under_their_class() {
        let symbols = vec![
            symbol("ArrayList", SymbolKind::Class, 10, 100),
            symbol("ArrayList", SymbolKind::Constructor, 20, 24),
            symbol("size", SymbolKind::Method, 30, 34),
            symbol("Itr", SymbolKind::Class, 40, 60),
            symbol("next", SymbolKind::Method, 45, 50),
        ];
        let items = build_outline_items(&symbols, "Java", "");
        let depths: Vec<(&str, usize)> = items
            .iter()
            .map(|item| (item.name.as_str(), item.depth))
            .collect();
        assert_eq!(
            depths,
            vec![
                ("ArrayList", 0),
                ("ArrayList", 1),
                ("size", 1),
                ("Itr", 1),
                ("next", 2),
            ]
        );
    }

    #[test]
    fn keeps_parent_rows_when_filtering_java_methods() {
        let symbols = vec![
            symbol("ArrayList", SymbolKind::Class, 10, 100),
            symbol("size", SymbolKind::Method, 30, 34),
            symbol("grow", SymbolKind::Method, 40, 44),
        ];
        let items = build_outline_items(&symbols, "Java", "grow");
        let names: Vec<&str> = items.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(names, vec!["ArrayList", "grow"]);
        assert_eq!(items[1].depth, 1);
    }

    #[test]
    fn groups_typescript_methods_under_their_class() {
        let symbols = vec![
            symbol("Circle", SymbolKind::Class, 10, 80),
            symbol("constructor", SymbolKind::Constructor, 20, 24),
            symbol("area", SymbolKind::Method, 30, 35),
            symbol("Shape", SymbolKind::Interface, 90, 120),
            symbol("area", SymbolKind::Method, 100, 104),
        ];
        let items = build_outline_items(&symbols, "TypeScript", "");
        assert_eq!(items[1].depth, 1);
        assert_eq!(items[2].depth, 1);
        assert_eq!(items[4].depth, 1);
    }

    #[test]
    fn groups_rust_methods_under_impls_and_modules() {
        let symbols = vec![
            symbol("parser", SymbolKind::Module, 1, 120),
            symbol("Expr", SymbolKind::Struct, 10, 20),
            symbol("impl Expr", SymbolKind::Impl, 30, 80),
            symbol("new", SymbolKind::Function, 35, 40),
            symbol("parse", SymbolKind::Method, 45, 60),
            symbol("impl Display for Expr", SymbolKind::Impl, 90, 110),
            symbol("fmt", SymbolKind::Method, 95, 105),
        ];
        let items = build_outline_items(&symbols, "Rust", "");
        let depths: Vec<(&str, usize)> = items
            .iter()
            .map(|item| (item.name.as_str(), item.depth))
            .collect();
        assert_eq!(
            depths,
            vec![
                ("parser", 0),
                ("Expr", 1),
                ("impl Expr", 1),
                ("new", 2),
                ("parse", 2),
                ("impl Display for Expr", 1),
                ("fmt", 2),
            ]
        );
    }

    #[test]
    fn groups_typescript_nested_modules() {
        let symbols = vec![
            symbol("compiler", SymbolKind::Module, 1, 100),
            symbol("parser", SymbolKind::Module, 10, 60),
            symbol("parse", SymbolKind::Function, 20, 30),
            symbol("scanner", SymbolKind::Module, 65, 95),
            symbol("scan", SymbolKind::Function, 70, 80),
        ];
        let items = build_outline_items(&symbols, "TypeScript", "scan");
        let depths: Vec<(&str, usize)> = items
            .iter()
            .map(|item| (item.name.as_str(), item.depth))
            .collect();
        assert_eq!(depths, vec![("compiler", 0), ("scanner", 1), ("scan", 2),]);
    }

    #[test]
    fn keeps_c_outline_flat() {
        let symbols = vec![
            symbol("list_resize", SymbolKind::Function, 10, 20),
            symbol("PyListObject", SymbolKind::Struct, 30, 40),
            symbol("PyList_New", SymbolKind::Function, 50, 60),
        ];
        let items = build_outline_items(&symbols, "C", "");
        assert!(items.iter().all(|item| item.depth == 0));
    }
}
