# FlashBridge

> 把任意 Windows 应用的任务栏闪烁信号桥接为系统 Toast 通知的后台工具。

FlashBridge bridges Windows taskbar flash events to system notifications **without modifying or injecting into target apps**.

微信、钉钉等大量国产软件收到消息时只会让任务栏图标闪烁，而不走系统通知，导致勿扰模式失效、无法同步到手机、通知中心里看不到历史记录、多显示器/虚拟桌面下极易漏消息。FlashBridge 作为一个轻量后台服务插在中间，通过接收 Windows Shell Hook 的窗口闪烁通知（`HSHELL_FLASH` / `HSHELL_REDRAW`），为这些"只会闪任务栏"的应用补上现代系统通知。

Many Chinese apps (WeChat, DingTalk, …) only flash the taskbar on new messages instead of firing a system notification, which breaks Focus Assist, phone sync, and the Action Center history. FlashBridge sits in the middle as a lightweight background service, listening for Windows Shell Hook flash notifications and converting them into proper system notifications.

---

## 目录 / Table of Contents

- [功能特性 / Features](#功能特性--features)
- [工作原理 / How it works](#工作原理--how-it-works)
- [构建 / Build](#构建--build)
- [运行 / Run](#运行--run)
- [配置 / Config](#配置--config)
- [原生设置窗口 / Native settings window](#原生设置窗口--native-settings-window)
- [自测 / Self-test](#自测--self-test)
- [限制 / Limitations](#限制--limitations)

---

## 功能特性 / Features

- **零侵入**：不修改、不注入目标应用，仅监听 Shell Hook 事件。/ Non-invasive: no patching or injection — only Shell Hook events.
- **白名单 / 黑名单**模式过滤目标进程。/ Whitelist or blacklist process filtering.
- **去抖动 + 去重 + 频率限制**：同一窗口 500ms 内只触发一次，相同标题不重复弹，每进程每分钟最多 N 条。/ Debounce, deduplication, and per-process rate limiting.
- **勿扰模式感知**：忙碌/勿扰时不强制弹出（`SHQueryUserNotificationState`）。/ Respects Windows quiet hours.
- **前台进程抑制**：抑制你正在使用的应用（含同名兄弟进程）的误触发，例如在微信里打开图片。/ Suppresses events from the app currently in the foreground.
- **原生 Win32 设置窗口**：模式切换、进程列表编辑、全局开关、实时日志、通知历史、测试通知，关闭即隐藏到托盘。/ Native Win32 settings window with live log, history, and test notification.
- **开机自启 + 静默模式**：注册表 `Run` 自启，`--minimized` 启动后只在托盘出现图标。/ Autostart with silent tray-only mode.
- **配置热重载**：1 秒检查 config 修改时间，无需重启。/ Hot-reloads the config file.
- **文件日志 + TSV 通知历史**：默认 `%APPDATA%\FlashBridge\`。/ File logging and TSV notification history.
- **可选本地 Web Dashboard**：默认关闭，便于远程/headless 场景。/ Optional localhost web dashboard (off by default).
- **零第三方依赖**：纯 Win32 FFI，受限环境也能编译。/ Zero third-party crates — pure Win32 FFI.

## 工作原理 / How it works

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

FlashBridge registers a hidden shell-hook window, listens for `HSHELL_FLASH` (and `HSHELL_REDRAW` with `lParam = TRUE`, since Windows often reports flashing that way), reads the process name and window title, matches it against the rule table, debounces, and fires a tray notification. Clicking the notification focuses the target window via `SetForegroundWindow`.

## 构建 / Build

```powershell
cargo build --release
```

## 运行 / Run

```powershell
.\target\release\flashbridge.exe
```

默认会创建并使用 / By default FlashBridge creates and uses:

```text
%APPDATA%\FlashBridge\config.toml
%APPDATA%\FlashBridge\flash.log
%APPDATA%\FlashBridge\history.tsv
```

也可显式传入配置路径 / You can pass a config path explicitly:

```powershell
.\target\release\flashbridge.exe .\config.toml.example
```

## 配置 / Config

参见 / See [config.toml.example](config.toml.example). 重要字段 / Important fields:

| 字段 / Field | 说明 / Description |
|---|---|
| `mode` | `whitelist` 或 / or `blacklist` |
| `debounce_ms` | 同一窗口去抖动窗口 / debounce window for the same window |
| `autostart` | 写入/移除 HKCU Run 自启项 / write/remove the HKCU Run entry |
| `hot_reload` | 运行时热重载配置 / reload config changes while running |
| `deduplicate_same_title` | 相同窗口标题不重复弹 / suppress repeated notifications with the same title |
| `max_per_minute` | 每进程每分钟频率上限，`0` 关闭 / per-process rate limit, `0` disables |
| `respect_quiet_hours` | 勿扰时抑制通知 / suppress when Windows reports busy/quiet |
| `listen_redraw_flash` | 默认开启，因 Windows 常把闪烁报告为 `HSHELL_REDRAW + lParam=true` / on by default |
| `ignore_foreground_process` | 抑制当前前台应用（含同名兄弟进程），避免如微信打开图片等误触发 / suppress the foreground app family |
| `web_ui` | 启用本地浏览器 Dashboard / start the local browser dashboard |
| `web_ui_port` | Dashboard 端口 / dashboard port on `127.0.0.1` |
| `history_path` | 通知历史 TSV 路径 / notification history TSV path |
| `history_limit` | 保留的历史行数 / number of history rows to retain |

## 原生设置窗口 / Native settings window

双击 `flashbridge.exe` → 不弹控制台，托盘出现图标并弹出设置窗口。关闭设置窗口只是隐藏（程序继续在托盘运行）；右键托盘 → "Open settings" 可重新打开。

Double-click `flashbridge.exe` → no console window, a tray icon appears and the settings window opens. Closing the settings window only hides it (the app keeps running in the tray); right-click the tray → "Open settings" to reopen it.

重启电脑后若由自启启动，FlashBridge 进入静默模式，只在托盘出现图标，不弹设置窗口。

When launched via autostart, FlashBridge enters silent mode — only a tray icon appears, no settings window.

## 自测 / Self-test

无需真实消息即可验证核心闪烁链路 / Verify the core flash pipeline without a real chat message:

```powershell
# 终端 1 / Terminal 1
.\target\debug\flashbridge.exe .\target\shell-flash-self-test.toml

# 终端 2 / Terminal 2
.\target\debug\flashbridge.exe --flash-test-window 4
```

自测配置需白名单 `flashbridge.exe`、关闭勿扰抑制、`history_path` 指向临时 TSV。通过的运行会在历史文件里写入 `FlashBridge Self Test`。

For the self-test config, whitelist `flashbridge.exe`, disable quiet-hour suppression, and point `history_path` at a temporary TSV file. A passing run writes `FlashBridge Self Test` to the history file.

若不发送真实消息也要验证默认微信白名单路径，把编译产物复制为 `target\WeChat.exe`，白名单 `WeChat.exe`，再运行 / To verify the default WeChat whitelist path without a real message, copy the built binary to `target\WeChat.exe`, whitelist `WeChat.exe`, then run:

```powershell
.\target\WeChat.exe --flash-test-window 4
```

通过的运行会在历史文件里记录 `WeChat.exe` 和 `FlashBridge Self Test`。

## 限制 / Limitations

- FlashBridge 只知道窗口闪烁了，**不读取**消息发送者或消息正文。/ FlashBridge only knows a window flashed — it does **not** read message sender or body.
- `RegisterShellHookWindow` 非通用 API 契约，Win10/Win11 行为需实测。/ `RegisterShellHookWindow` is not a general-purpose API contract; Win10/Win11 behavior must be tested.
- 当前通知后端为托盘气泡通知，非完整 WinRT Toast 通知中心集成（后续可补）。/ The current backend is a tray balloon, not full WinRT Toast center integration (planned).
- 误触发场景：微信在用户操作（如打开图片）时可能 redraw/flash，保持 `ignore_foreground_process = true`。/ False positives: WeChat may redraw/flash during user actions like opening a photo — keep `ignore_foreground_process = true`.

## License

MIT
