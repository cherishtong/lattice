//! Local OPUS-MT translation using ONNX Runtime.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ort::session::Session;
use ort::value::Tensor;
use tokenizers::Tokenizer;

const PAD_TOKEN: i64 = 65000;
const EOS_TOKEN: i64 = 0;
const MAX_NEW_TOKENS: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Translation {
    pub text: String,
    pub truncated: bool,
}

#[derive(Debug)]
pub struct Translator {
    tokenizer: Tokenizer,
    encoder: Session,
    decoder: Session,
    max_source_tokens: usize,
}

impl Translator {
    pub fn load(model_dir: &Path) -> Result<Self> {
        let tokenizer_path = model_dir.join("tokenizer.json");
        let raw = fs::read_to_string(&tokenizer_path)
            .with_context(|| format!("read {}", tokenizer_path.display()))?;
        let raw = raw.replace(
            "\"type\": \"Precompiled\",\n        \"precompiled_charsmap\": null",
            "\"type\": \"NFKC\"",
        );
        let tokenizer = Tokenizer::from_bytes(raw.as_bytes())
            .map_err(|error| anyhow::anyhow!("load tokenizer: {error}"))?;
        let encoder = Session::builder()
            .map_err(|error| anyhow::anyhow!("session builder: {error}"))?
            .commit_from_file(model_dir.join("onnx/encoder_model_quantized.onnx"))
            .context("load encoder")?;
        let decoder = Session::builder()
            .map_err(|error| anyhow::anyhow!("session builder: {error}"))?
            .commit_from_file(model_dir.join("onnx/decoder_model_quantized.onnx"))
            .context("load decoder")?;

        Ok(Self {
            tokenizer,
            encoder,
            decoder,
            max_source_tokens: 480,
        })
    }

    pub fn translate(&mut self, text: &str) -> Result<Translation> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(Translation {
                text: String::new(),
                truncated: false,
            });
        }
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|error| anyhow::anyhow!("tokenize: {error}"))?;
        let mut source_ids: Vec<i64> = encoding.get_ids().iter().map(|id| *id as i64).collect();
        let mut truncated = false;
        if source_ids.len() > self.max_source_tokens {
            source_ids.truncate(self.max_source_tokens);
            source_ids.push(EOS_TOKEN);
            truncated = true;
        }
        let source_len = source_ids.len();

        let input_ids = Tensor::from_array(([1usize, source_len], source_ids.into_boxed_slice()))?;
        let attention_mask = Tensor::from_array((
            [1usize, source_len],
            vec![1i64; source_len].into_boxed_slice(),
        ))?;
        let encoder_outputs = self
            .encoder
            .run(ort::inputs![
                "input_ids" => input_ids,
                "attention_mask" => attention_mask
            ])
            .context("run encoder")?;
        let (hidden_shape, hidden_data) = encoder_outputs["last_hidden_state"]
            .try_extract_tensor::<f32>()
            .context("extract encoder hidden state")?;
        let hidden_dim = hidden_shape[2] as usize;
        let hidden_data = hidden_data.to_vec();
        let encoder_hidden = Tensor::from_array((
            [1usize, source_len, hidden_dim],
            hidden_data.into_boxed_slice(),
        ))?;
        let encoder_mask = Tensor::from_array((
            [1usize, source_len],
            vec![1i64; source_len].into_boxed_slice(),
        ))?;

        let mut decoder_ids: Vec<i64> = vec![PAD_TOKEN];
        let mut generated: Vec<u32> = Vec::new();
        for _ in 0..MAX_NEW_TOKENS {
            let decoder_len = decoder_ids.len();
            let decoder_input = Tensor::from_array((
                [1usize, decoder_len],
                decoder_ids.clone().into_boxed_slice(),
            ))?;
            let outputs = self
                .decoder
                .run(ort::inputs![
                    "encoder_attention_mask" => encoder_mask.view(),
                    "input_ids" => decoder_input,
                    "encoder_hidden_states" => encoder_hidden.view()
                ])
                .context("run decoder")?;
            let logits = outputs["logits"]
                .try_extract_tensor::<f32>()
                .context("extract logits")?;
            let shape = logits.0;
            let vocab = shape[2] as usize;
            let offset = (shape[1] as usize - 1) * vocab;
            let row = &logits.1[offset..offset + vocab];
            let mut best_token = 0i64;
            let mut best_score = f32::NEG_INFINITY;
            for (index, score) in row.iter().enumerate() {
                if index as i64 == PAD_TOKEN {
                    continue;
                }
                if *score > best_score {
                    best_score = *score;
                    best_token = index as i64;
                }
            }
            decoder_ids.push(best_token);
            if best_token == EOS_TOKEN {
                break;
            }
            generated.push(best_token as u32);
        }

        let translated = self
            .tokenizer
            .decode(&generated, true)
            .map_err(|error| anyhow::anyhow!("decode: {error}"))?;
        Ok(Translation {
            text: cleanup(&translated),
            truncated,
        })
    }
}

fn cleanup(text: &str) -> String {
    let mut result = text.trim().to_owned();
    for punctuation in ["，", "。", "！", "？", "；", "：", "、", "）", "】", "》"] {
        result = result.replace(&format!(" {punctuation}"), punctuation);
    }
    for punctuation in ["（", "【", "《"] {
        result = result.replace(&format!("{punctuation} "), punctuation);
    }
    result
}

pub fn locate_model_dir() -> Option<PathBuf> {
    if let Some(directory) = std::env::var_os("LATTICE_MODEL_DIR") {
        let directory = PathBuf::from(directory);
        if valid_model_dir(&directory) {
            return Some(directory);
        }
    }
    if let Some(app_data) = std::env::var_os("APPDATA") {
        let directory = PathBuf::from(app_data)
            .join("Lattice")
            .join("models")
            .join("opus-mt-en-zh");
        if valid_model_dir(&directory) {
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
        if valid_model_dir(&directory) {
            return Some(directory);
        }
    }
    let directory = Path::new(".content/models/opus-mt-en-zh").to_path_buf();
    if valid_model_dir(&directory) {
        return Some(directory);
    }
    None
}

pub fn valid_model_dir(directory: &Path) -> bool {
    directory.join("tokenizer.json").is_file()
        && directory
            .join("onnx/encoder_model_quantized.onnx")
            .is_file()
        && directory
            .join("onnx/decoder_model_quantized.onnx")
            .is_file()
}
