//! 读取本地不可变 Source Pack。
//!
//! 当前垂直切片使用 CPython 3.13.2：源码位于 `.content/cpython-3.13.2`。
//! 内容包不进入 Git，应用只读，不修改上游源码。

use std::env;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

mod zip_support;
use zip_support::extract_zip;

const PACK_DIR: &str = "cpython-3.13.2";
const MAX_DOCUMENT_BYTES: u64 = 4 * 1024 * 1024;

pub const INSTALL_CANCELLED: &str = "导入已取消";

/// 复制阶段的可取消控制与进度计数，可跨线程读取。
#[derive(Clone, Default, Debug)]
pub struct InstallControl {
    inner: Arc<InstallControlInner>,
}

#[derive(Default, Debug)]
struct InstallControlInner {
    copied: AtomicU64,
    total: AtomicU64,
    cancelled: AtomicBool,
}

impl InstallControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Relaxed)
    }

    pub fn copied_files(&self) -> u64 {
        self.inner.copied.load(Ordering::Relaxed)
    }

    pub fn total_files(&self) -> u64 {
        self.inner.total.load(Ordering::Relaxed)
    }

    pub(crate) fn set_total(&self, total: u64) {
        self.inner.total.store(total, Ordering::Relaxed);
    }

    pub(crate) fn record_copy(&self) {
        self.inner.copied.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackManifest {
    pub id: String,
    pub name: String,
    pub implementation: String,
    pub version: String,
    pub language: String,
    pub source_language: String,
    pub license: String,
    pub default_entry: String,
    #[serde(default)]
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePack {
    root: PathBuf,
    manifest: PackManifest,
}

impl SourcePack {
    /// 从环境变量或仓库/当前目录的 `.content` 中发现默认源码包。
    pub fn discover() -> Result<Self, String> {
        if let Some(path) = env::var_os("LATTICE_SOURCE_PACK") {
            let root = PathBuf::from(path);
            return Self::open(root)
                .map_err(|error| format!("LATTICE_SOURCE_PACK 指向的目录不是有效源码包: {error}"));
        }

        for base in search_bases() {
            for ancestor in base.ancestors().take(6) {
                let candidate = ancestor.join(".content").join(PACK_DIR);
                if candidate.exists() {
                    return Self::open(candidate);
                }
            }
        }

        if let Ok(packs) = Self::installed()
            && let Some(pack) = packs.into_iter().next()
        {
            return Ok(pack);
        }

        Err("未找到可用的本地源码包。请先运行 scripts/fetch-cpython-pack.ps1 或 scripts/fetch-openjdk-pack.ps1".to_owned())
    }

    /// 列出可发现的本地源码包，返回可切换的书库条目。
    pub fn installed() -> Result<Vec<Self>, String> {
        let mut roots = Vec::new();
        if let Some(path) = env::var_os("LATTICE_SOURCE_PACK") {
            roots.push(PathBuf::from(path));
        }
        for base in search_bases() {
            for ancestor in base.ancestors().take(6) {
                let content = ancestor.join(".content");
                let Ok(entries) = fs::read_dir(&content) else {
                    continue;
                };
                for entry in entries.flatten() {
                    if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                        roots.push(entry.path());
                    }
                }
                if !roots.is_empty() {
                    break;
                }
            }
            if !roots.is_empty() {
                break;
            }
        }

        let mut packs: Vec<Self> = roots
            .into_iter()
            .filter_map(|root| Self::open(root).ok())
            .collect();
        packs.sort_by(|a, b| {
            b.manifest
                .version
                .cmp(&a.manifest.version)
                .then_with(|| a.manifest.name.cmp(&b.manifest.name))
        });
        packs.dedup_by(|a, b| a.root == b.root);
        Ok(packs)
    }

    /// 打开一个已存在的 Source Pack，优先读取 `.lattice/pack.json`。
    pub fn open(root: PathBuf) -> Result<Self, String> {
        if !root.is_dir() {
            return Err(format!("源码包目录不存在: {}", root.display()));
        }

        let manifest_path = root.join(".lattice").join("pack.json");
        let manifest = if manifest_path.is_file() {
            let bytes =
                fs::read(&manifest_path).map_err(|error| format!("读取包清单失败: {error}"))?;
            let text =
                String::from_utf8(bytes).map_err(|error| format!("包清单不是 UTF-8: {error}"))?;
            serde_json::from_str(text.trim_start_matches('\u{feff}'))
                .map_err(|error| format!("解析包清单失败: {error}"))?
        } else {
            infer_manifest(&root)?
        };

        if !root.join(&manifest.default_entry).is_file() {
            return Err(format!("源码包缺少默认入口: {}", manifest.default_entry));
        }

        Ok(Self { root, manifest })
    }

    pub fn manifest(&self) -> &PackManifest {
        &self.manifest
    }

    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    pub fn implementation(&self) -> &str {
        &self.manifest.implementation
    }

    pub fn language(&self) -> &str {
        &self.manifest.language
    }

    pub fn source_language(&self) -> &str {
        &self.manifest.source_language
    }

    pub fn implemented_language(&self) -> &str {
        &self.manifest.language
    }

    pub fn version(&self) -> &str {
        &self.manifest.version
    }

    pub fn default_entry(&self) -> &str {
        &self.manifest.default_entry
    }

    pub fn license(&self) -> &str {
        &self.manifest.license
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 按清单中的可选校验和验证当前源码包。
    pub fn verify_integrity(&self) -> Result<(), String> {
        let Some(expected) = self.manifest.checksum.as_deref() else {
            return Ok(());
        };
        let actual = compute_checksum(&self.root)?;
        if actual == expected {
            Ok(())
        } else {
            Err(format!(
                "源码包校验和不匹配：期望 {expected}，实际 {actual}"
            ))
        }
    }

    /// 计算并写回清单中的校验和，供打包脚本使用。
    pub fn stamp_checksum(&self) -> Result<String, String> {
        let checksum = compute_checksum(&self.root)?;
        let mut manifest = self.manifest.clone();
        manifest.checksum = Some(checksum.clone());
        let directory = self.root.join(".lattice");
        fs::create_dir_all(&directory).map_err(|error| format!("创建缓存目录失败: {error}"))?;
        let data = serde_json::to_vec_pretty(&manifest)
            .map_err(|error| format!("序列化包清单失败: {error}"))?;
        fs::write(directory.join("pack.json"), data)
            .map_err(|error| format!("写入包清单失败: {error}"))?;
        Ok(checksum)
    }

    /// 将外部源码目录安装到最近的 `.content`，并返回安装后的只读包。
    pub fn install_from(source: &Path) -> Result<Self, String> {
        Self::install_from_with(source, &InstallControl::new())
    }

    /// 从 ZIP 压缩包导入源码包，自动识别压缩包内的顶层目录。
    pub fn install_zip_with(zip_path: &Path, control: &InstallControl) -> Result<Self, String> {
        let root = content_root()?;
        fs::create_dir_all(&root).map_err(|error| format!("创建源码包目录失败: {error}"))?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let staging = root.join(format!(".import-{}-{stamp}", std::process::id()));
        if let Err(error) = fs::create_dir_all(&staging) {
            return Err(format!("创建临时目录失败: {error}"));
        }
        if let Err(error) = extract_zip(zip_path, &staging, control) {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
        let candidate = if Self::open(staging.clone()).is_ok() {
            staging.clone()
        } else {
            let child = fs::read_dir(&staging).ok().and_then(|entries| {
                entries
                    .flatten()
                    .map(|entry| entry.path())
                    .find(|path| path.is_dir() && Self::open(path.clone()).is_ok())
            });
            match child {
                Some(child) => child,
                None => {
                    let _ = fs::remove_dir_all(&staging);
                    return Err("ZIP 中没有可识别的源码包".to_owned());
                }
            }
        };
        let pack = match Self::open(candidate.clone()) {
            Ok(pack) => pack,
            Err(error) => {
                let _ = fs::remove_dir_all(&staging);
                return Err(error);
            }
        };
        if let Err(error) = pack.verify_integrity() {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
        let destination = root.join(install_directory_name(&pack));
        if destination.exists() {
            let _ = fs::remove_dir_all(&staging);
            return Err(format!("已存在同名源码包：{}", destination.display()));
        }
        if control.is_cancelled() {
            let _ = fs::remove_dir_all(&staging);
            return Err(INSTALL_CANCELLED.to_owned());
        }
        if let Err(error) = fs::rename(&candidate, &destination) {
            let _ = fs::remove_dir_all(&staging);
            return Err(format!("提交源码包失败: {error}"));
        }
        if staging != candidate {
            let _ = fs::remove_dir_all(&staging);
        }
        Self::open(destination)
    }

    /// 带复制进度与取消能力的安装入口。
    pub fn install_from_with(source: &Path, control: &InstallControl) -> Result<Self, String> {
        let pack = Self::open(source.to_path_buf())?;
        let root = content_root()?;
        let name = install_directory_name(&pack);
        let destination = root.join(name);
        if destination.exists() {
            let same = source
                .canonicalize()
                .ok()
                .zip(destination.canonicalize().ok())
                .is_some_and(|(source, destination)| source == destination);
            if same {
                return Ok(pack);
            }
            return Err(format!("已存在同名源码包：{}", destination.display()));
        }
        Self::install_to_with(source, &destination, control)
    }

    /// 把已验证的源码包复制到目标目录，跳过 `.lattice`、`.git` 缓存与元数据。
    pub fn install_to(source: &Path, destination: &Path) -> Result<Self, String> {
        Self::install_to_with(source, destination, &InstallControl::new())
    }

    pub fn install_to_with(
        source: &Path,
        destination: &Path,
        control: &InstallControl,
    ) -> Result<Self, String> {
        let source_pack = Self::open(source.to_path_buf())?;
        source_pack.verify_integrity()?;
        if destination.exists() {
            return Err(format!("目标目录已存在：{}", destination.display()));
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("创建源码包目录失败: {error}"))?;
        }
        control.set_total(count_files(source, 0).unwrap_or_default());
        if let Err(error) = copy_tree(source, destination, 0, control) {
            let _ = fs::remove_dir_all(destination);
            if control.is_cancelled() {
                return Err(INSTALL_CANCELLED.to_owned());
            }
            return Err(format!("复制源码包失败: {error}"));
        }
        if control.is_cancelled() {
            let _ = fs::remove_dir_all(destination);
            return Err(INSTALL_CANCELLED.to_owned());
        }
        match Self::open(destination.to_path_buf()) {
            Ok(pack) => Ok(pack),
            Err(error) => {
                let _ = fs::remove_dir_all(destination);
                Err(format!("安装后的源码包无效: {error}"))
            }
        }
    }

    /// 只允许删除 `.content` 内的源码包；外部目录不会被触碰。
    pub fn uninstall(&self) -> Result<(), String> {
        let content = content_root()?;
        self.uninstall_within(&content)
    }

    fn uninstall_within(&self, content: &Path) -> Result<(), String> {
        let content = content
            .canonicalize()
            .map_err(|error| format!("无法定位源码包根目录: {error}"))?;
        let root = self
            .root
            .canonicalize()
            .map_err(|error| format!("无法定位源码包: {error}"))?;
        if !root.starts_with(&content) || root == content {
            return Err("只能卸载 .content 内的源码包".to_owned());
        }
        fs::remove_dir_all(&root).map_err(|error| format!("卸载源码包失败: {error}"))
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.root.join(".lattice")
    }

    pub fn document_cache_path(&self, rel_path: &str) -> PathBuf {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for byte in rel_path.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self.cache_dir()
            .join("documents")
            .join(format!("{hash:016x}.postcard"))
    }

    /// 扫描文件树。只渲染展开的节点，因此一次扫描 5k 文件是可接受的。
    pub fn scan(&self) -> Result<SourceTree, String> {
        let mut file_count = 0;
        let entries = scan_dir(&self.root, "", 0, &mut file_count)?;
        Ok(SourceTree {
            entries,
            file_count,
        })
    }

    /// 只列出当前源码包语言的源码文件，供索引与定义查找使用。
    pub fn source_files(&self) -> Result<Vec<String>, String> {
        let extensions: &[&str] = match self.manifest.source_language.as_str() {
            "Java" => &["java"],
            "JavaScript" => &["js", "jsx", "mjs", "cjs"],
            "Rust" => &["rs"],
            "TypeScript" => &["ts", "tsx", "mts", "cts"],
            _ => &["c", "h", "cc", "cpp", "cxx", "c++", "hpp", "hh", "hxx", "m"],
        };
        let mut files = Vec::new();
        collect_source_files(&self.root, "", 0, extensions, &mut files)?;
        files.sort_by_key(|path| {
            let is_header = path.ends_with(".h")
                || path.ends_with(".hpp")
                || path.ends_with(".hh")
                || path.ends_with(".hxx");
            let in_include = path.starts_with("Include/");
            (!in_include, !is_header, path.clone())
        });
        Ok(files)
    }

    /// 只列出 C 系源码文件。
    pub fn c_source_files(&self) -> Result<Vec<String>, String> {
        let mut files = self.source_files()?;
        files.retain(|path| {
            matches!(
                Path::new(path)
                    .extension()
                    .and_then(|extension| extension.to_str()),
                Some("c" | "h" | "cc" | "cpp" | "cxx" | "c++" | "hpp" | "hh" | "hxx" | "m")
            )
        });
        Ok(files)
    }

    /// 只允许读取包内相对路径，拒绝绝对路径与 `..`。
    pub fn read_text(&self, rel_path: &str) -> Result<String, String> {
        let relative = Path::new(rel_path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
        {
            return Err("非法源码路径".to_owned());
        }

        let path = self.root.join(relative);
        let metadata = fs::metadata(&path).map_err(|error| format!("读取文件失败: {error}"))?;
        if !metadata.is_file() {
            return Err("目标不是文件".to_owned());
        }
        if metadata.len() > MAX_DOCUMENT_BYTES {
            return Err(format!(
                "文件超过 {} MiB，当前阅读器暂不打开",
                MAX_DOCUMENT_BYTES / 1024 / 1024
            ));
        }

        let bytes = fs::read(&path).map_err(|error| format!("读取文件失败: {error}"))?;
        String::from_utf8(bytes).map_err(|_| "文件不是 UTF-8 文本，暂不支持阅读".to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceTree {
    pub entries: Vec<SourceEntry>,
    pub file_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEntry {
    pub name: String,
    pub rel_path: String,
    pub is_dir: bool,
    pub children: Vec<SourceEntry>,
}

impl SourceEntry {
    pub fn extension(&self) -> Option<&str> {
        Path::new(&self.name)
            .extension()
            .and_then(|extension| extension.to_str())
    }
}

fn search_bases() -> Vec<PathBuf> {
    let mut bases = Vec::new();
    if let Ok(cwd) = env::current_dir() {
        bases.push(cwd);
    }
    if let Ok(exe) = env::current_exe()
        && let Some(parent) = exe.parent()
    {
        bases.push(parent.to_path_buf());
    }
    bases
}

fn content_root() -> Result<PathBuf, String> {
    for base in search_bases() {
        for ancestor in base.ancestors().take(6) {
            let candidate = ancestor.join(".content");
            if candidate.is_dir() {
                return Ok(candidate);
            }
        }
    }
    let cwd = env::current_dir().map_err(|error| format!("无法定位工作目录: {error}"))?;
    Ok(cwd.join(".content"))
}

fn install_directory_name(pack: &SourcePack) -> String {
    let candidate = if pack.manifest.id.trim().is_empty() {
        pack.root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("source-pack")
    } else {
        pack.manifest.id.trim()
    };
    let sanitized: String = candidate
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect();
    let sanitized = sanitized.trim_matches('-').to_owned();
    if sanitized.is_empty() {
        "source-pack".to_owned()
    } else {
        sanitized
    }
}

fn checksum_tree(
    root: &Path,
    relative: &Path,
    hasher: &mut crc32fast::Hasher,
    depth: usize,
) -> Result<(), String> {
    if depth > 32 {
        return Ok(());
    }
    let directory = root.join(relative);
    let mut entries: Vec<_> = fs::read_dir(&directory)
        .map_err(|error| format!("读取目录失败 {}: {error}", directory.display()))?
        .flatten()
        .collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if name_text == ".git" || name_text == ".lattice" {
            continue;
        }
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_symlink() {
            continue;
        }
        let child = relative.join(&name);
        if file_type.is_dir() {
            checksum_tree(root, &child, hasher, depth + 1)?;
        } else if file_type.is_file() {
            let path_text = child.to_string_lossy().replace('\\', "/");
            hasher.update(path_text.as_bytes());
            let mut file = fs::File::open(entry.path()).map_err(|error| error.to_string())?;
            let mut buffer = vec![0u8; 64 * 1024];
            loop {
                let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
        }
    }
    Ok(())
}

/// 计算源码包内容的确定性校验和，忽略 `.lattice`、`.git` 目录。
pub fn compute_checksum(root: &Path) -> Result<String, String> {
    let mut hasher = crc32fast::Hasher::new();
    checksum_tree(root, Path::new(""), &mut hasher, 0)?;
    Ok(format!("crc32:{:08x}", hasher.finalize()))
}

fn copy_tree(
    source: &Path,
    destination: &Path,
    depth: usize,
    control: &InstallControl,
) -> Result<(), String> {
    if depth > 32 {
        return Err("源码包目录层级过深".to_owned());
    }
    if control.is_cancelled() {
        return Err(INSTALL_CANCELLED.to_owned());
    }
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    let entries = fs::read_dir(source).map_err(|error| error.to_string())?;
    for entry in entries {
        if control.is_cancelled() {
            return Err(INSTALL_CANCELLED.to_owned());
        }
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if name_text == ".lattice" {
            let manifest = entry.path().join("pack.json");
            if manifest.is_file() {
                let manifest_dir = destination.join(".lattice");
                fs::create_dir_all(&manifest_dir).map_err(|error| error.to_string())?;
                fs::copy(&manifest, manifest_dir.join("pack.json"))
                    .map_err(|error| error.to_string())?;
            }
            continue;
        }
        if name_text == ".git" {
            continue;
        }
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_symlink() {
            continue;
        }
        let target = destination.join(&name);
        if file_type.is_dir() {
            copy_tree(&entry.path(), &target, depth + 1, control)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target).map_err(|error| error.to_string())?;
            control.record_copy();
        }
    }
    Ok(())
}

fn count_files(source: &Path, depth: usize) -> Result<u64, String> {
    if depth > 32 {
        return Ok(0);
    }
    let entries = fs::read_dir(source).map_err(|error| error.to_string())?;
    let mut total = 0;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if name_text == ".git" {
            continue;
        }
        if name_text == ".lattice" {
            if entry.path().join("pack.json").is_file() {
                total += 1;
            }
            continue;
        }
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            total += count_files(&entry.path(), depth + 1)?;
        } else if file_type.is_file() {
            total += 1;
        }
    }
    Ok(total)
}

fn infer_manifest(root: &Path) -> Result<PackManifest, String> {
    let dir_name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let version = dir_name
        .strip_prefix("cpython-")
        .ok_or_else(|| "源码包缺少 .lattice/pack.json 清单".to_owned())?
        .to_owned();
    if !root.join("Objects/listobject.c").is_file() {
        return Err("无法识别的源码包布局".to_owned());
    }
    Ok(PackManifest {
        id: dir_name.to_owned(),
        name: "CPython".to_owned(),
        implementation: "CPython".to_owned(),
        version,
        language: "Python".to_owned(),
        source_language: "C".to_owned(),
        license: "PSF-2.0".to_owned(),
        default_entry: "Objects/listobject.c".to_owned(),
        checksum: None,
    })
}

fn scan_dir(
    root: &Path,
    rel: &str,
    depth: usize,
    file_count: &mut usize,
) -> Result<Vec<SourceEntry>, String> {
    if depth > 24 {
        return Ok(Vec::new());
    }

    let dir = if rel.is_empty() {
        root.to_path_buf()
    } else {
        root.join(rel)
    };
    let read_dir = fs::read_dir(&dir).map_err(|error| format!("读取目录失败: {error}"))?;
    let mut entries = Vec::new();

    for entry in read_dir {
        let entry = entry.map_err(|error| format!("读取目录项失败: {error}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }

        let file_type = entry
            .file_type()
            .map_err(|error| format!("读取文件类型失败: {error}"))?;
        if file_type.is_symlink() {
            continue;
        }

        let child_rel = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };

        if file_type.is_dir() {
            let children = scan_dir(root, &child_rel, depth + 1, file_count)?;
            entries.push(SourceEntry {
                name,
                rel_path: child_rel,
                is_dir: true,
                children,
            });
        } else if file_type.is_file() {
            *file_count += 1;
            entries.push(SourceEntry {
                name,
                rel_path: child_rel,
                is_dir: false,
                children: Vec::new(),
            });
        }
    }

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

fn collect_source_files(
    root: &Path,
    rel: &str,
    depth: usize,
    extensions: &[&str],
    files: &mut Vec<String>,
) -> Result<(), String> {
    if depth > 24 || extensions.is_empty() {
        return Ok(());
    }
    let dir = if rel.is_empty() {
        root.to_path_buf()
    } else {
        root.join(rel)
    };
    let read_dir = fs::read_dir(&dir).map_err(|error| format!("读取目录失败: {error}"))?;

    for entry in read_dir {
        let entry = entry.map_err(|error| format!("读取目录项失败: {error}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let file_type = entry
            .file_type()
            .map_err(|error| format!("读取文件类型失败: {error}"))?;
        if file_type.is_symlink() {
            continue;
        }
        let child_rel = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        if file_type.is_dir() {
            collect_source_files(root, &child_rel, depth + 1, extensions, files)?;
        } else if file_type.is_file() {
            let extension = Path::new(&name)
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default();
            if extensions.contains(&extension) {
                files.push(child_rel);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_pack(root: PathBuf) -> SourcePack {
        SourcePack {
            root,
            manifest: PackManifest {
                id: "test".to_owned(),
                name: "CPython".to_owned(),
                implementation: "CPython".to_owned(),
                version: "3.13.2".to_owned(),
                language: "Python".to_owned(),
                source_language: "C".to_owned(),
                license: "PSF-2.0".to_owned(),
                default_entry: "Objects/listobject.c".to_owned(),
                checksum: None,
            },
        }
    }

    #[test]
    fn rejects_parent_directory_traversal() {
        let pack = fixture_pack(PathBuf::from("."));
        assert!(pack.read_text("../secret.txt").is_err());
        assert!(pack.read_text("Objects/../../secret.txt").is_err());
    }

    #[test]
    fn scans_directories_and_reads_relative_files() {
        let root = env::temp_dir().join(format!("lattice-content-test-{}", std::process::id()));
        let object_dir = root.join("Objects");
        fs::create_dir_all(&object_dir).expect("create fixture");
        fs::write(object_dir.join("listobject.c"), "int value;\n").expect("write fixture");
        fs::write(root.join(".hidden"), "hidden").expect("write fixture");

        let pack = fixture_pack(root.clone());
        let tree = pack.scan().expect("scan");
        assert_eq!(tree.file_count, 1);
        assert_eq!(tree.entries.len(), 1);
        assert_eq!(tree.entries[0].name, "Objects");
        assert!(tree.entries[0].is_dir);
        assert_eq!(
            pack.read_text("Objects/listobject.c").expect("read"),
            "int value;\n"
        );

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn definition_scan_prefers_include_headers() {
        let root = env::temp_dir().join(format!("lattice-content-defs-{}", std::process::id()));
        fs::create_dir_all(root.join("Include")).expect("create fixture");
        fs::create_dir_all(root.join("Objects")).expect("create fixture");
        fs::write(root.join("Include/listobject.h"), "int value;\n").expect("write fixture");
        fs::write(root.join("Objects/listobject.c"), "int value;\n").expect("write fixture");
        fs::write(root.join("README.rst"), "not C\n").expect("write fixture");

        let pack = fixture_pack(root.clone());
        let files = pack.c_source_files().expect("scan");
        assert_eq!(files[0], "Include/listobject.h");
        assert!(files.contains(&"Objects/listobject.c".to_owned()));
        assert!(!files.iter().any(|path| path.ends_with(".rst")));

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn selects_source_files_for_rust_and_typescript_packs() {
        let root = env::temp_dir().join(format!("lattice-lang-scan-{}", std::process::id()));
        fs::create_dir_all(root.join("src")).expect("create fixture");
        fs::write(root.join("src/main.rs"), "fn main() {}\n").expect("write rust");
        fs::write(root.join("src/index.ts"), "export const x = 1;\n").expect("write ts");
        fs::write(
            root.join("src/view.tsx"),
            "export const View = () => null;\n",
        )
        .expect("write tsx");
        fs::write(root.join("src/legacy.js"), "var x = 1;\n").expect("write js");

        let mut pack = fixture_pack(root.clone());
        pack.manifest.source_language = "Rust".to_owned();
        assert_eq!(
            pack.source_files().expect("rust files"),
            vec!["src/main.rs"]
        );

        pack.manifest.source_language = "TypeScript".to_owned();
        let files = pack.source_files().expect("typescript files");
        assert!(files.contains(&"src/index.ts".to_owned()));
        assert!(files.contains(&"src/view.tsx".to_owned()));
        assert!(!files.contains(&"src/legacy.js".to_owned()));

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn installs_pack_and_keeps_manifest_without_caches() {
        let root = env::temp_dir().join(format!("lattice-install-test-{}", std::process::id()));
        let source = root.join("source");
        let destination = root.join("installed");
        fs::create_dir_all(source.join(".lattice/documents")).expect("create manifest dir");
        fs::create_dir_all(source.join("Objects")).expect("create source dir");
        fs::write(
            source.join(".lattice/pack.json"),
            r#"{
                "id": "installed",
                "name": "Demo",
                "implementation": "Demo",
                "version": "1",
                "language": "Demo",
                "source_language": "C",
                "license": "MIT",
                "default_entry": "Objects/listobject.c"
            }"#,
        )
        .expect("write manifest");
        fs::write(source.join(".lattice/documents/cache.bin"), "cache").expect("write cache");
        fs::write(source.join("Objects/listobject.c"), "int value;\n").expect("write source");

        let pack = SourcePack::install_to(&source, &destination).expect("install");
        assert_eq!(pack.name(), "Demo");
        assert!(destination.join("Objects/listobject.c").is_file());
        assert!(destination.join(".lattice/pack.json").is_file());
        assert!(!destination.join(".lattice/documents/cache.bin").exists());

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn cancelled_install_removes_partial_copy() {
        let root = env::temp_dir().join(format!("lattice-cancel-test-{}", std::process::id()));
        let source = root.join("source");
        let destination = root.join("installed");
        fs::create_dir_all(source.join(".lattice")).expect("create manifest dir");
        fs::create_dir_all(source.join("Objects")).expect("create source dir");
        fs::write(
            source.join(".lattice/pack.json"),
            r#"{
                "id": "cancelled",
                "name": "Cancelled",
                "implementation": "Demo",
                "version": "1",
                "language": "Demo",
                "source_language": "C",
                "license": "MIT",
                "default_entry": "Objects/listobject.c"
            }"#,
        )
        .expect("write manifest");
        fs::write(source.join("Objects/listobject.c"), "int value;\n").expect("write source");

        let control = InstallControl::new();
        control.cancel();
        let error = SourcePack::install_to_with(&source, &destination, &control)
            .expect_err("install must cancel");
        assert_eq!(error, INSTALL_CANCELLED);
        assert!(!destination.exists());

        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn checksum_changes_with_file_content() {
        let root = env::temp_dir().join(format!("lattice-checksum-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create");
        fs::write(root.join("a.c"), "int value;\n").expect("write");
        let first = compute_checksum(&root).expect("checksum");
        assert!(first.starts_with("crc32:"));
        fs::write(root.join("a.c"), "int other;\n").expect("write");
        let second = compute_checksum(&root).expect("checksum");
        assert_ne!(first, second);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn uninstall_only_removes_packs_inside_content_root() {
        let root = env::temp_dir().join(format!("lattice-uninstall-test-{}", std::process::id()));
        let content = root.join("content");
        let inside = content.join("demo");
        let outside = root.join("outside");
        fs::create_dir_all(&inside).expect("create inside pack");
        fs::create_dir_all(&outside).expect("create outside pack");
        fs::write(inside.join("source.c"), "int value;\n").expect("write inside");
        fs::write(outside.join("source.c"), "int value;\n").expect("write outside");

        fixture_pack(inside.clone())
            .uninstall_within(&content)
            .expect("uninstall inside");
        assert!(!inside.exists());

        let error = fixture_pack(outside.clone())
            .uninstall_within(&content)
            .expect_err("must reject outside");
        assert!(error.contains(".content"));
        assert!(outside.exists());

        fs::remove_dir_all(root).expect("cleanup");
    }
}
