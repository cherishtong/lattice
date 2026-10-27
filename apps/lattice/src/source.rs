use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use dioxus::prelude::*;
use lattice_content::{SourceEntry, SourcePack, SourceTree};
use lattice_parser::{Document, LoadedDocument, ReferenceKind, Symbol, SymbolKind};
use lattice_query::SourceIndex;
use similar::{ChangeTag, TextDiff};

use crate::config::SessionConfig;

#[derive(Clone, Debug, PartialEq)]
pub struct DocumentTab {
    pub path: String,
    pub document: Option<Document>,
    pub selection: Option<SymbolSelection>,
    pub error: Option<String>,
    pub from_cache: bool,
}

impl DocumentTab {
    pub fn title(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    pub fn is_loading(&self) -> bool {
        self.document.is_none() && self.error.is_none()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymbolSelection {
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JumpLocation {
    pub path: String,
    pub line: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionLocation {
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferenceHit {
    pub path: String,
    pub line: u32,
    pub kind: ReferenceKind,
    pub conditional: bool,
    pub preview: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferenceResults {
    pub name: String,
    pub hits: Vec<ReferenceHit>,
    pub truncated: bool,
    pub cancelled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallTarget {
    pub name: String,
    pub line: u32,
    pub path: Option<String>,
    pub target_line: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    pub path: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffKind {
    Same,
    Added,
    Removed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    pub kind: DiffKind,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffSymbolStatus {
    Added,
    Removed,
    Changed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub status: DiffSymbolStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffView {
    pub path: String,
    pub base_label: String,
    pub other_label: String,
    pub lines: Vec<DiffLine>,
    pub symbols: Vec<DiffSymbol>,
    pub added: usize,
    pub removed: usize,
    pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReverseDependency {
    pub path: String,
    pub line: u32,
    pub hint: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallNode {
    pub id: u64,
    pub parent: Option<u64>,
    pub name: String,
    pub call_line: u32,
    pub path: Option<String>,
    pub def_line: Option<u32>,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
    pub depth: usize,
    pub expanded: bool,
    pub loading: bool,
    pub children_loaded: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct CallTree {
    pub nodes: Vec<CallNode>,
    pub next_id: u64,
}

/// 跨线程的引用扫描进度与取消控制。
#[derive(Clone, Debug, Default)]
pub struct ReferenceScan {
    inner: Arc<ReferenceScanInner>,
}

#[derive(Debug, Default)]
struct ReferenceScanInner {
    scanned: AtomicU64,
    total: AtomicU64,
    hits: AtomicU64,
    cancelled: AtomicBool,
}

impl ReferenceScan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Relaxed)
    }

    pub fn scanned(&self) -> u64 {
        self.inner.scanned.load(Ordering::Relaxed)
    }

    pub fn total(&self) -> u64 {
        self.inner.total.load(Ordering::Relaxed)
    }

    pub fn hits(&self) -> u64 {
        self.inner.hits.load(Ordering::Relaxed)
    }

    fn set_total(&self, total: u64) {
        self.inner.total.store(total, Ordering::Relaxed);
    }

    fn record_file(&self, scanned: u64) {
        self.inner.scanned.store(scanned, Ordering::Relaxed);
    }

    fn record_hits(&self, hits: u64) {
        self.inner.hits.store(hits, Ordering::Relaxed);
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceState {
    pub pack: Option<SourcePack>,
    pub tree: Vec<SourceEntry>,
    pub file_count: usize,
    pub tabs: Vec<DocumentTab>,
    pub active: Option<usize>,
    pub history: Vec<JumpLocation>,
    pub history_index: usize,
    pub bootstrap_error: Option<String>,
}

impl SourceState {
    pub fn active_tab(&self) -> Option<&DocumentTab> {
        self.active.and_then(|index| self.tabs.get(index))
    }

    pub fn active_path(&self) -> Option<&str> {
        self.active_tab().map(|tab| tab.path.as_str())
    }

    pub fn active_document(&self) -> Option<&Document> {
        self.active_tab().and_then(|tab| tab.document.as_ref())
    }

    pub fn can_go_back(&self) -> bool {
        self.history_index > 0 && !self.history.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        self.history_index + 1 < self.history.len()
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct SourceSession {
    pub state: Signal<SourceState>,
    pub pending_scroll: Signal<Option<u32>>,
    pub pending_scroll_top: Signal<Option<u32>>,
    pub lookup_status: Signal<Option<String>>,
    pub current_line: Signal<u32>,
    pub index: Signal<Option<Arc<SourceIndex>>>,
    pub index_loading: Signal<bool>,
    pub index_error: Signal<Option<String>>,
    pub references: Signal<Option<ReferenceResults>>,
    pub references_loading: Signal<bool>,
    pub references_error: Signal<Option<String>>,
    pub references_progress: Signal<Option<(u64, u64, u64)>>,
    pub callees: Signal<Option<Vec<CallTarget>>>,
    pub callees_loading: Signal<bool>,
    pub callees_error: Signal<Option<String>>,
    pub dependencies: Signal<Option<Vec<Dependency>>>,
    pub call_tree: Signal<Option<CallTree>>,
    pub reverse_dependencies: Signal<Option<Vec<ReverseDependency>>>,
    pub reverse_loading: Signal<bool>,
    pub reverse_error: Signal<Option<String>>,
    pub reverse_progress: Signal<Option<(u64, u64)>>,
    reverse_scan: Signal<Option<ReferenceScan>>,
    reverse_cache: Signal<HashMap<String, Vec<ReverseDependency>>>,
    pub diff: Signal<Option<DiffView>>,
    pub diff_loading: Signal<bool>,
    pub diff_error: Signal<Option<String>>,
    reference_scan: Signal<Option<ReferenceScan>>,
    pub installed_packs: Signal<Vec<SourcePack>>,
    pub collapsed: Signal<HashMap<String, HashSet<u32>>>,
    pub recent_searches: Signal<Vec<String>>,
    expanded: Signal<HashSet<String>>,
    scroll_positions: Signal<HashMap<String, u32>>,
    definition_cache: Signal<HashMap<String, Option<DefinitionLocation>>>,
    reference_cache: Signal<HashMap<String, ReferenceResults>>,
    index_started: Signal<bool>,
}

impl SourceSession {
    pub fn use_session() -> Self {
        let mut state = use_signal(SourceState::default);
        let mut expanded = use_signal(HashSet::new);
        let mut scroll_positions = use_signal(HashMap::<String, u32>::new);
        let pending_scroll = use_signal(|| None::<u32>);
        let pending_scroll_top = use_signal(|| None::<u32>);
        let lookup_status = use_signal(|| None::<String>);
        let current_line = use_signal(|| 1u32);
        let definition_cache = use_signal(HashMap::<String, Option<DefinitionLocation>>::new);
        let index = use_signal(|| None::<Arc<SourceIndex>>);
        let index_loading = use_signal(|| false);
        let index_error = use_signal(|| None::<String>);
        let references = use_signal(|| None::<ReferenceResults>);
        let references_loading = use_signal(|| false);
        let references_error = use_signal(|| None::<String>);
        let references_progress = use_signal(|| None::<(u64, u64, u64)>);
        let callees = use_signal(|| None::<Vec<CallTarget>>);
        let callees_loading = use_signal(|| false);
        let callees_error = use_signal(|| None::<String>);
        let dependencies = use_signal(|| None::<Vec<Dependency>>);
        let call_tree = use_signal(|| None::<CallTree>);
        let reverse_dependencies = use_signal(|| None::<Vec<ReverseDependency>>);
        let reverse_loading = use_signal(|| false);
        let reverse_error = use_signal(|| None::<String>);
        let reverse_progress = use_signal(|| None::<(u64, u64)>);
        let reverse_scan = use_signal(|| None::<ReferenceScan>);
        let reverse_cache = use_signal(HashMap::<String, Vec<ReverseDependency>>::new);
        let diff = use_signal(|| None::<DiffView>);
        let diff_loading = use_signal(|| false);
        let diff_error = use_signal(|| None::<String>);
        let reference_scan = use_signal(|| None::<ReferenceScan>);
        let reference_cache = use_signal(HashMap::<String, ReferenceResults>::new);
        let installed_packs = use_signal(Vec::<SourcePack>::new);
        let collapsed = use_signal(|| {
            SessionConfig::startup()
                .folds
                .iter()
                .map(|(path, starts)| {
                    (
                        path.clone(),
                        starts.iter().copied().collect::<HashSet<u32>>(),
                    )
                })
                .collect::<HashMap<String, HashSet<u32>>>()
        });
        let recent_searches = use_signal(|| SessionConfig::startup().recent_searches.clone());
        let index_started = use_signal(|| false);
        let mut installed = installed_packs;
        let mut pending_top = pending_scroll_top;

        use_future(move || async move {
            let result = tokio::task::spawn_blocking(load_bootstrap).await;

            match result {
                Ok(Ok(restored)) => {
                    installed.set(restored.available);
                    let active_path = restored.tabs[restored.active].path.clone();
                    let saved_line = restored.saved_line;
                    let mut next = SourceState {
                        pack: Some(restored.pack),
                        file_count: restored.tree.file_count,
                        tree: restored.tree.entries,
                        tabs: restored.tabs,
                        active: Some(restored.active),
                        history: vec![JumpLocation {
                            path: active_path,
                            line: saved_line,
                        }],
                        history_index: 0,
                        bootstrap_error: None,
                    };
                    expanded.set(restored.expanded);
                    scroll_positions.set(restored.positions);
                    std::mem::swap(&mut *state.write(), &mut next);
                    pending_top.set(Some(saved_line));
                }
                Ok(Err(error)) => {
                    let mut current = state.write();
                    current.tabs.clear();
                    current.active = None;
                    current.bootstrap_error = Some(error);
                }
                Err(error) => {
                    let mut current = state.write();
                    current.tabs.clear();
                    current.active = None;
                    current.bootstrap_error = Some(format!("后台读取源码包失败: {error}"));
                }
            }
        });

        use_effect(move || {
            let mut index_started = index_started;
            if index_started() {
                return;
            }
            let Some((pack, tree)) = ({
                let current = state.read();
                current.pack.as_ref().map(|pack| {
                    (
                        pack.clone(),
                        SourceTree {
                            entries: current.tree.clone(),
                            file_count: current.file_count,
                        },
                    )
                })
            }) else {
                return;
            };

            index_started.set(true);
            let mut index = index;
            let mut loading = index_loading;
            let mut error = index_error;
            loading.set(true);
            error.set(None);

            dioxus::dioxus_core::spawn_forever(async move {
                let result =
                    tokio::task::spawn_blocking(move || SourceIndex::load_or_build(&pack, &tree))
                        .await;
                match result {
                    Ok(Ok(loaded)) => index.set(Some(Arc::new(loaded.index))),
                    Ok(Err(message)) => error.set(Some(message)),
                    Err(join_error) => error.set(Some(format!("建立符号索引失败: {join_error}"))),
                }
                loading.set(false);
            });
        });

        Self {
            state,
            pending_scroll,
            pending_scroll_top,
            lookup_status,
            current_line,
            index,
            index_loading,
            index_error,
            references,
            references_loading,
            references_error,
            references_progress,
            callees,
            callees_loading,
            callees_error,
            dependencies,
            call_tree,
            reverse_dependencies,
            reverse_loading,
            reverse_error,
            reverse_progress,
            reverse_scan,
            reverse_cache,
            diff,
            diff_loading,
            diff_error,
            reference_scan,
            installed_packs,
            collapsed,
            recent_searches,
            expanded,
            scroll_positions,
            definition_cache,
            reference_cache,
            index_started,
        }
    }

    pub fn is_expanded(self, path: &str) -> bool {
        self.expanded.read().contains(path)
    }

    pub fn toggle_directory(self, path: String) {
        let mut expanded = self.expanded;
        let mut directories = expanded.write();
        if !directories.remove(&path) {
            directories.insert(path);
        }
    }

    fn request_top(mut self, line: u32) {
        let line = line.max(1);
        self.pending_scroll.set(None);
        self.pending_scroll_top.set(Some(line));
        self.current_line.set(line);
    }

    fn request_center(mut self, line: u32) {
        let line = line.max(1);
        self.pending_scroll_top.set(None);
        self.pending_scroll.set(Some(line));
        self.current_line.set(line);
    }

    fn clear_pending_scroll(mut self) {
        self.pending_scroll.set(None);
        self.pending_scroll_top.set(None);
    }

    pub fn activate_tab(mut self, index: usize) {
        if index < self.state.read().tabs.len() {
            self.state.write().active = Some(index);
            let path = self.state.read().tabs[index].path.clone();
            let line = self.saved_line(&path);
            self.request_top(line);
        }
    }

    pub fn cycle_tab(self, delta: isize) {
        let (len, active) = {
            let state = self.state.read();
            (state.tabs.len(), state.active.unwrap_or(0))
        };
        if len < 2 {
            return;
        }
        let next = (active as isize + delta).rem_euclid(len as isize) as usize;
        self.activate_tab(next);
    }

    pub fn close_active_tab(self) {
        let Some(index) = self.state.read().active else {
            return;
        };
        self.close_tab(index);
    }

    pub fn remember_search(self, query: String) {
        let query = query.trim().to_owned();
        if query.is_empty() {
            return;
        }
        let mut recent = self.recent_searches;
        let mut entries = recent();
        entries.retain(|entry| !entry.eq_ignore_ascii_case(&query));
        entries.insert(0, query);
        entries.truncate(8);
        recent.set(entries);
    }

    pub fn toggle_fold(self, path: String, start_line: u32) {
        let mut collapsed = self.collapsed;
        let mut guard = collapsed.write();
        let entry = guard.entry(path).or_default();
        if !entry.remove(&start_line) {
            entry.insert(start_line);
        }
    }

    pub fn collapse_folds(self, path: String, starts: Vec<u32>) {
        let mut collapsed = self.collapsed;
        let mut guard = collapsed.write();
        if starts.is_empty() {
            guard.remove(&path);
        } else {
            guard.insert(path, starts.into_iter().collect());
        }
    }

    pub fn clear_folds(self, path: &str) {
        let mut collapsed = self.collapsed;
        collapsed.write().remove(path);
    }

    pub fn collapse_current_file_folds(self) {
        let (path, starts) = {
            let state = self.state.read();
            let Some(path) = state.active_path().map(str::to_owned) else {
                return;
            };
            let starts = state.active_document().map_or_else(Vec::new, |document| {
                document
                    .symbols
                    .iter()
                    .filter(|symbol| {
                        is_foldable_symbol(symbol.kind) && symbol.end_line > symbol.start_line
                    })
                    .map(|symbol| symbol.start_line)
                    .collect()
            });
            (path, starts)
        };
        self.collapse_folds(path, starts);
    }

    pub fn expand_current_file_folds(self) {
        let path = self.state.read().active_path().map(str::to_owned);
        if let Some(path) = path {
            self.clear_folds(&path);
        }
    }

    pub fn toggle_fold_at_current(self) {
        let line = *self.current_line.read();
        let found = {
            let state = self.state.read();
            let Some(path) = state.active_path().map(str::to_owned) else {
                return;
            };
            let start = state.active_document().and_then(|document| {
                document
                    .symbols
                    .iter()
                    .filter(|symbol| {
                        is_foldable_symbol(symbol.kind)
                            && symbol.end_line > symbol.start_line
                            && symbol.start_line <= line
                            && symbol.end_line >= line
                    })
                    .min_by_key(|symbol| symbol.end_line.saturating_sub(symbol.start_line))
                    .or_else(|| {
                        document
                            .symbols
                            .iter()
                            .filter(|symbol| {
                                is_foldable_symbol(symbol.kind)
                                    && symbol.end_line > symbol.start_line
                                    && symbol.start_line <= line
                            })
                            .max_by_key(|symbol| symbol.start_line)
                    })
                    .map(|symbol| symbol.start_line)
            });
            start.map(|start| (path, start))
        };
        if let Some((path, start)) = found {
            self.toggle_fold(path, start);
        }
    }

    pub fn remember_scroll(mut self, line: u32) {
        let Some(path) = self.state.read().active_path().map(str::to_owned) else {
            return;
        };
        let line = line.max(1);
        let mut current_line = self.current_line;
        if current_line() != line {
            current_line.set(line);
        }
        let mut positions = self.scroll_positions.write();
        if positions.get(&path).copied() != Some(line) {
            positions.insert(path, line);
        }
    }

    fn saved_line(self, path: &str) -> u32 {
        self.scroll_positions
            .read()
            .get(path)
            .copied()
            .unwrap_or(1)
            .max(1)
    }

    pub fn refresh_installed_packs(self) {
        let mut installed = self.installed_packs;
        dioxus::dioxus_core::spawn_forever(async move {
            let packs = tokio::task::spawn_blocking(SourcePack::installed)
                .await
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default();
            installed.set(packs);
        });
    }

    /// 生成可持久化的会话快照；尚未加载源码包时返回 `None`。
    pub fn session_config(self, theme_id: &str, zoom: f64) -> Option<SessionConfig> {
        let state = self.state.read();
        let pack = state.pack.as_ref()?;
        let mut expanded: Vec<String> = self.expanded.read().iter().cloned().collect();
        expanded.sort();
        let positions: std::collections::BTreeMap<String, u32> = self
            .scroll_positions
            .read()
            .iter()
            .map(|(path, line)| (path.clone(), *line))
            .collect();
        let folds: std::collections::BTreeMap<String, Vec<u32>> = self
            .collapsed
            .read()
            .iter()
            .filter(|(_, starts)| !starts.is_empty())
            .map(|(path, starts)| {
                let mut starts: Vec<u32> = starts.iter().copied().collect();
                starts.sort_unstable();
                (path.clone(), starts)
            })
            .collect();
        Some(SessionConfig {
            theme: Some(theme_id.to_owned()),
            pack_root: Some(pack.root().to_string_lossy().into_owned()),
            tabs: state.tabs.iter().map(|tab| tab.path.clone()).collect(),
            active_tab: state.active.unwrap_or(0),
            positions,
            expanded,
            folds,
            recent_searches: self.recent_searches.read().clone(),
            zoom,
            window: None,
        })
    }

    pub fn switch_pack(mut self, pack: SourcePack) {
        self.clear_pending_scroll();
        self.lookup_status.set(None);
        self.references.set(None);
        self.references_loading.set(false);
        self.references_error.set(None);
        self.references_progress.set(None);
        self.callees.set(None);
        self.callees_loading.set(false);
        self.callees_error.set(None);
        self.dependencies.set(None);
        self.call_tree.set(None);
        self.reverse_dependencies.set(None);
        self.reverse_loading.set(false);
        self.reverse_error.set(None);
        self.reverse_progress.set(None);
        self.reverse_scan.set(None);
        self.reverse_cache.write().clear();
        self.diff.set(None);
        self.diff_loading.set(false);
        self.diff_error.set(None);
        self.reference_scan.set(None);
        self.reference_cache.write().clear();
        self.definition_cache.write().clear();
        self.expanded.write().clear();
        self.scroll_positions.write().clear();
        self.collapsed.write().clear();
        self.current_line.set(1);

        let mut index = self.index;
        let mut index_loading = self.index_loading;
        let mut index_error = self.index_error;
        let mut index_started = self.index_started;
        index.set(None);
        index_loading.set(false);
        index_error.set(None);
        index_started.set(false);

        let mut state = self.state;
        let mut expanded = self.expanded;
        let mut pending_top = self.pending_scroll_top;
        state.write().tabs.clear();
        state.write().active = None;
        state.write().tree.clear();
        state.write().file_count = 0;
        state.write().history.clear();
        state.write().history_index = 0;
        state.write().bootstrap_error = None;
        state.write().pack = Some(pack.clone());

        dioxus::dioxus_core::spawn_forever(async move {
            let result = tokio::task::spawn_blocking(move || load_pack(pack)).await;
            let mut current = state.write();
            match result {
                Ok(Ok((pack, tree, loaded))) => {
                    let path = loaded.document.path.clone();
                    let open_dirs = default_open_dirs(&tree, &pack);
                    expanded.set(open_dirs);
                    *current = SourceState {
                        pack: Some(pack),
                        file_count: tree.file_count,
                        tree: tree.entries,
                        tabs: vec![DocumentTab {
                            path: path.clone(),
                            document: Some(loaded.document),
                            selection: None,
                            error: None,
                            from_cache: loaded.from_cache,
                        }],
                        active: Some(0),
                        history: vec![JumpLocation { path, line: 1 }],
                        history_index: 0,
                        bootstrap_error: None,
                    };
                    drop(current);
                    pending_top.set(Some(1));
                }
                Ok(Err(error)) => {
                    current.tabs.clear();
                    current.active = None;
                    current.bootstrap_error = Some(error);
                }
                Err(error) => {
                    current.tabs.clear();
                    current.active = None;
                    current.bootstrap_error = Some(format!("切换源码包失败: {error}"));
                }
            }
        });
    }

    pub fn close_tab(self, index: usize) {
        let mut state = self.state;
        let mut current = state.write();
        if index >= current.tabs.len() {
            return;
        }
        current.tabs.remove(index);
        current.active = if current.tabs.is_empty() {
            None
        } else {
            Some(match current.active {
                Some(active) if active > index => active - 1,
                Some(active) if active == index => active.min(current.tabs.len() - 1),
                Some(active) => active,
                None => 0,
            })
        };
        let next_path = current
            .active
            .and_then(|active| current.tabs.get(active))
            .map(|tab| tab.path.clone());
        drop(current);
        match next_path {
            Some(path) => {
                let line = self.saved_line(&path);
                self.request_top(line);
            }
            None => self.clear_pending_scroll(),
        }
    }

    pub fn open(self, path: String) {
        self.open_internal(path, true);
    }

    pub fn open_at(self, path: String, line: u32) {
        self.open_internal(path.clone(), false);
        self.record_history(path, line);
        self.request_center(line);
    }

    fn open_internal(mut self, path: String, record_history: bool) {
        let existing = self
            .state
            .read()
            .tabs
            .iter()
            .position(|tab| tab.path == path);
        let saved_line = self.saved_line(&path);

        if let Some(index) = existing {
            {
                let mut current = self.state.write();
                current.active = Some(index);
            }
            if record_history {
                self.record_history(path, saved_line);
                self.request_top(saved_line);
            }
            return;
        }

        let Some(pack) = self.state.read().pack.clone() else {
            return;
        };

        {
            let mut current = self.state.write();
            current.tabs.push(DocumentTab {
                path: path.clone(),
                document: None,
                selection: None,
                error: None,
                from_cache: false,
            });
            let index = current.tabs.len() - 1;
            current.active = Some(index);
        }
        if record_history {
            self.record_history(path.clone(), saved_line);
            self.request_top(saved_line);
        }

        let mut state = self.state;
        dioxus::dioxus_core::spawn_forever(async move {
            let requested = path.clone();
            let result = tokio::task::spawn_blocking(move || {
                let source = pack.read_text(&requested)?;
                let cache_path = pack.document_cache_path(&requested);
                lattice_parser::load_or_parse_cached(&requested, &source, &cache_path)
            })
            .await;

            let mut current = state.write();
            let Some(tab) = current.tabs.iter_mut().find(|tab| tab.path == path) else {
                return;
            };
            match result {
                Ok(Ok(loaded)) => {
                    tab.document = Some(loaded.document);
                    tab.from_cache = loaded.from_cache;
                    tab.error = None;
                }
                Ok(Err(error)) => {
                    tab.document = None;
                    tab.error = Some(error);
                }
                Err(error) => {
                    tab.document = None;
                    tab.error = Some(format!("后台解析失败: {error}"));
                }
            }
        });
    }

    pub fn select_symbol(mut self, start_line: u32, end_line: u32) {
        let Some(index) = self.state.read().active else {
            return;
        };
        let Some(path) = self.state.read().active_path().map(str::to_owned) else {
            return;
        };

        {
            let mut current = self.state.write();
            if let Some(tab) = current.tabs.get_mut(index) {
                tab.selection = Some(SymbolSelection {
                    start_line,
                    end_line,
                });
            }
        }
        self.record_history(path, start_line);
        self.request_center(start_line);
    }

    /// 扫描源码包中的候选引用并按名称缓存。
    pub fn lookup_references(mut self, raw_name: String) {
        let name = raw_name.trim().to_owned();
        if !is_identifier(&name) {
            return;
        }

        if let Some(cached) = self.reference_cache.read().get(&name).cloned() {
            self.references.set(Some(cached));
            self.references_loading.set(false);
            self.references_error.set(None);
            self.references_progress.set(None);
            return;
        }

        let Some(pack) = self.state.read().pack.clone() else {
            return;
        };
        self.references.set(None);
        self.references_loading.set(true);
        self.references_error.set(None);
        self.references_progress.set(Some((0, 0, 0)));

        let scan = ReferenceScan::new();
        let mut scan_signal = self.reference_scan;
        scan_signal.set(Some(scan.clone()));
        let ticker_scan = scan.clone();
        let ticker_done = Arc::new(AtomicBool::new(false));
        let ticker_done_flag = ticker_done.clone();
        let mut progress = self.references_progress;
        dioxus::dioxus_core::spawn_forever(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(160)).await;
                if ticker_done_flag.load(Ordering::Relaxed) {
                    break;
                }
                progress.set(Some((
                    ticker_scan.scanned(),
                    ticker_scan.total(),
                    ticker_scan.hits(),
                )));
            }
        });

        let mut references = self.references;
        let mut loading = self.references_loading;
        let mut error = self.references_error;
        let mut cache_signal = self.reference_cache;

        dioxus::dioxus_core::spawn_forever(async move {
            let requested = name.clone();
            let scan_task = scan.clone();
            let result =
                tokio::task::spawn_blocking(move || find_references(&pack, &requested, &scan_task))
                    .await;
            ticker_done.store(true, Ordering::Relaxed);
            scan_signal.set(None);
            progress.set(None);
            loading.set(false);
            match result {
                Ok(results) if results.cancelled => {
                    error.set(Some("引用扫描已取消。".to_owned()));
                }
                Ok(results) => {
                    cache_signal.write().insert(name, results.clone());
                    references.set(Some(results));
                }
                Err(join_error) => error.set(Some(format!("引用查找失败: {join_error}"))),
            }
        });
    }

    pub fn cancel_references(self) {
        if let Some(scan) = self.reference_scan.read().as_ref() {
            scan.cancel();
        }
    }

    /// 扫描哪些文件导入了当前文件。
    pub fn lookup_reverse_dependencies(mut self) {
        let Some(pack) = self.state.read().pack.clone() else {
            return;
        };
        let Some(target) = self.state.read().active_path().map(str::to_owned) else {
            return;
        };
        if let Some(cached) = self.reverse_cache.read().get(&target).cloned() {
            self.reverse_dependencies.set(Some(cached));
            self.reverse_loading.set(false);
            self.reverse_error.set(None);
            self.reverse_progress.set(None);
            return;
        }
        let scan = ReferenceScan::new();
        let mut scan_signal = self.reverse_scan;
        scan_signal.set(Some(scan.clone()));
        self.reverse_dependencies.set(None);
        self.reverse_loading.set(true);
        self.reverse_error.set(None);
        self.reverse_progress.set(Some((0, 0)));

        let ticker_scan = scan.clone();
        let ticker_done = Arc::new(AtomicBool::new(false));
        let ticker_flag = ticker_done.clone();
        let mut progress = self.reverse_progress;
        dioxus::dioxus_core::spawn_forever(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(160)).await;
                if ticker_flag.load(Ordering::Relaxed) {
                    break;
                }
                progress.set(Some((ticker_scan.scanned(), ticker_scan.total())));
            }
        });

        let mut results_signal = self.reverse_dependencies;
        let mut loading = self.reverse_loading;
        let mut error = self.reverse_error;
        let mut cache_signal = self.reverse_cache;
        dioxus::dioxus_core::spawn_forever(async move {
            let scan_task = scan.clone();
            let target_task = target.clone();
            let result = tokio::task::spawn_blocking(move || {
                find_reverse_dependencies(&pack, &target_task, &scan_task)
            })
            .await;
            ticker_done.store(true, Ordering::Relaxed);
            scan_signal.set(None);
            progress.set(None);
            loading.set(false);
            match result {
                Ok((_results, cancelled)) if cancelled => {
                    error.set(Some("反向依赖扫描已取消。".to_owned()));
                }
                Ok((results, _)) => {
                    cache_signal.write().insert(target, results.clone());
                    results_signal.set(Some(results));
                }
                Err(join_error) => error.set(Some(format!("反向依赖扫描失败: {join_error}"))),
            }
        });
    }

    pub fn cancel_reverse_dependencies(self) {
        if let Some(scan) = self.reverse_scan.read().as_ref() {
            scan.cancel();
        }
    }

    /// 解析当前符号范围内的直接调用，并尽量解析到定义位置。
    pub fn lookup_callees(mut self, name: String, start_line: u32, end_line: u32) {
        let Some(pack) = self.state.read().pack.clone() else {
            return;
        };
        let (path, hints, language) = {
            let state = self.state.read();
            let Some(path) = state.active_path().map(str::to_owned) else {
                return;
            };
            let hints = state
                .active_document()
                .map(|document| definition_hints(document, &path))
                .unwrap_or_default();
            let language = state
                .active_document()
                .map_or("C", |document| document.language)
                .to_owned();
            (path, hints, language)
        };
        let index = self.index.read().clone();
        let root_path = path.clone();
        self.callees.set(None);
        self.dependencies.set(None);
        self.call_tree.set(None);
        self.callees_loading.set(true);
        self.callees_error.set(None);

        let mut callees = self.callees;
        let mut dependencies = self.dependencies;
        let mut call_tree = self.call_tree;
        let mut loading = self.callees_loading;
        let mut error = self.callees_error;
        dioxus::dioxus_core::spawn_forever(async move {
            let result = tokio::task::spawn_blocking(move || {
                let source = pack.read_text(&path)?;
                let sites = lattice_parser::call_sites(&path, &source, start_line, end_line)?;
                let targets = sites
                    .into_iter()
                    .map(|site| {
                        let found = index.as_ref().and_then(|index| {
                            index.definition_with_hints(&site.name, Some(&path), &language, &hints)
                        });
                        CallTarget {
                            name: site.name,
                            line: site.line,
                            path: found.map(|symbol| symbol.path.clone()),
                            target_line: found.map(|symbol| symbol.start_line),
                        }
                    })
                    .collect::<Vec<_>>();
                let files = pack.source_files().unwrap_or_default();
                let mut dependencies = Vec::new();
                let mut seen = HashSet::new();
                for hint in hints {
                    if !seen.insert(hint.clone()) {
                        continue;
                    }
                    let resolved = resolve_dependency(&files, &path, &hint);
                    dependencies.push(Dependency {
                        name: hint,
                        path: resolved,
                    });
                    if dependencies.len() >= 80 {
                        break;
                    }
                }
                Ok::<_, String>((targets, dependencies))
            })
            .await;
            loading.set(false);
            match result {
                Ok(Ok((targets, deps))) => {
                    let mut tree = CallTree {
                        next_id: 1,
                        nodes: vec![CallNode {
                            id: 0,
                            parent: None,
                            name,
                            call_line: start_line,
                            path: Some(root_path),
                            def_line: Some(start_line),
                            start_line: Some(start_line),
                            end_line: Some(end_line),
                            depth: 0,
                            expanded: true,
                            loading: false,
                            children_loaded: true,
                            error: None,
                        }],
                    };
                    for target in targets.iter() {
                        let id = tree.next_id;
                        tree.next_id += 1;
                        tree.nodes.push(CallNode {
                            id,
                            parent: Some(0),
                            name: target.name.clone(),
                            call_line: target.line,
                            path: target.path.clone(),
                            def_line: target.target_line,
                            start_line: target.target_line,
                            end_line: None,
                            depth: 1,
                            expanded: false,
                            loading: false,
                            children_loaded: false,
                            error: None,
                        });
                    }
                    callees.set(Some(targets));
                    dependencies.set(Some(deps));
                    call_tree.set(Some(tree));
                }
                Ok(Err(message)) => error.set(Some(format!("调用解析失败: {message}"))),
                Err(join_error) => error.set(Some(format!("调用解析任务失败: {join_error}"))),
            }
        });
    }

    pub fn toggle_call_node(self, id: u64) {
        let node = {
            let guard = self.call_tree.read();
            guard
                .as_ref()
                .and_then(|tree| tree.nodes.iter().find(|node| node.id == id).cloned())
        };
        let Some(node) = node else {
            return;
        };
        if node.loading {
            return;
        }
        if node.children_loaded {
            let mut call_tree = self.call_tree;
            let mut guard = call_tree.write();
            if let Some(tree) = guard.as_mut()
                && let Some(node) = tree.nodes.iter_mut().find(|node| node.id == id)
            {
                node.expanded = !node.expanded;
            }
            return;
        }
        self.expand_call_node(id);
    }

    fn expand_call_node(self, id: u64) {
        const MAX_CALL_DEPTH: usize = 5;
        const MAX_CALL_NODES: usize = 240;
        let Some(node) = ({
            let guard = self.call_tree.read();
            guard
                .as_ref()
                .and_then(|tree| tree.nodes.iter().find(|node| node.id == id).cloned())
        }) else {
            return;
        };
        let (Some(path), Some(def_line)) = (node.path.clone(), node.def_line) else {
            return;
        };
        if node.depth >= MAX_CALL_DEPTH {
            return;
        }
        let Some(pack) = self.state.read().pack.clone() else {
            return;
        };
        let index = self.index.read().clone();
        let mut call_tree = self.call_tree;
        {
            let mut guard = call_tree.write();
            if let Some(tree) = guard.as_mut()
                && let Some(node) = tree.nodes.iter_mut().find(|node| node.id == id)
            {
                node.loading = true;
                node.error = None;
            }
        }

        dioxus::dioxus_core::spawn_forever(async move {
            let result = tokio::task::spawn_blocking(move || {
                let source = pack.read_text(&path)?;
                let document = lattice_parser::parse(&path, &source)?;
                let symbol = document
                    .symbols
                    .iter()
                    .filter(|symbol| symbol.start_line <= def_line && symbol.end_line >= def_line)
                    .min_by_key(|symbol| symbol.end_line.saturating_sub(symbol.start_line));
                let (start_line, end_line) = symbol.map_or((def_line, def_line), |symbol| {
                    (symbol.start_line, symbol.end_line)
                });
                let hints = definition_hints(&document, &path);
                let sites = lattice_parser::call_sites(&path, &source, start_line, end_line)?;
                let mut seen = HashSet::new();
                let targets = sites
                    .into_iter()
                    .filter_map(|site| {
                        let found = index.as_ref().and_then(|index| {
                            index.definition_with_hints(
                                &site.name,
                                Some(&path),
                                document.language,
                                &hints,
                            )
                        });
                        let target_path = found.map(|symbol| symbol.path.clone());
                        let target_line = found.map(|symbol| symbol.start_line);
                        let key = (site.name.clone(), target_path.clone(), target_line);
                        if !seen.insert(key) {
                            return None;
                        }
                        Some(CallTarget {
                            name: site.name,
                            line: site.line,
                            path: target_path,
                            target_line,
                        })
                    })
                    .collect::<Vec<_>>();
                Ok::<_, String>((start_line, end_line, targets))
            })
            .await;

            let mut guard = call_tree.write();
            let Some(tree) = guard.as_mut() else {
                return;
            };
            let Some(index) = tree.nodes.iter().position(|node| node.id == id) else {
                return;
            };
            let depth = tree.nodes[index].depth;
            let parent_path = tree.nodes[index].path.clone();
            tree.nodes[index].loading = false;
            tree.nodes[index].children_loaded = true;
            match result {
                Ok(Ok((start_line, end_line, targets))) => {
                    tree.nodes[index].start_line = Some(start_line);
                    tree.nodes[index].end_line = Some(end_line);
                    tree.nodes[index].expanded = true;
                    let room = MAX_CALL_NODES.saturating_sub(tree.nodes.len());
                    for target in targets.into_iter().take(room) {
                        let child_id = tree.next_id;
                        tree.next_id += 1;
                        tree.nodes.push(CallNode {
                            id: child_id,
                            parent: Some(id),
                            name: target.name,
                            call_line: target.line,
                            path: target.path.or_else(|| parent_path.clone()),
                            def_line: target.target_line,
                            start_line: target.target_line,
                            end_line: None,
                            depth: depth + 1,
                            expanded: false,
                            loading: false,
                            children_loaded: false,
                            error: None,
                        });
                    }
                }
                Ok(Err(message)) => tree.nodes[index].error = Some(message),
                Err(join_error) => {
                    tree.nodes[index].error = Some(format!("调用链解析失败: {join_error}"));
                }
            }
        });
    }

    /// 对比当前文件与另一个源码包中的同路径文件。
    pub fn open_diff(mut self, other: SourcePack) {
        let Some(base_pack) = self.state.read().pack.clone() else {
            return;
        };
        let Some(path) = self.state.read().active_path().map(str::to_owned) else {
            return;
        };
        let base_label = format!("{} {}", base_pack.name(), base_pack.version());
        let other_label = format!("{} {}", other.name(), other.version());
        self.diff.set(None);
        self.diff_loading.set(true);
        self.diff_error.set(None);

        let mut diff_signal = self.diff;
        let mut loading = self.diff_loading;
        let mut error = self.diff_error;
        dioxus::dioxus_core::spawn_forever(async move {
            let requested = path.clone();
            let result = tokio::task::spawn_blocking(move || {
                let left = base_pack.read_text(&requested)?;
                let right = other.read_text(&requested)?;
                let (lines, added, removed, truncated) = diff_lines(&requested, &left, &right);
                let left_symbols = lattice_parser::parse(&requested, &left)
                    .map(|document| document.symbols)
                    .unwrap_or_default();
                let right_symbols = lattice_parser::parse(&requested, &right)
                    .map(|document| document.symbols)
                    .unwrap_or_default();
                let symbols = diff_symbols(&left_symbols, &right_symbols);
                Ok::<_, String>((lines, added, removed, truncated, symbols))
            })
            .await;
            loading.set(false);
            match result {
                Ok(Ok((lines, added, removed, truncated, symbols))) => {
                    diff_signal.set(Some(DiffView {
                        path,
                        base_label,
                        other_label,
                        lines,
                        symbols,
                        added,
                        removed,
                        truncated,
                    }));
                }
                Ok(Err(message)) => error.set(Some(format!("对比失败: {message}"))),
                Err(join_error) => error.set(Some(format!("对比任务失败: {join_error}"))),
            }
        });
    }

    pub fn close_diff(mut self) {
        self.diff.set(None);
        self.diff_error.set(None);
        self.diff_loading.set(false);
    }

    /// 带当前行号的定义查找：优先包含该行、且种类更合适的本地符号。
    pub fn open_definition_at(mut self, raw_name: String, clicked_line: u32) {
        let name = raw_name.trim().to_owned();
        if !is_identifier(&name) {
            return;
        }
        self.lookup_status.set(None);

        let local = {
            let state = self.state.read();
            let language = state
                .active_document()
                .map_or("C", |document| document.language);
            state.active_document().and_then(|document| {
                let mut candidates: Vec<&lattice_parser::Symbol> = document
                    .symbols
                    .iter()
                    .filter(|symbol| symbol.name == name)
                    .collect();
                candidates.sort_by_key(|symbol| {
                    let contains = clicked_line > 0
                        && symbol.start_line <= clicked_line
                        && clicked_line <= symbol.end_line;
                    (
                        definition_kind_rank(language, &name, symbol.kind),
                        !contains,
                        symbol.start_line.abs_diff(clicked_line.max(1)),
                        symbol.start_line,
                    )
                });
                candidates
                    .first()
                    .map(|symbol| (symbol.start_line, symbol.end_line))
            })
        };
        if let Some((start_line, end_line)) = local {
            self.select_symbol(start_line, end_line);
            return;
        }

        let hints = {
            let state = self.state.read();
            state
                .active_document()
                .map(|document| {
                    definition_hints(
                        document,
                        state.active_path().unwrap_or(document.path.as_str()),
                    )
                })
                .unwrap_or_default()
        };
        let (context_path, language) = {
            let state = self.state.read();
            (
                state.active_path().map(str::to_owned),
                state
                    .pack
                    .as_ref()
                    .map_or_else(|| "C".to_owned(), |pack| pack.source_language().to_owned()),
            )
        };
        let indexed = {
            let index = self.index.read();
            index.as_ref().and_then(|index| {
                index
                    .definition_with_hints(&name, context_path.as_deref(), &language, &hints)
                    .map(|symbol| DefinitionLocation {
                        path: symbol.path.clone(),
                        start_line: symbol.start_line,
                        end_line: symbol.end_line,
                    })
            })
        };
        if let Some(location) = indexed {
            self.definition_cache
                .write()
                .insert(name, Some(location.clone()));
            self.navigate_definition(location);
            return;
        }

        let cached = self.definition_cache.read().get(&name).cloned();
        match cached {
            Some(Some(location)) => {
                self.navigate_definition(location);
                return;
            }
            Some(None) => {
                self.lookup_status.set(Some(format!("未找到定义：{name}")));
                return;
            }
            None => {}
        }

        let Some(pack) = self.state.read().pack.clone() else {
            return;
        };
        self.lookup_status
            .set(Some(format!("正在查找 {name} 的定义…")));

        let mut session = self;
        let mut cache = self.definition_cache;
        dioxus::dioxus_core::spawn_forever(async move {
            let requested = name.clone();
            let result =
                tokio::task::spawn_blocking(move || find_definition(&pack, &requested)).await;

            match result {
                Ok(Some(location)) => {
                    cache.write().insert(name.clone(), Some(location.clone()));
                    session.lookup_status.set(None);
                    session.navigate_definition(location);
                }
                Ok(None) => {
                    cache.write().insert(name.clone(), None);
                    session
                        .lookup_status
                        .set(Some(format!("未找到定义：{name}")));
                }
                Err(error) => {
                    session
                        .lookup_status
                        .set(Some(format!("定义查找失败：{error}")));
                }
            }
        });
    }

    fn navigate_definition(mut self, location: DefinitionLocation) {
        let existing = self
            .state
            .read()
            .tabs
            .iter()
            .position(|tab| tab.path == location.path);

        if let Some(index) = existing {
            self.state.write().active = Some(index);
        } else {
            self.open_internal(location.path.clone(), false);
        }

        {
            let mut current = self.state.write();
            if let Some(tab) = current
                .tabs
                .iter_mut()
                .find(|tab| tab.path == location.path)
            {
                tab.selection = Some(SymbolSelection {
                    start_line: location.start_line,
                    end_line: location.end_line,
                });
            }
        }

        self.record_history(location.path, location.start_line);
        self.request_center(location.start_line);
    }

    pub fn go_history(mut self, delta: isize) {
        let target = {
            let current = self.state.read();
            let next = if delta < 0 {
                current.history_index.checked_sub(1)
            } else {
                current
                    .history_index
                    .checked_add(1)
                    .filter(|index| *index < current.history.len())
            };
            next.and_then(|index| {
                current
                    .history
                    .get(index)
                    .cloned()
                    .map(|location| (index, location))
            })
        };

        let Some((index, location)) = target else {
            return;
        };

        self.state.write().history_index = index;
        let existing = self
            .state
            .read()
            .tabs
            .iter()
            .position(|tab| tab.path == location.path);

        if let Some(tab_index) = existing {
            self.state.write().active = Some(tab_index);
        } else {
            self.open_internal(location.path, false);
        }
        self.request_center(location.line);
    }

    fn record_history(self, path: String, line: u32) {
        let mut state = self.state;
        let mut current = state.write();
        let location = JumpLocation { path, line };
        if current.history.get(current.history_index) == Some(&location) {
            return;
        }

        let keep = current.history_index + 1;
        current.history.truncate(keep);
        current.history.push(location);
        if current.history.len() > 64 {
            current.history.remove(0);
        }
        current.history_index = current.history.len().saturating_sub(1);
    }
}

fn diff_symbols(left: &[Symbol], right: &[Symbol]) -> Vec<DiffSymbol> {
    let mut changes = Vec::new();
    for left_symbol in left {
        match right
            .iter()
            .find(|symbol| symbol.name == left_symbol.name && symbol.kind == left_symbol.kind)
        {
            None => changes.push(DiffSymbol {
                name: left_symbol.name.clone(),
                kind: left_symbol.kind,
                status: DiffSymbolStatus::Removed,
            }),
            Some(right_symbol) if right_symbol.signature != left_symbol.signature => {
                changes.push(DiffSymbol {
                    name: left_symbol.name.clone(),
                    kind: left_symbol.kind,
                    status: DiffSymbolStatus::Changed,
                });
            }
            Some(_) => {}
        }
    }
    for right_symbol in right {
        if !left
            .iter()
            .any(|symbol| symbol.name == right_symbol.name && symbol.kind == right_symbol.kind)
        {
            changes.push(DiffSymbol {
                name: right_symbol.name.clone(),
                kind: right_symbol.kind,
                status: DiffSymbolStatus::Added,
            });
        }
    }
    changes.sort_by(|a, b| a.name.cmp(&b.name).then(a.kind.label().cmp(b.kind.label())));
    changes.truncate(500);
    changes
}

fn diff_lines(_path: &str, left: &str, right: &str) -> (Vec<DiffLine>, usize, usize, bool) {
    const MAX_DIFF_LINES: usize = 4_000;
    let diff = TextDiff::from_lines(left, right);
    let mut lines = Vec::new();
    let mut added = 0usize;
    let mut removed = 0usize;
    let mut truncated = false;
    for change in diff.iter_all_changes() {
        let kind = match change.tag() {
            ChangeTag::Equal => DiffKind::Same,
            ChangeTag::Insert => {
                added += 1;
                DiffKind::Added
            }
            ChangeTag::Delete => {
                removed += 1;
                DiffKind::Removed
            }
        };
        if lines.len() >= MAX_DIFF_LINES {
            truncated = true;
            break;
        }
        lines.push(DiffLine {
            old_line: change.old_index().map(|index| index as u32 + 1),
            new_line: change.new_index().map(|index| index as u32 + 1),
            kind,
            text: change.value().trim_end_matches(['\r', '\n']).to_owned(),
        });
    }
    (lines, added, removed, truncated)
}

fn find_reverse_dependencies(
    pack: &SourcePack,
    target: &str,
    scan: &ReferenceScan,
) -> (Vec<ReverseDependency>, bool) {
    const MAX_RESULTS: usize = 500;
    let Ok(files) = pack.source_files() else {
        return (Vec::new(), false);
    };
    scan.set_total(files.len() as u64);
    let mut results = Vec::new();
    let mut cancelled = false;
    for (index, path) in files.iter().enumerate() {
        if scan.is_cancelled() {
            cancelled = true;
            break;
        }
        scan.record_file(index as u64 + 1);
        if path == target {
            continue;
        }
        let Ok(source) = pack.read_text(path) else {
            continue;
        };
        let language = language_for_path(path);
        for (line_index, line) in source.lines().enumerate() {
            if !might_contain_dependency(language, line) {
                continue;
            }
            let hints = dependency_hints_for_line(language, line, path);
            if let Some(hint) = hints
                .into_iter()
                .find(|hint| hint_matches_target(hint, target))
            {
                results.push(ReverseDependency {
                    path: path.clone(),
                    line: line_index as u32 + 1,
                    hint,
                });
                break;
            }
        }
        if results.len() >= MAX_RESULTS {
            break;
        }
    }
    results.sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
    (results, cancelled)
}

fn language_for_path(path: &str) -> &'static str {
    match std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("java") => "Java",
        Some("rs") => "Rust",
        Some("ts" | "tsx" | "mts" | "cts") => "TypeScript",
        _ => "C",
    }
}

fn might_contain_dependency(language: &str, line: &str) -> bool {
    let trimmed = line.trim_start();
    match language {
        "Java" => trimmed.starts_with("import ") || trimmed.starts_with("package "),
        "Rust" => trimmed.starts_with("use ") || trimmed.starts_with("mod "),
        "TypeScript" => {
            trimmed.starts_with("import ")
                || trimmed.starts_with("export ")
                || trimmed.contains("require(")
        }
        _ => trimmed.starts_with("#include"),
    }
}

fn dependency_hints_for_line(language: &str, line: &str, active_path: &str) -> Vec<String> {
    let mut hints = Vec::new();
    match language {
        "Java" => collect_java_hints(line, &mut hints),
        "TypeScript" => collect_typescript_hints(line, active_path, &mut hints),
        "Rust" => collect_rust_hints(line, &mut hints),
        _ => collect_c_hints(line, &mut hints),
    }
    hints
}

fn hint_matches_target(hint: &str, target: &str) -> bool {
    let hint = hint.trim().replace('\\', "/");
    if hint.is_empty() {
        return false;
    }
    if hint.ends_with('/') {
        return target.contains(&hint);
    }
    if target == hint || target.ends_with(&format!("/{hint}")) {
        return true;
    }
    if !hint.contains('/') {
        return target.rsplit('/').next().is_some_and(|name| name == hint);
    }
    false
}

fn resolve_dependency(files: &[String], active_path: &str, hint: &str) -> Option<String> {
    let hint = hint.trim().replace('\\', "/");
    if hint.is_empty() || hint.ends_with('/') {
        return None;
    }
    if let Some(exact) = files.iter().find(|path| **path == hint) {
        return Some(exact.clone());
    }
    if let Some((directory, _)) = active_path.rsplit_once('/') {
        let local = format!("{directory}/{hint}");
        if let Some(found) = files.iter().find(|path| **path == local) {
            return Some(found.clone());
        }
    }
    let suffix = format!("/{hint}");
    if let Some(found) = files.iter().find(|path| path.ends_with(&suffix)) {
        return Some(found.clone());
    }
    let file_name = hint.rsplit('/').next().unwrap_or(&hint);
    let name_suffix = format!("/{file_name}");
    files
        .iter()
        .find(|path| path.ends_with(&name_suffix) || path.as_str() == file_name)
        .cloned()
}

fn definition_hints(document: &Document, active_path: &str) -> Vec<String> {
    let mut hints = Vec::new();
    for line in &document.lines {
        let text: String = line.spans.iter().map(|span| span.text.as_str()).collect();
        let line = text.trim();
        if line.is_empty() {
            continue;
        }
        match document.language {
            "Java" => collect_java_hints(line, &mut hints),
            "TypeScript" => collect_typescript_hints(line, active_path, &mut hints),
            "Rust" => collect_rust_hints(line, &mut hints),
            _ => collect_c_hints(line, &mut hints),
        }
    }
    hints.sort();
    hints.dedup();
    hints.truncate(96);
    hints
}

fn collect_java_hints(line: &str, hints: &mut Vec<String>) {
    if let Some(package) = line.strip_prefix("package ") {
        let package = package.trim_end_matches(';').trim();
        if !package.is_empty() {
            hints.push(format!("{}/", package.replace('.', "/")));
        }
        return;
    }
    let Some(import) = line.strip_prefix("import ") else {
        return;
    };
    let import = import.trim_end_matches(';').trim();
    let static_import = import.strip_prefix("static ").unwrap_or(import);
    let mut parts: Vec<&str> = static_import.split('.').collect();
    if parts.last() == Some(&"*") {
        parts.pop();
        if !parts.is_empty() {
            if import.starts_with("static ") {
                hints.push(format!("{}.java", parts.join("/")));
            } else {
                hints.push(format!("{}/", parts.join("/")));
            }
        }
        return;
    }
    if import.starts_with("static ") {
        parts.pop();
    }
    if let Some(class) = parts.last() {
        hints.push(format!(
            "{}.java",
            parts[..parts.len() - 1].join("/") + "/" + class
        ));
    }
}

fn collect_typescript_hints(line: &str, active_path: &str, hints: &mut Vec<String>) {
    let specifier = if let Some(index) = line.find(" from ") {
        quoted_module(&line[index + 6..])
    } else if line.starts_with("import ") && (line.contains('"') || line.contains('\'')) {
        quoted_module(line)
    } else if let Some(index) = line.find("require(") {
        quoted_module(&line[index + 8..])
    } else {
        None
    };
    let Some(specifier) = specifier else {
        return;
    };
    if !specifier.starts_with('.') {
        return;
    }
    let Some(base) = resolve_relative_module(active_path, specifier) else {
        return;
    };
    if base.ends_with(".ts") || base.ends_with(".tsx") || base.ends_with(".d.ts") {
        hints.push(base);
    } else {
        hints.push(format!("{base}.ts"));
        hints.push(format!("{base}.tsx"));
        hints.push(format!("{base}/index.ts"));
        hints.push(format!("{base}/index.tsx"));
    }
}

fn collect_rust_hints(line: &str, hints: &mut Vec<String>) {
    if let Some(module) = line.strip_prefix("mod ") {
        let module = module.trim_end_matches(';').trim();
        let module = module.split_whitespace().next().unwrap_or_default();
        if !module.is_empty() {
            hints.push(format!("{module}.rs"));
            hints.push(format!("{module}/mod.rs"));
        }
        return;
    }
    let Some(use_line) = line.strip_prefix("use ") else {
        return;
    };
    let use_line = use_line.trim_end_matches(';').trim();
    let use_line = use_line.split(" as ").next().unwrap_or(use_line);
    let mut module = use_line
        .split('{')
        .next()
        .unwrap_or_default()
        .trim_end_matches("::")
        .trim_end_matches('*')
        .trim_end_matches("::")
        .to_owned();
    for prefix in ["crate::", "self::", "super::"] {
        module = module.trim_start_matches(prefix).to_owned();
    }
    let mut parts: Vec<&str> = module.split("::").filter(|part| !part.is_empty()).collect();
    if !use_line.contains('{') && parts.len() > 1 {
        parts.pop();
    }
    if parts.is_empty() {
        return;
    }
    let path = parts.join("/");
    hints.push(format!("{path}.rs"));
    hints.push(format!("{path}/mod.rs"));
}

fn collect_c_hints(line: &str, hints: &mut Vec<String>) {
    let Some(include) = line.strip_prefix("#include") else {
        return;
    };
    let include = include.trim();
    let name = include
        .trim_matches(|character| character == '"' || character == '<' || character == '>')
        .trim();
    if !name.is_empty() {
        hints.push(name.to_owned());
    }
}

fn quoted_module(text: &str) -> Option<&str> {
    let start = text.find(['"', '\''])?;
    let quote = text.as_bytes()[start] as char;
    let rest = &text[start + 1..];
    let end = rest.find(quote)?;
    Some(&rest[..end])
}

fn resolve_relative_module(active_path: &str, specifier: &str) -> Option<String> {
    let directory = active_path
        .rsplit_once('/')
        .map(|(directory, _)| directory)
        .unwrap_or_default();
    let joined = if directory.is_empty() {
        specifier.to_owned()
    } else {
        format!("{directory}/{specifier}")
    };
    let mut parts = Vec::new();
    for part in joined.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

fn definition_kind_rank(language: &str, name: &str, kind: SymbolKind) -> u8 {
    if language != "Java" {
        return 0;
    }
    let type_name = name
        .chars()
        .next()
        .is_some_and(|character| character.is_uppercase());
    match kind {
        SymbolKind::Class
        | SymbolKind::Interface
        | SymbolKind::Enum
        | SymbolKind::Record
        | SymbolKind::Annotation => {
            if type_name {
                0
            } else {
                3
            }
        }
        SymbolKind::Constructor => {
            if type_name {
                1
            } else {
                0
            }
        }
        SymbolKind::Method | SymbolKind::Function => {
            if type_name {
                2
            } else {
                0
            }
        }
        _ => 4,
    }
}

pub fn is_foldable_symbol(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Class
            | SymbolKind::Interface
            | SymbolKind::Enum
            | SymbolKind::Record
            | SymbolKind::Annotation
            | SymbolKind::Struct
            | SymbolKind::Union
            | SymbolKind::Trait
            | SymbolKind::Module
            | SymbolKind::Impl
            | SymbolKind::Function
            | SymbolKind::Method
            | SymbolKind::Constructor
            | SymbolKind::Macro
    )
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn find_references(pack: &SourcePack, name: &str, scan: &ReferenceScan) -> ReferenceResults {
    const MAX_REFERENCES: usize = 20_000;
    let mut hits = Vec::new();
    let mut truncated = false;
    let mut cancelled = false;
    let Ok(files) = pack.source_files() else {
        return ReferenceResults {
            name: name.to_owned(),
            hits,
            truncated: false,
            cancelled: false,
        };
    };
    scan.set_total(files.len() as u64);

    'files: for (index, path) in files.iter().enumerate() {
        if scan.is_cancelled() {
            cancelled = true;
            break;
        }
        scan.record_file(index as u64 + 1);
        let Ok(source) = pack.read_text(path) else {
            continue;
        };
        if !contains_identifier(&source, name) {
            continue;
        }
        let Ok(occurrences) = lattice_parser::references(path, &source, name) else {
            continue;
        };
        let lines: Vec<&str> = source.lines().collect();
        for occurrence in occurrences {
            let preview = lines
                .get(occurrence.line.saturating_sub(1) as usize)
                .map(|line| reference_preview_line(line))
                .unwrap_or_default();
            if hits
                .iter()
                .any(|hit: &ReferenceHit| hit.path == *path && hit.line == occurrence.line)
            {
                continue;
            }
            hits.push(ReferenceHit {
                path: path.clone(),
                line: occurrence.line,
                kind: occurrence.kind,
                conditional: occurrence.conditional,
                preview,
            });
            scan.record_hits(hits.len() as u64);
            if hits.len() >= MAX_REFERENCES {
                truncated = true;
                break 'files;
            }
        }
    }

    hits.sort_by_key(|hit| {
        (
            reference_rank(hit.kind),
            hit.conditional,
            hit.path.clone(),
            hit.line,
        )
    });
    ReferenceResults {
        name: name.to_owned(),
        hits,
        truncated,
        cancelled,
    }
}

const fn reference_rank(kind: ReferenceKind) -> u8 {
    match kind {
        ReferenceKind::Definition => 0,
        ReferenceKind::Call => 1,
        ReferenceKind::Type => 2,
        ReferenceKind::Macro => 3,
        ReferenceKind::Field => 4,
        ReferenceKind::Other => 5,
    }
}

fn contains_identifier(source: &str, name: &str) -> bool {
    let bytes = source.as_bytes();
    let needle = name.as_bytes();
    let mut cursor = 0;
    while cursor + needle.len() <= bytes.len() {
        let Some(relative) = source[cursor..].find(name) else {
            return false;
        };
        let start = cursor + relative;
        let end = start + needle.len();
        cursor = end.max(start + 1);
        let before_ok = start == 0 || !is_identifier_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_identifier_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

fn is_identifier_byte(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

fn reference_preview_line(line: &str) -> String {
    let line = line.trim();
    let mut preview: String = line.chars().take(140).collect();
    if line.chars().count() > 140 {
        preview.push('…');
    }
    preview
}

fn find_definition(pack: &SourcePack, name: &str) -> Option<DefinitionLocation> {
    let files = pack.source_files().ok()?;
    for path in files {
        let Ok(source) = pack.read_text(&path) else {
            continue;
        };
        let Ok(symbols) = lattice_parser::outline(&path, &source) else {
            continue;
        };
        if let Some(symbol) = symbols.into_iter().find(|symbol| symbol.name == name) {
            return Some(DefinitionLocation {
                path,
                start_line: symbol.start_line,
                end_line: symbol.end_line,
            });
        }
    }
    None
}

struct RestoredSession {
    pack: SourcePack,
    tree: SourceTree,
    tabs: Vec<DocumentTab>,
    active: usize,
    saved_line: u32,
    positions: HashMap<String, u32>,
    expanded: HashSet<String>,
    available: Vec<SourcePack>,
}

fn load_bootstrap() -> Result<RestoredSession, String> {
    let config = SessionConfig::startup().clone();
    let saved_pack = config.pack_root.clone();
    let restored_pack = saved_pack
        .as_deref()
        .and_then(|root| SourcePack::open(PathBuf::from(root)).ok());
    let restore_saved = restored_pack
        .as_ref()
        .is_some_and(|pack| saved_pack.as_deref() == Some(pack.root().to_string_lossy().as_ref()));
    let pack = match restored_pack {
        Some(pack) => pack,
        None => SourcePack::discover()?,
    };

    let available = SourcePack::installed().unwrap_or_default();
    let tree = pack.scan()?;

    let mut tabs: Vec<DocumentTab> = Vec::new();
    let mut active = 0;
    if restore_saved {
        let saved_active = config.tabs.get(config.active_tab).cloned();
        for path in config.tabs.iter() {
            if tabs.len() >= 32 {
                break;
            }
            if tabs.iter().any(|tab| &tab.path == path) {
                continue;
            }
            let Some(tab) = load_document_tab(&pack, path) else {
                continue;
            };
            if saved_active.as_deref() == Some(path.as_str()) {
                active = tabs.len();
            }
            tabs.push(tab);
        }
    }

    if tabs.is_empty() {
        let path = pack.default_entry().to_owned();
        let tab = load_document_tab(&pack, &path)
            .ok_or_else(|| format!("无法打开默认源码文件: {path}"))?;
        tabs.push(tab);
        active = 0;
    }
    if active >= tabs.len() {
        active = 0;
    }

    let positions: HashMap<String, u32> = if restore_saved {
        config.positions.into_iter().collect()
    } else {
        HashMap::new()
    };
    let active_path = tabs[active].path.clone();
    let saved_line = positions.get(&active_path).copied().unwrap_or(1).max(1);
    let expanded = if restore_saved && !config.expanded.is_empty() {
        config.expanded.into_iter().collect()
    } else {
        default_open_dirs(&tree, &pack)
    };

    Ok(RestoredSession {
        pack,
        tree,
        tabs,
        active,
        saved_line,
        positions,
        expanded,
        available,
    })
}

fn load_document_tab(pack: &SourcePack, path: &str) -> Option<DocumentTab> {
    let source = pack.read_text(path).ok()?;
    let cache_path = pack.document_cache_path(path);
    let loaded = lattice_parser::load_or_parse_cached(path, &source, &cache_path).ok()?;
    Some(DocumentTab {
        path: path.to_owned(),
        document: Some(loaded.document),
        selection: None,
        error: None,
        from_cache: loaded.from_cache,
    })
}

fn default_open_dirs(tree: &SourceTree, pack: &SourcePack) -> HashSet<String> {
    let candidates: &[&str] = match pack.source_language() {
        "Java" => &["src"],
        "Rust" => &["compiler", "library", "src"],
        "TypeScript" => &["src", "lib"],
        _ => &["Objects", "Include", "Python"],
    };
    candidates
        .iter()
        .filter(|segment| {
            tree.entries
                .iter()
                .any(|entry| entry.is_dir && entry.name == **segment)
        })
        .map(|segment| (*segment).to_owned())
        .collect()
}

fn load_pack(
    pack: SourcePack,
) -> Result<(SourcePack, lattice_content::SourceTree, LoadedDocument), String> {
    let tree = pack.scan()?;
    let path = pack.default_entry();
    let source = pack.read_text(path)?;
    let cache_path = pack.document_cache_path(path);
    let loaded = lattice_parser::load_or_parse_cached(path, &source, &cache_path)?;
    Ok((pack, tree, loaded))
}

pub fn scroll_to_top_row(row: u32) {
    let script = format!(
        "requestAnimationFrame(() => {{ const el = document.querySelector('.code-scroll'); if (el) {{ const row = 22; el.scrollTop = Math.max(0, ({} - 1) * row); }} }});",
        row.max(1)
    );
    dioxus::document::eval(&script);
}

pub fn scroll_to_row(row: u32) {
    let script = format!(
        "requestAnimationFrame(() => {{ const el = document.querySelector('.code-scroll'); if (el) {{ const row = 22; const target = ({} - 1) * row; el.scrollTop = Math.max(0, target - el.clientHeight / 2 + row / 2); }} }});",
        row.max(1)
    );
    dioxus::document::eval(&script);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(language: &'static str, lines: &[&str]) -> Document {
        Document {
            path: "test".to_owned(),
            language,
            lines: lines
                .iter()
                .enumerate()
                .map(|(index, text)| lattice_parser::Line {
                    number: index as u32 + 1,
                    spans: vec![lattice_parser::Span {
                        text: (*text).to_owned(),
                        class: "plain",
                    }],
                })
                .collect(),
            symbols: Vec::new(),
        }
    }

    #[test]
    fn detects_symbol_level_changes_between_versions() {
        let symbol = |name: &str, kind: SymbolKind, signature: &str| Symbol {
            name: name.to_owned(),
            kind,
            start_line: 1,
            end_line: 2,
            signature: signature.to_owned(),
        };
        let left = vec![
            symbol("old_fn", SymbolKind::Function, "void old_fn(void)"),
            symbol("parse", SymbolKind::Function, "int parse(int value)"),
        ];
        let right = vec![
            symbol("parse", SymbolKind::Function, "int parse(long value)"),
            symbol("new_fn", SymbolKind::Function, "void new_fn(void)"),
        ];
        let changes = diff_symbols(&left, &right);
        assert!(changes.iter().any(|change| {
            change.name == "old_fn" && change.status == DiffSymbolStatus::Removed
        }));
        assert!(changes.iter().any(|change| {
            change.name == "parse" && change.status == DiffSymbolStatus::Changed
        }));
        assert!(
            changes.iter().any(|change| {
                change.name == "new_fn" && change.status == DiffSymbolStatus::Added
            })
        );
    }

    #[test]
    fn computes_a_line_diff_between_pack_versions() {
        let (lines, added, removed, truncated) =
            diff_lines("src/main.c", "one\ntwo\nthree\n", "one\nthree\nfour\n");
        assert!(!truncated);
        assert_eq!(added, 1);
        assert_eq!(removed, 1);
        assert!(
            lines
                .iter()
                .any(|line| line.kind == DiffKind::Removed && line.text == "two")
        );
        assert!(
            lines
                .iter()
                .any(|line| line.kind == DiffKind::Added && line.text == "four")
        );
    }

    #[test]
    fn resolves_dependency_hints_to_pack_files() {
        let files = vec![
            "src/main.c".to_owned(),
            "src/sqliteInt.h".to_owned(),
            "ext/misc/json1.c".to_owned(),
        ];
        assert_eq!(
            resolve_dependency(&files, "src/main.c", "sqliteInt.h").as_deref(),
            Some("src/sqliteInt.h")
        );
        assert_eq!(
            resolve_dependency(&files, "src/main.c", "ext/misc/json1.c").as_deref(),
            Some("ext/misc/json1.c")
        );
        assert!(resolve_dependency(&files, "src/main.c", "stdio.h").is_none());
    }

    #[test]
    fn extracts_java_import_hints() {
        let doc = document(
            "Java",
            &[
                "package java.util;",
                "import java.util.ArrayList;",
                "import static java.lang.Math.max;",
            ],
        );
        let hints = definition_hints(&doc, "src/java/util/List.java");
        assert!(hints.contains(&"java/util/".to_owned()));
        assert!(hints.contains(&"java/util/ArrayList.java".to_owned()));
        assert!(hints.contains(&"java/lang/Math.java".to_owned()));
    }

    #[test]
    fn extracts_typescript_relative_import_hints() {
        let doc = document(
            "TypeScript",
            &[
                "import { parse } from \"./parser\";",
                "export * from '../util';",
            ],
        );
        let hints = definition_hints(&doc, "src/compiler/checker.ts");
        assert!(hints.contains(&"src/compiler/parser.ts".to_owned()));
        assert!(hints.contains(&"src/util/index.ts".to_owned()));
    }

    #[test]
    fn extracts_rust_use_and_mod_hints() {
        let doc = document(
            "Rust",
            &[
                "use crate::ast::Expr;",
                "mod parser;",
                "use crate::token::{Token, Kind};",
            ],
        );
        let hints = definition_hints(&doc, "src/lib.rs");
        assert!(hints.contains(&"ast.rs".to_owned()));
        assert!(hints.contains(&"parser.rs".to_owned()));
        assert!(hints.contains(&"token.rs".to_owned()));
    }

    #[test]
    fn extracts_c_include_hints() {
        let doc = document("C", &["#include \"Python.h\"", "#include <stdio.h>"]);
        let hints = definition_hints(&doc, "Objects/listobject.c");
        assert!(hints.contains(&"Python.h".to_owned()));
        assert!(hints.contains(&"stdio.h".to_owned()));
    }
}
