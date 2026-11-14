# 《知气（Qi）》零本机 Xcode · 云端打包 + `xtool` 真机安装指南（付费开发者版）

## 为什么这套方案本机只需约 `21 MB`？

| 环节 | 执行位置 | 原因与磁盘开销 |
| :--- | :--- | :--- |
| **前端调样式 / 交互热更** | 本机 Mac（`nsh run`） | 仅用系统已装的 `CommandLineTools`，**0 额外磁盘 & 0 云端额度** |
| **iOS 交叉编译 & 打包** | GitHub Actions（`macos-15`） | `aarch64-apple-ios` 与 `iPhoneOS.sdk` 均在云端运行；本机**不安装 40 GB+ `Xcode.app` 和模拟器** |
| **正式签名 & USB 装真机** | 本机 Mac（`xtool` CLI） | `xtool.app` 仅约 **`21 MB`**，通过官方 App Store Connect API 完成 **1 年有效期**签名并经 USB 安装到 iPhone |

> 本项目 `.github/workflows/ios-package.yml` 设为 **仅 `workflow_dispatch` 手动触发**（不监听 `push`/`PR`），开启 Rust 增量缓存后每次云端打包约 3–5 分钟，产物保留 1 天自动清理；私有仓库每月 200 分钟 macOS 免费额度可支撑约 **40–65 次** 真机包构建。

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

## 二、首次推送私有 GitHub 仓库（如尚未绑定远端）

首先运行环境预检确认本机配置是否完整：

```bash
./scripts/ios_xtool_install.sh --dry-run
```

如果 `gh` 已登录（未登录先运行 `gh auth login`）且本地尚未设置 GitHub `origin` 远端，可执行一条命令创建私有仓库并推送当前代码：

```bash
gh repo create qi --private --source=. --remote=origin --push
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
