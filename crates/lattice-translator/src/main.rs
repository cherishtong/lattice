use std::io::{BufRead, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use lattice_translator::{Translator, locate_model_dir};
use serde_json::{Value, json};

fn main() {
    if let Err(error) = run() {
        let message = format!("{error:#}");
        eprintln!("lattice-translator: {message}");
        let _ = writeln!(
            std::io::stdout(),
            "{}",
            json!({"event": "error", "message": message})
        );
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let model_dir = args
        .next()
        .filter(|argument| !argument.trim().is_empty())
        .map(PathBuf::from)
        .or_else(locate_model_dir)
        .context("OPUS-MT model directory not found")?;
    let worker = args
        .next()
        .and_then(|argument| argument.parse::<u64>().ok())
        .unwrap_or(0);
    let mut translator = Translator::load(&model_dir)?;
    let mut stdout = std::io::stdout();
    writeln!(
        stdout,
        "{}",
        json!({
            "event": "ready",
            "worker": worker,
            "model": "opus-mt-en-zh-int8",
            "path": model_dir.display().to_string()
        })
    )?;
    stdout.flush()?;

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(error) => {
                writeln!(
                    stdout,
                    "{}",
                    json!({"event": "error", "message": format!("invalid request: {error}")})
                )?;
                stdout.flush()?;
                continue;
            }
        };
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        match request.get("cmd").and_then(Value::as_str) {
            Some("translate") => {
                let text = request
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                match translator.translate(text) {
                    Ok(translation) => {
                        writeln!(
                            stdout,
                            "{}",
                            json!({
                                "event": "result",
                                "id": id,
                                "text": translation.text,
                                "truncated": translation.truncated
                            })
                        )?;
                    }
                    Err(error) => {
                        let message = format!("{error:#}");
                        eprintln!("translate failed: {message}");
                        writeln!(
                            stdout,
                            "{}",
                            json!({"event": "error", "id": id, "message": message})
                        )?;
                    }
                }
                stdout.flush()?;
            }
            Some("ping") => {
                writeln!(stdout, "{}", json!({"event": "pong", "id": id}))?;
                stdout.flush()?;
            }
            Some("shutdown") => break,
            _ => {
                writeln!(
                    stdout,
                    "{}",
                    json!({"event": "error", "id": id, "message": "unknown command"})
                )?;
                stdout.flush()?;
            }
        }
    }
    Ok(())
}
