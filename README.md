<div align="center">

# 🔔 FlashBridge

<p>
  <em>Bridge Windows taskbar flash events to system notifications — no patching, no injection.</em><br/>
  <em>把 Windows 任务栏闪烁信号桥接为系统通知 —— 不修改、不注入目标应用。</em>
</p>

<p>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2021-CE422B?logo=rust&logoColor=white">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows-0078D6?logo=windows&logoColor=white">
  <img alt="Dependencies" src="https://img.shields.io/badge/dependencies-0-success">
  <img alt="License" src="https://img.shields.io/badge/license-MIT-blue">
</p>

<p>
  <strong>🌐 Language / 语言：</strong>
  <a href="#-english"><strong>English</strong></a> ·
  <a href="#-中文"><strong>中文</strong></a>
</p>

<sub>Use the collapsible sections below — only one language expanded at a time keeps the README scannable.</sub>

</div>

---

<a id="-english"></a>
<details>
<summary><h2>English</h2></summary>

### Overview

**FlashBridge** is a lightweight background service that gives "taskbar-flash-only" Windows apps (WeChat, DingTalk, Feishu, …) the modern system notifications they never send. These apps flash the taskbar on new messages instead of firing a real notification, which breaks Focus Assist, phone sync, and Action Center history — and is easy to miss on multi-monitor / virtual-desktop setups.

FlashBridge sits in the middle **without modifying or injecting into any target app**: it registers a hidden shell-hook window, listens for `HSHELL_FLASH` (and `HSHELL_REDRAW` with `lParam = TRUE`, since Windows often reports flashing that way), reads the process name and window title, matches it against a rule table, debounces, and fires a tray notification. Clicking the notification focuses the target window.

The project is pure Rust with **zero third-party crates** — every Win32 call is raw FFI — so it builds in restricted environments with no crates.io access and ships as a single `.exe`.

### Highlights

- **Zero-invasive** — no patching, no DLL injection, no UIAutomation polling. Only Shell Hook events.
- **Whitelist / blacklist filtering** by process name, with per-app display name and icon override.
- **Debounce + dedup + rate limiting** — one flash per window per `debounce_ms`, suppress repeated identical titles, cap at `max_per_minute` per process.
- **Focus Assist awareness** — queries `SHQueryUserNotificationState`; stays quiet when Windows reports busy / quiet.
- **Foreground-process suppression** — ignores the app family you're currently using (including sibling processes with the same display name), so opening a photo in WeChat doesn't fire a false positive.
- **Native Win32 settings window** — mode switch, process-list editing, global toggles, live log, notification history, "Test notification", save config. Closing the window hides it to the tray instead of quitting.
- **Autostart + silent mode** — writes the HKCU `Run` entry; launched with `--minimized` it shows only a tray icon, no window.
- **Config hot-reload** — checks the config file mtime every second; edits apply without restart.
- **File logging + TSV notification history** — under `%APPDATA%\FlashBridge\` by default.
- **Optional localhost web dashboard** — off by default; useful for remote / headless scenarios.
- **Process-icon extraction** — `SHGetFileInfoW` pulls the icon from the target exe so the notification shows it.
- **Click-to-focus** — clicking a notification calls `SetForegroundWindow` on the source window.
- **Zero third-party crates** — pure Win32 FFI; a resource script (`build.rc`) embeds the app icon at link time.

### Quick start

```powershell
# Prerequisites: Rust stable toolchain, Windows 10/11
cargo build --release
.\target\release\flashbridge.exe
```

A tray icon appears and the native settings window opens. Add your target process names (e.g. `WeChat.exe`) under **Process rules**, pick whitelist/blacklist mode, and you're done. Closing the settings window hides it to the tray; right-click the tray → **Open settings** to reopen.

### How it works

```
Target app calls FlashWindow / FlashWindowEx
  → Shell posts a SHELLHOOK message to the registered window
    → HSHELL_FLASH received (or HSHELL_REDRAW with lParam = TRUE)
      → read HWND → get PID → get process name
        → match against rule table (whitelist or blacklist)
          → debounce → extract window title
            → fire system notification (Shell_NotifyIconW)
              → click notification → SetForegroundWindow(hwnd)
```

### Config reference

See [config.toml.example](config.toml.example). Important fields:

| Field                     | Description                                                                       |
|---------------------------|-----------------------------------------------------------------------------------|
| `mode`                    | `whitelist` or `blacklist`                                                        |
| `debounce_ms`             | debounce window for the same window                                               |
| `autostart`               | write/remove the HKCU `Run` entry                                                 |
| `hot_reload`              | reload config changes while running                                               |
| `deduplicate_same_title`  | suppress repeated notifications with the same window title                        |
| `max_per_minute`          | per-process rate limit, `0` disables it                                           |
| `respect_quiet_hours`     | suppress when Windows reports a busy/quiet notification state                     |
| `listen_redraw_flash`     | on by default — Windows often reports flashing as `HSHELL_REDRAW + lParam=true`   |
| `ignore_foreground_process` | suppress the foreground app family (incl. sibling processes with the same name) |
| `web_ui`                  | start the local browser dashboard                                                 |
| `web_ui_port`             | dashboard port on `127.0.0.1`                                                     |
| `history_path`            | notification history TSV path (empty = default)                                   |
| `history_limit`           | number of history rows to retain                                                  |

Each `[[apps]]` (or `[[ignore]]`) entry: `process`, `display_name`, `icon`.

### Native settings window

- **No console window** — compiled with `#![windows_subsystem = "windows"]`.
- Double-click `flashbridge.exe` → tray icon + settings window appear.
- Closing the settings window only hides it; the app keeps running in the tray.
- Right-click tray → **Open settings** reopens it; **Test notification** fires a sample balloon; **Pause/Resume** toggles the bridge.
- When launched via autostart (or `--minimized`), it enters silent mode — tray icon only, no settings window.

### Self-test

Verify the core flash pipeline without waiting for a real chat message:

```powershell
# Terminal 1 — run with the self-test config (whitelist flashbridge.exe, quiet hours off)
.\target\debug\flashbridge.exe .\target\shell-flash-self-test.toml

# Terminal 2 — a test window calls FlashWindowEx
.\target\debug\flashbridge.exe --flash-test-window 4
```

A passing run writes `FlashBridge Self Test` to the history file. To verify the default WeChat whitelist path without a real message, copy the built binary to `target\WeChat.exe`, whitelist `WeChat.exe`, then run `.\target\WeChat.exe --flash-test-window 4`.

### Build & test

```powershell
# Debug build
cargo build

# Release build
cargo build --release

# Formatting / lint / test gates (same ones CI enforces)
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --release
```

### Release artifacts

The CI workflow (`.github/workflows/build.yml`) builds on every PR and on `v*` tags. Pushing a `v*` tag publishes a GitHub Release with:

| Artifact                       | What it is                                  |
|--------------------------------|---------------------------------------------|
| `flashbridge.exe`              | the release executable                      |
| `FlashBridge-portable.zip`     | portable bundle: exe + config example + README |

Download from the [Releases page](https://github.com/Cloudstill/FlashBridge/releases).

### Project structure

```
flashbridge/
├── Cargo.toml              # package metadata, build = "build.rs"
├── build.rs                # compiles build.rc -> embeds the app icon via rc.exe
├── build.rc                # Windows resource script (app icon)
├── config.toml.example     # annotated example config
├── icons/                  # icon.ico + generate-icon.ps1
└── src/
    ├── main.rs             # entry: arg parsing, --minimized, --flash-test-window
    ├── shell_hook.rs       # hidden shell-hook window + RegisterShellHookWindow + message loop
    ├── processor.rs        # event handling, debounce, rule matching, rate limit, quiet hours
    ├── toast.rs            # Shell_NotifyIconW tray notifications + tray context menu
    ├── native_ui.rs        # native Win32 settings window (replaces browser dashboard)
    ├── web_ui.rs           # optional localhost web dashboard (off by default)
    ├── config.rs           # TOML config read/write + default paths
    ├── history.rs          # TSV notification history persistence
    ├── logger.rs           # file logger
    ├── autostart.rs        # HKCU Run autostart entry
    ├── self_test.rs        # FlashWindowEx end-to-end self-test trigger
    └── win.rs              # Win32 FFI types & helpers
```

### Roadmap

- WinRT Toast backend (full Action Center integration + AUMID), replacing the tray balloon.
- Optional phone push (POST to ntfy alongside the toast) for true cross-device sync.
- `SetWinEventHook` / UIAutomation fallback for apps that don't use `FlashWindow`.
- Code signing to avoid antivirus false positives on the global event listener.
- Optional SQLite history backend replacing the TSV file.

### Limitations

- FlashBridge only knows a window flashed — it does **not** read message sender or body.
- `RegisterShellHookWindow` is not a general-purpose API contract; Win10/Win11 behavior must be tested in each environment.
- The current backend is a tray balloon, not full WinRT Toast center integration (planned — see Roadmap).
- False positives: WeChat may redraw/flash during user actions like opening a photo — keep `ignore_foreground_process = true`.

</details>

---

<a id="-中文"></a>
<details>
<summary><h2>中文</h2></summary>

### 概览

**FlashBridge** 是一个轻量后台服务，为微信、钉钉、飞书等"只会闪任务栏"的 Windows 应用补上它们从不发送的现代系统通知。这些应用收到消息时只闪任务栏图标，不走系统通知，导致勿扰模式失效、无法同步到手机、通知中心里看不到历史记录，在多显示器/虚拟桌面下还极易漏消息。

FlashBridge 插在中间，**不修改、不注入任何目标应用**：它注册一个隐藏的 shell-hook 窗口，监听 `HSHELL_FLASH`（以及 `lParam = TRUE` 的 `HSHELL_REDRAW`，因为 Windows 常把闪烁报告成这种），读取进程名和窗口标题，匹配规则表，去抖动后发托盘通知。点击通知会聚焦目标窗口。

项目是纯 Rust，**零第三方 crate**——所有 Win32 调用都是原始 FFI——因此在没有 crates.io 访问的受限环境也能编译，最终只有一个 `.exe`。

### 亮点

- **零侵入** —— 不打补丁、不注入 DLL、不轮询 UIAutomation，只监听 Shell Hook 事件。
- **白名单 / 黑名单**按进程名过滤，支持每个应用自定义显示名和图标。
- **去抖动 + 去重 + 频率限制** —— 同一窗口 `debounce_ms` 内只触发一次，相同标题不重复弹，每进程每分钟最多 `max_per_minute` 条。
- **勿扰模式感知** —— 查询 `SHQueryUserNotificationState`，Windows 报告忙碌/勿扰时保持安静。
- **前台进程抑制** —— 忽略你正在使用的应用家族（含同名兄弟进程），避免如微信打开图片等误触发。
- **原生 Win32 设置窗口** —— 模式切换、进程列表编辑、全局开关、实时日志、通知历史、"测试通知"、保存配置。关闭窗口只是隐藏到托盘，不退出。
- **开机自启 + 静默模式** —— 写入 HKCU `Run` 项；带 `--minimized` 启动时只显示托盘图标，不弹窗。
- **配置热重载** —— 每秒检查 config 文件修改时间，编辑后无需重启即生效。
- **文件日志 + TSV 通知历史** —— 默认在 `%APPDATA%\FlashBridge\` 下。
- **可选本地 Web Dashboard** —— 默认关闭，便于远程 / headless 场景。
- **进程图标提取** —— `SHGetFileInfoW` 从目标 exe 提取图标，通知里显示。
- **点击聚焦** —— 点击通知对源窗口调用 `SetForegroundWindow`。
- **零第三方 crate** —— 纯 Win32 FFI；资源脚本 `build.rc` 在链接时嵌入应用图标。

### 快速开始

```powershell
# 前置：Rust stable 工具链，Windows 10/11
cargo build --release
.\target\release\flashbridge.exe
```

托盘出现图标并弹出原生设置窗口。在 **Process rules** 下加入目标进程名（如 `WeChat.exe`），选择白名单/黑名单模式即可。关闭设置窗口只是隐藏到托盘；右键托盘 → **Open settings** 重新打开。

### 工作原理

```
目标应用调用 FlashWindow / FlashWindowEx
  → Shell 向注册窗口投递 SHELLHOOK 消息
    → 收到 HSHELL_FLASH（或 HSHELL_REDRAW 且 lParam = TRUE）
      → 读取 HWND → 获取 PID → 获取进程名
        → 查规则表（白名单 or 黑名单模式）
          → 去抖动 → 提取窗口标题
            → 发系统通知（Shell_NotifyIconW）
              → 点击通知 → SetForegroundWindow(hwnd)
```

### 配置参考

参见 [config.toml.example](config.toml.example)。重要字段：

| 字段                         | 说明                                                              |
|------------------------------|-------------------------------------------------------------------|
| `mode`                       | `whitelist` 或 `blacklist`                                        |
| `debounce_ms`                | 同一窗口去抖动窗口                                                |
| `autostart`                  | 写入/移除 HKCU `Run` 自启项                                       |
| `hot_reload`                 | 运行时热重载配置                                                  |
| `deduplicate_same_title`     | 相同窗口标题不重复弹                                              |
| `max_per_minute`             | 每进程每分钟频率上限，`0` 关闭                                    |
| `respect_quiet_hours`        | Windows 报告忙碌/勿扰时抑制通知                                   |
| `listen_redraw_flash`        | 默认开启 —— Windows 常把闪烁报告为 `HSHELL_REDRAW + lParam=true`  |
| `ignore_foreground_process`  | 抑制当前前台应用家族（含同名兄弟进程）                            |
| `web_ui`                     | 启用本地浏览器 Dashboard                                          |
| `web_ui_port`                | Dashboard 端口（`127.0.0.1`）                                     |
| `history_path`               | 通知历史 TSV 路径（留空 = 默认）                                  |
| `history_limit`              | 保留的历史行数                                                    |

每个 `[[apps]]`（或 `[[ignore]]`）条目：`process`、`display_name`、`icon`。

### 原生设置窗口

- **无控制台黑窗口** —— 用 `#![windows_subsystem = "windows"]` 编译。
- 双击 `flashbridge.exe` → 托盘图标 + 设置窗口出现。
- 关闭设置窗口只是隐藏，程序继续在托盘运行。
- 右键托盘 → **Open settings** 重新打开；**Test notification** 弹一个测试气泡；**Pause/Resume** 切换桥接开关。
- 由自启启动（或带 `--minimized`）时进入静默模式 —— 只有托盘图标，不弹设置窗口。

### 自测

无需真实消息即可验证核心闪烁链路：

```powershell
# 终端 1 —— 用自测配置运行（白名单 flashbridge.exe，关闭勿扰抑制）
.\target\debug\flashbridge.exe .\target\shell-flash-self-test.toml

# 终端 2 —— 一个测试窗口调用 FlashWindowEx
.\target\debug\flashbridge.exe --flash-test-window 4
```

通过的运行会在历史文件写入 `FlashBridge Self Test`。若不发送真实消息也要验证默认微信白名单路径，把编译产物复制为 `target\WeChat.exe`，白名单 `WeChat.exe`，再运行 `.\target\WeChat.exe --flash-test-window 4`。

### 构建与测试

```powershell
# Debug 构建
cargo build

# Release 构建
cargo build --release

# 格式化 / lint / 测试门禁（与 CI 一致）
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --release
```

### 发布产物

CI（`.github/workflows/build.yml`）在每个 PR 和 `v*` tag 上构建。推送 `v*` tag 会发布一个 GitHub Release，附带：

| 产物                       | 说明                                  |
|----------------------------|---------------------------------------|
| `flashbridge.exe`          | release 可执行文件                    |
| `FlashBridge-portable.zip` | 便携包：exe + 配置示例 + README       |

从 [Releases 页面](https://github.com/Cloudstill/FlashBridge/releases) 下载。

### 项目结构

```
flashbridge/
├── Cargo.toml              # 包元数据，build = "build.rs"
├── build.rs                # 编译 build.rc -> 通过 rc.exe 嵌入应用图标
├── build.rc                # Windows 资源脚本（应用图标）
├── config.toml.example     # 带注释的示例配置
├── icons/                  # icon.ico + generate-icon.ps1
└── src/
    ├── main.rs             # 入口：参数解析，--minimized，--flash-test-window
    ├── shell_hook.rs       # 隐藏 shell-hook 窗口 + RegisterShellHookWindow + 消息循环
    ├── processor.rs        # 事件处理、去抖动、规则匹配、频率限制、勿扰感知
    ├── toast.rs            # Shell_NotifyIconW 托盘通知 + 托盘右键菜单
    ├── native_ui.rs        # 原生 Win32 设置窗口（替代浏览器 Dashboard）
    ├── web_ui.rs           # 可选本地 Web Dashboard（默认关闭）
    ├── config.rs           # TOML 配置读写 + 默认路径
    ├── history.rs          # TSV 通知历史持久化
    ├── logger.rs           # 文件日志
    ├── autostart.rs        # HKCU Run 自启项
    ├── self_test.rs        # FlashWindowEx 端到端自测触发器
    └── win.rs              # Win32 FFI 类型与工具函数
```

### 路线图

- WinRT Toast 后端（完整通知中心集成 + AUMID），替代当前托盘气泡。
- 可选手机推送（发 toast 的同时 POST 到 ntfy），实现真正的跨设备同步。
- `SetWinEventHook` / UIAutomation 兜底，覆盖不走 `FlashWindow` 的应用。
- 代码签名，避免杀毒软件对全局事件监听器的误报。
- 可选 SQLite 历史后端替代 TSV 文件。

### 限制

- FlashBridge 只知道窗口闪烁了，**不读取**消息发送者或正文。
- `RegisterShellHookWindow` 非通用 API 契约，Win10/Win11 行为需在各环境实测。
- 当前后端是托盘气泡，非完整 WinRT Toast 通知中心集成（计划中，见路线图）。
- 误触发：微信在用户操作（如打开图片）时可能 redraw/flash，保持 `ignore_foreground_process = true`。

</details>

---

<div align="center">
<sub>Built with <a href="https://www.rust-lang.org/">Rust</a> · Win32 FFI · <a href="https://learn.microsoft.com/en-us/windows/win32/shell/registershellhookwindow">Shell Hook</a></sub>
</div>
