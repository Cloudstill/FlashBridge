# FlashBridge

FlashBridge bridges Windows taskbar flash events to system notifications without modifying or injecting into target apps.

## Current Status

The current build is a native Win32 MVP:

- Registers a hidden shell-hook window and listens for `HSHELL_FLASH`.
- Filters target windows by process name.
- Sends a tray notification balloon through `Shell_NotifyIconW`.
- Right-click tray menu: pause/resume, open config, test notification, exit.
- Hot-reloads the config file.
- Supports autostart through `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
- Writes logs to `%APPDATA%\FlashBridge\flash.log` by default.
- Serves a local dashboard on `127.0.0.1` for settings, live logs, and notification history.
- Writes notification history to `%APPDATA%\FlashBridge\history.tsv` by default.

WinRT Toast and a Tauri shell can be added later. This version avoids third-party crates so it can build in restricted environments.

## Build

```powershell
cargo +stable build
```

## Run

```powershell
.\target\debug\flashbridge.exe
```

By default FlashBridge creates and uses:

```text
%APPDATA%\FlashBridge\config.toml
```

You can pass a config path explicitly:

```powershell
.\target\debug\flashbridge.exe .\config.toml.example
```

## Config

See [config.toml.example](config.toml.example).

Important fields:

- `mode`: `whitelist` or `blacklist`.
- `debounce_ms`: suppress repeated flash events from the same window.
- `autostart`: write/remove the HKCU Run entry.
- `hot_reload`: reload config changes while running.
- `deduplicate_same_title`: suppress repeated notifications with the same window title.
- `max_per_minute`: per-process rate limit, `0` disables it.
- `respect_quiet_hours`: suppress notifications when Windows reports a busy/quiet notification state.
- `listen_redraw_flash`: enabled by default because Windows often reports taskbar flashing as `HSHELL_REDRAW + lParam=true`.
- `ignore_foreground_process`: suppress events from the app you are currently using, including configured sibling processes with the same display name. This avoids false positives such as opening a photo in WeChat.
- `web_ui`: start the local browser dashboard.
- `web_ui_port`: dashboard port on `127.0.0.1`.
- `history_path`: notification history TSV path.
- `history_limit`: number of history rows to retain.

## Dashboard

Right-click the tray icon and choose `Open dashboard`, or open:

```text
http://127.0.0.1:47621
```

The dashboard lets you edit settings, inspect running process names, manage configured process lists, view live logs, and inspect notification history. It only binds to localhost.

## Shell Hook Self-Test

Use this to verify the core flash pipeline without waiting for a real chat message:

```powershell
# Terminal 1
.\target\debug\flashbridge.exe .\target\shell-flash-self-test.toml

# Terminal 2
.\target\debug\flashbridge.exe --flash-test-window 4
```

For the self-test config, whitelist `flashbridge.exe`, disable quiet-hour suppression, and point `history_path` at a temporary TSV file. A passing run writes `FlashBridge Self Test` to the history file.

To verify the default WeChat whitelist path without sending a real chat message, copy the built binary to `target\WeChat.exe`, whitelist `WeChat.exe`, then run:

```powershell
.\target\WeChat.exe --flash-test-window 4
```

A passing run records `WeChat.exe` and `FlashBridge Self Test` in the history file.

## Test Path

1. Run FlashBridge.
2. Confirm the tray icon appears.
3. Right-click the tray icon and choose `Test notification`.
4. Minimize WeChat, DingTalk, or another configured app.
5. Trigger a message and check whether a notification appears.

If no notification appears, inspect:

```text
%APPDATA%\FlashBridge\flash.log
```

Notification history is stored at:

```text
%APPDATA%\FlashBridge\history.tsv
```

## Limitations

- FlashBridge only knows that a window flashed. It does not read message sender or message body.
- `RegisterShellHookWindow` is not a general-purpose API contract, so Win10/Win11 behavior must be tested.
- The current notification backend is tray balloon notification, not full WinRT Toast notification center integration.
- The current dashboard is a local browser UI rather than a Tauri-packaged window.

## False Positives

WeChat may redraw or flash windows during user-initiated actions such as opening a photo. Keep `ignore_foreground_process = true` so FlashBridge suppresses events from the app family currently in the foreground. Keep `listen_redraw_flash = true` unless you confirm your Windows build emits reliable `HSHELL_FLASH` events, because many flash events arrive as `HSHELL_REDRAW` with `lParam = TRUE`.
