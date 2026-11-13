# 知气 (Qi) 设计系统、空间收纳与物理动效规范 v5.0

> 遵循 `nsh-design-guides` 与 NSH 0.5.0 规范：全端颜色、圆角、字阶与视觉层级均源于 `theme.json`（`cssVars`）、`ui/themes.ts`（三境实时画风引擎）与 `nsh.json`。所有 UI 原语、GPU 着色器与物理动效统一收口至 `ui/` 层（`ui/motion.ts`、`ui/components.tsx`、`ui/sanctuary.tsx`、`ui/charts.tsx`、`ui/icons.tsx`、`ui/atelier.tsx`、`ui/i18n.ts`），业务入口与页面（`App.tsx`、`pages/*.tsx`）严禁直接导入 `nsh/base` 或硬编码裸色与中文文案。

---

## 一、 品牌与三套灵境主题体系（100% 实装实时切换）

- **品牌命名规范**：中文端全称严格为 **「知气」**；英文端全称严格为 **`Qi`**；严禁出现混拼或冗余分隔副标。
- **双语国际化契约**：全端所有可见文案、无障碍标签（`accessibilityLabel`）、错误提示与海报落款均由 `ui/i18n.ts` `tr(key, vars?)` 驱动，支持 `zh-CN`（雅致东方手账汉语）与 `en`（Sentence case 自然英语）即时热切换。

默认主推设计师推荐方案 **`cat_window`（《窗畔有猫 · 小满》）**，同时完整支持 **`zen_garden`（《掌上山水 · 养气灵境》）** 与 **`playful_bento`（《元气手账机 · 煤球》）** 一键水墨水光转场切换，各主题均具备完整的 Light / Dark 双色板且保持精确的 Semantic Token 键名对齐：

| 主题 ID | 画风名称 | 主视觉主角 & 剧场氛围 | 盖章隐喻 & 晨露藏品 |
| :--- | :--- | :--- | :--- |
| `cat_window`（默认推荐） | 窗畔有猫 · 小满 | 水彩木窗茶席 + 三花猫小满伸懒腰（日/充沛）vs 雨夜裹针织毯酣睡（夜/调养） | 猫爪肉垫印章 · 天青茶盏 / 金桂小瓶 / 编织软垫 / 暖纸夜灯 |
| `zen_garden` | 掌上山水 · 养气灵境 | 宋式设色微缩太湖石苍松 + 青瓷香炉轻烟（日）vs 石灯夜雨流萤（夜） | 朱砂方篆闲章 · 汝窑香茗 / 迎客苍松 / 寿山闲章 / 庭院石灯 |
| `playful_bento` | 元气手账机 · 煤球 | 3D 哑光软胶黑猫煤球 + 奶油抹茶囊舱（日）vs 薰衣草云枕靛紫夜舱（夜） | 立体星爪贴纸 · 抹茶云朵杯 / 奶油四叶草 / 立体星爪章 / 薰衣草夜灯 |

### 语义令牌体系（`theme.json` & `ui/themes.ts`）

- **手作便当磁贴底色**：`bento-sage`、`bento-amber`、`bento-lavender`、`bento-peach`（浅/深模式自适应柔和釉色）；
- **印章与晨露情绪色**：`stamp-ink` / `stamp-soft`（朱砂/肉垫盖章），`dew-accent` / `dew-soft`（晨露收集系统）；
- **插画剧场浮层遮罩与文字**：`stage-scrim`（半透深墨玻璃），`stage-ink`（高对比暖白字，保障任意插画背景上清晰可读）；
- **雅集与扩展 8 组令牌**：`atelier-accent` / `atelier-soft`、`wax-seal` / `wax-seal-soft`、`duo-accent` / `duo-soft`、`ink-lapis`、`ink-gold`。

---

## 二、 「一屏一境 · 空间化收纳」信息架构（v5.0 Spatial Decluttering）

针对「场景主画布清爽呼吸、重信息工具抽屉收敛」的目标，v5.0 确立以下四项空间架构铁律：

1. **Sanctuary 剧场极简角标律（画幅之上至多 2 枚静默角标）**
   - 插画主框（`SanctuaryStage`，`rounded-3xl`，无厚重卡片边框）内部**严禁堆叠多排文字胶囊**，仅允许左上与右上至多 **2 枚轻薄毛玻璃角标**：
     - **左上角标**：今日元气读数（Score）+ 状态微标（如 `顺势舒展` / `Calmly Ready`）；
     - **右上角标**：当前物候/晨露计数（或一键展开 `今日日签` 的隐蔽轻按键）。
   - 插画中央与下缘 75%+ 面积完全留白给自然光线、GPU 环境微动着色器与陪伴主角。

2. **自定义爱宠模切框与星光念想模式（Custom Pet Companion & Starlight Memorial）**
   - 当用户配置了 `customPet`（来自端侧自选写真路径 / 本地裁切头像）时，在窗台焦位渲染带 **2px 宣纸模切白边 + 柔和落地投影** 的实拍宠物画框，替代或依偎于默认画风伙伴旁，支持点击触发 **`Animated.spring` 轻弹抱抱反馈**；
   - 若开启 `starlight_memorial`（星光念想模式），自动将环境着色器切换为微凉月银与暖金流萤交织的 **`sun_motes / starlight`** 频谱，并在宠物画框周缘附着低噪静谧星芒角标与温柔思念寄语，绝不打扰主阅读流。

3. **底缘「窗台物候抽屉 / Spatial Scene Dock」**
   - 紧贴插画剧场底部边缘（位于画框外底沿，绝不遮压插画主体），收纳 4 个高频空间动作按键，采用 44pt+ 触控热区与轻量图标＋短词：
     - **食光（Meals）**：直达饮食贴纸拍立得餐垫与 3 秒快记；
     - **慢递家书（Letters）**：开启火漆封缄《周日夜话慢递》或父母大字平安家书；
     - **同窗（Co-Window）**：查看伴侣 1 字节心情、递热茶/按爪印与金桂盆栽；
     - **伙伴（Pet）**：进入自定义宠物设置、窗台 3 格摆件整理与星光念想开关。

4. **全站次级页面 `SegmentedTabs` 三子视口收纳 + 渐进披露**
   - 各一级页面通过基于原生 `<Tabs>`（`spring_move` 滑动药丸指示器）的 `SegmentedTabs` 将长页面收敛为 3 个清晰空间：
     - **`TodayPage`**：1 屏 Sanctuary 剧场 + 2 枚错落手作便当磁贴 + 1 枚触感盖章微行动 + `CollapsibleCard` 折叠手账（早安简报 / 14 天节律 / 30 天个人常态）；
     - **`LogLabsPage`**：`[随记与饮食 | 体检与实验室 | 试验场]` 三子视口；
     - **`InsightsPage`**：`[节律趋势 | 因果归因 | 四时风物]` 三子视口；
     - **`MeVaultPage`**：`[灵境与宠物 | 端侧保险箱 | 雅集与偏好]` 三子视口。

---

## 三、 WGSL 着色器与物理动效清单（Motion & Shader Inventory）

> 严格遵守 NSH 0.5.0 静态门禁与渲染契约：
> - 全工程 **禁止无限循环 JS 动画**（所有状态过渡与反馈均为有界单次动画）；
> - 全工程 **统一使用 `<If when={...}>` 或 TSX 条件表达式**，严禁使用旧式 Show 标签；
> - 所有 WGSL 着色器均通过 `<Shader paused={reduceMotion()} accessibilityElementsHidden={true} />` 或 `<ShaderTransition>` 挂载，仅实现 `fn shade(uv: vec2f) -> vec4f`（直通非预乘 sRGB + Alpha，`uv` 为左上角 `0..1`），内置字段仅访问 `nsh.size / nsh.time / nsh.scale / nsh.progress / nsh.overflow`，转场访问 `sample_from(uv) / sample_to(uv)`，自定义 `struct Uniforms` 严格 **16-byte 对齐**（`intensity: f32, warmth: f32, wind: f32, seed: f32`），绝不手写 `@fragment`、`@group` 或 `@binding`。

### 1. GPU WGSL 着色器契约与清单（`ui/motion.ts`）

| 着色器常量 / 组件 | 触发与适用境态 | WGSL 算法与 Uniforms（16-byte 对齐） | `reduceMotion()` 降级策略 |
| :--- | :--- | :--- | :--- |
| `RAIN_GLASS_WGSL`<br>(`<AmbientSceneShader mode="rain" />`) | `weatherMood === "misty_rain"` 或微雨调养节律 | 3 层景深对角春雨细丝 + 窗面湿润水雾柔折射高光；Alpha 克制在 `<= 0.24` 不掩盖底图。<br>`Uniforms { intensity, warmth, wind, seed }` | `paused={true}` 冻结 `nsh.time`，仅展示极淡静态细雨微润窗晕（或完全透明退场）。 |
| `HEARTH_STEAM_WGSL`<br>(`<AmbientSceneShader mode="hearth_steam" />`) | `weatherMood === "first_snow"`、夜间静养或冬日围炉 | 底部炭火低频呼吸暖橘光晕 + 两束正弦摇曳茶烟丝带渐消；`warmth` 控制琥珀/霜雪比重。<br>`Uniforms { intensity, warmth, wind, seed }` | `paused={true}` 停在静态暖炉柔晕帧，不运行呼吸闪烁与茶烟上飘。 |
| `SUN_MOTES_WGSL`<br>(`<AmbientSceneShader mode="sun_motes" \| "starlight" />`) | 日间晴暖 / 夜间萤火 / `customPet.starlight_memorial` | 柔焦暖阳尘埃漂浮 + 夜间/星光模式下转换为月银微金流萤光点，聚焦保护自定宠物区域。<br>`Uniforms { intensity, warmth, wind, seed }` | `paused={true}` 锁定星尘/光斑位置为静谧微点缀，禁止连续漂流。 |
| `SOLAR_CROSSFADE_WGSL`<br>(`<SceneCrossfade value={key} />`) | 主题画风切换、`weatherMood` 变化或日夜切换（`380ms`） | 基于 `<ShaderTransition>`，沿对角墨晕 + 暖阳柔光扫过 `sample_from(uv)` 渐融至 `sample_to(uv)`，边缘伴极轻水墨噪点羽化。<br>`Uniforms { intensity, warmth, wind, seed }` | `paused={true}` 且 `<ShaderTransition fallback="none">`，零延迟直接切换新场景。 |

### 2. 物理动效与原生组件交互清单（`ui/motion.ts` & `ui/components.tsx`）

| 动效原语 / Preset | 物理参数 / NSH 实现 | 适用交互场景 | `reduceMotion()` 降级策略 |
| :--- | :--- | :--- | :--- |
| `MOTION_TOKENS` | `instant: 120ms`、`fast: 180ms`、`base: 240ms`、`slow: 360ms`、`slower: 500ms`；柔顺曲线 `Easing.bezier(0.22, 1, 0.36, 1)` | 全端按钮按压、徽章浮现、抽屉与卡片层级过渡时间基准 | 全部回退为即时状态赋值（`0ms` 无弹跳）。 |
| `createPetBounceMotion()` | `Animated.spring`（`stiffness: 260, damping: 15, mass: 0.9`）：点击时微压至 `0.94` 再柔弹至 `1.04` 归位 `1.0` | `SanctuaryStage` 轻抚画风伙伴 / 自定义实拍宠物框 | 跳过 `.spring()`，直接调用 `.set(1)` 并即时触发对话/晨露反馈。 |
| `createSealStampMotion()`<br>（`SealStamp` 组件） | `createMotion` + `Animated.spring`（`stiffness: 300, damping: 18`）：盖印落锤下压 `0.92 -> 1.0` 并稳停 `-8deg` | `StampCheckinCard` 每日微行动落印、火漆家书封缄确认 | 跳过落锤缩放，直接渲染静态 `-8deg` 已盖印章态。 |
| `createPolaroidTiltMotion()` | `createMotion` 绑定 `[{ perspective: 800 }, ...motion.transform]`：轻微 `rotateX` / `rotateY`（`±4deg`）＋ 缩放 `0.985` | `DailyPosterModal` 竖版日签预览、`StickerMealTile` 模切食光贴纸按压 | 撤销三维透视倾角，保持平坦正视态。 |
| `runDrawerLayoutTransition()` | `LayoutAnimation.configureNext(LayoutAnimation.Presets.easeInEaseOut)`（240ms） | Spatial Scene Dock 面板切出、健康手账状态变更 | 检查 `if (reduceMotion()) return;` 后直接更新状态，无布局补间。 |
| `SegmentedTabs` / 导航指示器 | `nsh/base` `<Tabs>` 原生 `spring_move` + `indicatorClass` 药丸滑块（不额外叠加 JS 坐标动画） | `NavRail`、`BottomTabBar`、所有三子视口 `SegmentedTabs` | 原生 `<Tabs>` 自动随系统无障碍减弱动态偏好直接落位。 |
| `CollapsibleCard` | `nsh/base` `<Collapsible reveal={true}>` 原生高度裁剪展开 | 早安简报、14 天趋势、30 天个人常态对照折页 | 原生即刻展开/收起内容高度。 |
| `DrawerSheet` / `SheetDialog` | `nsh/base` `<Sheet side="bottom" closeOnSwipe={true}>` 底层滑出抽屉 | Spatial Scene Dock 弹层、QA 验真工坊、同窗共养抽屉 | 取消滑入弹动，即时透明显隐。 |

---

## 四、 知气雅集商业化、礼赠与反胁迫伦理规范（v5.0 延续）

1. **核心健康权永久不锁**：原始生理指标、单人化验单 OCR 脱敏、本地 Ollama 晨报、JSONL / `.qivault` 明文导出及「安心静养（`recovery_priority`）」模式永不设限。
2. **窗台添柴礼赠券（Hearth Gift Pass）商业透明机制**：采用纯商业友好的「窗台添柴礼赠券」——付费会员可按清晰公示价格单次购买并生成社区漂流席位礼赠券，明码标价、无隐藏绑定、无道德绑架。
3. **无暗黑模式（Anti-Dark Patterns）与低电量礼貌沉默**：
   - 严禁倒计时紧迫感逼单、禁止羞辱式拒绝文案（次选 CTA 始终为平静的「继续素心清欢」或「先逛逛」）；
   - 当用户处于 `recovery_priority` 低电量调养状态时，绝不弹出商业订阅引导；退订后历史档案永久落入可全量导出的只读保险箱。
4. **全站 `ui/` 边界与 14 项完工自检门禁**：
   - `App.tsx` 与 `pages/*.tsx` 全部组件（含 `Button` / `Tabs` / `Sheet` / `Dialog` / `Collapsible` / `Switch`）必须从 `ui/components.tsx` 导入，零直接依赖 `nsh/base`；
   - 所有交互按钮具备最小 `44pt` 热区、`focus-visible:` 键盘焦点环、`accessibilityLabel` / `accessibilityRole` 与多语言双语排版空间冗余。
