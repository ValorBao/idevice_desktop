# 无人值守真机测试台

在一台长期插着几部 iPhone 的 Apple Silicon Mac mini 上，用 GitHub Actions 自托管 runner 跑 `verify_*` 和 ignored 硬件测试。没有人在旁边点屏幕。

本仓库的 Linux / GitHub-hosted CI（Frontend、Rust）不跑这些测试。设备测试只在带 `self-hosted, macOS, ios-devices` 标签的 runner 上执行。

## 一次性人工准备（Mac mini + 手机）

只做一次。之后 nightly、`device-test` 标签和手动触发都不需要人在场。

### Mac mini

1. 用一个会自动登录的 macOS 用户跑 runner。LaunchAgent 跟用户会话走；头less 机器必须打开「自动登录」，否则重启后 runner 是离线的。
2. 安装 **Xcode Command Line Tools**（`xcode-select --install`）。iOS 17+ 个性化 DDI 还需要 **Xcode 15+**（`devicectl`）。
3. 手机走 **带供电的 USB 集线器**，不要用键盘/充电口那种只供电不传数据的口。每条线都要是数据线。
4. 机器要通风。几部手机长期充电会发热，不要闷在柜子里。
5. 配对记录留在这台 Mac 上：`/var/db/lockdown/` 里的 host 记录、以及 `~/Library/Application Support/dev.idevice.desktop/remote-pairing-*.plist`。重装系统或换用户等于要重新 Trust。

在仓库根目录执行（token 只走环境变量，不要写进脚本或 plist）：

```bash
export GITHUB_REPOSITORY=ValorBao/idevice_desktop
export GITHUB_RUNNER_TOKEN='…'   # Settings → Actions → Runners → New self-hosted runner，一小时过期
# 或者：export GH_TOKEN='…'      # 有仓库 admin 的 PAT，脚本会自己去领 registration token
bash scripts/device-rig/setup-runner.sh
```

脚本是幂等的：已安装的 Homebrew 公式、已配置的 runner、已装上的 launchd 服务会跳过。换一台 runner 或改名时设 `REPLACE_RUNNER=1`。

默认额外标签是 `ios-devices`。GitHub 还会自动加上 `self-hosted`、`macOS`、`ARM64`。workflow 用的是 `[self-hosted, macOS, ios-devices]`。

### 每部 iPhone（每个开发服务世代一部）

| 世代 | 版本 | 运输 |
| --- | --- | --- |
| `legacy` | iOS ≤ 16 | Lockdown + DeveloperDiskImage |
| `core-device-remote` | iOS 17.0–17.3 | RemotePairing / RSD |
| `core-device-lockdown` | iOS 17.4+ | CoreDeviceProxy / RSD |

对每部手机，人要点的只有这些：

1. **Trust This Computer**：解锁，插上，点「信任」，输入密码（如果有）。只做一次。
2. **关掉密码**。有密码时锁屏后 Lockdown 会话会失败，也无法远程打开 Developer Mode。无人值守必须关密码。
3. **自动锁定 = 永不**（设置 → 显示与亮度 → 自动锁定）。否则过几分钟就锁上，preflight 会失败。
4. **Developer Mode 打开**（iOS 16+：设置 → 隐私与安全性 → 开发者模式）。会重启。iOS 14/15 没有这项，AMFI 不存在，preflight 报 `n/a` 是对的。
5. **优化电池充电打开**（设置 → 电池 → 充电）。长期插电时把上限大约留在 80%，保护电池。不要为了测试去关它。
6. 屏幕可以暗，但不要手动锁机，不要开「需要注意」那种会挡住 USB 信任的提示。

### 自检

```bash
bash scripts/device-rig/preflight.sh
```

应看到三部 USB 手机，各自的 UDID、iOS、世代、paired/trusted、锁状态、Developer Mode、电量和是否在充。某部锁着、没信任、或没插，进程以非 0 退出并打印原因。同一部机的 Wi-Fi 记录标成 `OPTIONAL (Wi-Fi)`，失败不改变退出码。

指定世代（CI 矩阵用）：

```bash
IDEVICE_TEST_GENERATION=legacy bash scripts/device-rig/preflight.sh --require-generation legacy
```

## 怎么触发

workflow：`.github/workflows/device-tests.yml`

| 触发 | 条件 |
| --- | --- |
| 每天 UTC 16:00 | `master` 上的 schedule（约北京时间 0:00） |
| 本仓库 PR 标签 `device-test` | `pull_request_target`，workflow YAML 来自 base。仅当 head 就是本仓库且不是 fork 时才 checkout PR SHA |
| `workflow_dispatch` | Actions 页手动 Run（只能选本仓库的分支） |

**Fork 的代码永远不会上这台 Mac mini。** 不用 `pull_request`：那个事件会跑 PR head 里的 workflow YAML，fork 只要删掉 job 的 `if:` 就能在自托管 runner 上执行任意命令。现在用 `pull_request_target`（YAML 来自受信任的 base）+ 同仓库且 `fork == false` 的双重门闩 + checkout 前再拒绝一次。`contents: read`，不持久化 credentials。schedule / `workflow_dispatch` 只能从本仓库触发。

Wi-Fi / Bonjour 的 usbmuxd 记录是 **optional**：失败会记下来，但不会把 preflight 打红。USB 锁定、未信任、或根本没插 USB 才会非 0 退出。

跑完会：

- 先 preflight，再按世代矩阵跑只读 `verify_*` 和 `#[ignore]` 硬件测试
- 某世代没插手机：该矩阵格 **跳过（成功）**，不把整次 run 打红
- 已插入但锁着/未信任：preflight 失败
- 日志以 artifact 上传（`device-preflight`、`device-legacy` 等）
- 本仓库 PR 上发（或更新）一条摘要评论

本地只跑一部：

```bash
export IDEVICE_TEST_UDID=00008101-…
export IDEVICE_TEST_GENERATION=core-device-remote
bash scripts/device-rig/run-harnesses.sh
```

`verify_*` 也可以不写 UDID 参数，改读 `IDEVICE_TEST_UDID`。它们不读 stdin、不弹窗。

## CI 会跑什么、不会跑什么

**会跑（只读、可无人值守）**

- `device_preflight`、`verify_discovery`、`verify_pairing`（没有 `--repair`）
- 每个世代：`verify_ddi`、`verify_crash_transport`、`verify_processes`、`verify_processes_command`、`verify_large_crash`、`verify_trollstore`（只列状态）
- Legacy：`verify_legacy_ddi`（只看本机镜像是否已装，不 `--install` / `--mount` / `--developer-mode`）
- iOS 17+：`verify_performance`、`verify_jit`（只列可调试应用，不带 bundle、不 attach）
- `cargo test --lib … -- --ignored`（需要 `IDEVICE_TEST_UDID`）

**不会跑（要人点屏幕或会拆信任）**

- `verify_pairing --repair`
- `verify_ddi --unmount`
- `verify_legacy_ddi --developer-mode` / `--mount`
- `verify_jit <bundle>`、`verify_jit_stop`、带 bundle 的 processes launch/stop
- `verify_trollstore --ipa` / `--probe-url`（TrollStore 确认框在手机上）
- Personal Signing 真账号登录 / 2FA

## 故障排除

### preflight：没有设备

- 换数据线、换供电口。`ioreg -p IOUSB -w0` 应能看到 iPhone。
- 重启 `usbmuxd` 相关的 Apple Mobile Device 服务，或拔掉集线器再插上。
- 用 `uhubctl` 给口循环上电（需集线器支持 per-port power）：

```bash
brew install uhubctl
uhubctl -l <hub-location> -a cycle
```

location 用 `uhubctl` 不带参数列出来。循环后等几秒再跑 preflight。

### 未信任 / 没有 pair record

手机上重新点 Trust，或在已解锁时：

```bash
cargo run --manifest-path src-tauri/Cargo.toml --example verify_pairing -- --repair "$UDID"
```

这会拆掉现有配对，**必须有人点 Trust**。做完再把自动锁定改回「永不」。

iOS 17.0–17.3 还要有 RemotePairing 文件：`~/Library/Application Support/dev.idevice.desktop/remote-pairing-$UDID.plist`。用桌面应用连过一次就会写下。

### 显示锁定

关密码、自动锁定设永不、点亮屏幕一次。不要开锁屏密码做「安全测试」——那会让整台架子无法无人值守。

### Developer Mode 关闭

设置 → 隐私与安全性 → 开发者模式。远程 `enable_developer_mode` 在有密码的机器上会失败；本架子假设密码已关，但仍建议只在第一次人工打开，避免测试里乱重启。

### Runner 离线

```bash
cd ~/actions-runner
./svc.sh status
./svc.sh start
```

LaunchAgent 在 `~/Library/LaunchAgents/actions.runner.*.plist`。重启后若没自动登录，runner 不会起来。

### 世代被 skip

preflight 的 JSON 里 `by_generation` 对应项是 `null`。检查那部机的 iOS 是否还落在该桶：17.4 会从 `core-device-remote` 变成 `core-device-lockdown`。矩阵 skip 是故意的，不是 CI 红。

## 可选：WebDriverAgent（只需写在这里，不接入 CI）

绝大多数 `verify_*` 走 Lockdown / RSD，不需要点屏幕。少数例外在手机 UI 上：

- TrollStore 安装确认
- 第一次 Trust
- Developer Mode 重启后的确认

这些 **不** 进 GitHub Actions。若以后要覆盖 TrollStore 确认，可以在该机装 WebDriverAgent（或类似 XCTest runner），用它点系统告警。那是另一套签名、WDA 保活和端口转发，本仓库的 device-rig **不安装、不调用** WDA。在考虑之前，先保证三部机 Trust、无密码、永不锁定、Developer Mode 已开。

## 环境变量

| 变量 | 作用 |
| --- | --- |
| `IDEVICE_TEST_UDID` | 选哪一部手机。CLI 里看起来像 UDID 的参数优先 |
| `IDEVICE_TEST_GENERATION` | `legacy` / `core-device-remote` / `core-device-lockdown` |
| `GITHUB_RUNNER_TOKEN` | 自托管 runner 注册 token，不要写进仓库 |
| `GH_TOKEN` | 可选，用来领取 registration token |
| `GITHUB_REPOSITORY` / `GITHUB_REPO_URL` | runner 注册目标 |
| `RUNNER_DIR` | 默认 `~/actions-runner` |
| `REPLACE_RUNNER=1` | 用同名 runner 覆盖配置 |
| `DEVICE_RIG_LOG_DIR` | harness 日志目录 |

Linux 上 `cargo test` / `npm test` 不会跑 ignored 硬件测试；没有 usbmuxd 时它们保持忽略。
