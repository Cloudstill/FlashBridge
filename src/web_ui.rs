use crate::{
    config::{AppRule, Config, Mode},
    history, win, Result,
};
use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

pub struct WebUiHandle {
    url: String,
    state: Arc<Mutex<WebUiState>>,
}

#[derive(Clone)]
struct WebUiState {
    config_path: PathBuf,
    log_path: PathBuf,
    history_path: PathBuf,
    history_limit: usize,
}

impl WebUiHandle {
    pub fn start(
        config_path: PathBuf,
        log_path: PathBuf,
        history_path: PathBuf,
        history_limit: usize,
        port: u16,
    ) -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        let url = format!("http://{}", listener.local_addr()?);
        let state = Arc::new(Mutex::new(WebUiState {
            config_path,
            log_path,
            history_path,
            history_limit,
        }));
        let thread_state = Arc::clone(&state);

        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let state = Arc::clone(&thread_state);
                        if let Err(error) = handle_connection(stream, state) {
                            eprintln!("web ui request failed: {error}");
                        }
                    }
                    Err(error) => eprintln!("web ui accept failed: {error}"),
                }
            }
        });

        Ok(Self { url, state })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn update_paths(&self, log_path: PathBuf, history_path: PathBuf, history_limit: usize) {
        if let Ok(mut state) = self.state.lock() {
            state.log_path = log_path;
            state.history_path = history_path;
            state.history_limit = history_limit;
        }
    }
}

fn handle_connection(mut stream: TcpStream, state: Arc<Mutex<WebUiState>>) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    let request = read_request(&mut stream)?;
    let state = state
        .lock()
        .map_err(|_| "web ui state lock poisoned")?
        .clone();

    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => respond_html(&mut stream, &render_dashboard(&state)),
        ("GET", "/log") => respond_text(&mut stream, &tail_text(&state.log_path, 240)),
        ("GET", "/history") => respond_html(&mut stream, &render_history_rows(&state)),
        ("POST", "/config") => {
            save_config_from_form(&state.config_path, &request.body)?;
            respond_redirect(&mut stream, "/")
        }
        _ => respond_not_found(&mut stream),
    }
}

struct Request {
    method: String,
    path: String,
    body: String,
}

fn read_request(stream: &mut TcpStream) -> Result<Request> {
    let mut buffer = Vec::new();
    let mut temp = [0u8; 4096];
    let mut header_end = None;
    let mut content_length = 0usize;

    loop {
        let read = stream.read(&mut temp)?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&temp[..read]);

        if header_end.is_none() {
            header_end = find_header_end(&buffer);
            if let Some(end) = header_end {
                let header = String::from_utf8_lossy(&buffer[..end]);
                content_length = parse_content_length(&header);
            }
        }

        if let Some(end) = header_end {
            if buffer.len() >= end + 4 + content_length {
                break;
            }
        }
    }

    let end = header_end.ok_or("invalid HTTP request")?;
    let header = String::from_utf8_lossy(&buffer[..end]);
    let first_line = header.lines().next().ok_or("empty HTTP request")?;
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts
        .next()
        .unwrap_or("/")
        .split('?')
        .next()
        .unwrap_or("/")
        .to_string();
    let body_bytes = &buffer[end + 4..buffer.len().min(end + 4 + content_length)];
    let body = String::from_utf8_lossy(body_bytes).into_owned();

    Ok(Request { method, path, body })
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn parse_content_length(header: &str) -> usize {
    header
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn render_dashboard(state: &WebUiState) -> String {
    let config = Config::load(&state.config_path).unwrap_or_default();
    let apps = rules_to_text(&config.apps);
    let ignore = rules_to_text(&config.ignore);
    let mode_whitelist = selected(config.mode == Mode::Whitelist);
    let mode_blacklist = selected(config.mode == Mode::Blacklist);

    format!(
        r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <title>FlashBridge</title>
  <style>
    body {{ margin: 0; font: 14px/1.4 "Segoe UI", Arial, sans-serif; color: #202124; background: #f7f8fa; }}
    header {{ background: #17202a; color: #fff; padding: 16px 24px; }}
    main {{ display: grid; grid-template-columns: minmax(360px, 520px) 1fr; gap: 16px; padding: 16px; }}
    section {{ background: #fff; border: 1px solid #d9dee7; border-radius: 8px; padding: 16px; }}
    h1 {{ margin: 0; font-size: 20px; }}
    h2 {{ margin: 0 0 12px; font-size: 16px; }}
    label {{ display: block; margin: 10px 0 4px; font-weight: 600; }}
    input[type=text], input[type=number], select, textarea {{ width: 100%; box-sizing: border-box; padding: 8px; border: 1px solid #c8ced8; border-radius: 6px; font: inherit; }}
    textarea {{ min-height: 104px; font-family: Consolas, monospace; }}
    .row {{ display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }}
    .check {{ display: flex; gap: 8px; align-items: center; margin: 8px 0; font-weight: 400; }}
    button {{ margin-top: 12px; padding: 8px 14px; border: 0; border-radius: 6px; background: #1b6ef3; color: white; font-weight: 600; cursor: pointer; }}
    pre {{ overflow: auto; max-height: 320px; background: #101418; color: #d8dee9; padding: 12px; border-radius: 6px; }}
    table {{ width: 100%; border-collapse: collapse; }}
    th, td {{ border-bottom: 1px solid #e3e7ee; padding: 8px; text-align: left; vertical-align: top; }}
    th {{ background: #f0f3f8; }}
    .hint {{ color: #667085; font-size: 12px; }}
  </style>
</head>
<body>
  <header><h1>FlashBridge</h1><div>{config_path}</div></header>
  <main>
    <section>
      <h2>Settings</h2>
      <form method="post" action="/config">
        <label>Mode</label>
        <select name="mode">
          <option value="whitelist" {mode_whitelist}>Whitelist</option>
          <option value="blacklist" {mode_blacklist}>Blacklist</option>
        </select>
        <div class="row">
          <div><label>Debounce ms</label><input type="number" name="debounce_ms" min="0" value="{debounce_ms}"></div>
          <div><label>Max per minute</label><input type="number" name="max_per_minute" min="0" value="{max_per_minute}"></div>
        </div>
        {checks}
        <div class="row">
          <div><label>Web UI port</label><input type="number" name="web_ui_port" min="0" max="65535" value="{web_ui_port}"></div>
          <div><label>History limit</label><input type="number" name="history_limit" min="0" value="{history_limit}"></div>
        </div>
        <label>Log path</label><input type="text" name="log_path" value="{log_path}">
        <label>History path</label><input type="text" name="history_path" value="{history_path}">
        <label>Apps</label>
        <textarea name="apps">{apps}</textarea>
        <div class="hint">One per line: process.exe | Display name | optional-icon-path</div>
        <label>Ignore</label>
        <textarea name="ignore">{ignore}</textarea>
        <button type="submit">Save settings</button>
      </form>
      <h2 style="margin-top:18px">Running processes</h2>
      <div class="hint">Copy process names into Apps or Ignore.</div>
      <pre>{processes}</pre>
    </section>
    <div>
      <section>
        <h2>Live log</h2>
        <pre id="log">Loading...</pre>
      </section>
      <section style="margin-top:16px">
        <h2>Notification history</h2>
        <table><thead><tr><th>Time</th><th>Process</th><th>Title</th><th>Body</th></tr></thead><tbody id="history"></tbody></table>
      </section>
    </div>
  </main>
  <script>
    async function refresh() {{
      document.getElementById('log').textContent = await (await fetch('/log')).text();
      document.getElementById('history').innerHTML = await (await fetch('/history')).text();
    }}
    refresh();
    setInterval(refresh, 2000);
  </script>
</body>
</html>"#,
        config_path = html_escape(&state.config_path.display().to_string()),
        debounce_ms = config.debounce_ms,
        max_per_minute = config.max_per_minute,
        web_ui_port = config.web_ui_port,
        history_limit = config.history_limit,
        log_path = html_escape(
            &config
                .log_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default()
        ),
        history_path = html_escape(
            &config
                .history_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default()
        ),
        checks = render_checks(&config),
        apps = html_escape(&apps),
        ignore = html_escape(&ignore),
        processes = html_escape(&win::running_process_names().join("\n")),
    )
}

fn render_checks(config: &Config) -> String {
    [
        ("sound", "Sound", config.sound),
        ("autostart", "Autostart", config.autostart),
        ("hot_reload", "Hot reload", config.hot_reload),
        (
            "listen_redraw_flash",
            "Treat HSHELL_REDRAW as flash",
            config.listen_redraw_flash,
        ),
        (
            "ignore_foreground_process",
            "Ignore foreground process",
            config.ignore_foreground_process,
        ),
        (
            "deduplicate_same_title",
            "Deduplicate same title",
            config.deduplicate_same_title,
        ),
        (
            "respect_quiet_hours",
            "Respect Windows quiet/busy state",
            config.respect_quiet_hours,
        ),
        ("web_ui", "Enable web dashboard", config.web_ui),
    ]
    .into_iter()
    .map(|(name, label, checked)| {
        format!(
            r#"<label class="check"><input type="checkbox" name="{name}" value="1" {checked}> {label}</label>"#,
            checked = if checked { "checked" } else { "" }
        )
    })
    .collect::<Vec<_>>()
    .join("")
}

fn render_history_rows(state: &WebUiState) -> String {
    history::read_tail(&state.history_path, state.history_limit)
        .into_iter()
        .rev()
        .map(|entry| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                entry.timestamp,
                html_escape(&entry.process),
                html_escape(&entry.title),
                html_escape(&entry.body)
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn save_config_from_form(path: &Path, body: &str) -> Result<()> {
    let form = parse_form(body);
    let text = format!(
        r#"# Global settings
mode = "{mode}"
debounce_ms = {debounce_ms}
sound = {sound}
autostart = {autostart}
hot_reload = {hot_reload}
listen_redraw_flash = {listen_redraw_flash}
ignore_foreground_process = {ignore_foreground_process}
deduplicate_same_title = {deduplicate_same_title}
max_per_minute = {max_per_minute}
respect_quiet_hours = {respect_quiet_hours}
log_path = "{log_path}"
history_path = "{history_path}"
history_limit = {history_limit}
web_ui = {web_ui}
web_ui_port = {web_ui_port}

{apps}
{ignore}
"#,
        mode = form_value(&form, "mode", "whitelist"),
        debounce_ms = number_value(&form, "debounce_ms", "500"),
        sound = checkbox(&form, "sound"),
        autostart = checkbox(&form, "autostart"),
        hot_reload = checkbox(&form, "hot_reload"),
        listen_redraw_flash = checkbox(&form, "listen_redraw_flash"),
        ignore_foreground_process = checkbox(&form, "ignore_foreground_process"),
        deduplicate_same_title = checkbox(&form, "deduplicate_same_title"),
        max_per_minute = number_value(&form, "max_per_minute", "20"),
        respect_quiet_hours = checkbox(&form, "respect_quiet_hours"),
        log_path = toml_escape(&form_value(&form, "log_path", "")),
        history_path = toml_escape(&form_value(&form, "history_path", "")),
        history_limit = number_value(&form, "history_limit", "500"),
        web_ui = checkbox(&form, "web_ui"),
        web_ui_port = number_value(&form, "web_ui_port", "47621"),
        apps = rules_from_text("apps", &form_value(&form, "apps", "")),
        ignore = rules_from_text("ignore", &form_value(&form, "ignore", "")),
    );

    fs::write(path, text)?;
    Ok(())
}

fn rules_from_text(section: &str, text: &str) -> String {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            let mut parts = line.split('|').map(str::trim);
            let process = parts.next().unwrap_or_default();
            if process.is_empty() {
                return None;
            }
            let display = parts.next().unwrap_or_default();
            let icon = parts.next().unwrap_or_default();
            let mut block = format!("[[{section}]]\nprocess = \"{}\"\n", toml_escape(process));
            if !display.is_empty() {
                block.push_str(&format!("display_name = \"{}\"\n", toml_escape(display)));
            }
            if !icon.is_empty() {
                block.push_str(&format!("icon = \"{}\"\n", toml_escape(icon)));
            }
            Some(block)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn rules_to_text(rules: &[AppRule]) -> String {
    rules
        .iter()
        .map(|rule| {
            format!(
                "{} | {} | {}",
                rule.process,
                rule.display_name.clone().unwrap_or_default(),
                rule.icon
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn tail_text(path: &Path, limit: usize) -> String {
    let text = fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<&str> = text.lines().collect();
    if lines.len() > limit {
        let keep_from = lines.len() - limit;
        lines.drain(0..keep_from);
    }
    lines.join("\n")
}

fn parse_form(body: &str) -> HashMap<String, String> {
    body.split('&')
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            Some((url_decode(key).ok()?, url_decode(value).ok()?))
        })
        .collect()
}

fn form_value(form: &HashMap<String, String>, key: &str, default: &str) -> String {
    form.get(key)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| default.to_string())
}

fn number_value(form: &HashMap<String, String>, key: &str, default: &str) -> String {
    form_value(form, key, default)
        .parse::<u64>()
        .map(|value| value.to_string())
        .unwrap_or_else(|_| default.to_string())
}

fn checkbox(form: &HashMap<String, String>, key: &str) -> bool {
    form.contains_key(key)
}

fn url_decode(value: &str) -> Result<String> {
    let mut bytes = Vec::with_capacity(value.len());
    let mut iter = value.as_bytes().iter().copied();
    while let Some(byte) = iter.next() {
        match byte {
            b'+' => bytes.push(b' '),
            b'%' => {
                let high = iter.next().ok_or("incomplete percent escape")?;
                let low = iter.next().ok_or("incomplete percent escape")?;
                let hex = [high, low];
                let text = std::str::from_utf8(&hex)?;
                bytes.push(u8::from_str_radix(text, 16)?);
            }
            other => bytes.push(other),
        }
    }
    Ok(String::from_utf8(bytes)?)
}

fn selected(value: bool) -> &'static str {
    if value {
        "selected"
    } else {
        ""
    }
}

fn toml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn respond_html(stream: &mut TcpStream, body: &str) -> Result<()> {
    respond(
        stream,
        "200 OK",
        "text/html; charset=utf-8",
        body.as_bytes(),
    )
}

fn respond_text(stream: &mut TcpStream, body: &str) -> Result<()> {
    respond(
        stream,
        "200 OK",
        "text/plain; charset=utf-8",
        body.as_bytes(),
    )
}

fn respond_not_found(stream: &mut TcpStream) -> Result<()> {
    respond(
        stream,
        "404 Not Found",
        "text/plain; charset=utf-8",
        b"Not found",
    )
}

fn respond_redirect(stream: &mut TcpStream, location: &str) -> Result<()> {
    let response = format!(
        "HTTP/1.1 303 See Other\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(response.as_bytes())?;
    Ok(())
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) -> Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    Ok(())
}
