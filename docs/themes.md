# 主题与设计令牌

Lattice 当前提供两种视觉风格，每种包含暗色与纸白版本：

| 主题 | ID | 风格 |
| --- | --- | --- |
| 无为 · 暗 | `wuwei-dark` | 墨色、宣纸、朱砂，衬线标题 |
| 无为 · 纸 | `wuwei-light` | 宣纸底色、墨色文字、朱砂强调 |
| 矩阵 · 暗 | `matrix-dark` | 黑底荧光绿、等宽标题、终端感 |
| 矩阵 · 白 | `matrix-light` | 白底森林绿、清爽高对比 |

旧会话中的 `dark`、`light`、`midnight` 会自动迁移到新主题。

## 令牌边界

- `assets/themes.css` 是唯一的颜色与阴影定义处。
- `assets/app.css` 只允许使用 `var(--token)`，不得出现十六进制、`rgb()` 或 `rgba()` 字面量。
- 标题字体、品牌标记、语法高亮、选区、滚动条、弹层阴影、代码行高亮全部令牌化。
- `app-shell` 显式设置 `color` 与 `background`，避免浅色主题继承到暗色文字令牌。

令牌按用途分组：

- 表面：`--bg`、`--panel`、`--panel-raised`、`--editor`、`--card`、`--field`
- 文字：`--text`、`--text-soft`、`--subtext`、`--muted`
- 强调：`--accent`、`--accent-bright`、`--accent-faint`、`--accent-soft`、`--accent-border`
- 状态：`--success-*`、`--insight-*`、`--danger-*`
- 语法：`--token-*`
- 交互：`--surface-*`、`--code-*`、`--selection-*`、`--scrollbar-*`
- 层级：`--shadow-*`、`--overlay-bg`
- 字体：`--sans`、`--mono`、`--heading-font`

## 自定义主题

1. 在 `themes.css` 复制一个主题块，改为新的 `data-theme` ID。
2. 填满同一组令牌；测试会检查四套主题是否都在每个主题块中定义了组件使用的变量。
3. 在 `Theme` 枚举中注册 ID、名称和描述，并在主题菜单中自动出现。
4. 不需要修改 `app.css` 或任何组件代码。

自动化约束：

- `app_css_contains_no_literal_colors`：组件样式不能写死颜色。
- `every_css_variable_used_by_components_is_defined_by_a_theme`：每个主题必须完整定义组件使用的令牌。
