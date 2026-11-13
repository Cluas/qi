# 知气 · Qi 设计系统与消费级体验规范（宋韵宣纸极简 2.0）

> 遵循 `nsh-design-guides`：全端颜色、圆角、字阶与视觉层级均源于 `theme.json`（`cssVars`）与 `nsh.json` 语义 Token。业务页面严禁硬编码裸 Hex / RGBA 颜色，严禁绕过 `ui/` 原语层直接手写无语义容器；图标与 Canvas 必须在 `draw` 回调内部通过 `token(...)` 动态取色，确保浅色「宣纸」与深色「松烟」实时切换即时重绘。

---

## 一、 视觉语言：宋韵宣纸极简 2.0（Song-Minimalist Vitalism 2.0）

以宋代青瓷与手工宣纸的「温润、留白、克制、有序」消解数字健康产品的打卡焦虑，让生理信号以自然呼吸的节奏呈现：

### 1. 双模式 Surface 与 Ink 层级

| 语义 Token | 浅色 · 宣纸晨光 | 深色 · 松烟夜息 | 适用场景 |
| :--- | :--- | :--- | :--- |
| `background` | `#F7F5F0` 温润宣纸 | `#121413` 沉静松烟 | 页面全局画布底色 |
| `card` | `#FFFFFF` 素绢瓷白 | `#1B1E1C` 墨玉卡片 | 一级信息卡片容器 |
| `surface-elevated` | `#FFFFFF` 浮层宣白 | `#222624` 轻抬墨石 | 悬浮指示器、胶囊滑块、浮层控件 |
| `surface-sunken` | `#EFECE4` 淡青石 | `#171A18` 内凹玄石 | 指标磁贴、数据预览槽、分段底座 |
| `border` | `#E4DFD3` 素麻细线 | `#2E3330` 暗青石线 | 1px 极细分割与卡片边框 |
| `foreground` | `#181A18` 徽墨主字 | `#EDEAE2` 暖玉主字 | 标题、核心读数、正文 |
| `muted-foreground` | `#666B65` 淡墨次字 | `#9A9E96` 烟灰次字 | 副标题、说明文案、单位 |
| `ink-subtle` | `#8C918A` 浅石辅字 | `#757A73` 暗石辅字 | 坐标轴刻度、参考线标注、弱提示 |

### 2. 三态节律与睡眠四期语义色（颜色与文字成对出现，绝不单靠色块传意）

- **三态节律语义色**：
  - `vital-prime`（竹青 · 充沛）：浅 `#2A6F5B` / 深 `#4CA68A`（配对底色 `vital-prime-soft`）
  - `vital-steady`（琥珀 · 守中）：浅 `#B87D2B` / 深 `#D99E46`（配对底色 `vital-steady-soft`）
  - `vital-recover`（朱砂赭 · 调养）：浅 `#B84A39` / 深 `#E06C59`（配对底色 `vital-recover-soft`）
- **睡眠四期语义色（低饱和矿物色系）**：
  - `sleep-deep`（深睡修复）：浅 `#2A6F5B` / 深 `#4CA68A`
  - `sleep-core`（浅睡平稳）：浅 `#5E9683` / 深 `#357A66`
  - `sleep-rem`（快速眼动 / 梦境整理）：浅 `#8574A8` / 深 `#9C8BC4`
  - `sleep-awake`（短暂微醒）：浅 `#D49A45` / 深 `#D99E46`

---

## 二、 消费级 UI/UX 核心原则

### 1. 去工程化渐进披露原则（Progressive Disclosure & De-Engineering）

- **主界面零统计黑话**：首页与一级视图严禁直接裸露 `Z = +1.42`、`RMSSD`、`SDNN`、`ACWR 7d/28d`、`p50/p95` 等工程与统计符号。
- **人话翻译层**：通过 `ConsumerStatusPill` 与自然语言副标题，将底层统计态转译为日常感知（如「状态充沛」「处于常态」「略偏低于平时」「需多留意休息」「基线积累中」）；负荷轨道采用「恢复期 / 黄金适度区 / 偏累预警」大众语汇。
- **按需展开细节**：完整 30 天个人基线对照、采样来源、算法口径等专业数据统一收纳进 `CollapsibleCard`，默认折叠保持首屏清朗，点击后通过原生弹簧高度动画顺滑揭示。

### 2. 视觉呼吸留白规范（Visual Breathing & Rhythm Spacing）

- **纵向节律**：页面一级卡片间保持 `gap-4`（16pt）至 `gap-5`（20pt）呼吸间距；卡片内边距统一 `p-4`（16pt），卡片内部元素间距 `gap-3` / `gap-3.5`。
- **层次克制**：避免卡片套卡片超过两层；次级指标并排采用 `SunkenBox` 或 `StatMetricTile` 沉降排版，以底色差代替厚重描边。
- **数字排版**：所有元气评分、心率、时长、化验单数值强制启用 `tabular-nums` 等宽数字对齐，防止滚动或交互悬停时文字水平抖动。

### 3. 44pt 触控安全区规范（44pt Touch Target & Safe Area）

- **最小热区**：所有主按钮、分段切换 Tab（`SegmentedTabs`）、折叠卡片触发头部（`CollapsibleCard`）、开关行及导航控件最小高度 `min-h-11`（44pt）；紧凑型辅助选择芯片不低于 `min-h-10`（40pt）并保留充足水平内边距（`px-3.5`）。
- **屏幕安全区**：根容器与模态抽屉（`SheetDialog`）严格适配 `env(safe-area-inset-top / bottom / left / right)`，杜绝刘海屏、灵动岛与底部手势条遮挡。

### 4. 图表语义与颜色克制标准（Chart Semantics & Color Restraint）

- **主视觉单焦点**：折线/面积趋势图主曲线统一采用 `primary` 单色相渐变填充，参考网格与基准线采用低对比度 `border` 虚线（`[3, 3]`），避免多条高饱和折线同框竞争。
- **交互先于静态**：`InteractiveRhythmPlot` 基于 `nsh/base/plot` 原生手势与指针悬停驱动顶部选中日读数栏实时联动；空数组状态下必须降级渲染柔和提示态，严禁越界索引。
- **临床边界可视化**：`LabRangeBulletCanvas` 以 `vital-prime-soft` 标示临床参考安全带，空心圆点表示上期对照值，高对比实心圆点（带卡片底色外晕环）标示本期读数，一眼看清位置与趋势变化。
- **全 Canvas 动态取色**：图标库（`QiIcon`）与全套自绘仪表盘（`VitalityDialCanvas`、`AcwrSweetSpotCanvas`、`SleepArchitectureBar`、`LabRangeBulletCanvas`）全部在 `draw` 回调内实时调用 `token(...)` 读取颜色，保证主题切换零延迟重绘。

---

## 三、 组件与图表原语清单

- **矢量图标库（`ui/icons.tsx`）**：`QiIconName`（20 个 24×24 Retina 圆头线性矢量图标）与 `QiIcon`，零外部字体依赖。
- **消费级 UI 原语（`ui/components.tsx`）**：
  - 基础容器与控件：`Card`、`SunkenBox`、`SectionHeader`、`VitalBadge`、`ActionButton`（支持前后语义图标）、`SelectChip`、`SwitchRow`、`FieldInput`、`SheetDialog`。
  - 消费级新增原语：`SegmentedTabs`（原生弹簧滑动指示器）、`CollapsibleCard`（`reveal` 顺滑高度展开）、`StatMetricTile`、`ConsumerStatusPill`。
- **数据可视化原语（`ui/charts.tsx`）**：
  - `VitalityDialCanvas`（兼容别名 `VitalityRingCanvas`）：136×136 双层宋韵瓷盘仪表弧。
  - `InteractiveRhythmPlot`（兼容封装 `RhythmSparklineCanvas`）：顶部读数栏联动 `nsh/base/plot` 交互曲线。
  - `SleepArchitectureBar`：四期睡眠圆角分段条与分钟数图例。
  - `LabRangeBulletCanvas`：临床参考区间安全带与前后期对比子弹图。
  - `AcwrSweetSpotCanvas`（兼容别名 `AcwrZoneBarCanvas`）：恢复期 / 黄金适度区 / 偏累预警轨道。

---

## 四、 Motion Inventory（动效清单）

| 交互场景 | 实现机制（NSH 原生） | 减弱动态效果（`motion-reduce:`）策略 |
| :--- | :--- | :--- |
| 主/次按钮与芯片按压 | `active:scale-[0.98] active:opacity-85 transition duration-150` | 保留透明度微变，移除缩放形变 |
| 分段控件滑块切换 | `nsh/base` `<Tabs>` 原生弹簧指示器位移 | 直接落位选中项 |
| 渐进披露折叠展开 | `nsh/base` `<Collapsible reveal={true}>` 原生裁剪高度动画 | 即时展开/收起内容 |
| 趋势图指针悬停联动 | `<Plot interactive={true}>` 十字准星、高亮点与顶部读数同步 | 保持静态高亮，移除过渡渐变 |
| 模态抽屉弹出 | `<Dialog presentation="pageSheet">` 系统原生层级呈现 | 遵循系统无障碍减弱动画 |

---

## 五、 语言与非医疗合规红线（Copy Rules）

- 全端统一使用「元气、节律、恢复参考、临界偏高提示、生活方式建议」；
- 严禁出现「诊断、确诊、治疗、处方、心律失常预警」等医疗器械词汇；
- 所有 AI 建议与体检解读卡片底部固定挂载不可关闭的非医疗声明尾注。
