# 知气 · Qi 设计系统与消费级体验规范（宋韵宣纸极简 2.0）

> 遵循 `nsh-design-guides`：全端颜色、圆角、字阶与视觉层级均源于 `theme.json`（`cssVars`）与 `nsh.json` 语义 Token。所有 UI 原语统一收口至 `ui/` 层（`ui/components.tsx`、`ui/charts.tsx`、`ui/icons.tsx`），业务页面（`App.tsx`、`pages/*.tsx`）仅组合原语而不直接导入 `nsh/base` 控件或手写硬编码裸色。

---

## 一、 视觉语言：宋韵宣纸极简 2.0（Song-Minimalist Vitalism 2.0）

以宋代青瓷与手工宣纸的「温润、留白、克制、有序」消解数字健康产品的打卡焦虑，让生理信号以自然呼吸的节奏呈现：

### 1. 双模式 Surface 与 Ink 层级

| 语义 Token | 浅色 · 宣纸晨光 | 深色 · 松烟夜息 | 适用场景 |
| :--- | :--- | :--- | :--- |
| `background` | `#F7F5F0` 温润宣纸 | `#121413` 沉静松烟 | 页面全局画布底色 |
| `card` | `#FFFFFF` 素绢瓷白 | `#1B1E1C` 墨玉卡片 | 一级信息卡片容器（`rounded-2xl`） |
| `surface-elevated` | `#FFFFFF` 浮层宣白 | `#222624` 轻抬墨石 | 悬浮指示器、胶囊滑块、浮层控件 |
| `surface-sunken` | `#EFECE4` 淡青石 | `#171A18` 内凹玄石 | 指标磁贴、数据阅读槽（同心 `rounded-xl`） |
| `border` | `#E4DFD3` 素麻细线 | `#2E3330` 暗青石线 | `1px` 极细分割与卡片边界 |
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
- **按需展开细节**：完整 30 天个人基线对照统一收纳进 `CollapsibleCard`，默认折叠保持首屏清朗，内部采用单容器 + `Hairline` 极细分割，避免多层套盒。

### 2. 视觉呼吸留白、同心圆角与对齐脊线（Concentricity & Alignment Spine）

- **同心圆角规律**：外层 `Card` 为 `rounded-2xl`（`16pt`），内嵌 `SunkenBox` 与 `ChoiceCard` 采用同心 `rounded-xl`（`12pt`），内层子图标槽 `rounded-lg`（`8pt`），保持内外间隙一致。
- **纵向脊线**：`NavRail`、`ChoiceCard` 与 `StatMetricTile` 为左侧图标分配固定槽位宽度（`w-5` / `size-9` / `w-4`），所有文字标题严格垂直对齐于同一脊线；可比数字启用 `tabular-nums` 且右对齐。
- **一卡一主操作、印章克制**：同一张卡片内最多一个实心主操作按钮（`variant="primary"`）；分类性质徽章（如「双周对比」「个人实证」「自动口径区分」）采用中性 `neutral` 浅底，高饱和语义色仅保留给真实状态变化与临床超标提示。

### 3. 完整交互四态与 44pt 触控安全区

- **四态全覆盖**：`ui/components.tsx` 内全部可点击原语完整实现 `hover:`（桌面静谧底色微变）、`active:`（按钮微缩 `0.98` 回弹；列表行/导航 Tab/折叠头仅加深底色不缩放）、`focus-visible:ring-2 focus-visible:ring-ring` 与 `disabled:opacity-45`。
- **触控热区与省略号语义**：按钮与导航空间均保证 `>= 40–44pt` 热区及 `hitSlop={4}`；打开二级抽屉或弹窗的触发按钮统一以单字符省略号 `…` 结尾（如 `速记…`、`跨设备恢复…`、`打开 QA 测试数据与诊断工坊…`、`物理清空本机健康数据…`）。

---

## 三、 组件与图表原语清单

- **矢量图标库（`ui/icons.tsx`）**：`QiIconName`（20 个 `24×24` Retina 圆头线性矢量图标）与 `QiIcon`，零外部字体依赖。
- **基础与导航原语（`ui/components.tsx`）**：
  - 容器与排版：`Card`、`SunkenBox`、`Hairline`、`SectionHeader`（含 `role="header"`）、`VitalBadge`、`StatMetricTile`、`ConsumerStatusPill`。
  - 交互控件：`ActionButton`（`sm | md | lg` 阶梯与 `rounded | pill` 形态）、`ChoiceCard`、`SelectChip`、`SwitchRow`、`FieldInput`、`MeterBar`。
  - 导航与浮层：`NavRail`（桌面侧栏原生 `spring_move` 滑块）、`BottomTabBar`（移动底栏原生 `spring_move` 滑块）、`SegmentedTabs`、`CollapsibleCard`、`ToastBanner`、`ModalSurface`、`SheetDialog`。
- **数据可视化原语（`ui/charts.tsx`）**：
  - `VitalityDialCanvas`（兼容别名 `VitalityRingCanvas`）：`136×136` 双层宋韵瓷盘仪表弧（外环 29 刻度点 + 等宽大数字，不再与右侧印章重复文案）。
  - `InteractiveRhythmPlot`（兼容封装 `RhythmSparklineCanvas`）：顶部选中日读数栏联动 `nsh/base/plot` 交互曲线。
  - `SleepArchitectureBar`：四期睡眠圆角分段条与分钟数图例。
  - `LabRangeBulletCanvas`：临床参考区间安全带与前后期对比子弹图。
  - `AcwrSweetSpotCanvas`（兼容别名 `AcwrZoneBarCanvas`）：恢复期 / 黄金适度区 / 偏累预警轨道。

---

## 四、 动效 Token 与 Motion Inventory（动效清单）

### 1. 标准动效时长 Token

| Token 档位 | 时长 / 物理参数 | 适用场景 | 减弱动态效果（`motion-reduce:`） |
| :--- | :--- | :--- | :--- |
| `instant` | `120ms ease-out` | 桌面悬停底色微变、边界亮度提示 | `motion-reduce:transition-none` |
| `fast` | `150ms–180ms` | 按钮/芯片按压回弹（`scale 0.98`）、选择行变色、Toast 进入 | 移除形变缩放，仅瞬时切换底色 |
| `base` | `240ms spring_move` | `NavRail`、`BottomTabBar`、`SegmentedTabs` 滑块切换与 `CollapsibleCard` 高度揭幕 | 直接落位选中项 / 即时展开 |
| `slow` | `300ms–360ms` | `MeterBar` 进度条增长、原生 `pageSheet`/`formSheet` 抽屉升降 | `motion-reduce:transition-none` |

### 2. 逐屏 Motion Inventory

| 页面 / 区域 | 触发交互 | 表现形式（NSH 原生实现） |
| :--- | :--- | :--- |
| 全局导航（`NavRail` / `BottomTabBar`） | 切换四大页面 | `<Tabs indicatorClass="...">` 原生 `spring_move` 单背景块滑移，不缩放 Tab |
| 全局顶栏 & 抽屉（`App.tsx` / `SheetDialog`） | 点击 `速记…` / `跨设备恢复…` / AI 授权 | `<Dialog presentation="pageSheet \| formSheet">` 系统原生层级升降 |
| 今日页（`TodayPage`） | 拖拽/悬停近 14 天趋势图 | `<Plot interactive={true}>` 原生十字准星与顶部读数栏毫秒联动 |
| 今日页（`TodayPage`） | 点击 30 天常态对照卡 | `<Collapsible reveal={true}>` 原生裁剪高度顺滑展开 |
| 记录页（`LogLabsPage`） | 切换 饮食 / 体检 / 穿戴 | `<SegmentedTabs>` 原生滑块位移 + 高频饮食 `ChoiceCard` 温润高亮 |
| 洞察页（`InsightsPage`） | 微习惯实验打卡 | `<MeterBar>`（`nsh/base` `Progress`）平滑推进填充条 + `ActionButton` 回弹 |
| 我的与引导（`MeVaultPage` / `OnboardingModal`） | 开关切换、选项点击 | `SwitchRow` 滑块平移动效 + `ChoiceCard` 选中面高亮 |

---

## 五、 语言与非医疗合规红线（Copy Rules）

- 全端统一使用「元气、节律、恢复参考、临界偏高提示、生活方式建议」；
- 严禁出现「诊断、确诊、治疗、处方、心律失常预警」等医疗器械词汇；
- 所有 AI 建议与体检解读卡片底部固定挂载不可关闭的非医疗声明尾注；短标题、占位符与按钮不加句末句号，日常提示避免感叹号。
