# 知气 · Qi 设计系统与动效规范（Design Standards）

> 遵循 `nsh-design-guides`：所有界面颜色、圆角与层级均来源于 `theme.json`（`cssVars`）与 `nsh.json` 语义 Token，业务页面严禁硬编码裸 Hex / RGBA 颜色，严禁跳过 `ui/` 基础组件层直接使用原生可点击容器。

## 一、 视觉语言：宋韵现代节律美学（Song-Minimalist Vitalism）

以宋瓷与宣纸的「留白、克制、温润、有序」缓解现代人的电量与打卡焦虑：

- **Surface 层级**：
  - 浅色（宣纸晨光）：底色 `#F7F5F0`（温润米纸）、卡片 `#FFFFFF`（素绢白）、下沉区 `#EFECE4`（淡青石）、描边 `#E4DFD3`
  - 深色（玄石夜息）：底色 `#121413`（玄石）、卡片 `#1B1E1C`（墨玉）、下沉区 `#242825`、描边 `#2E3330`
- **三态节律语义色（全局统一，附文字标签，绝不单靠颜色传意）**：
  - `vital-prime`（竹青 · 充沛）：浅 `#2A6F5B` / 深 `#4CA68A`（对应配对底色 `vital-prime-soft`）
  - `vital-steady`（琥珀 · 守中）：浅 `#B87D2B` / 深 `#D99E46`（对应配对底色 `vital-steady-soft`）
  - `vital-recover`（朱砂赭 · 调养）：浅 `#B84A39` / 深 `#E06C59`（对应配对底色 `vital-recover-soft`）
- **字阶与触控规范**：
  - 移动端优先，可触达按钮最小高度 `44pt`（`min-h-11`），正文 ≥ `15pt–16pt`，关键元气值与数字使用 `tabular-nums` 等宽对齐。
  - 圆角几何：页面主卡 `rounded-card`（`18px`），内部胶囊/子项 `rounded-xl`（`12px`），按钮与芯片 `rounded-lg` / `rounded-full`。

## 二、 目标平台规范

- **Mobile（V1.0 核心）**：iOS / Android 3-Tab（`今日` / `记录与体检` / `我的`）单手拇指友好布局，严格尊重 `env(safe-area-inset-*)`。
- **Desktop / Web（预览与 Phase 2 伴侣端）**：居中最大宽度容器，支持悬停态与键盘 Tab 焦点环。

## 三、 Motion Inventory（动效清单）

| 交互场景 | 实现机制（NSH 原生） | 减弱动态效果（`motion-reduce:`）策略 |
| :--- | :--- | :--- |
| 主/次按钮按压反馈 | `active:scale-[0.98] active:opacity-85 transition duration-150` | 保留透明度微变，移除缩放形变 |
| 底部 3-Tab 切换与胶囊高亮 | 选中态过渡 `transition-colors duration-200` + 面板切换 | 即时切换语义色与状态 |
| 元气电量条 / 睡眠蓄水池填充 | 宽度绑定信号变化 + `transition-all duration-300` | 直接落位目标百分比 |
| 快捷记录芯片选择与微行动勾选 | 图标/边框状态过渡 `transition-colors duration-150` | 即时切换选中印章样式 |
| 引导卡片 / 恢复码 / 开源致谢弹窗 | `Dialog` 系统原生蒙层与内容弹出 | 系统默认减弱动画降级 |

## 四、 语言与合规红线（Copy Rules）

- 全端使用「元气、节律、恢复参考、临界偏高提示、生活方式建议」；
- 严禁出现「诊断、确诊、治疗、处方、心律失常预警」等医疗器械词汇；
- 所有 AI 建议与体检解读卡底部固定挂载不可关闭的非医疗声明尾注。
