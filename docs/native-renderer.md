# Dioxus Native（Blitz）试验记录

## 目的

验证同一套 Dioxus RSX + CSS 能否从 WebView 后端切换到 Dioxus Native / Blitz，从而减少对 WebView2 的依赖，并观察原生 GPU 渲染的视觉质量、交互和字体表现。

测试版本：Dioxus 0.7.10、dioxus-native 0.7.10、Windows 11、debug 构建。

## 运行

原生构建需要 Python 生成 Stylo 的 CSS 属性代码。项目提供了隔离脚本，把 Python 3.12 只下载到 `.tools/python`，不修改系统环境：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run-native.ps1
```

直接使用 Cargo：

```powershell
$env:PYTHON3 = "$PWD\.tools\python\python.exe"
cargo run -p lattice --no-default-features --features native
```

默认 WebView 构建保持不变：

```powershell
cargo run -p lattice
```

## 结论

**继续保留为实验后端，不替换 WebView 默认后端。**

已经验证可用的部分：

- Flex/Grid 三栏布局整体成立。
- 中文字体、等宽代码字体和行号可以正常渲染。
- 内联 SVG 图标可以显示。
- 主题变量能够应用，四套主题共用同一令牌集。
- 字体层级、面板、状态栏和课程信息结构可辨识。

与 WebView 的差异：

- 半透明背景和 `box-shadow` 的实际合成结果不同，选中代码区域明显过亮，需要预混合实色与 renderer-specific CSS。
- `translateY(-50%)` 没有按预期生效；已改为不依赖百分比 transform 的 `calc()` 定位。
- 部分滚动、透明层、边框与阴影行为仍具有实验性。
- 调试工具链需要 Python；首次构建还要编译 Stylo、Vello、WGPU 等大量依赖。

当前通过 `data-renderer="native"` 隔离原生后端差异，业务组件不感知渲染器。等 Blitz 达到稳定级别后，再重新评估是否切换默认后端。

## 建议

- 原型和 P1 垂直切片先使用 WebView2，确保导师/用户体验稳定。
- 每个 UI 里程碑都在原生后端跑一次截图检查，避免积累不可移植的 CSS。
- 不要把原生渲染差异扩散进组件逻辑；所有适配放在 `assets/themes.css` 的 `data-renderer="native"` 下。
