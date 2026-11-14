# 《知气（Qi）》零本机 Xcode · 云端打包 + `xtool` 真机安装指南（付费开发者版）

## 为什么这套方案本机只需约 `21 MB`？

| 环节 | 执行位置 | 原因与磁盘开销 |
| :--- | :--- | :--- |
| **前端调样式 / 交互热更** | 本机 Mac（`nsh run`） | 仅用系统已装的 `CommandLineTools`，**0 额外磁盘 & 0 云端额度** |
| **iOS 交叉编译 & 打包** | GitHub Actions（`macos-15`） | `aarch64-apple-ios` 与 `iPhoneOS.sdk` 均在云端运行；本机**不安装 40 GB+ `Xcode.app` 和模拟器** |
| **正式签名 & USB 装真机** | 本机 Mac（`xtool` CLI） | `xtool.app` 仅约 **`21 MB`**，通过官方 App Store Connect API 完成 **1 年有效期**签名并经 USB 安装到 iPhone |

> 本项目已托管于 Public 公开仓库 `Cluas/qi`，享有 GitHub Actions `macos-15` **无限免费分钟数**；`.github/workflows/ios-package.yml` 设为 **仅 `workflow_dispatch` 手动触发**（不监听 `push`/`PR`），开启 Rust 增量缓存后每次云端打包约 3–5 分钟，产物保留 1 天自动清理。默认分支 `main` 仅保留工作流注册定义、暂不存放业务代码，日常开发与云端构建均在 `dev` 分支进行。

---

## 一、一次性初始化（约 3 分钟）

### 1. 安装 `xtool`（仅约 21 MB）

```bash
brew install --cask xtool-org/tap/xtool
```

验证命令行可用：

```bash
xtool --help
```

### 2. 在 App Store Connect 生成官方 API Key

1. 登录 [App Store Connect](https://appstoreconnect.apple.com/)，进入 **用户和访问 › 整合 › App Store Connect API › 团队密钥（Team Keys）**。
2. 点击 `+` 新建密钥，角色选择 **`App 管理`（App Manager）**（或 `Admin`）。
3. 记录页面上的 **`Issuer ID`** 与 **`Key ID`**，下载一次性私钥文件 `AuthKey_XXXXXXXXXX.p8`。
4. **安全提示**：切勿把 `.p8` 私钥放进 Git 仓库目录；建议存在 `~/.config/apple-keys/`（本仓库 `.gitignore` 也已默认拦截 `*.p8`、`*.p12`、`*.ipa`、`*.mobileprovision`）。

### 3. 完成 `xtool` 登录认证并开启 iPhone 开发者模式

在终端运行：

```bash
xtool setup
```

按提示输入：
1. 登录模式选择 **`0`**（`API Key (requires paid Apple Developer Program membership)`）；
2. 依次填入上一步的 **`Key ID`**、**`Issuer ID`** 以及 **`AuthKey_XXXXXXXXXX.p8` 的本地绝对路径**。

在 iPhone 上确认开启：
- **设置 › 隐私与安全性 › 开发者模式 › 打开并重启**；
- 首次通过 USB 连上 Mac 时点击 **「信任此电脑」**。

---

## 二、分支约定与远端仓库结构（main 空壳占位 + dev 开发分支）

远端公开仓库为 `https://github.com/Cluas/qi`（Public 仓库），采用以下双分支结构：

- **`main`（默认分支，空壳占位 + workflow 注册）**：仅保留 `.gitignore`、`LICENSE`、`README.md` 与 `.github/workflows/` 工作流定义文件，暂不存放业务源码，确保 GitHub Actions 能正常注册手动打包与 PR 检查流水线。
- **`dev`（日常开发分支）**：存放完整应用源码与原生宿主工程，所有日常迭代与云端构建均在 `dev` 分支进行。

首先切换到 `dev` 分支并运行环境预检，确认本机配置与目标分支就绪：

```bash
git checkout dev
./scripts/ios_xtool_install.sh --dry-run
```

在 `dev` 分支下直接执行 `./scripts/ios_xtool_install.sh --build --launch`（或显式传入 `--ref dev`）即可触发远端打包并装机；若误停留在空壳 `main` 分支执行脚本，也会自动检测并回退到 `dev` 分支触发云端构建。

如需首次初始化远端仓库并推送双分支，可按序执行：

```bash
gh repo create qi --public --source=. --remote=origin -d "Qi (知气)"
git push -u origin main
git push -u origin dev
```

---

## 三、日常真机安装命令

| 场景 | 命令 | 说明 |
| :--- | :--- | :--- |
| **触发云端出包并装机启动** | `./scripts/ios_xtool_install.sh --build --launch` | 触发 `ios-package.yml`，实时等待构建完成后下载 `.ipa`、自动签名装入 iPhone 并拉起 |
| **复用最近一次云端产物重装** | `./scripts/ios_xtool_install.sh --launch` | **0 分钟云端额度消耗**，直接拉取最近一次成功构建的 `.ipa` 装机或换机安装 |
| **安装本地已有 `.ipa` / `.app`** | `./scripts/ios_xtool_install.sh --ipa /path/to/app.ipa` | 跳过 GitHub，直接调用 `xtool install` 签名装机 |
| **多台真机指定安装目标** | `xtool devices --no-wait`<br>`./scripts/ios_xtool_install.sh --udid <设备UDID>` | 查看各设备 UDID 后精准安装 |

---

## 四、关于原生 Capability 与扩展的说明

- `nsh package --target ios` 产出的未签名安装包会自动附带 `hosts/ios/PrivacyInfo.xcprivacy` 及 HealthKit / 麦克风 / 语音识别中文权限文案（已在 CI 中自动合并到 `Info.plist`）。
- 付费开发者账号签发成功后，单应用有效期为 **1 年**，**不受 7 天过期和 3 个 App 数量上限限制**；如需在真实真机验证 `hosts/ios/Sources/` 下的 HealthKit / StoreKit2 / WidgetKit 扩展完整原生宿主持久化能力，待团队密钥准备好后可直接在 Apple Developer Portal 注册对应的 App ID 与 App Group（`group.eu.org.cluas.qi`）。

---

## 五、防本机磁盘再次被 Rust 缓存填满（可选）

如果未来会在本机频繁跑桌面调试（`nsh run`）或测试，建议安装 NSH 官方推荐的共享限额版 Cargo（全局复用编译产物、自动控容，替代 Stock Cargo 默认每仓膨胀十余 GB）：

```bash
curl -fsSL https://huacnlee.github.io/cargo/install.sh | sh
```
