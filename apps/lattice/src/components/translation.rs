use dioxus::prelude::*;

use crate::translation::{TranslationSession, TranslationStatus};

use super::icon::{Icon, IconKind};

#[component]
pub fn TranslationPanel(mut translation: TranslationSession) -> Element {
    let visible = *translation.visible.read();
    let selection = translation.selection.read().clone();

    if !visible {
        let Some(selection) = selection else {
            return rsx! {};
        };
        let mut open_translation = translation.clone();
        let mut dismiss_translation = translation.clone();
        return rsx! {
            section { class: "translation-fab", aria_label: "翻译选中的注释",
                button {
                    r#type: "button",
                    class: "translation-fab-open",
                    title: "翻译选中的源码",
                    onclick: move |_| open_translation.open(),
                    Icon { kind: IconKind::Lightbulb, size: 12.0 }
                    span { "翻译选中" }
                    span { class: "translation-fab-range",
                        "{selection.start_line}–{selection.end_line}"
                    }
                }
                button {
                    r#type: "button",
                    class: "translation-fab-close",
                    title: "取消选择",
                    onclick: move |_| dismiss_translation.close(),
                    "×"
                }
            }
        };
    }

    let result = translation.result.read().clone();
    let error = translation.error.read().clone();
    let status = *translation.status.read();
    let available = *translation.available.read();
    let model_label = translation.model_label.read().clone();
    let worker_ready = *translation.worker_ready.read();
    let truncated = *translation.truncated.read();
    let has_result = result.is_some();
    let mut close_translation = translation.clone();
    let mut translate_translation = translation.clone();
    let mut cancel_translation = translation.clone();

    rsx! {
        section { class: "translation-panel", aria_label: "本地注释翻译",
            div { class: "translation-head",
                Icon { kind: IconKind::Lightbulb, size: 14.0 }
                div { class: "translation-head-copy",
                    strong { "本地翻译" }
                    span { "{model_label}" }
                }
                if let Some(selection) = selection.as_ref() {
                    span { class: "translation-range", "{selection.start_line}–{selection.end_line}" }
                }
                button {
                    r#type: "button",
                    class: "search-close",
                    title: "关闭",
                    onclick: move |_| close_translation.close(),
                    "×"
                }
            }

            if !available {
                div { class: "translation-empty",
                    strong { "未找到本地翻译模型" }
                    span { "请先运行 scripts/fetch-opus-mt-model.ps1，或设置 LATTICE_MODEL_DIR。" }
                }
            } else if let Some(selection) = selection.as_ref() {
                div { class: "translation-block",
                    span { class: "translation-label", "选中的注释" }
                    pre { class: "translation-source", "{selection.text}" }
                }
                if status == TranslationStatus::Translating {
                    div { class: "translation-loading",
                        span { class: "source-spinner" }
                        if worker_ready {
                            "正在用本地模型翻译…"
                        } else {
                            "正在加载本地翻译模型…"
                        }
                    }
                } else if let Some(error) = error {
                    div { class: "translation-error", "{error}" }
                } else if let Some(result) = result.as_ref() {
                    div { class: "translation-block",
                        span { class: "translation-label", "译文" }
                        pre { class: "translation-result", "{result}" }
                        if truncated {
                            span { class: "translation-warning",
                                "原文超过模型上下文，已截断后翻译。"
                            }
                        }
                    }
                }
                div { class: "translation-actions",
                    if status == TranslationStatus::Translating {
                        button {
                            r#type: "button",
                            class: "translation-cancel",
                            onclick: move |_| cancel_translation.cancel(),
                            "取消"
                        }
                    } else {
                        button {
                            r#type: "button",
                            class: "translation-primary",
                            onclick: move |_| translate_translation.translate_selected(),
                            if has_result { "重新翻译" } else { "翻译成中文" }
                        }
                    }
                    if has_result && status != TranslationStatus::Translating {
                        if let Some(result) = result.as_ref() {
                            button {
                                r#type: "button",
                                class: "translation-copy",
                                onclick: {
                                    let result = result.clone();
                                    move |_| {
                                        if let Ok(text) = serde_json::to_string(&result) {
                                            dioxus::document::eval(&format!(
                                                "navigator.clipboard.writeText({text});"
                                            ));
                                        }
                                    }
                                },
                                "复制译文"
                            }
                        }
                    }
                    span { class: "translation-hint", "只读 · 本地" }
                }
            } else {
                div { class: "translation-empty",
                    strong { "选中文本后开始翻译" }
                    span { "在代码区选中注释，点击右下角“翻译选中”；也可以把光标放在注释行上，点击工具栏“翻译注释”。" }
                }
            }
        }
    }
}
