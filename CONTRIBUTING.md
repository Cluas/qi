# Contributing to 知气 · Qi / 贡献指南与贡献者许可协议（CLA）

Thank you for your interest in contributing to **知气 · Qi**!
感谢你关注并希望为 **知气 · Qi** 贡献代码、文档或测试改进！

Because **知气 · Qi** uses a dual-licensing model (`GNU AGPL-3.0-only` for public
source code alongside proprietary commercial and official App Store / Google Play
distribution by the copyright holder — see [`LICENSE`](LICENSE)), **all
external contributors must accept the Contributor License Agreement (CLA) below**
before any Pull Request can be merged.

由于 **知气 · Qi** 采用双轨授权架构（公开源码遵循 `GNU AGPL-3.0-only`，版权人同时保留通过 Apple App Store、Google Play 及商业协议发行官方闭源版本的权利，详见 [`LICENSE`](LICENSE)），**所有外部贡献者在 Pull Request 合入前均须同意并签署下方《贡献者许可协议（CLA）》。**

---

## 1. How to Sign the CLA / 如何签署 CLA

For every Pull Request submitted by a contributor other than the repository
owner, you **MUST** include the following line in **either your Pull Request
description or at least one commit message in the PR**:

对于非仓库所有者提交的每一个 Pull Request，你**必须**在 **PR 正文描述** 或 **该 PR 任一 Commit Message** 中附上如下签署声明行：

```text
CLA-Signed-off-by: Your Name <your-email@example.com>
```

Example / 示例：

```text
CLA-Signed-off-by: Alice Zhang <alice@example.com>
```

The automated CI gate (`cla-and-license-gate` in
[`.github/workflows/ci-pr.yml`](.github/workflows/ci-pr.yml)) checks for this
`CLA-Signed-off-by:` declaration on pull requests targeting `main`. By adding
this line and submitting your contribution, you legally agree to all terms of
the Qi Contributor License Agreement in Section 2 below.

仓库 CI 门禁（[`.github/workflows/ci-pr.yml`](.github/workflows/ci-pr.yml) 中的 `cla-and-license-gate`）会在针对 `main` 分支的 PR 中自动校验 `CLA-Signed-off-by:` 声明。填写该声明并提交贡献，即代表你完全同意并接受下文第 2 节所列《知气 · Qi 贡献者许可协议》的全部条款。

---

## 2. Qi Contributor License Agreement (CLA) / 贡献者许可协议条款

### English

By submitting a Contribution (any source code, documentation, test, patch, or
other work of authorship intentionally submitted for inclusion in the Project)
and including `CLA-Signed-off-by: Name <email>`, you ("Contributor") agree to
the following terms with the Project maintainer and copyright holder ("Maintainer",
Cluas):

1. **Definitions:**
   - "Project" means the **知气 · Qi** software repository and related official
     distributions managed by the Maintainer.
   - "Contribution" means any original work of authorship, including any
     modifications or additions to an existing work, that you intentionally
     submit to the Project for inclusion.
2. **Grant of Copyright License:**
   Subject to the terms of this Agreement, you hereby grant to the Maintainer
   and to recipients of software distributed by the Maintainer a **perpetual,
   worldwide, non-exclusive, no-charge, royalty-free, irrevocable copyright
   license** to reproduce, prepare derivative works of, publicly display,
   publicly perform, **sublicense**, and distribute your Contributions and such
   derivative works under **any licensing terms**, including but not limited to
   `GNU AGPL-3.0-only`, commercial/proprietary licenses, and official
   distribution through the **Apple App Store**, **Google Play**, and other
   application marketplaces.
3. **Grant of Patent License:**
   Subject to the terms of this Agreement, you hereby grant to the Maintainer
   and to recipients of software distributed by the Maintainer a **perpetual,
   worldwide, non-exclusive, no-charge, royalty-free, irrevocable (except as
   stated below) patent license** to make, have made, use, offer to sell, sell,
   import, and otherwise transfer the Work, where such license applies only to
   those patent claims licensable by you that are necessarily infringed by your
   Contribution(s) alone or by combination of your Contribution(s) with the
   Project. If any entity institutes patent litigation against you or any other
   entity alleging that the Contribution or the Project constitutes direct or
   contributory patent infringement, then any patent licenses granted to that
   entity under this Agreement for that Contribution or Project shall terminate
   as of the date such litigation is filed.
4. **Representations and Ownership:**
   You represent that:
   - You are legally entitled to grant the above licenses; if your employer has
     rights to intellectual property that you create, you have received explicit
     permission from your employer to make Contributions on its behalf or your
     employer has waived such rights;
   - Each Contribution is your original creation (or you have full authority and
     compatible third-party license clearance to submit it), and your submission
     does not include unauthorized proprietary code, secrets, or protected
     personal data;
   - You retain ownership of the copyright in your original Contribution, while
     understanding that the Maintainer has no obligation to include any
     Contribution in the Project.
5. **No Warranty:**
   Except as stated in Section 4, you provide your Contributions on an "AS IS"
   BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, EITHER EXPRESS OR
   IMPLIED.

### 中文

一旦你向本项目提交任何“贡献”（包括但不限于源代码、文档、测试、补丁或其他有意识提交以期合入本项目的智力成果）并附上 `CLA-Signed-off-by: Name <email>` 声明，即表示你（下称“贡献者”）与本项目维护者及版权所有人（下称“维护者”，即 Cluas）达成并同意以下协议：

1. **定义**：
   - “本项目”指由维护者管理的 **知气 · Qi** 软件仓库及相关官方发行版本。
   - “贡献”指你主动提交至本项目以求合入的任何原创作品，以及对已有作品的任何修改或增补。
2. **版权许可授予**：
   在本协议条款约束下，贡献者特此向维护者及通过维护者获取本软件的接收方授予一项**全球范围内的、永久的、非独占的、免费的、免版税的、不可撤销的版权许可**，允许维护者复制、制作衍生作品、公开展示、公开运行、**再许可（Sublicense）**并分发你的贡献及其衍生作品——**包括但不限于以 `GNU AGPL-3.0-only` 开源形式分发，以及由维护者依据商业专有协议或通过 Apple App Store、Google Play 及其他官方应用商店以商业双许可形式发行**。
3. **专利许可授予**：
   在本协议条款约束下，贡献者特此向维护者及接收方授予一项**全球范围内的、永久的、非独占的、免费的、免版税的、不可撤销的（下文另有约定除外）专利许可**，允许制造、委托制造、使用、许诺销售、销售、进口及以其他方式转让包含该贡献的作品；该许可仅适用于因你单独提交的贡献或其与本项目结合而必然构成侵权且你有权许可的专利权利要求。若任何实体就贡献或本项目提起专利侵权诉讼（含反诉或交叉诉讼），则本协议向该实体授予的相应专利许可自诉讼提起之日起自动终止。
4. **权利保证与著作权归属**：
   你声明并保证：
   - 你具备签署本协议并授予上述全部许可的完全合法权利；若你的雇主对你创作的知识产权享有权益，你已获得雇主的明确书面授权或放弃声明；
   - 每一项贡献均为你的原创成果（或你已获得充分授权且满足协议兼容性），不包含任何未经授权的第三方专有代码、密钥凭据或真实公民个人隐私数据；
   - 你仍保留你所提交原创贡献的底层著作权，但理解并同意维护者无义务必须合入任何贡献。
5. **免责声明**：
   除第 4 条明示保证外，你的贡献均按“现状（AS IS）”提供，不附带任何明示或默示的担保或保证责任。

---

## 3. Contribution Guidelines / 提交规范与本地检查

Before opening a Pull Request, please ensure:
提交 PR 前，请确认满足以下工程与合规要求：

- **No Secrets or Personal Data:** Never commit API keys, `.p8`/`.p12` signing
  certificates, `QI_CLOUD_SIGNING_SEED_HEX`, real user health exports, or
  hardcoded local machine paths.
- **Asset & Brand Separation:** Do not alter [`LICENSE`](LICENSE),
  [`LICENSE-ASSETS`](LICENSE-ASSETS), [`TRADEMARK.md`](TRADEMARK.md), or
  [`NOTICE`](NOTICE) attribution unless explicitly discussed and approved.
  Do not upload third-party copyrighted artwork into `assets/`.
- **Zero-Plaintext Control Plane Principle:** Changes under `cloud/` must never
  store plaintext biometric, dietary, or clinical lab records in SQLite or logs.
- **Checks / 测试校验：**
  - Run `cargo test --manifest-path cloud/Cargo.toml` for `cloud/` changes;
  - Run `cargo check --release --manifest-path Cargo.toml` and `nsh build`
    (without private signing keys) to verify client compilation;
  - When updating UI copy, keep `locales/zh-CN.json` and `locales/en.json`
    leaf keys synchronized without introducing Emoji characters into legal/notice
    documents or i18n bundles.
