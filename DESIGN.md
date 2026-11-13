# 知气 · Qi 设计系统与消费级体验规范 3.0（三境共生 · 重场景轻仪表盘）

> 遵循 `nsh-design-guides`：全端颜色、圆角、字阶与视觉层级均源于 `theme.json`（`cssVars`）、`ui/themes.ts`（`setTheme` 三画风引擎）与 `nsh.json`。所有 UI 原语统一收口至 `ui/` 层（`ui/sanctuary.tsx`、`ui/components.tsx`、`ui/charts.tsx`、`ui/icons.tsx`），业务页面（`App.tsx`、`pages/*.tsx`）严禁直接导入 `nsh/base` 或硬编码裸色。

---

## 一、 三套灵境主题体系（100% 实装实时切换）

默认主推设计师推荐融合方案 **`cat_window`（《窗畔有猫 · 小满》）**，同时完整支持 **`zen_garden`（《掌上山水 · 养气灵境》）** 与 **`playful_bento`（《元气手账机 · 煤球》）** 一键热切换，各主题均具备完整的 Light / Dark 双色板且保持精确的 Semantic Token 键名对齐：

| 主题 ID | 画风名称 | 主视觉主角 & 剧场氛围 | 盖章隐喻 & 晨露藏品 |
| :--- | :--- | :--- | :--- |
| `cat_window`（默认推荐） | 窗畔有猫 · 小满 | 水彩木窗茶席 + 三花猫小满伸懒腰（日/充沛）vs 雨夜裹针织毯酣睡（夜/调养） | 猫爪肉垫印章 · 天青茶盏 / 金桂小瓶 / 编织软垫 / 暖纸夜灯 |
| `zen_garden` | 掌上山水 · 养气灵境 | 宋式设色微缩太湖石苍松 + 青瓷香炉轻烟（日）vs 石灯夜雨流萤（夜） | 朱砂方篆闲章 · 汝窑香茗 / 迎客苍松 / 寿山闲章 / 庭院石灯 |
| `playful_bento` | 元气手账机 · 煤球 | 3D 哑光软胶黑猫煤球 + 奶油抹茶囊舱（日）vs 薰衣草云枕靛紫夜舱（夜） | 立体星爪贴纸 · 抹茶云朵杯 / 奶油四叶草 / 立体星爪章 / 薰衣草夜灯 |

### 扩展语义令牌（`theme.json` & `ui/themes.ts`）

- **手作便当磁贴底色**：`bento-sage`、`bento-amber`、`bento-lavender`、`bento-peach`（浅/深模式自适应柔和釉色）；
- **印章与晨露情绪色**：`stamp-ink` / `stamp-soft`（朱砂/肉垫盖章），`dew-accent` / `dew-soft`（💧 晨露收集系统）；
- **插画剧场浮层遮罩与文字**：`stage-scrim`（半透深墨玻璃），`stage-ink`（高对比暖白字，保障任意插画背景上清晰可读）。

---

## 二、 「重场景、轻仪表盘」信息架构与温柔激励机制

1. **首屏上半部：无框感插画剧场（`SanctuaryStage`）**
   - 取消生硬后台仪表卡片壳，采用 `rounded-3xl` 细腻画框 + 无字高分水彩/软胶插画（`assets/scenes/*.jpg`）+ 轻量静态 `<Canvas>` 雨丝/茶烟/星芒点缀层；
   - 浮动顶栏胶囊承载 `今日元气指数`、`💧 晨露 ×N` 与 `今日日签…` 入口；
   - 底部展示当前已点亮的窗台摆件与 `轻抚互动` 弹性交互，配以紧贴角色口吻的零说教温暖对话气泡。
2. **首屏中段：错落手作便当磁贴 + 盖章打卡**
   - **上排错落双拼**：`夜间恢复`（`bento-sage`）与 `昨晚好眠`（`bento-lavender`，含四期睡眠彩带）；
   - **下排宽幅磁贴**：`身体负荷`（`bento-amber`）；若处于无穿戴 `tier_c` 模式，自然展示 `今日晨起精神感受` 五档选择；
   - **触感盖章打卡（`StampCheckinCard`）**：每日只推 1 件高杠杆微行动，点击即盖上微倾 `rotate: "-8deg"` 实体印章并立得 `+2 💧 晨露`；
   - **温柔零压力原则**：熬夜或 `recovery_priority` 状态时，伙伴自动进入裹毯/石灯/云枕夜息安抚模式，绝不扣分、绝不清零或责备用户。
3. **首屏下半部：身体手账折页（渐进披露）**
   - 将 `08:30 早安简报`（含负荷甜区轨道）、`近 14 天元气节律`（交互折线）、`查看 30 天个人常态对照` 统一收纳为简洁的 `CollapsibleCard` 手账折页，主视口清朗而不失专业深度。
4. **食光贴纸餐垫与小红书风「今日知气日签」**
   - 「记录 → 饮食随记」引入模切白边 `StickerMealTile` 高频饮食贴纸；
   - 点击 `今日日签…` 弹出 `DailyPosterModal` 竖版插画日签海报，一键复制治愈配文即可分享。

---

## 三、 动效 Token 与 Motion Inventory（动效清单）

> 严格遵守：空闲页面保持静止（禁止运行 `iterations: "infinite"` 的动画），所有交互动效在无动画偏好下平滑降级（`motion-reduce:` / `reduceMotion()`）。

| Token 档位 | 时长 / 物理参数 | 适用场景 | 减弱动态效果（`motion-reduce:`） |
| :--- | :--- | :--- | :--- |
| `instant` | `120ms ease-out` | 桌面悬停底色微变、画风胶囊切换指示 | `motion-reduce:transition-none` |
| `fast` | `150ms–180ms` | 按钮/贴纸按压回弹（`scale 0.98`）、轻抚互动响应 | 取消缩放，仅切换底色反馈 |
| `spring` | `createMotion` (`rubberBand` / `popIn`) | `SanctuaryStage` 轻抚伙伴回弹、`StampCheckinCard` 落印盖章 | 直接停驻终态，无弹跳 |
| `base` | `240ms spring_move` | `NavRail`、`BottomTabBar`、`SegmentedTabs` 滑块切换与手账折页展开 | 直接落位选中项 / 即时展开 |
| `slow` | `300ms–360ms` | `MeterBar` 进度推进、原生 `DailyPosterModal` / `SheetDialog` 升降 | `motion-reduce:transition-none` |

### 逐屏 Motion Inventory

| 页面 / 区域 | 触发交互 | 表现形式（NSH 原生实现） |
| :--- | :--- | :--- |
| 全局顶栏 & 导航（`App.tsx` / `NavRail` / `BottomTabBar`） | 切换四页签 / 切换画风与宣纸松烟 | `<Tabs>` 原生 `spring_move` 背景滑块 + `setTheme` 全端语义色彩瞬时平滑切换 |
| 今日剧场（`SanctuaryStage`） | 点击 `轻抚互动` / 场景区域 | `createMotion()` 播放一次性 `rubberBand` 轻弹并刷新温暖对话气泡、累积晨露 |
| 今日微行动（`StampCheckinCard`） | 点击 `完成今日微行动` | `createMotion()` 播放 `popIn` 盖章动效，落下 `-8deg` 实体猫爪/朱砂/星爪印章 |
| 今日日签（`DailyPosterModal`） | 点击 `今日日签…` | `<Dialog presentation="pageSheet">` 原生抽屉升起展示竖版插画日签海报 |
| 身体手账折页（`TodayPage`） | 展开早安简报 / 14 天曲线 / 30 天对照 | `<Collapsible reveal={true}>` 原生高度裁剪揭幕 + `<Plot interactive={true}>` 十字准星 |
| 食光贴纸与洞察（`LogLabsPage` / `InsightsPage`） | 轻贴饮食贴纸 / 实验打卡 | `StickerMealTile` 轻压回弹 + `<MeterBar>` 进度推进 + `KeepsakeShelfCard` 摆件点亮 |
| 我的与引导（`MeVaultPage` / `OnboardingModal`） | 切换三套画风预览 / 开关配置 | `ThemeSwitcherStrip` 选中高亮 + `SwitchRow` 滑块平移动效 |

---

## 四、 v4.0 «知气雅集 · Qi Atelier» 视觉与反胁迫商业伦理规范

### 1. v4.0 新增 8 组语义色彩 Token（Light / Dark 全三境适配）

在 `theme.json` 与 `ui/themes.ts`（`WorldTheme` / `WORLD_THEMES` / `semanticColor`）中新增 8 个雅集语义令牌，严禁在 `ui/*.tsx` 中硬编码 `#hex`：

| Token 键名 | 语义职责 | 典型应用场景 |
| :--- | :--- | :--- |
| `atelier-accent` / `atelier-soft` | 暖铜雅集主点缀与宣纸底 | `QiAtelierModal` 礼遇标签、雅集轨卡片、90 天归因效应强调 |
| `wax-seal` / `wax-seal-soft` | 慢递火漆红与浅封蜡底 | `CompanionLetterModal` 火漆信封、家书落款、`窗台添柴计划` |
| `duo-accent` / `duo-soft` | 双人同窗暖珊瑚点缀与柔底 | `CoSanctuarySheet` 串门徽章、金桂盆栽成长条、`UlnTrajectoryCanvas` 伴侣曲线 |
| `ink-lapis` | 宋韵石青墨色 | `DailyPosterModal` 石青印泥、藏书票边框、`UlnTrajectoryCanvas` 父母曲线、四时手卷标题 |
| `ink-gold` | 泥金画墨色 | `SanctuaryStage` 节气天象微标、`DailyPosterModal` 泥金印泥、宋瓷成就勋章 |

### 2. 新增矢量图标与可视化组件（`ui/icons.tsx` / `ui/charts.tsx` / `ui/sanctuary.tsx` / `ui/atelier.tsx`）

- **8 枚 24×24 Retina Canvas 矢量图标**：`SolarTermIcon`、`WaxSealIcon`、`EnvelopeIcon`、`CoSanctuaryIcon`、`WidgetIcon`、`HandscrollIcon`、`BadgeCeramicIcon`、`FamilyHeartIcon`。
- **深度洞察图表（`ui/charts.tsx`）**：
  - `UlnTrajectoryCanvas`：多亲友（`本人` / `伴侣` / `父母`）`%ULN` 归一化曲线，含红色虚线 `100% ULN 参考上限`、档案筛选胶囊与检视时点精确读数。
  - `MultiLagRadarBars`：90 天窗口 `D+0 / D+1 / D+2` 双向归因效应条 + Cohen's `d` 徽章 + 已控混杂芯片 + 样本量 `n`。
- **灵境剧场与日签升级（`ui/sanctuary.tsx`）**：
  - `SanctuaryStage` 新增可选 `solarTerm`、`lunarPhase`、`weatherMood`（`misty_rain` 映射 `solar_spring_rain.jpg`，`first_snow` 映射 `solar_winter_hearth.jpg`）、`equippedKeepsakes`（至多 3 件窗台摆件）、`partnerMoodBadge` 与 `atelierUnlocked`。
  - `DailyPosterModal` 新增可选 `sealInk`（`cinnabar` / `gold` / `lapis`）三色印泥与 `hideNumbers`（隐藏数字只留诗意分享）。
- **知气雅集组件套件（`ui/atelier.tsx`）**：
  - `QiAtelierModal`（7 档透明方案 + 30 天陪伴礼遇价 + 窗台添柴公益 + 3 步 Blinkist 透明 7 日线 + 红线承诺）
  - `SeasonalPassPanel`（清欢/雅集双轨季票 + 往期节气博物架 + 服饰/印泥/天象/3 格摆件）
  - `StickerCameraMatPanel`（水彩/拍立得/青瓷 Canvas 模切贴纸造物机 + 5 张/月免费额 + 食光月历垫）
  - `CompanionLetterModal`（《周日夜话慢递》火漆信封 + 暖米信纸 + 节气藏书票）
  - `CoSanctuarySheet`（《同窗共养》1 字节心情 + 递热茶/盖薄被/按爪印 + 金桂盆栽 + 零日记泄漏盾牌）
  - `FourSeasonsScrollModal`（《我的四时身体风物长卷》横向卷轴 + 宋瓷勋章）
  - `WristWidgetPreviewCard`（《抬腕嘴替》诗意隐喻 vs 精准数值预览）
  - `FamilyPeaceLetterModal`（父母大字版端到端加密《气韵平安家书》）

### 3. 反胁迫与伦理付费墙原则（Ethical Paywall Principles）

1. **核心健康权永久不锁**：原始生理指标、单人化验单 OCR、本地 Ollama 晨报、JSONL / `.qivault` 明文导出及「安心静养（`recovery_priority`）」模式永不设限。
2. **零焦虑推广时机**：当用户处于低电量静养状态时，绝不弹出会员引导弹窗；订阅入口仅作为轻量温润选项呈现。
3. **无暗黑模式（Anti-Dark Patterns）**：禁止倒计时紧迫感逼单、禁止羞辱式拒绝文案（次选 CTA 始终是平静的「继续素心清欢」或「先逛逛」），支持随时退出且历史资产永久落入可读可导出的「只读保险箱」。
