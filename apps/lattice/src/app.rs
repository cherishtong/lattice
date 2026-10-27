use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use dioxus::prelude::*;

use crate::components::chrome::{DiffPanel, StatusBar, TitleBar};
use crate::components::code_view::CodeView;
use crate::components::guide::GuidePanel;
use crate::components::source_explorer::SourceExplorer;
use crate::components::translation::TranslationPanel;
use crate::config::SessionConfig;
#[cfg(all(feature = "desktop", not(feature = "native")))]
use crate::config::WindowGeometry;
use crate::source::SourceSession;
use crate::theme::Theme;
use crate::translation::TranslationSession;

static APP_CSS: &str = include_str!("../assets/app.css");
static THEME_CSS: &str = include_str!("../assets/themes.css");
static SESSION_SAVE_EPOCH: AtomicU64 = AtomicU64::new(0);

const MIN_ZOOM: f64 = 0.8;
const MAX_ZOOM: f64 = 2.0;
#[cfg(not(feature = "native"))]
const ZOOM_STEP: f64 = 0.1;

fn clamp_zoom(zoom: f64) -> f64 {
    (zoom.clamp(MIN_ZOOM, MAX_ZOOM) * 10.0).round() / 10.0
}

#[cfg(not(feature = "native"))]
const SHORTCUT_SCRIPT: &str = r#"
(() => {
  if (window.__latticeShortcutHandler) {
    window.removeEventListener('keydown', window.__latticeShortcutHandler, true);
  }
  if (window.__latticeWheelHandler) {
    window.removeEventListener('wheel', window.__latticeWheelHandler, true);
  }
  const handler = (event) => {
    const key = typeof event.key === 'string' ? event.key : '';
    const lower = key.toLowerCase();
    const target = event.target;
    const editing = target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable);
    let command = null;
    if ((event.ctrlKey || event.metaKey) && !event.altKey) {
      if (key === '=' || key === '+') command = 'zoom-in';
      else if (key === '-' || key === '_') command = 'zoom-out';
      else if (key === '0') command = 'zoom-reset';
      else if (key === '/') command = 'help';
      else if (lower === 'k' && !event.shiftKey) command = 'search';
      else if (lower === 'f' && !event.shiftKey) command = 'find';
      else if (key === 'Tab') command = event.shiftKey ? 'prev-tab' : 'next-tab';
      else if (lower === 'w' && !event.shiftKey) command = 'close-tab';
    } else if (key === '?' && !editing) {
      command = 'help';
    } else if (event.altKey && !event.ctrlKey && key === '[') {
      command = 'fold-current';
    } else if (event.altKey && !event.ctrlKey && key === ']') {
      command = 'unfold-all';
    } else if (event.altKey && event.shiftKey && key === '{') {
      command = 'fold-all';
    } else if (event.altKey && key === 'ArrowLeft') {
      command = 'back';
    } else if (event.altKey && key === 'ArrowRight') {
      command = 'forward';
    } else if (key === 'Escape') {
      command = 'escape';
    }
    if (!command) return;
    event.preventDefault();
    event.stopPropagation();
    dioxus.send(command);
  };
  window.__latticeShortcutHandler = handler;
  window.addEventListener('keydown', handler, true);
  if (window.__latticeSelectionHandler) {
    window.removeEventListener('mouseup', window.__latticeSelectionHandler, true);
  }
  const lineOf = (node) => {
    const element = node && (node.nodeType === 3 ? node.parentElement : node);
    const line = element && element.closest ? element.closest('.code-line') : null;
    return line ? parseInt(line.dataset.line || '0', 10) : 0;
  };
  const selectionHandler = () => {
    const selection = window.getSelection();
    if (!selection || selection.isCollapsed || selection.rangeCount === 0) return;
    const text = selection.toString().trim();
    if (!text) return;
    const start = lineOf(selection.anchorNode);
    const end = lineOf(selection.focusNode);
    if (!start && !end) return;
    dioxus.send(JSON.stringify({
      command: 'translate-selection',
      text,
      line: Math.min(start || end, end || start),
      end_line: Math.max(start || end, end || start),
    }));
  };
  window.__latticeSelectionHandler = selectionHandler;
  window.addEventListener('mouseup', selectionHandler, true);
  const wheelHandler = (event) => {
    if (!event.ctrlKey && !event.metaKey) return;
    event.preventDefault();
    event.stopPropagation();
    const now = performance.now();
    if (now - (window.__latticeLastZoom || 0) < 70) return;
    window.__latticeLastZoom = now;
    dioxus.send(event.deltaY < 0 ? 'zoom-in' : 'zoom-out');
  };
  window.__latticeWheelHandler = wheelHandler;
  window.addEventListener('wheel', wheelHandler, { capture: true, passive: false });
  dioxus.send('ready');
  new Promise(() => {});
})();
"#;

#[component]
pub fn App() -> Element {
    let startup = SessionConfig::startup();
    let initial_theme = startup
        .theme
        .as_deref()
        .and_then(Theme::from_id)
        .unwrap_or_default();
    let theme = use_signal(|| initial_theme);
    let theme_id = theme.read().id();
    let mut zoom = use_signal(|| clamp_zoom(startup.zoom));
    let source = SourceSession::use_session();
    let translation = TranslationSession::use_session();
    let mut search_open = use_signal(|| false);
    let mut library_open = use_signal(|| false);
    let mut find_open = use_signal(|| false);
    let mut help_open = use_signal(|| false);
    let renderer = if cfg!(feature = "native") {
        "native"
    } else {
        "webview"
    };

    #[cfg(all(feature = "desktop", not(feature = "native")))]
    let window = dioxus::desktop::use_window();

    #[cfg(all(feature = "desktop", not(feature = "native")))]
    let zoom_window = window.clone();
    #[cfg(all(feature = "desktop", not(feature = "native")))]
    use_effect(move || {
        zoom_window.set_zoom_level(zoom());
    });

    #[cfg(all(feature = "desktop", not(feature = "native")))]
    let session_window = window.clone();
    use_effect(move || {
        let Some(mut config) = source.session_config(theme.read().id(), zoom()) else {
            return;
        };
        #[cfg(all(feature = "desktop", not(feature = "native")))]
        {
            config.window = current_window_geometry(&session_window);
        }
        schedule_session_save(config);
    });

    #[cfg(all(feature = "desktop", not(feature = "native")))]
    let geometry_window = window.clone();
    #[cfg(all(feature = "desktop", not(feature = "native")))]
    use_future(move || {
        let window = geometry_window.clone();
        async move {
            let mut last = current_window_geometry(&window);
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let Some(geometry) = current_window_geometry(&window) else {
                    continue;
                };
                if last == Some(geometry) {
                    continue;
                }
                let Some(mut config) = source.session_config(theme.read().id(), zoom()) else {
                    continue;
                };
                config.window = Some(geometry);
                schedule_session_save(config);
                last = Some(geometry);
            }
        }
    });

    #[cfg(not(feature = "native"))]
    let translation_commands = translation.clone();
    #[cfg(not(feature = "native"))]
    let translation_escape = translation.clone();
    #[cfg(not(feature = "native"))]
    use_future(move || {
        let mut translation_commands = translation_commands.clone();
        let mut translation_escape = translation_escape.clone();
        async move {
            let mut eval = dioxus::document::eval(SHORTCUT_SCRIPT);
            while let Ok(command) = eval.recv::<String>().await {
                if command.starts_with('{')
                    && let Ok(payload) = serde_json::from_str::<serde_json::Value>(&command)
                    && payload.get("command").and_then(serde_json::Value::as_str)
                        == Some("translate-selection")
                {
                    let text = payload
                        .get("text")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    let line = payload
                        .get("line")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(1) as u32;
                    let end_line = payload
                        .get("end_line")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(line as u64) as u32;
                    translation_commands.select(text, line, end_line);
                    continue;
                }
                match command.as_str() {
                    "search" => {
                        search_open.set(true);
                        library_open.set(false);
                        find_open.set(false);
                        help_open.set(false);
                    }
                    "find" => {
                        find_open.set(true);
                        search_open.set(false);
                        library_open.set(false);
                        help_open.set(false);
                    }
                    "help" => {
                        help_open.set(true);
                        search_open.set(false);
                        library_open.set(false);
                        find_open.set(false);
                    }
                    "zoom-in" => zoom.set(clamp_zoom(zoom() + ZOOM_STEP)),
                    "zoom-out" => zoom.set(clamp_zoom(zoom() - ZOOM_STEP)),
                    "zoom-reset" => zoom.set(1.0),
                    "next-tab" => source.cycle_tab(1),
                    "prev-tab" => source.cycle_tab(-1),
                    "close-tab" => source.close_active_tab(),
                    "back" => source.go_history(-1),
                    "forward" => source.go_history(1),
                    "fold-current" => source.toggle_fold_at_current(),
                    "fold-all" => source.collapse_current_file_folds(),
                    "unfold-all" => source.expand_current_file_folds(),
                    "escape" => {
                        search_open.set(false);
                        library_open.set(false);
                        find_open.set(false);
                        help_open.set(false);
                        source.close_diff();
                        translation_escape.close();
                    }
                    _ => {}
                }
            }
        }
    });

    rsx! {
        style { {APP_CSS} }
        style { {THEME_CSS} }
        div { class: "app-shell", "data-theme": theme_id, "data-renderer": renderer,
            TitleBar { theme, source, search_open, library_open, help_open }
            DiffPanel { source }
            TranslationPanel { translation: translation.clone() }
            main { class: "learning-workbench",
                SourceExplorer {
                    source,
                    theme,
                    on_open_library: move |_| {
                        help_open.set(false);
                        library_open.set(true);
                    },
                }
                CodeView { source, find_open, theme, translation }
                GuidePanel { source }
            }
            StatusBar { source, zoom }
        }
    }
}

#[cfg(all(feature = "desktop", not(feature = "native")))]
fn current_window_geometry(window: &dioxus::desktop::DesktopContext) -> Option<WindowGeometry> {
    if window.is_minimized() {
        return None;
    }
    let position = window.outer_position().ok()?;
    let size = window.inner_size();
    Some(WindowGeometry {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
        maximized: window.is_maximized(),
    })
}

fn schedule_session_save(config: SessionConfig) {
    let epoch = SESSION_SAVE_EPOCH.fetch_add(1, Ordering::Relaxed) + 1;
    dioxus::dioxus_core::spawn_forever(async move {
        tokio::time::sleep(Duration::from_millis(600)).await;
        if SESSION_SAVE_EPOCH.load(Ordering::Relaxed) != epoch {
            return;
        }
        let _ = tokio::task::spawn_blocking(move || config.save()).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_is_clamped_and_rounded() {
        assert_eq!(clamp_zoom(0.3), MIN_ZOOM);
        assert_eq!(clamp_zoom(3.0), MAX_ZOOM);
        assert_eq!(clamp_zoom(1.04), 1.0);
        assert_eq!(clamp_zoom(1.26), 1.3);
    }

    #[test]
    fn app_css_contains_no_literal_colors() {
        for (index, line) in APP_CSS.lines().enumerate() {
            let lower = line.to_ascii_lowercase();
            assert!(
                !lower.contains("rgb(") && !lower.contains("rgba("),
                "literal rgb color in app.css line {}: {line}",
                index + 1
            );
            if let Some(hash) = line.find('#') {
                let candidate: String = line[hash + 1..].chars().take(8).collect();
                assert!(
                    !candidate.chars().take(3).all(|c| c.is_ascii_hexdigit()),
                    "literal hex color in app.css line {}: {line}",
                    index + 1
                );
            }
        }
    }

    #[test]
    fn every_css_variable_used_by_components_is_defined_by_a_theme() {
        let used = used_css_variables(APP_CSS);
        let mut shared = block_variables(THEME_CSS, ":root {").unwrap_or_default();
        shared.extend(block_variables(THEME_CSS, ".app-shell {").unwrap_or_default());
        for theme in Theme::ALL {
            let marker = format!(".app-shell[data-theme=\"{}\"] {{", theme.id());
            let defined = block_variables(THEME_CSS, &marker)
                .unwrap_or_else(|| panic!("missing theme block: {}", theme.id()));
            let missing: Vec<&str> = used
                .iter()
                .map(String::as_str)
                .filter(|name| !defined.iter().any(|item| item.as_str() == *name))
                .filter(|name| !shared.iter().any(|item| item.as_str() == *name))
                .collect();
            assert!(
                missing.is_empty(),
                "theme {} is missing variables: {missing:?}",
                theme.id()
            );
        }
    }

    fn used_css_variables(css: &str) -> Vec<String> {
        let mut rest = css;
        let mut names = Vec::new();
        while let Some(start) = rest.find("var(--") {
            rest = &rest[start + 6..];
            let end = rest.find([')', ',']).unwrap_or(rest.len());
            let name = format!("--{}", &rest[..end]);
            if !names.contains(&name) {
                names.push(name);
            }
        }
        names
    }

    fn block_variables(css: &str, marker: &str) -> Option<Vec<String>> {
        let start = css.find(marker)? + marker.len();
        let rest = &css[start..];
        let end = rest.find("\n}")?;
        let block = &rest[..end];
        let mut names = Vec::new();
        for line in block.lines() {
            let line = line.trim_start();
            if !line.starts_with("--") {
                continue;
            }
            if let Some(end) = line.find(':') {
                names.push(line[..end].to_owned());
            }
        }
        Some(names)
    }
}
