# 知气（Qi）iOS 原生体验设计规范与体验团队宪章 v7.0

> 遵循 `nsh-design-guides`、Apple Human Interface Guidelines（iOS 18 / iOS 26 Liquid Glass）与 NSH 0.5.0 架构规范：全端视觉令牌源于 `theme.json`、`ui/themes.ts` 与 `nsh.json`。所有底层原语统一封装于 `ui/` 目录，`App.tsx` 与 `pages/*.tsx` 严禁直接导入 `nsh/base` 或 `nsh/mobile`，严禁硬编码裸色与中文文案。

---

## 一、 完整 UX 体验团队（Qi Experience Team）编制与闭环

为确保持续达到 App Store 获奖级健康手账体验（对标 *Apple Health*、*Gentler Streak*、*Apple Journal*、*Things 3*），设立 5 席位体验团队闭环运行：

1. **体验总监与 HIG 系统架构师（Coordinator）**：负责全站视觉纯净度、中性分组色板、`nsh/mobile` 原生 `UITabBar` / Large Title / Inset Grouped List 原语、全屏感官引导动效（`OnboardingModal`）及信息减法把关。
2. **今日主页与插画意境设计师（`ux-today-sanctuary`）**：打磨 `TodayPage` 与 `SanctuaryStage` 的呼吸留白、单一主焦点与轻量落印，清除冗余数字、徽章与双卡片拥挤感。
3. **手账记录与健康图表设计师（`ux-log-insights`）**：打磨 `LogLabsPage`、`InsightsPage` 与 `ui/charts.tsx`，以纯净连续样条曲线与渐进披露替换高认知负荷表格与千卡堆砌。
4. **个人中心与雅集工坊设计师（`ux-me-atelier`）**：打磨 `MeVaultPage` 与 `ui/atelier.tsx`，严格落实 iOS Settings 单行清透行高与唯一职责分组。
5. **Apple HIG 视觉与交互体验评审官（`ux-critic`）**：独立审阅 `393×852` 浅/深色 iOS 竖屏渲染截图，逐项拦截“做题式引导、数据杂乱、拼贴感、框套框、撞色与文字截断”，未通过不准发布真机。

---

## 二、 iOS 原生设计铁律（v7.0 感官序幕与减法秩序标准）

1. **v7.0 零做题与信息减法总则（Subtraction Law）**
   - **首启零做题（Animated Sensory Prelude）**：首次进入知气时，严禁出现目标勾选清单、穿戴模式单选题、主题选择器与大段隐私说明列表；直接以全屏极光水雾 WGSL 着色器（`QI_PRELUDE_AURORA_WGSL`）+ 60fps 呼吸元气环 `<Canvas>` + 3 段柔和诗意节拍呈现在用户眼前，触碰光晕产生弹性涟漪，单手一键「推开窗扉」无缝丝滑溶入今日窗台（默认设定 `energy_sleep` / `tier_a` / `cat_window`，日后皆可微调）。
   - **主屏视觉预算（Max 3 Groups）**：任何一级页首屏不超过 3 个视觉重心模块；同一屏幕绝不重复出现两个相同数值（如元气分）；折叠态 Settings 行严格保持单行可扫读（零多行灰色长文）；XML / 原始导入框 / 多档案配置 / N-of-1 高阶实验工坊默认收入可展开抽屉或折叠卡片。

2. **命名与双语契约**
   - 中文全称严格为 **「知气」**；英文全称严格为 **`Qi`**；严禁出现混合拼接标题或慈善/捐赠字眼。
   - 所有可见文案及 `accessibilityLabel` 由 `ui/i18n.ts` 的 `tr(key, vars?)` 驱动，支持中英文瞬时热切换。

3. **纯净中性分组画布（Apple Grouped Neutral Palette）**
   - **浅色画布**：清透微瓷白中性背景（`cat_window: #F3F4F6` / `zen_garden: #F1F4F2` / `playful_bento: #F5F4F8`）配 `#FFFFFF` 纯净分组卡片与 `0.5px` 级微弱边界（`border-border/45`）；严禁全屏铺设泛黄泥泞底色或高饱和彩色便当底块大面积撞色。
   - **深色画布**：纯粹 iOS OLED `#000000` 画布配 `#1C1C1E` 一级分组卡片与 `#2C2C2E` 次级填充，确保夜间对比清晰、不刺眼。
   - **克制点睛主色**：全局主色仅保留温润宋瓷青（浅 `#1B6B5A` / 深 `#48CFAF`），其余体征色仅作极简图标、进度条或微标签点缀。

4. **正统系统导航结构（UITabBar + Large Title + Slim SegmentedControl）**
   - **底部导航**：真实 iOS 宿主直接使用 `nsh/mobile` `<TabBar>`（iOS 26 Liquid Glass 原生毛玻璃 + SF Symbols：`sun.max.fill`、`square.and.pencil`、`chart.xyaxis.line`、`person.crop.circle`），桌面宿主（`width >= 920`）使用 `<NavRail>`，无头窄屏预览走 1:1 高保真 4 槽 `UITabBar` 形态；高频 `+ 速记` 优雅收口于右上角轻胶囊工具按钮。
   - **页面顶部（`IosLargeHeader`）**：四大一级页顶部保留简洁左对齐大标题（移除多余顶部副标题句子）与干净右侧胶囊动作栏。
   - **二级分段切换（`SegmentedTabs`）**：采用苹果标准 `36pt`（`h-9`）纤薄滑块分段控件。

5. **反拼贴、反套娃与反假拟物铁律**
   - **零歪斜假胶带、零红字未读轰炸**：`SanctuaryStage` 窗台插画保留完整 `rounded-3xl` 画面与底部 5 项快捷条（用 5px 小圆点代替红色数字角标）。
   - **零锯齿撕纸小票 & 零千卡噪音**：`SerratedReceiptBanner` 保持中性分割；`近期落笔` 列表只显示项目名称与日期，不在日记流中堆砌加粗 `kcal` 数字。
   - **单归属边界法则（Single-Owner Boundaries）**：卡片内部次级区块无多余描边；设置页折叠态使用单行 `IosGroupedSection` + `IosListRow`；图表区消除红绿交通灯底块条。
   - **全局 `tabular-nums` & `>= 44pt` 热区**：所有数值强制等宽数字；所有可点击按钮与行满足 44pt+ 热区或 `hitSlop`。

---

## 三、 NSH 动效清单（Motion Inventory）

| 交互或状态变化 | 实现原语与令牌 | 低动效（`reduceMotion()`）退避 |
| --- | --- | --- |
| 首启全屏序幕极光背景 | `<PreludeAuroraShader>`（`QI_PRELUDE_AURORA_WGSL`，`alpha <= 0.34`） | `paused={true}`，停驻静止柔光底色 |
| 首启中央元气呼吸光环 | UI 线程 `Animated.loop`（`4.4s` 呼/吸缓动）+ `<Canvas>` `AnimatedNode` 半径/透明度绑定 | 停驻 0.45 静态平衡态三环 |
| 轻触序幕呼吸光环 | `createOrbRippleMotion()`（`scale: 0.94 -> 1.05 -> 1.0` spring）+ `hapticSelection()` | 即刻跳过缩放脉冲，仅切换文案节拍 |
| 序幕 3 段诗意文案轮换 / 推开窗扉入场 | `<SceneCrossfade>`（`SOLAR_CROSSFADE_WGSL`，`slow: 360ms` / `slower: 500ms`） | `duration = 0`，`fallback = "none"` 瞬切 |
| 常规按钮与卡片轻按反馈 | `active:scale-[0.97]` + `transition-colors duration-150` | `motion-reduce:active:scale-100 motion-reduce:transition-none` |
| 晨起体感折叠 / 折叠卡片展开 | `runDrawerLayoutTransition("easeInEaseOut")` | 直接切换布局，不触发动画 |
| 窗台抚摸陪伴 / 日签落印 | `createPetBounceMotion()` / `createSealStampMotion()` spring | `motion.reset()` 即刻落定终态 |
