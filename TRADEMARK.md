# Trademark and Brand Policy / 商标与品牌使用规范

Copyright (c) 2026-present Cluas. All Rights Reserved.

---

## English

### 1. Purpose

Source code in this repository is made available under `GNU AGPL-3.0-only` (see
[`LICENSE`](LICENSE)). Consistent with GNU AGPL-3.0 Section 7(e), open-source
code licensing does **not** grant any license or right to use the trademarks,
trade names, service marks, bundle identifiers, or visual brand identity of
**知气 · Qi**. Inspired by upstream trademark separation practices (such as
PulsHealth), this policy protects users from counterfeit apps, impersonation,
and confusingly similar forks.

### 2. Protected Trademarks & Brand Identifiers

The following wordmarks, product names, persona names, identifiers, and visual
marks are protected trademarks and proprietary brand identifiers of Cluas:

- **Product & Brand Names:** `知气`, `Qi`, `知气 · Qi`, `知气雅集` (and
  confusingly similar variations or translations thereof in health/wellness
  software)
- **Character & Feature Personas:** `小满` (in the context of the Qi window
  sanctuary companion and branding)
- **Application / Bundle Identifiers:** `eu.org.cluas.qi` and `group.eu.org.cluas.qi`
- **Visual Identity Marks:** The vermilion seal (`朱砂印章`) emblem, the `气`
  seal app icon (`assets/icon-1024.png`), and the signature cat-window (`猫窗`)
  brand presentation

### 3. Mandatory Requirements for Forks & Third-Party Distributions

If you fork, modify, self-host, or redistribute any software derived from this
repository outside personal local compilation, you **MUST**:

1. **Change the App Name:** Choose a distinct product name that does not
   contain or imitate `知气`, `Qi`, `知气 · Qi`, `小满`, or `知气雅集`;
2. **Change the Bundle ID / Package Name:** Replace `eu.org.cluas.qi` and
   `group.eu.org.cluas.qi` across all host manifests, entitlements, Info.plist,
   and server configuration before distributing or submitting any build;
3. **Replace All Visual Assets & Icons:** Completely replace the app icon
   (`assets/icon-1024.png`), vermilion seal emblem, cat-window branding, and all
   files under `assets/` and `tests/fixtures/*.png` (see
   [`LICENSE-ASSETS`](LICENSE-ASSETS)).

### 4. Permitted Factual Attribution (No Implied Endorsement)

You may make truthful, purely factual statements about the origin of your
codebase (e.g., *"Based on the open-source Qi codebase"* or *"Forked from the
Qi project"*), provided that:

- The reference appears only in factual documentation, credits, or release
  notes rather than as your product's primary name, store title, subtitle, or
  logo;
- It clearly identifies your version as an independent third-party build and
  **does not imply official endorsement, sponsorship, affiliation, or review** by
  Cluas or the official `知气 · Qi` project.

### 5. Prohibited Uses

Without prior written permission from the trademark owner, you may not use any
Protected Mark in an App Store, Google Play, or Android marketplace listing,
domain name, social media account, keyword advertising, or misleading UI badge.

---

## 中文

### 1. 规范目的

本仓库源代码基于 `GNU AGPL-3.0-only` 开放（详见 [`LICENSE`](LICENSE)）。根据 GNU AGPL-3.0 第 7(e) 条及现行商标与反不正当竞争法律，开源代码授权**不包含**对「知气 · Qi」任何商标、商号、产品名称、应用包名或视觉品牌标识的授权。参照上游 `PulsHealth` 等开源项目的商标隔离实践，本规范旨在防止山寨仿冒包误导用户或损及项目声誉。

### 2. 受保护商标与品牌标识清单

以下文字标识、产品名称、形象命名、技术包名与视觉标识均归 Cluas 独家所有并受法律保护：

- **产品与品牌名称**：「知气」、「Qi」、「知气 · Qi」、「知气雅集」（以及在健康、节律、手账软件领域易引起混淆的近似名称）；
- **核心陪伴形象名称**：「小满」（在知气窗台陪伴剧场与品牌语境下的标识）；
- **应用标识符（Bundle ID / App Group）**：`eu.org.cluas.qi`、`group.eu.org.cluas.qi`；
- **视觉识别标识**：朱砂印章图形、「气」字方印应用图标（`assets/icon-1024.png`）以及标志性猫窗视觉识别体系（含 `assets/scenes/*.jpg` 中的品牌视觉特征）。

### 3. 第三方 Fork 与公开分发的强制隔离要求

任何第三方在个人本地自编译调试之外，若公开发布、托管或向应用市场提交基于本仓库源码的衍生版本（Fork），**必须**完整履行以下隔离义务：

1. **彻底更换应用名称**：使用独立的新名称，不得包含、拼接或仿冒「知气」「Qi」「知气 · Qi」「小满」「知气雅集」；
2. **彻底更换 Bundle ID / 包名**：在打包或分发前将 `eu.org.cluas.qi` 与 `group.eu.org.cluas.qi` 全部替换为第三方自有域名标识；
3. **彻底替换应用图标与视觉资产**：移除并替换 `assets/icon-1024.png`、朱砂印章图形、猫窗品牌形象及 `assets/`、`tests/fixtures/*.png` 下全部受保护资产（详见 [`LICENSE-ASSETS`](LICENSE-ASSETS)）。

### 4. 允许的事实性来源标注（严禁暗示官方背书）

第三方仅可在「关于页」、`NOTICE`、README 或发版说明中进行**克制、客观的事实性来源说明**（例如：“基于开源项目 Qi 源代码构建” 或 “Derived from the open-source Qi codebase”），且须同时满足：

- 不得将该标注作为第三方产品的标题、副标题、商店检索品牌词或主视觉 Logo；
- 必须明确告知用户该构建为独立第三方版本，**绝不可明示或暗示获得 Cluas 或《知气 · Qi》官方的背书、合作、赞助或安全认证**。

### 5. 禁止行为

未经商标权人事先书面许可，严禁在 Apple App Store、Google Play、国内外安卓应用商店页面、域名、自媒体账号、搜索引擎竞价关键词或应用内徽章中使用上述受保护商标或近似易混淆标识。
