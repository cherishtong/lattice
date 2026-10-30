# Native / Blitz 实验后端

WebView2 是 Lattice 的唯一正式后端。Dioxus Native / Blitz 仅保留为实验入口，**不在当前产品目标内**，也不要求功能对齐。

## 状态

- 可构建并启动基础三栏界面。
- 主题变量、代码字体、图标和部分布局可渲染。
- 半透明、阴影、滚动、部分 transform 与 WebView2 存在差异。
- 首次构建需要 Python/Stylo 工具链和大量 GPU 依赖。
- 不参与正式发布、安装器和体积基线。

## 运行

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run-native.ps1
```

或：

```powershell
$env:PYTHON3 = "$PWD\.tools\python\python.exe"
cargo run -p lattice --no-default-features --features native
```

## 约束

- 不把 Native 差异扩散到业务组件。
- 渲染器差异只通过 `data-renderer="native"` 和主题变量隔离。
- 不以 Native 后端作为发布验收条件。
- 后续是否继续维护，取决于 WebView2 是否出现不可接受的限制；当前不投入功能对齐。
