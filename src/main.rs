//! desktop — desktop UI automation: screenshot + click + type + key.
//! Linux/X11: scrot|import + xdotool. macOS: screencapture + cliclick or
//! osascript. Wayland and Windows report honest "no backend" errors.

use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::process::{Command, Stdio};

const NAME: &str = "desktop";

fn manifest() -> Value {
    json!({
        "name": NAME, "display_name": "Desktop",
        "version": env!("CARGO_PKG_VERSION"), "protocol": "1.1",
        "tools": [
            {"name": "desktop_shot",
             "description": "Screenshot the screen (or a region) to a PNG; returns the path — cat it to view.",
             "parameters": {"type": "object", "properties": {
                "path": {"type": "string"},
                "x": {"type": "integer"}, "y": {"type": "integer"},
                "w": {"type": "integer"}, "h": {"type": "integer"}}}},
            {"name": "desktop_click",
             "description": "Click at screen coordinates (button: left|middle|right, clicks N).",
             "parameters": {"type": "object", "properties": {
                "x": {"type": "integer"}, "y": {"type": "integer"},
                "button": {"type": "string"}, "clicks": {"type": "integer"}},
             "required": ["x","y"]}},
            {"name": "desktop_type",
             "description": "Type text at the current focus, then optionally press keys (e.g. 'Return', 'ctrl+s').",
             "parameters": {"type": "object", "properties": {
                "text": {"type": "string"},
                "key": {"type": "string", "description": "optional key chord after typing"}},
             "required": ["text"]}},
            {"name": "desktop_key",
             "description": "Press a key or chord (X11 syntax: 'ctrl+s', 'Return', 'alt+Tab').",
             "parameters": {"type": "object", "properties": {
                "key": {"type": "string"}}, "required": ["key"]}},
            {"name": "desktop_windows",
             "description": "List visible windows (title — geometry) where the platform supports it.",
             "parameters": {"type": "object", "properties": {}}},
        ],
        "commands": ["/desktop"], "subcommands": ["desktop"],
    })
}

fn have(b: &str) -> bool {
    #[cfg(windows)]
    let mut probe = {
        let mut c = Command::new("where");
        c.arg(b);
        c
    };
    #[cfg(not(windows))]
    let mut probe = {
        let mut c = Command::new("sh");
        c.args(["-c", &format!("command -v {b}")]);
        c
    };
    probe
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
fn is_macos() -> bool {
    cfg!(target_os = "macos")
}
fn wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

fn state_dir() -> std::path::PathBuf {
    let home = std::env::var_os("GRAY_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".gray")))
        .or_else(|| {
            std::env::var_os("USERPROFILE").map(|h| std::path::PathBuf::from(h).join(".gray"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    home.join("desktop")
}

fn run(b: &str, args: &[String]) -> Result<String, String> {
    let o = Command::new(b)
        .args(args)
        .output()
        .map_err(|e| format!("{b}: {e}"))?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).to_string())
    } else {
        Err(format!(
            "{b}: {}",
            String::from_utf8_lossy(&o.stderr).trim()
        ))
    }
}

fn shot(a: &Value) -> Result<Value, String> {
    let dir = state_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = a["path"].as_str().map(str::to_string).unwrap_or_else(|| {
        dir.join(format!("screen-{}.png", std::process::id()))
            .display()
            .to_string()
    });
    if is_macos() {
        let mut args = vec!["-x".to_string()];
        if let (Some(x), Some(y), Some(w), Some(h)) = (
            a["x"].as_i64(),
            a["y"].as_i64(),
            a["w"].as_i64(),
            a["h"].as_i64(),
        ) {
            args.push("-R".into());
            args.push(format!("{x},{y},{w},{h}"));
        }
        args.push(path.clone());
        run("screencapture", &args)?;
    } else if wayland() && have("grim") {
        let mut args = vec![];
        if let (Some(x), Some(y), Some(w), Some(h)) = (
            a["x"].as_i64(),
            a["y"].as_i64(),
            a["w"].as_i64(),
            a["h"].as_i64(),
        ) {
            args.push("-g".into());
            args.push(format!("{x},{y} {w}x{h}"));
        }
        args.push(path.clone());
        run("grim", &args)?;
    } else if have("import") {
        let mut args = vec!["-window".to_string(), "root".to_string()];
        if let (Some(x), Some(y), Some(w), Some(h)) = (
            a["x"].as_i64(),
            a["y"].as_i64(),
            a["w"].as_i64(),
            a["h"].as_i64(),
        ) {
            args.push("-crop".into());
            args.push(format!("{w}x{h}+{x}+{y}"));
        }
        args.push(path.clone());
        run("import", &args)?;
    } else if have("scrot") {
        run("scrot", std::slice::from_ref(&path))?;
    } else {
        return Err(
            "no screenshot backend — install scrot/imagemagick (X11), grim (Wayland), or use macOS"
                .into(),
        );
    }
    if !std::path::Path::new(&path).exists() {
        return Err("backend produced no image".into());
    }
    Ok(json!({"content": format!("screenshot → {path}"),
              "images": [{"path": path}]}))
}

fn click(a: &Value) -> Result<String, String> {
    let x = a["x"].as_i64().ok_or("x required")?;
    let y = a["y"].as_i64().ok_or("y required")?;
    let n = a["clicks"].as_u64().unwrap_or(1).to_string();
    let btn = match a["button"].as_str().unwrap_or("left") {
        "middle" => "2",
        "right" => "3",
        _ => "1",
    }
    .to_string();
    if is_macos() {
        if have("cliclick") {
            run("cliclick", &[format!("c:{x},{y}")])?;
        } else {
            run(
                "osascript",
                &[
                    "-e".into(),
                    format!("tell application \"System Events\" to click at {{{x}, {y}}}"),
                ],
            )?;
        }
    } else if have("xdotool") {
        run(
            "xdotool",
            &[
                "mousemove".into(),
                x.to_string(),
                y.to_string(),
                "click".into(),
                "--repeat".into(),
                n,
                btn,
            ],
        )?;
    } else if wayland() && have("wtype") {
        return Err("wtype can't click — pair desktop with ydotool for input on Wayland".into());
    } else {
        return Err("no input backend — install xdotool (X11) or cliclick (macOS)".into());
    }
    Ok(format!("clicked {x},{y}"))
}

fn type_text(a: &Value) -> Result<String, String> {
    let text = a["text"].as_str().ok_or("text required")?;
    if is_macos() {
        run(
            "osascript",
            &[
                "-e".into(),
                format!(
                    "tell application \"System Events\" to keystroke {}",
                    serde_json::to_string(&json!(text)).unwrap_or_else(|_| "\"\"".into())
                ),
            ],
        )?;
    } else if have("xdotool") {
        run(
            "xdotool",
            &["type".into(), "--delay".into(), "18".into(), text.into()],
        )?;
    } else if have("wtype") {
        run("wtype", &[text.into()])?;
    } else {
        return Err("no typing backend — xdotool (X11) / wtype (Wayland) / macOS".into());
    }
    if let Some(k) = a["key"].as_str() {
        key(&json!({"key": k}))?;
    }
    Ok(format!("typed {} chars", text.chars().count()))
}

fn key(a: &Value) -> Result<String, String> {
    let k = a["key"].as_str().ok_or("key required")?;
    if is_macos() {
        // chords like "cmd+s" → keystroke "s" using {command down};
        // bare keys map to key codes for the common names.
        let (mods, key) = {
            let parts: Vec<&str> = k.split('+').collect();
            let (m, k) = parts.split_at(parts.len().saturating_sub(1));
            (m.to_vec(), k.first().copied().unwrap_or(""))
        };
        let named: &[(&str, u32)] = &[
            ("Return", 36),
            ("Enter", 36),
            ("Escape", 53),
            ("esc", 53),
            ("Tab", 48),
            ("Delete", 51),
            ("BackSpace", 51),
            ("space", 49),
            ("Left", 123),
            ("Right", 124),
            ("Down", 125),
            ("Up", 126),
        ];
        let modifier = |m: &str| match m {
            "ctrl" | "control" => Some("control down"),
            "alt" | "opt" | "option" => Some("option down"),
            "shift" => Some("shift down"),
            "cmd" | "command" | "super" => Some("command down"),
            _ => None,
        };
        let script =
            if let Some((_, code)) = named.iter().find(|(n, _)| n.eq_ignore_ascii_case(key)) {
                if mods.is_empty() {
                    format!("tell application \"System Events\" to key code {code}")
                } else {
                    let ms: Vec<&str> = mods.iter().filter_map(|m| modifier(m)).collect();
                    format!(
                        "tell application \"System Events\" to key code {code} using {{{}}}",
                        ms.join(", ")
                    )
                }
            } else {
                let ms: Vec<&str> = mods.iter().filter_map(|m| modifier(m)).collect();
                if ms.is_empty() {
                    format!(
                        "tell application \"System Events\" to keystroke {}",
                        serde_json::to_string(&json!(key)).unwrap()
                    )
                } else {
                    format!(
                        "tell application \"System Events\" to keystroke {} using {{{}}}",
                        serde_json::to_string(&json!(key)).unwrap(),
                        ms.join(", ")
                    )
                }
            };
        run("osascript", &["-e".into(), script])?;
    } else if have("xdotool") {
        run("xdotool", &["key".into(), k.into()])?;
    } else if have("wtype") && !k.contains('+') {
        run("wtype", &["-k".into(), k.into()])?;
    } else {
        return Err("no key backend".into());
    }
    Ok(format!("pressed {k}"))
}

fn windows() -> Result<String, String> {
    if is_macos() {
        return run("osascript", &["-e".into(),
            "tell application \"System Events\" to get name of (processes where visible is true)".into()]);
    }
    if have("xdotool") {
        return run(
            "xdotool",
            &[
                "search".into(),
                "--onlyvisible".into(),
                "--name".into(),
                ".".into(),
                "getwindowname".into(),
                "%@".into(),
            ],
        );
    }
    if have("wmctrl") {
        return run("wmctrl", &["-lG".into()]);
    }
    Err("no window backend — install xdotool or wmctrl".into())
}

fn tool(name: &str, args: &Value) -> Value {
    if cfg!(windows) {
        return json!({"result": {"content": format!(
            "{name}: no Windows backend — desktop automation needs Linux (X11/Wayland) or macOS"),
            "is_error": true}});
    }
    let r = match name {
        "desktop_shot" => shot(args),
        "desktop_click" => click(args).map(|v| json!({"content": v})),
        "desktop_type" => type_text(args).map(|v| json!({"content": v})),
        "desktop_key" => key(args).map(|v| json!({"content": v})),
        "desktop_windows" => windows().map(|v| json!({"content": v})),
        t => Err(format!("unknown tool {t}")),
    };
    match r {
        Ok(v) => json!({"result": v}),
        Err(e) => json!({"result": {"content": e, "is_error": true}}),
    }
}

fn status() -> String {
    format!(
        "desktop\n  os: {}\n  shot: {}\n  input: {}",
        if is_macos() {
            "macOS"
        } else if cfg!(windows) {
            "Windows"
        } else if wayland() {
            "Linux/Wayland"
        } else {
            "Linux/X11?"
        },
        if is_macos() {
            "screencapture"
        } else if have("grim") {
            "grim"
        } else if have("import") {
            "import"
        } else if have("scrot") {
            "scrot"
        } else {
            "NONE"
        },
        if is_macos() {
            if have("cliclick") {
                "cliclick"
            } else {
                "osascript"
            }
        } else if have("xdotool") {
            "xdotool"
        } else if have("wtype") {
            "wtype"
        } else {
            "NONE"
        }
    )
}

fn sidecar() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let method = v["method"].as_str().unwrap_or("");
        match v.get("id").cloned() {
            None if method == "plugin/shutdown" => return Ok(()),
            None => {}
            Some(id) => {
                let reply = if method == "plugin/manifest" {
                    json!({"id": id, "result": manifest()})
                } else if method == "tool/call" {
                    let mut r = tool(
                        v["params"]["name"].as_str().unwrap_or(""),
                        &v["params"]["args"],
                    );
                    r["id"] = id;
                    r
                } else if method == "command/run" {
                    json!({"id": id, "result": {"text": status()}})
                } else {
                    json!({"id": id, "error": {"code": -32601,
                        "message": format!("method not found: {method}")}})
                };
                writeln!(out, "{reply}")?;
                out.flush()?;
            }
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<String> = match args.first().map(String::as_str) {
        Some("desktop") => args[1..].to_vec(),
        _ => args,
    };
    match args.first().map(String::as_str) {
        Some("manifest") => println!("{}", manifest()),
        Some("sidecar") => {
            let _ = sidecar();
        }
        Some("status") => println!("{}", status()),
        Some("-h" | "--help" | "help") => println!("{}", status()),
        _ => {
            if args.is_empty() {
                if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                    println!("{}", status());
                    return;
                }
                let _ = sidecar();
                return;
            }
            eprintln!("unknown verb");
            std::process::exit(1);
        }
    }
}
