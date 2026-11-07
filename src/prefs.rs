//! Shelf's own settings file. The tray and the settings window share the path
//! under Application Support. Tests each get a fresh file.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// The hour stepper stops at 30 days. Zero turns the hold off.
pub const MAX_KEEP_HOURS: u32 = 720;

/// Home-relative roots the user cannot remove. A leaf under one of these stays locked.
pub const ALWAYS: &[(&str, &str)] = &[
    ("Mail", "~/Library/Mail"),
    ("Messages", "~/Library/Messages"),
    ("iCloud Drive", "~/Library/Mobile Documents"),
    ("Cloud storage", "~/Library/CloudStorage"),
    ("Keychains", "~/Library/Keychains"),
    ("SSH", "~/.ssh"),
    ("GnuPG", "~/.gnupg"),
    ("AWS", "~/.aws"),
    ("Kubernetes", "~/.kube"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prefs {
    pub agent_id: String,
    pub agent_bin: String,
    pub allow_start: bool,
    pub send_paths: bool,
    pub gauge_memory: bool,
    pub gauge_cpu: bool,
    pub gauge_gpu: bool,
    pub shortcut_hints: bool,
    /// Extra hold, in hours. A leaf used more recently than this stays unticked. Zero is off.
    pub keep_hours: u32,
    pub kept: Vec<String>,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            agent_id: String::new(),
            agent_bin: String::new(),
            allow_start: false,
            send_paths: false,
            gauge_memory: true,
            gauge_cpu: true,
            gauge_gpu: true,
            shortcut_hints: true,
            keep_hours: 0,
            kept: Vec::new(),
        }
    }
}

pub struct Agent {
    pub id: &'static str,
    pub name: &'static str,
    pub bin: PathBuf,
}

pub fn user_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    home.join("Library/Application Support/Shelf/settings.txt")
}

pub fn path_for_state() -> PathBuf {
    #[cfg(test)]
    {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        return std::env::temp_dir().join(format!("shelf-prefs-{}-{n}.txt", std::process::id()));
    }
    #[cfg(not(test))]
    {
        user_path()
    }
}

pub fn load_from(path: &Path) -> Prefs {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Prefs::default();
    };
    let mut prefs = Prefs::default();
    prefs.kept.clear();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "agent_id" => prefs.agent_id = value.to_string(),
            "agent_bin" => prefs.agent_bin = value.to_string(),
            "allow_start" => prefs.allow_start = value == "1",
            "send_paths" => prefs.send_paths = value == "1",
            "gauge_memory" => prefs.gauge_memory = value == "1",
            "gauge_cpu" => prefs.gauge_cpu = value == "1",
            "gauge_gpu" => prefs.gauge_gpu = value == "1",
            "shortcut_hints" => prefs.shortcut_hints = value == "1",
            "keep_hours" => {
                prefs.keep_hours = value.parse::<u32>().unwrap_or(0).min(MAX_KEEP_HOURS)
            }
            "kept" if !value.is_empty() => prefs.kept.push(value.to_string()),
            _ => {}
        }
    }
    prefs
}

pub fn save_to(path: &Path, prefs: &Prefs) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut text = String::new();
    let flag = |on: bool| if on { "1" } else { "0" };
    text.push_str(&format!("agent_id={}\n", prefs.agent_id));
    text.push_str(&format!("agent_bin={}\n", prefs.agent_bin));
    text.push_str(&format!("allow_start={}\n", flag(prefs.allow_start)));
    text.push_str(&format!("send_paths={}\n", flag(prefs.send_paths)));
    text.push_str(&format!("gauge_memory={}\n", flag(prefs.gauge_memory)));
    text.push_str(&format!("gauge_cpu={}\n", flag(prefs.gauge_cpu)));
    text.push_str(&format!("gauge_gpu={}\n", flag(prefs.gauge_gpu)));
    text.push_str(&format!("shortcut_hints={}\n", flag(prefs.shortcut_hints)));
    text.push_str(&format!("keep_hours={}\n", prefs.keep_hours));
    for path in &prefs.kept {
        text.push_str("kept=");
        text.push_str(path);
        text.push('\n');
    }
    let tmp = path.with_extension("txt.tmp");
    if std::fs::write(&tmp, text).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

/// Command names from the ai-launcher catalog. The first binary found on PATH wins.
/// Shelf does not spawn these tools to detect them.
const TOOLS: &[(&str, &str, &[&str])] = &[
    ("claude", "Claude", &["claude"]),
    ("cline", "Cline", &["cline"]),
    ("gemini", "Gemini", &["gemini"]),
    ("agy", "Antigravity", &["agy"]),
    ("opencode", "OpenCode", &["opencode"]),
    ("amp", "Amp", &["amp"]),
    ("copilot", "Copilot", &["copilot"]),
    ("codex", "Codex", &["codex"]),
    ("interpreter", "Interpreter", &["interpreter"]),
    ("devin", "Devin", &["devin"]),
    ("kilo", "Kilo", &["kilo"]),
    ("pi", "Pi", &["pi"]),
    ("droid", "Droid", &["droid"]),
    ("ollama", "Ollama", &["ollama"]),
    ("cursor", "Cursor", &["cursor-agent", "cursor"]),
    ("cmd", "Command Code", &["cmd"]),
    ("freebuff", "Freebuff", &["freebuff"]),
    ("grok", "Grok", &["grok"]),
    ("mimo", "MiMo", &["mimo"]),
    ("kimi", "Kimi", &["kimi"]),
    ("reasonix", "Reasonix", &["reasonix"]),
    ("omp", "Oh My Pi", &["omp"]),
    ("fx", "fx", &["fx"]),
    ("jcode", "J-Code", &["jcode"]),
    ("pool", "Poolside", &["pool"]),
    ("muse", "Muse", &["muse"]),
    ("qodercli", "Qoder", &["qodercli", "qoder"]),
    ("dsh", "DeepSeek", &["dsh"]),
    ("kiro", "Kiro", &["kiro-cli", "kiro"]),
    ("ccs", "CCS", &["ccs"]),
];

pub fn detect_agents(exists: impl Fn(&str) -> Option<PathBuf>) -> Vec<Agent> {
    TOOLS
        .iter()
        .filter_map(|(id, name, bins)| {
            let bin = bins.iter().find_map(|bin| exists(bin))?;
            Some(Agent { id, name, bin })
        })
        .collect()
}

pub fn agents() -> &'static [Agent] {
    static CACHE: OnceLock<Vec<Agent>> = OnceLock::new();
    CACHE.get_or_init(|| detect_agents(find_bin))
}

fn find_bin(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        let candidate = dir.join(name);
        candidate.is_file().then_some(candidate)
    })
}

pub fn agent_name(id: &str) -> Option<&'static str> {
    if id == "custom" {
        return Some("your agent");
    }
    TOOLS
        .iter()
        .find(|(tool_id, _, _)| *tool_id == id)
        .map(|(_, name, _)| *name)
}

/// `path` is `root`, or a file inside `root`. `~/Library/Mail` does not match `~/Library/Mailer`.
pub fn path_under(path: &str, root: &str) -> bool {
    let path = path.trim_end_matches('/');
    let root = root.trim_end_matches('/');
    if root.is_empty() || path.is_empty() {
        return false;
    }
    path == root || path.starts_with(&format!("{root}/"))
}

pub fn expand_home(path: &str) -> String {
    if path == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).to_string_lossy().into_owned();
        }
    }
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(rest)
                .to_string_lossy()
                .into_owned();
        }
    }
    path.to_string()
}

pub fn always_name(path: &str) -> Option<&'static str> {
    ALWAYS
        .iter()
        .find_map(|(name, root)| path_under(path, &expand_home(root)).then_some(*name))
}

/// The folder picker. Cancel and a test run add nothing.
pub fn choose_folder() -> Option<String> {
    #[cfg(test)]
    {
        return None;
    }
    #[cfg(not(test))]
    {
        let output = Command::new("osascript")
            .args([
                "-e",
                "POSIX path of (choose folder with prompt \"Protected folder\")",
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let path = String::from_utf8(output.stdout).ok()?;
        let path = path.trim().trim_end_matches('/').to_string();
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    }
}

/// The question Ask would send, or why it cannot start.
pub fn prepare_ask(
    prefs: &Prefs,
    name: &str,
    path: &str,
    detail: &str,
) -> Result<String, &'static str> {
    if prefs.agent_bin.is_empty() {
        return Err("Choose an agent in Settings.");
    }
    if !prefs.allow_start {
        return Err("Grant access in Settings before Shelf starts an agent.");
    }
    let mut prompt = format!("Shelf item: {name}. {detail}. Confirm does not delete files.");
    if prefs.send_paths && !path.is_empty() {
        prompt.push_str(" Path: ");
        prompt.push_str(path);
        prompt.push('.');
    }
    Ok(prompt)
}

pub fn launch(bin: &str, prompt: &str) -> Result<(), String> {
    let dir = user_path()
        .parent()
        .ok_or_else(|| "Shelf has no settings folder.".to_string())?
        .to_path_buf();
    std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let ask = dir.join("ask.txt");
    std::fs::write(&ask, prompt).map_err(|err| err.to_string())?;
    let mut command = Command::new("open");
    if bin.ends_with(".app") {
        command.arg("-a").arg(bin);
        return spawn(command);
    }
    let script_path = dir.join("ask.command");
    let script = format!(
        "#!/bin/bash\nexec {} \"$(cat {})\"\n",
        shell_quote(bin),
        shell_quote(&ask.to_string_lossy())
    );
    std::fs::write(&script_path, script).map_err(|err| err.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755));
    }
    command.arg(&script_path);
    spawn(command)
}

fn spawn(mut command: Command) -> Result<(), String> {
    command
        .stdin(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|err| err.to_string())
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

pub fn open_full_disk_access() {
    let url =
        "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_AllFiles";
    let _ = Command::new("open").arg(url).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_round_trip_keeps_the_agent_and_the_grant() {
        let path = std::env::temp_dir().join(format!("shelf-prefs-round-{}", std::process::id()));
        let prefs = Prefs {
            agent_id: "claude".into(),
            agent_bin: "/usr/local/bin/claude".into(),
            allow_start: true,
            send_paths: true,
            keep_hours: 36,
            kept: vec!["/tmp/keep".into()],
            ..Prefs::default()
        };
        save_to(&path, &prefs);
        assert_eq!(load_from(&path), prefs);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn ask_waits_for_an_agent_and_for_access() {
        let mut prefs = Prefs::default();
        assert_eq!(
            prepare_ask(&prefs, "smolvm", "/tmp/smolvm", "In use"),
            Err("Choose an agent in Settings.")
        );
        prefs.agent_bin = "claude".into();
        assert_eq!(
            prepare_ask(&prefs, "smolvm", "/tmp/smolvm", "In use"),
            Err("Grant access in Settings before Shelf starts an agent.")
        );
        prefs.allow_start = true;
        let prompt = prepare_ask(&prefs, "smolvm", "/tmp/smolvm", "In use").unwrap();
        assert!(!prompt.contains("/tmp/smolvm"));
        prefs.send_paths = true;
        let prompt = prepare_ask(&prefs, "smolvm", "/tmp/smolvm", "In use").unwrap();
        assert!(prompt.contains("/tmp/smolvm"));
    }

    #[test]
    fn an_old_file_leaves_the_hour_hold_off() {
        let path = std::env::temp_dir().join(format!("shelf-prefs-old-{}", std::process::id()));
        std::fs::write(&path, "agent_id=grok\nkeep_hours=nope\n").unwrap();
        let prefs = load_from(&path);
        assert_eq!(prefs.agent_id, "grok");
        assert_eq!(prefs.keep_hours, 0);
        assert!(prefs.gauge_cpu);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_protected_root_stops_at_the_folder_boundary() {
        assert!(path_under(
            "/Users/me/Library/Mail/V10",
            "/Users/me/Library/Mail"
        ));
        assert!(path_under(
            "/Users/me/Library/Mail",
            "/Users/me/Library/Mail/"
        ));
        assert!(!path_under(
            "/Users/me/Library/Mailer",
            "/Users/me/Library/Mail"
        ));
        assert!(!path_under(
            "/Users/me/Library/MailExtra",
            "/Users/me/Library/Mail"
        ));
        assert!(!path_under("/tmp/x", ""));
        let mail = expand_home("~/Library/Mail/V10");
        assert_eq!(always_name(&mail), Some("Mail"));
        assert_eq!(
            always_name(&expand_home("~/.kube/config")),
            Some("Kubernetes")
        );
        assert_eq!(always_name(&expand_home("~/Library/Mailer")), None);
        assert_eq!(always_name("/fixture/logs"), None);
    }

    #[test]
    fn detection_uses_the_first_binary_and_skips_the_rest() {
        let dir = std::env::temp_dir().join(format!("shelf-agents-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["kimi", "cursor", "cursor-agent", "qoder", "kiro"] {
            std::fs::write(dir.join(name), b"").unwrap();
        }
        let found = detect_agents(|name| {
            let candidate = dir.join(name);
            candidate.is_file().then_some(candidate)
        });
        let ids: Vec<_> = found.iter().map(|agent| agent.id).collect();
        assert_eq!(ids, vec!["cursor", "kimi", "qodercli", "kiro"]);
        assert_eq!(
            found.iter().find(|agent| agent.id == "cursor").unwrap().bin,
            dir.join("cursor-agent")
        );
        assert_eq!(
            found.iter().find(|agent| agent.id == "kiro").unwrap().bin,
            dir.join("kiro")
        );
        assert!(agent_name("kimi") == Some("Kimi"));
        assert!(agent_name("ccs") == Some("CCS"));
        assert!(agent_name("custom") == Some("your agent"));
        assert!(agent_name("missing").is_none());
        assert!(choose_folder().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
