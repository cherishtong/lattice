use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use dioxus::prelude::*;
use serde_json::{Value, json};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TranslationStatus {
    Idle,
    Translating,
    Done,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranslationSelection {
    pub text: String,
    pub start_line: u32,
    pub end_line: u32,
}

struct Worker {
    child: Child,
    stdin: ChildStdin,
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Clone)]
pub struct TranslationSession {
    pub status: Signal<TranslationStatus>,
    pub selection: Signal<Option<TranslationSelection>>,
    pub result: Signal<Option<String>>,
    pub error: Signal<Option<String>>,
    pub visible: Signal<bool>,
    pub worker_ready: Signal<bool>,
    pub available: Signal<bool>,
    pub model_label: Signal<String>,
    pub truncated: Signal<bool>,
    pending: Signal<Option<u64>>,
    sender: UnboundedSender<Value>,
    worker: Arc<Mutex<Option<Worker>>>,
    next_id: Arc<AtomicU64>,
    generation: Arc<AtomicU64>,
    model_dir: Option<PathBuf>,
    worker_path: Option<PathBuf>,
}

impl PartialEq for TranslationSession {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.worker, &other.worker)
    }
}

impl TranslationSession {
    pub fn use_session() -> Self {
        let status = use_signal(|| TranslationStatus::Idle);
        let selection = use_signal(|| None::<TranslationSelection>);
        let result = use_signal(|| None::<String>);
        let error = use_signal(|| None::<String>);
        let visible = use_signal(|| false);
        let worker_ready = use_signal(|| false);
        let truncated = use_signal(|| false);
        let pending = use_signal(|| None::<u64>);
        let model_dir = discover_model_dir();
        let worker_path = discover_worker();
        let available = use_signal(|| model_dir.is_some() && worker_path.is_some());
        let model_label = use_signal(|| {
            if model_dir.is_some() {
                "OPUS-MT English → Chinese".to_owned()
            } else {
                "未安装本地翻译模型".to_owned()
            }
        });
        let (sender, receiver) = unbounded_channel::<Value>();
        let sender = use_hook(move || sender);
        let receiver_slot = use_hook(move || Arc::new(Mutex::new(Some(receiver))));
        let worker = use_hook(|| Arc::new(Mutex::new(None::<Worker>)));
        let next_id = use_hook(|| Arc::new(AtomicU64::new(1)));
        let generation = use_hook(|| Arc::new(AtomicU64::new(0)));

        let mut event_status = status;
        let mut event_result = result;
        let mut event_error = error;
        let mut event_ready = worker_ready;
        let mut event_truncated = truncated;
        let mut event_pending = pending;
        let event_worker = worker.clone();
        let event_generation = generation.clone();
        let event_receiver = receiver_slot.clone();
        use_future(move || {
            let receiver = event_receiver.lock().ok().and_then(|mut slot| slot.take());
            let event_worker = event_worker.clone();
            let event_generation = event_generation.clone();
            async move {
                let Some(mut receiver) = receiver else {
                    return;
                };
                while let Some(event) = receiver.recv().await {
                    let event_worker_id = event.get("worker").and_then(Value::as_u64);
                    let current_worker = event_generation.load(Ordering::Relaxed);
                    match event.get("event").and_then(Value::as_str) {
                        Some("ready") => {
                            if event_worker_id == Some(current_worker) {
                                event_ready.set(true);
                            }
                        }
                        Some("result") => {
                            let event_id = event.get("id").and_then(Value::as_u64);
                            if event_id != *event_pending.read() {
                                continue;
                            }
                            event_result
                                .set(event.get("text").and_then(Value::as_str).map(str::to_owned));
                            event_truncated.set(
                                event
                                    .get("truncated")
                                    .and_then(Value::as_bool)
                                    .unwrap_or(false),
                            );
                            event_pending.set(None);
                            event_status.set(TranslationStatus::Done);
                        }
                        Some("error") => {
                            let event_id = event.get("id").and_then(Value::as_u64);
                            if event_id.is_some() && event_id != *event_pending.read() {
                                continue;
                            }
                            event_error.set(
                                event
                                    .get("message")
                                    .and_then(Value::as_str)
                                    .map(str::to_owned)
                                    .or_else(|| Some("本地翻译失败".to_owned())),
                            );
                            event_pending.set(None);
                            event_status.set(TranslationStatus::Error);
                        }
                        Some("exited") => {
                            if event_worker_id != Some(current_worker) {
                                continue;
                            }
                            event_ready.set(false);
                            event_pending.set(None);
                            if let Ok(mut worker) = event_worker.lock() {
                                worker.take();
                            }
                            if *event_status.read() == TranslationStatus::Translating {
                                event_error.set(Some(
                                    "本地翻译进程已退出，请检查模型文件或重试。".to_owned(),
                                ));
                                event_status.set(TranslationStatus::Error);
                            }
                        }
                        _ => {}
                    }
                }
            }
        });

        Self {
            status,
            selection,
            result,
            error,
            visible,
            worker_ready,
            available,
            model_label,
            truncated,
            pending,
            sender,
            worker,
            next_id,
            generation,
            model_dir,
            worker_path,
        }
    }

    pub fn select(&mut self, text: String, start_line: u32, end_line: u32) {
        let text = text.trim().to_owned();
        if text.is_empty() {
            return;
        }
        self.selection.set(Some(TranslationSelection {
            text,
            start_line,
            end_line,
        }));
        self.result.set(None);
        self.error.set(None);
        self.truncated.set(false);
        self.pending.set(None);
        self.status.set(TranslationStatus::Idle);
    }

    pub fn select_and_open(&mut self, text: String, start_line: u32, end_line: u32) {
        self.select(text, start_line, end_line);
        self.visible.set(true);
    }

    pub fn open(&mut self) {
        if self.selection.read().is_some() {
            self.visible.set(true);
        }
    }

    pub fn cancel(&mut self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut worker) = self.worker.lock() {
            worker.take();
        }
        self.pending.set(None);
        self.result.set(None);
        self.error.set(None);
        self.truncated.set(false);
        self.worker_ready.set(false);
        self.status.set(TranslationStatus::Idle);
    }

    pub fn translate_selected(&mut self) {
        let Some(selection) = self.selection.read().clone() else {
            return;
        };
        self.translate(selection.text);
    }

    pub fn translate(&mut self, text: String) {
        if !*self.available.read() {
            self.error.set(Some(
                "未找到 lattice-translator.exe 或 OPUS-MT 模型。".to_owned(),
            ));
            self.status.set(TranslationStatus::Error);
            self.visible.set(true);
            return;
        }
        self.error.set(None);
        self.result.set(None);
        self.truncated.set(false);
        self.status.set(TranslationStatus::Translating);
        self.visible.set(true);

        let Some(worker_path) = self.worker_path.clone() else {
            return;
        };
        let Some(model_dir) = self.model_dir.clone() else {
            return;
        };
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.pending.set(Some(id));
        let mut guard = match self.worker.lock() {
            Ok(guard) => guard,
            Err(_) => {
                self.pending.set(None);
                return;
            }
        };
        if guard.is_none() {
            self.worker_ready.set(false);
            let generation = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
            match spawn_worker(&worker_path, &model_dir, generation, self.sender.clone()) {
                Ok(worker) => *guard = Some(worker),
                Err(error) => {
                    self.pending.set(None);
                    self.error.set(Some(error));
                    self.status.set(TranslationStatus::Error);
                    return;
                }
            }
        }
        let Some(worker) = guard.as_mut() else {
            self.pending.set(None);
            return;
        };
        let payload = json!({
            "cmd": "translate",
            "id": id,
            "text": text
        });
        if writeln!(worker.stdin, "{payload}")
            .and_then(|_| worker.stdin.flush())
            .is_err()
        {
            *guard = None;
            self.pending.set(None);
            self.error.set(Some("本地翻译进程已退出。".to_owned()));
            self.status.set(TranslationStatus::Error);
        }
    }

    pub fn close(&mut self) {
        self.visible.set(false);
        self.selection.set(None);
        self.result.set(None);
        self.error.set(None);
        self.truncated.set(false);
        self.pending.set(None);
        self.status.set(TranslationStatus::Idle);
    }
}

fn spawn_worker(
    worker_path: &Path,
    model_dir: &Path,
    generation: u64,
    sender: UnboundedSender<Value>,
) -> Result<Worker, String> {
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;
    let mut command = Command::new(worker_path);
    command
        .arg(model_dir)
        .arg(generation.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    let mut child = command
        .spawn()
        .map_err(|error| format!("启动本地翻译进程失败: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "无法读取翻译进程输出".to_owned())?;
    let stderr = child.stderr.take();
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "无法写入翻译进程".to_owned())?;
    if let Some(stderr) = stderr {
        let log_path = translator_log_path();
        std::thread::spawn(move || {
            let mut log = log_path
                .and_then(|path| OpenOptions::new().create(true).append(true).open(path).ok());
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                if let Some(log) = log.as_mut() {
                    let seconds = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_or(0, |duration| duration.as_secs());
                    let _ = writeln!(log, "[{seconds}] {line}");
                    let _ = log.flush();
                }
            }
        });
    }
    let exit_sender = sender.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(value) = serde_json::from_str::<Value>(&line) {
                let _ = sender.send(value);
            }
        }
        let _ = exit_sender.send(json!({"event": "exited", "worker": generation}));
    });
    Ok(Worker { child, stdin })
}

fn translator_log_path() -> Option<PathBuf> {
    let root = std::env::var_os("LATTICE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("APPDATA").map(|directory| PathBuf::from(directory).join("Lattice"))
        })
        .or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .map(|directory| PathBuf::from(directory).join("Lattice"))
        })?;
    let directory = root.join("logs");
    fs::create_dir_all(&directory).ok()?;
    let path = directory.join("translator.log");
    if path
        .metadata()
        .is_ok_and(|metadata| metadata.len() > 2 * 1024 * 1024)
    {
        let _ = fs::remove_file(&path);
    }
    Some(path)
}

fn discover_worker() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let directory = executable.parent()?;
    let path = directory.join("lattice-translator.exe");
    path.is_file().then_some(path)
}

fn discover_model_dir() -> Option<PathBuf> {
    if let Some(directory) = std::env::var_os("LATTICE_MODEL_DIR") {
        let directory = PathBuf::from(directory);
        if valid_model(&directory) {
            return Some(directory);
        }
    }
    if let Some(app_data) = std::env::var_os("APPDATA") {
        let directory = PathBuf::from(app_data)
            .join("Lattice")
            .join("models")
            .join("opus-mt-en-zh");
        if valid_model(&directory) {
            return Some(directory);
        }
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        let directory = directory
            .join(".content")
            .join("models")
            .join("opus-mt-en-zh");
        if valid_model(&directory) {
            return Some(directory);
        }
    }
    let directory = PathBuf::from(".content/models/opus-mt-en-zh");
    if valid_model(&directory) {
        return Some(directory);
    }
    None
}

fn valid_model(directory: &std::path::Path) -> bool {
    directory.join("tokenizer.json").is_file()
        && directory
            .join("onnx/encoder_model_quantized.onnx")
            .is_file()
        && directory
            .join("onnx/decoder_model_quantized.onnx")
            .is_file()
}
