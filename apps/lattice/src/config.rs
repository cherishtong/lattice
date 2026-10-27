use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

const SESSION_FILE: &str = "session.json";
static STARTUP_SESSION: OnceLock<SessionConfig> = OnceLock::new();

/// 本地会话：只记录阅读位置与界面偏好，不写入源码包。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionConfig {
    pub theme: Option<String>,
    pub pack_root: Option<String>,
    pub tabs: Vec<String>,
    pub active_tab: usize,
    pub positions: BTreeMap<String, u32>,
    pub expanded: Vec<String>,
    pub folds: BTreeMap<String, Vec<u32>>,
    pub recent_searches: Vec<String>,
    #[serde(default = "default_zoom")]
    pub zoom: f64,
    pub window: Option<WindowGeometry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

fn default_zoom() -> f64 {
    1.0
}

impl SessionConfig {
    /// 在启动 UI 之前读取一次，避免把文件 I/O 放进渲染线程。
    pub fn initialize() {
        let _ = STARTUP_SESSION.set(Self::load());
    }

    /// 返回启动时缓存的会话，未初始化时返回空会话。
    pub fn startup() -> &'static Self {
        STARTUP_SESSION.get_or_init(Self::load)
    }

    pub fn load() -> Self {
        let Some(path) = session_path() else {
            return Self::default();
        };
        Self::load_from(&path).unwrap_or_default()
    }

    pub fn load_from(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| format!("读取会话配置失败: {error}"))?;
        let text =
            String::from_utf8(bytes).map_err(|error| format!("会话配置不是 UTF-8: {error}"))?;
        serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|error| format!("解析会话配置失败: {error}"))
    }

    pub fn save(&self) -> Result<(), String> {
        let Some(path) = session_path() else {
            return Err("无法确定会话配置目录".to_owned());
        };
        self.save_to(&path)
    }

    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("创建配置目录失败: {error}"))?;
        }
        let data = serde_json::to_vec_pretty(self)
            .map_err(|error| format!("序列化会话配置失败: {error}"))?;
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, data).map_err(|error| format!("写入会话配置失败: {error}"))?;
        if path.exists() {
            fs::remove_file(path).map_err(|error| format!("替换会话配置失败: {error}"))?;
        }
        fs::rename(&temporary, path).map_err(|error| format!("提交会话配置失败: {error}"))
    }
}

fn session_path() -> Option<PathBuf> {
    session_dir().map(|directory| directory.join(SESSION_FILE))
}

fn session_dir() -> Option<PathBuf> {
    if let Some(directory) = std::env::var_os("LATTICE_CONFIG_DIR") {
        return Some(PathBuf::from(directory));
    }

    #[cfg(windows)]
    {
        if let Some(app_data) = std::env::var_os("APPDATA") {
            return Some(PathBuf::from(app_data).join("Lattice"));
        }
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            return Some(PathBuf::from(local_app_data).join("Lattice"));
        }
        std::env::var_os("USERPROFILE").map(|profile| PathBuf::from(profile).join(".lattice"))
    }

    #[cfg(not(windows))]
    {
        if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(config_home).join("lattice"));
        }
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config").join("lattice"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_config_round_trips_on_disk() {
        let root =
            std::env::temp_dir().join(format!("lattice-session-test-{}", std::process::id()));
        let path = root.join("session.json");
        let _ = fs::remove_dir_all(&root);

        let config = SessionConfig {
            theme: Some("midnight".to_owned()),
            pack_root: Some("E:/sources/cpython".to_owned()),
            tabs: vec![
                "Objects/listobject.c".to_owned(),
                "Include/Python.h".to_owned(),
            ],
            active_tab: 1,
            positions: BTreeMap::from([
                ("Objects/listobject.c".to_owned(), 120),
                ("Include/Python.h".to_owned(), 8),
            ]),
            expanded: vec!["Include".to_owned(), "Objects".to_owned()],
            folds: BTreeMap::from([("Objects/listobject.c".to_owned(), vec![113, 258])]),
            recent_searches: vec!["list_resize".to_owned(), "PyList_New".to_owned()],
            zoom: 1.25,
            window: Some(WindowGeometry {
                x: 80,
                y: 60,
                width: 1520,
                height: 940,
                maximized: false,
            }),
        };
        config.save_to(&path).expect("save session");
        assert_eq!(
            SessionConfig::load_from(&path).expect("load session"),
            config
        );

        let missing = SessionConfig::load_from(&root.join("missing.json"));
        assert!(missing.is_err());
        let _ = fs::remove_dir_all(&root);
    }
}
