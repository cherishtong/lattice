# 本地注释翻译

Lattice 的可选本地翻译使用 **OPUS-MT English → Chinese**，通过独立工作进程运行，默认不随主程序加载模型。

## 组件

- `lattice.exe`：主程序，负责选中文本、翻译面板和结果展示
- `lattice-translator.exe`：独立翻译工作进程
- `%APPDATA%\Lattice\models\opus-mt-en-zh` 或 `.content/models/opus-mt-en-zh`：本地模型目录
- ONNX Runtime：由 `ort` 链接，模型推理完全在本机完成

## 获取模型

```powershell
powershell -ExecutionPolicy Bypass -File scripts/fetch-opus-mt-model.ps1
```

默认安装到 `.content/models/opus-mt-en-zh`，也可以指定目录：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/fetch-opus-mt-model.ps1 -Destination "$env:APPDATA\Lattice\models\opus-mt-en-zh"
```

脚本会下载：

- `tokenizer.json`
- `source.spm` / `target.spm`
- `onnx/encoder_model_quantized.onnx`
- `onnx/decoder_model_quantized.onnx`

并修正 Transformers.js tokenizer 中为空的 precompiled charset map。

## 使用

1. 打开任意源码文件
2. 用鼠标选中注释文本
3. 右下角出现“翻译选中”，点击后打开翻译面板
4. 点击“翻译成中文”
5. 可复制译文、重新翻译，或翻译过程中取消

也可以把光标放在注释行上，点击代码区工具栏的“翻译注释”，直接提取整段相邻注释并打开面板。

模型与推理都在本地，不访问网络。

## 状态与日志

- 翻译进程意外退出时，面板会显示错误，而不是一直停留在“正在翻译”。
- 只有当前选中的请求会更新结果；切换选择后，旧请求的结果会被忽略。
- 原文超过模型上下文时，面板会提示“已截断后翻译”。
- 翻译进程的 stderr 写入 `%APPDATA%\Lattice\logs\translator.log`（或 `LATTICE_CONFIG_DIR` 对应目录），日志超过 2 MiB 会在下次启动时轮换。

## 模型搜索顺序

1. `LATTICE_MODEL_DIR` 环境变量
2. `%APPDATA%\Lattice\models\opus-mt-en-zh`
3. `lattice.exe` 旁边的 `.content\models\opus-mt-en-zh`
4. 当前工作目录 `.content/models/opus-mt-en-zh`

## 模型信息

- 模型：`Helsinki-NLP/opus-mt-en-zh`
- 量化：INT8 ONNX
- 体积：约 117 MiB（含 tokenizer）
- 许可证：CC-BY-4.0
- 推理：CPU，贪心解码，最长 256 个新 token

## 协议

工作进程使用 NDJSON：

```json
{"event":"ready","worker":1,"model":"opus-mt-en-zh-int8"}
{"cmd":"translate","id":1,"text":"Return the list length."}
{"event":"result","id":1,"text":"返回列表长度。","truncated":false}
```

主程序按需启动进程，并在每个事件中带上工作进程代号，避免旧进程退出或旧请求结果影响当前翻译。进程退出后，下一次翻译会自动重新启动。

## 边界

- 当前只支持英语 → 简体中文
- 翻译对象是用户选中的文本；不会修改源码
- 不进行在线翻译或远程模型调用
- 长注释按模型上下文截断，并在界面提示
- 模型不随主程序发布，需单独获取
