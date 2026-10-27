//! 源码包级文件与符号索引。
//!
//! 索引在后台建立，只保存阅读导航需要的轻量结果；源码内容仍由 Source Pack
//! 按需读取，不在内存中复制整个仓库。

use lattice_content::{SourceEntry, SourcePack, SourceTree};
use lattice_parser::SymbolKind;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

const CACHE_FORMAT_VERSION: u32 = 2;

pub const INDEX_CANCELLED: &str = "索引建立已取消";

/// 索引构建的进度与取消控制，可跨线程读取。
#[derive(Clone, Default, Debug)]
pub struct IndexControl {
    inner: Arc<IndexControlInner>,
}

#[derive(Default, Debug)]
struct IndexControlInner {
    done: AtomicU64,
    total: AtomicU64,
    cancelled: AtomicBool,
}

impl IndexControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Relaxed)
    }

    pub fn files_done(&self) -> u64 {
        self.inner.done.load(Ordering::Relaxed)
    }

    pub fn files_total(&self) -> u64 {
        self.inner.total.load(Ordering::Relaxed)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexedSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub signature: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedFile {
    pub name: String,
    pub path: String,
}

#[derive(Debug)]
pub struct IndexLoad {
    pub index: SourceIndex,
    pub loaded_from_cache: bool,
    pub persisted: bool,
}

#[derive(Serialize, Deserialize)]
struct CachedIndex {
    version: u32,
    fingerprint: String,
    symbols: Vec<CachedSymbol>,
    files: Vec<IndexedFile>,
}

#[derive(Serialize, Deserialize)]
struct CachedSymbol {
    name: String,
    kind: u8,
    path: String,
    start_line: u32,
    end_line: u32,
    signature: String,
}

#[derive(Debug, Default)]
pub struct SourceIndex {
    symbols: Vec<IndexedSymbol>,
    files: Vec<IndexedFile>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchResults {
    pub symbols: Vec<IndexedSymbol>,
    pub files: Vec<IndexedFile>,
}

impl SourceIndex {
    pub fn build(pack: &SourcePack, tree: &SourceTree) -> Result<Self, String> {
        Self::build_with_control(pack, tree, &IndexControl::new())
            .and_then(|index| index.ok_or_else(|| INDEX_CANCELLED.to_owned()))
    }

    /// 可取消构建；取消时返回 `None`，不写缓存。
    pub fn build_with_control(
        pack: &SourcePack,
        tree: &SourceTree,
        control: &IndexControl,
    ) -> Result<Option<Self>, String> {
        let mut files = Vec::new();
        collect_files(&tree.entries, &mut files);
        files.sort_by(|a, b| a.path.cmp(&b.path));

        let source_files = pack.source_files()?;
        control
            .inner
            .total
            .store(source_files.len() as u64, Ordering::Relaxed);
        let mut symbols = Vec::new();
        for (index, path) in source_files.iter().enumerate() {
            if control.is_cancelled() {
                return Ok(None);
            }
            control
                .inner
                .done
                .store(index as u64 + 1, Ordering::Relaxed);
            let Ok(source) = pack.read_text(path) else {
                continue;
            };
            let Ok(outline) = lattice_parser::outline(path, &source) else {
                continue;
            };
            symbols.extend(outline.into_iter().map(|symbol| IndexedSymbol {
                name: symbol.name,
                kind: symbol.kind,
                path: path.clone(),
                start_line: symbol.start_line,
                end_line: symbol.end_line,
                signature: symbol.signature,
            }));
        }
        if control.is_cancelled() {
            return Ok(None);
        }

        symbols.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.path.cmp(&b.path))
                .then_with(|| a.start_line.cmp(&b.start_line))
        });
        symbols
            .dedup_by(|a, b| a.name == b.name && a.path == b.path && a.start_line == b.start_line);

        Ok(Some(Self { symbols, files }))
    }

    /// 预构建缓存默认与 Source Pack 放在一起，便于离线包携带。
    pub fn cache_path(pack: &SourcePack) -> PathBuf {
        pack.root()
            .join(".lattice")
            .join(format!("symbol-index-v{CACHE_FORMAT_VERSION}.postcard"))
    }

    /// 有有效缓存时直接加载；否则构建并尽力写回缓存。
    pub fn load_or_build(pack: &SourcePack, tree: &SourceTree) -> Result<IndexLoad, String> {
        Self::load_or_build_with_control(pack, tree, &IndexControl::new())
            .and_then(|load| load.ok_or_else(|| INDEX_CANCELLED.to_owned()))
    }

    /// 可取消的缓存加载与构建；取消时返回 `None`。
    pub fn load_or_build_with_control(
        pack: &SourcePack,
        tree: &SourceTree,
        control: &IndexControl,
    ) -> Result<Option<IndexLoad>, String> {
        let fingerprint = fingerprint(pack)?;
        let cache_path = Self::cache_path(pack);
        if let Some(index) = load_cache(&cache_path, &fingerprint) {
            return Ok(Some(IndexLoad {
                index,
                loaded_from_cache: true,
                persisted: true,
            }));
        }

        let Some(index) = Self::build_with_control(pack, tree, control)? else {
            return Ok(None);
        };
        if control.is_cancelled() {
            return Ok(None);
        }
        let persisted = index.write_cache(&cache_path, &fingerprint).is_ok();
        Ok(Some(IndexLoad {
            index,
            loaded_from_cache: false,
            persisted,
        }))
    }

    fn write_cache(&self, path: &Path, fingerprint: &str) -> Result<(), String> {
        let cached = CachedIndex {
            version: CACHE_FORMAT_VERSION,
            fingerprint: fingerprint.to_owned(),
            symbols: self
                .symbols
                .iter()
                .map(|symbol| CachedSymbol {
                    name: symbol.name.clone(),
                    kind: symbol_kind_code(symbol.kind),
                    path: symbol.path.clone(),
                    start_line: symbol.start_line,
                    end_line: symbol.end_line,
                    signature: symbol.signature.clone(),
                })
                .collect(),
            files: self.files.clone(),
        };
        let bytes =
            postcard::to_stdvec(&cached).map_err(|error| format!("序列化索引缓存失败: {error}"))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("创建索引缓存目录失败: {error}"))?;
        }
        let temporary = path.with_extension("postcard.tmp");
        fs::write(&temporary, bytes).map_err(|error| format!("写入索引缓存失败: {error}"))?;
        if path.exists() {
            fs::remove_file(path).map_err(|error| format!("替换索引缓存失败: {error}"))?;
        }
        fs::rename(&temporary, path).map_err(|error| format!("提交索引缓存失败: {error}"))
    }

    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn definition(&self, name: &str) -> Option<&IndexedSymbol> {
        self.definition_from(name, None, "C")
    }

    /// 在缺少完整语义信息时，按当前文件所在目录与符号种类做保守排序。
    pub fn definition_from(
        &self,
        name: &str,
        context_path: Option<&str>,
        language: &str,
    ) -> Option<&IndexedSymbol> {
        self.definition_with_hints(name, context_path, language, &[])
    }

    /// 使用当前文件解析出的 import/use/include 路径提示排序定义候选。
    pub fn definition_with_hints(
        &self,
        name: &str,
        context_path: Option<&str>,
        language: &str,
        hints: &[String],
    ) -> Option<&IndexedSymbol> {
        let typed = matches!(language, "Java" | "TypeScript" | "Rust");
        let context_dir = context_path.and_then(|path| {
            Path::new(path)
                .parent()
                .and_then(|parent| parent.to_str())
                .map(str::to_owned)
        });
        let mut candidates: Vec<&IndexedSymbol> = self
            .symbols
            .iter()
            .filter(|symbol| symbol.name == name)
            .collect();
        candidates.sort_by_key(|symbol| {
            let same_dir = context_dir.as_deref()
                == Path::new(&symbol.path)
                    .parent()
                    .and_then(|parent| parent.to_str());
            let hint_rank = u8::from(
                !hints
                    .iter()
                    .any(|hint| path_matches_hint(&symbol.path, hint)),
            );
            if typed {
                (
                    hint_rank,
                    u8::from(!same_dir),
                    typed_kind_rank(name, symbol.kind),
                    symbol.path.clone(),
                    symbol.start_line,
                )
            } else {
                let header = symbol.path.ends_with(".h")
                    || symbol.path.ends_with(".hpp")
                    || symbol.path.ends_with(".hh")
                    || symbol.path.ends_with(".hxx");
                (
                    hint_rank,
                    u8::from(!header),
                    u8::from(!same_dir),
                    symbol.path.clone(),
                    symbol.start_line,
                )
            }
        });
        candidates.first().copied()
    }

    pub fn search(&self, query: &str, symbol_limit: usize, file_limit: usize) -> SearchResults {
        self.search_filtered(query, symbol_limit, file_limit, false, "")
    }

    pub fn search_filtered(
        &self,
        query: &str,
        symbol_limit: usize,
        file_limit: usize,
        case_sensitive: bool,
        path_filter: &str,
    ) -> SearchResults {
        let normalized = if case_sensitive {
            query.trim().to_owned()
        } else {
            query.trim().to_lowercase()
        };
        if normalized.is_empty() {
            return SearchResults::default();
        }
        let path_filter = path_filter.trim();
        let path_filter = if case_sensitive {
            path_filter.to_owned()
        } else {
            path_filter.to_lowercase()
        };
        let matches = |text: &str| {
            if case_sensitive {
                text.contains(&normalized)
            } else {
                text.to_lowercase().contains(&normalized)
            }
        };
        let path_allowed = |path: &str| {
            path_filter.is_empty()
                || if case_sensitive {
                    path.contains(&path_filter)
                } else {
                    path.to_lowercase().contains(&path_filter)
                }
        };

        let mut symbols: Vec<(u8, usize, &IndexedSymbol)> = self
            .symbols
            .iter()
            .filter(|symbol| path_allowed(&symbol.path))
            .filter_map(|symbol| {
                let name = if case_sensitive {
                    symbol.name.clone()
                } else {
                    symbol.name.to_lowercase()
                };
                let rank = if name == normalized {
                    0
                } else if name.starts_with(&normalized) {
                    1
                } else if matches(&symbol.name) {
                    2
                } else {
                    return None;
                };
                Some((rank, symbol.name.len(), symbol))
            })
            .collect();
        symbols.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.path.cmp(&b.2.path))
                .then_with(|| a.2.start_line.cmp(&b.2.start_line))
        });

        let mut files: Vec<(u8, usize, &IndexedFile)> = self
            .files
            .iter()
            .filter(|file| path_allowed(&file.path))
            .filter_map(|file| {
                let name = if case_sensitive {
                    file.name.clone()
                } else {
                    file.name.to_lowercase()
                };
                let rank = if name == normalized {
                    0
                } else if name.starts_with(&normalized) {
                    1
                } else if matches(&file.name) {
                    2
                } else if matches(&file.path) {
                    3
                } else {
                    return None;
                };
                Some((rank, file.path.len(), file))
            })
            .collect();
        files.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.path.cmp(&b.2.path))
        });

        SearchResults {
            symbols: symbols
                .into_iter()
                .take(symbol_limit)
                .map(|(_, _, symbol)| symbol.clone())
                .collect(),
            files: files
                .into_iter()
                .take(file_limit)
                .map(|(_, _, file)| file.clone())
                .collect(),
        }
    }
}

fn collect_files(entries: &[SourceEntry], files: &mut Vec<IndexedFile>) {
    for entry in entries {
        if entry.is_dir {
            collect_files(&entry.children, files);
        } else {
            files.push(IndexedFile {
                name: entry.name.clone(),
                path: entry.rel_path.clone(),
            });
        }
    }
}

fn fingerprint(pack: &SourcePack) -> Result<String, String> {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    feed_hash(&mut hash, pack.name().as_bytes());
    feed_hash(&mut hash, pack.version().as_bytes());
    for path in pack.source_files()? {
        feed_hash(&mut hash, path.as_bytes());
        let absolute = pack.root().join(&path);
        if let Ok(metadata) = fs::metadata(absolute) {
            feed_hash(&mut hash, &metadata.len().to_le_bytes());
        }
    }
    Ok(format!("{hash:016x}"))
}

fn feed_hash(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

fn load_cache(path: &Path, fingerprint: &str) -> Option<SourceIndex> {
    let bytes = fs::read(path).ok()?;
    let cached: CachedIndex = postcard::from_bytes(&bytes).ok()?;
    if cached.version != CACHE_FORMAT_VERSION || cached.fingerprint != fingerprint {
        return None;
    }

    let symbols = cached
        .symbols
        .into_iter()
        .filter_map(|symbol| {
            symbol_kind_from_code(symbol.kind).map(|kind| IndexedSymbol {
                name: symbol.name,
                kind,
                path: symbol.path,
                start_line: symbol.start_line,
                end_line: symbol.end_line,
                signature: symbol.signature,
            })
        })
        .collect();

    Some(SourceIndex {
        symbols,
        files: cached.files,
    })
}

fn path_matches_hint(path: &str, hint: &str) -> bool {
    if hint.ends_with('/') {
        return path.contains(hint);
    }
    if path == hint {
        return true;
    }
    path.strip_suffix(hint)
        .is_some_and(|prefix| prefix.ends_with('/'))
}

fn typed_kind_rank(name: &str, kind: SymbolKind) -> u8 {
    let type_name = name
        .chars()
        .next()
        .is_some_and(|character| character.is_uppercase());
    match kind {
        SymbolKind::Class
        | SymbolKind::Interface
        | SymbolKind::Enum
        | SymbolKind::Record
        | SymbolKind::Annotation
        | SymbolKind::Struct
        | SymbolKind::Union
        | SymbolKind::Trait
        | SymbolKind::Type => {
            if type_name {
                0
            } else {
                3
            }
        }
        SymbolKind::Module | SymbolKind::Impl => {
            if type_name {
                0
            } else {
                2
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

fn symbol_kind_code(kind: SymbolKind) -> u8 {
    match kind {
        SymbolKind::Function => 0,
        SymbolKind::Struct => 1,
        SymbolKind::Union => 2,
        SymbolKind::Enum => 3,
        SymbolKind::Type => 4,
        SymbolKind::Macro => 5,
        SymbolKind::Class => 6,
        SymbolKind::Interface => 7,
        SymbolKind::Method => 8,
        SymbolKind::Constructor => 9,
        SymbolKind::Record => 10,
        SymbolKind::Annotation => 11,
        SymbolKind::Trait => 12,
        SymbolKind::Module => 13,
        SymbolKind::Constant => 14,
        SymbolKind::Impl => 15,
    }
}

fn symbol_kind_from_code(code: u8) -> Option<SymbolKind> {
    match code {
        0 => Some(SymbolKind::Function),
        1 => Some(SymbolKind::Struct),
        2 => Some(SymbolKind::Union),
        3 => Some(SymbolKind::Enum),
        4 => Some(SymbolKind::Type),
        5 => Some(SymbolKind::Macro),
        6 => Some(SymbolKind::Class),
        7 => Some(SymbolKind::Interface),
        8 => Some(SymbolKind::Method),
        9 => Some(SymbolKind::Constructor),
        10 => Some(SymbolKind::Record),
        11 => Some(SymbolKind::Annotation),
        12 => Some(SymbolKind::Trait),
        13 => Some(SymbolKind::Module),
        14 => Some(SymbolKind::Constant),
        15 => Some(SymbolKind::Impl),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_parser::SymbolKind;

    fn index() -> SourceIndex {
        let symbol = |name: &str, path: &str| IndexedSymbol {
            name: name.to_owned(),
            kind: SymbolKind::Function,
            path: path.to_owned(),
            start_line: 10,
            end_line: 20,
            signature: format!("int {name}(void)"),
        };
        SourceIndex {
            symbols: vec![
                symbol("list_resize", "Objects/listobject.c"),
                symbol("list_append", "Objects/listobject.c"),
                symbol("PyList_New", "Objects/listobject.c"),
            ],
            files: vec![
                IndexedFile {
                    name: "listobject.c".to_owned(),
                    path: "Objects/listobject.c".to_owned(),
                },
                IndexedFile {
                    name: "listobject.h".to_owned(),
                    path: "Include/listobject.h".to_owned(),
                },
            ],
        }
    }

    #[test]
    fn ranks_exact_and_prefix_symbol_matches_first() {
        let results = index().search("list_resize", 10, 10);
        assert_eq!(results.symbols[0].name, "list_resize");

        let results = index().search("list", 10, 10);
        assert!(results.symbols[0].name.starts_with("list_"));
        assert!(
            results
                .symbols
                .iter()
                .any(|symbol| symbol.name == "PyList_New")
        );
        assert!(results.files.iter().any(|file| file.name == "listobject.c"));
    }

    #[test]
    fn search_filters_by_case_and_path() {
        let results = index().search_filtered("PyList", 10, 10, true, "");
        assert_eq!(results.symbols.len(), 1);
        assert!(
            results
                .symbols
                .iter()
                .all(|symbol| symbol.name.contains("PyList"))
        );
        let results = index().search_filtered("pylist", 10, 10, true, "");
        assert!(results.symbols.is_empty());

        let results = index().search_filtered("list", 10, 10, false, "Include/");
        assert!(
            results
                .files
                .iter()
                .all(|file| file.path.starts_with("Include/"))
        );
        assert!(
            results
                .symbols
                .iter()
                .all(|symbol| symbol.path.starts_with("Include/"))
        );
    }

    #[test]
    fn java_definition_prefers_same_package_and_type_kind() {
        let symbol = |name: &str, kind: SymbolKind, path: &str, line: u32| IndexedSymbol {
            name: name.to_owned(),
            kind,
            path: path.to_owned(),
            start_line: line,
            end_line: line + 10,
            signature: String::new(),
        };
        let index = SourceIndex {
            symbols: vec![
                symbol(
                    "size",
                    SymbolKind::Method,
                    "src/java.base/share/classes/java/lang/String.java",
                    100,
                ),
                symbol(
                    "size",
                    SymbolKind::Method,
                    "src/java.base/share/classes/java/util/ArrayList.java",
                    200,
                ),
                symbol(
                    "ArrayList",
                    SymbolKind::Class,
                    "src/java.base/share/classes/java/util/ArrayList.java",
                    40,
                ),
            ],
            files: Vec::new(),
        };
        let context = Some("src/java.base/share/classes/java/util/HashMap.java");

        let method = index
            .definition_from("size", context, "Java")
            .expect("method definition");
        assert!(method.path.ends_with("util/ArrayList.java"));

        let class = index
            .definition_from("ArrayList", context, "Java")
            .expect("class definition");
        assert_eq!(class.kind, SymbolKind::Class);
    }

    #[test]
    fn import_hints_prefer_the_imported_module() {
        let symbol = |name: &str, path: &str| IndexedSymbol {
            name: name.to_owned(),
            kind: SymbolKind::Function,
            path: path.to_owned(),
            start_line: 10,
            end_line: 20,
            signature: String::new(),
        };
        let index = SourceIndex {
            symbols: vec![
                symbol("parse", "src/compiler/checker.ts"),
                symbol("parse", "src/compiler/parser.ts"),
                symbol("parse", "src/compiler/program.ts"),
            ],
            files: Vec::new(),
        };
        let found = index
            .definition_with_hints(
                "parse",
                Some("src/compiler/checker.ts"),
                "TypeScript",
                &["src/compiler/parser.ts".to_owned()],
            )
            .expect("definition");
        assert_eq!(found.path, "src/compiler/parser.ts");
    }

    #[test]
    fn definition_prefers_headers() {
        let mut source = index();
        source.symbols.push(IndexedSymbol {
            name: "list_append".to_owned(),
            kind: SymbolKind::Function,
            path: "Include/listobject.h".to_owned(),
            start_line: 12,
            end_line: 12,
            signature: String::new(),
        });
        let definition = source.definition("list_append").expect("definition");
        assert!(definition.path.ends_with(".h"));
    }

    #[test]
    fn cache_round_trip_and_fingerprint_validation() {
        let path = std::env::temp_dir().join(format!(
            "lattice-index-cache-{}.postcard",
            std::process::id()
        ));
        let source = index();
        source
            .write_cache(&path, "fingerprint-a")
            .expect("write cache");

        let loaded = load_cache(&path, "fingerprint-a").expect("load cache");
        assert_eq!(loaded.symbol_count(), source.symbol_count());
        assert_eq!(loaded.file_count(), source.file_count());
        assert_eq!(
            loaded.search("list_resize", 5, 5).symbols[0].name,
            "list_resize"
        );
        assert!(load_cache(&path, "fingerprint-b").is_none());

        std::fs::remove_file(path).expect("cleanup cache");
    }

    #[test]
    fn cancelled_index_build_reports_none_and_keeps_progress() {
        let root =
            std::env::temp_dir().join(format!("lattice-index-cancel-{}", std::process::id()));
        fs::create_dir_all(root.join(".lattice")).expect("create manifest dir");
        fs::write(
            root.join(".lattice/pack.json"),
            r#"{
                "id": "index-cancel",
                "name": "Index Cancel",
                "implementation": "Demo",
                "version": "1",
                "language": "Demo",
                "source_language": "C",
                "license": "MIT",
                "default_entry": "main.c"
            }"#,
        )
        .expect("write manifest");
        fs::write(root.join("main.c"), "int main(void) { return 0; }\n").expect("write source");
        let pack = SourcePack::open(root.clone()).expect("open pack");
        let tree = pack.scan().expect("scan");

        let control = IndexControl::new();
        control.cancel();
        assert!(
            SourceIndex::build_with_control(&pack, &tree, &control)
                .expect("build")
                .is_none()
        );
        assert!(control.is_cancelled());

        let control = IndexControl::new();
        let built = SourceIndex::build_with_control(&pack, &tree, &control)
            .expect("build")
            .expect("index");
        assert_eq!(control.files_total(), 1);
        assert_eq!(control.files_done(), 1);
        assert!(built.symbol_count() > 0);

        fs::remove_dir_all(root).expect("cleanup");
    }
}
