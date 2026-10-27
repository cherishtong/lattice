param(
    [string] $Destination = ""
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($Destination)) {
    $Destination = Join-Path $root ".content\models\opus-mt-en-zh"
}
$base = "https://hf-mirror.com/Xenova/opus-mt-en-zh/resolve/main"
New-Item -ItemType Directory -Path (Join-Path $Destination "onnx") -Force | Out-Null

$files = @(
    "config.json",
    "tokenizer.json",
    "special_tokens_map.json",
    "tokenizer_config.json",
    "source.spm",
    "target.spm",
    "vocab.json",
    "onnx/encoder_model_quantized.onnx",
    "onnx/decoder_model_quantized.onnx"
)

foreach ($file in $files) {
    $target = Join-Path $Destination ($file -replace '/', '\')
    $parent = Split-Path -Parent $target
    if (-not (Test-Path -LiteralPath $parent)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    if (Test-Path -LiteralPath $target) {
        Write-Host "reuse $file"
        continue
    }
    Write-Host "download $file"
    curl.exe -L --fail --retry 3 --retry-delay 2 --max-time 1200 --silent --show-error `
        -o $target "$base/$file"
    if ($LASTEXITCODE -ne 0) {
        throw "download failed: $file"
    }
}

$tokenizerPath = Join-Path $Destination "tokenizer.json"
$tokenizer = [System.IO.File]::ReadAllText($tokenizerPath)
$tokenizer = $tokenizer.Replace(
    '"type": "Precompiled",' + "`r`n" + '        "precompiled_charsmap": null',
    '"type": "NFKC"')
$tokenizer = $tokenizer.Replace(
    '"type": "Precompiled",' + "`n" + '        "precompiled_charsmap": null',
    '"type": "NFKC"')
[System.IO.File]::WriteAllText(
    $tokenizerPath,
    $tokenizer,
    (New-Object System.Text.UTF8Encoding $false))

$manifest = @"
{
  "id": "opus-mt-en-zh-int8",
  "name": "OPUS-MT English to Chinese",
  "version": "1",
  "backend": "onnx-marian",
  "source_language": "en",
  "target_language": "zh",
  "license": "CC-BY-4.0",
  "model": "Helsinki-NLP/opus-mt-en-zh",
  "quantization": "int8",
  "files": [
    "tokenizer.json",
    "onnx/encoder_model_quantized.onnx",
    "onnx/decoder_model_quantized.onnx"
  ]
}
"@
[System.IO.File]::WriteAllText(
    (Join-Path $Destination "manifest.json"),
    $manifest,
    (New-Object System.Text.UTF8Encoding $false))

$files = Get-ChildItem -LiteralPath $Destination -Recurse -File
$bytes = ($files | Measure-Object Length -Sum).Sum
Write-Host ("OPUS-MT ready: {0:N0} files, {1:N1} MiB at {2}" -f $files.Count, ($bytes / 1MB), $Destination)
