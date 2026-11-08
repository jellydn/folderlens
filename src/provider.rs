//! Provider-specific CLI arguments and private run configuration, shared by analysis and resume.

use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Provider {
    Grok,
    Antigravity,
    Copilot,
    Pi,
    OpenCode,
}

impl Provider {
    pub fn for_bin(bin: &str) -> Result<Self, String> {
        match Path::new(bin).file_name().and_then(|name| name.to_str()) {
            Some("grok") => Ok(Self::Grok),
            Some("agy") => Ok(Self::Antigravity),
            Some("copilot") => Ok(Self::Copilot),
            Some("pi") => Ok(Self::Pi),
            Some("opencode") => Ok(Self::OpenCode),
            _ => Err("Choose Grok, Antigravity, Copilot, Pi or OpenCode in Agent settings. This executable has no Shelf adapter; Shelf will not switch agents.".into()),
        }
    }

    pub fn validate_model(self, model: &str) -> Result<(), String> {
        if model.len() > 256
            || model.starts_with('-')
            || model.chars().any(|c| c.is_whitespace() || c.is_control())
        {
            return Err(
                "Enter one model ID, not CLI flags or a command. Do not enter API keys.".into(),
            );
        }
        if matches!(self, Self::Pi | Self::OpenCode)
            && !model.is_empty()
            && !model
                .split_once('/')
                .is_some_and(|(provider, id)| !provider.is_empty() && !id.is_empty())
        {
            return Err("Use an exact model ID with its provider prefix. Shelf does not use fuzzy model matches or a paid fallback.".into());
        }
        if self == Self::OpenCode && model.is_empty() {
            return Err("Choose an explicit model with its provider prefix for OpenCode. Shelf will not select an automatic paid model.".into());
        }
        Ok(())
    }
}

pub(super) struct Launch {
    pub provider: Provider,
    pub command: Command,
    pub resume: Command,
    pub model: String,
}

impl Launch {
    pub fn prepare(
        bin: &str,
        model: &str,
        dir: &Path,
        socket: &Path,
        cancel: &AtomicU8,
    ) -> Result<Self, String> {
        let provider = Provider::for_bin(bin)?;
        provider.validate_model(model)?;
        let exe = std::env::current_exe().map_err(|_| "Cannot locate Shelf reader executable")?;
        let mut common = Vec::<String>::new();
        let mut env = Vec::<(&str, PathBuf)>::new();
        let mut unset = Vec::new();
        let (analysis, resume): (&[&str], &[&str]) = match provider {
            Provider::Grok => {
                common.extend(
                    ["--permission-mode", "plan", "--sandbox", "read-only"].map(String::from),
                );
                env.push(("GROK_FOLDER_TRUST", PathBuf::from("0")));
                (&["agent", "stdio"], &["--resume"])
            }
            Provider::Antigravity => {
                common.extend(["--mode", "plan", "--sandbox"].map(String::from));
                (&["acp"], &["--conversation"])
            }
            Provider::Copilot => {
                let mcp = json!({"mcpServers":{"shelf":{"type":"stdio","command":exe,"args":["--shelf-reader",socket],"tools":["read_selected"]}}});
                common.extend(
                    [
                        "--mode",
                        "plan",
                        "--sandbox",
                        "--no-auto-update",
                        "--no-custom-instructions",
                        "--disable-builtin-mcps",
                        "--no-bash-env",
                        "--sync-session=off",
                        "--available-tools=shelf-read_selected",
                        "--allow-tool=shelf(read_selected)",
                        "--additional-mcp-config",
                    ]
                    .map(String::from),
                );
                common.push(mcp.to_string());
                unset.extend(["COPILOT_ALLOW_ALL", "COPILOT_ASSISTED_APPROVAL"]);
                (&["--acp"], &["--resume"])
            }
            Provider::Pi => {
                let extension = dir.join("shelf-reader.mjs");
                // Only this explicit extension loads. The model never controls its executable or socket.
                std::fs::write(&extension, include_str!("pi-reader.mjs"))
                    .map_err(|_| "Cannot save Shelf reader extension")?;
                env.extend([
                    ("SHELF_READER_EXECUTABLE", exe.clone()),
                    ("SHELF_READER_SOCKET", socket.into()),
                    ("SHELF_EXPECTED_MODEL", model.into()),
                ]);
                common.extend(
                    [
                        "--no-builtin-tools",
                        "--no-extensions",
                        "--no-mcp",
                        "--no-skills",
                        "--no-prompt-templates",
                        "--no-context-files",
                        "--no-approve",
                        "--offline",
                        "--tools",
                        "shelf_read_selected",
                        "--extension",
                    ]
                    .map(String::from),
                );
                common.push(extension.to_string_lossy().into_owned());
                common.push("--session-dir".into());
                common.push(dir.join("sessions").to_string_lossy().into_owned());
                (&["--mode", "rpc"], &["--session"])
            }
            Provider::OpenCode => {
                // These schemas are verified against 2.0.23, not current 2.2+ web examples.
                verify_opencode(std::ffi::OsStr::new(bin), cancel)?;
                let permissions = json!([{"action":"*","resource":"*","effect":"deny"},{"action":"shelf_read_selected","resource":"*","effect":"allow"}]);
                let config = json!({
                    "model":model,"default_agent":"shelf-readonly","update":"disable","share":"disabled","snapshots":false,"warming":false,
                    "permissions":permissions,
                    "agents":{
                        "shelf-readonly":{"mode":"primary","model":model,"description":"Shelf selected-folder review only","permissions":permissions},
                        "title":{"model":model},"compaction":{"model":model}
                    },
                    "mcp":{"servers":{"shelf":{"type":"local","command":[exe,"--shelf-reader",socket],"codemode":false}}},
                    "experimental":{"policies":[{"action":"permission","resource":"*","effect":"deny"},{"action":"permission","resource":"shelf_read_selected:*","effect":"allow"}]}
                });
                let config_path = dir.join("opencode.json");
                std::fs::write(&config_path, config.to_string())
                    .map_err(|_| "Cannot save private OpenCode configuration")?;
                for name in ["config", "home"] {
                    std::fs::create_dir_all(dir.join(name))
                        .map_err(|_| "Cannot create private OpenCode configuration directory")?;
                }
                env.extend([
                    ("OPENCODE_CONFIG", config_path),
                    ("OPENCODE_CONFIG_DIR", dir.join("config")),
                    // This installed version uses this override for ambient ~/.agents and ~/.claude discovery.
                    ("OPENCODE_TEST_HOME", dir.join("home")),
                    ("OPENCODE_CONFIG_PROJECT_DISABLE", "1".into()),
                ]);
                unset.push("OPENCODE_CONFIG_CONTENT");
                (&["acp"], &["--standalone", "--session"])
            }
        };
        if !model.is_empty() && provider != Provider::OpenCode {
            common.extend(["--model".into(), model.into()]);
        } else if provider == Provider::Copilot {
            common.extend(["--model".into(), "auto".into()]);
        }
        let make = |tail: &[&str]| {
            let mut command = Command::new(bin);
            command
                .args(&common)
                .args(tail)
                .current_dir(dir)
                .env("NO_COLOR", "1");
            for (key, value) in &env {
                command.env(key, value);
            }
            for key in &unset {
                command.env_remove(key);
            }
            command
        };
        let mut resume_command = make(resume);
        if provider == Provider::OpenCode {
            // A saved launcher must recheck after CLI upgrades, even while Shelf is closed.
            let mut checked = Command::new(&exe);
            checked
                .arg("--shelf-opencode-resume")
                .arg(bin)
                .args(resume_command.get_args())
                .current_dir(dir);
            for (key, value) in resume_command.get_envs() {
                if let Some(value) = value {
                    checked.env(key, value);
                } else {
                    checked.env_remove(key);
                }
            }
            resume_command = checked;
        }
        Ok(Self {
            provider,
            command: make(analysis),
            resume: resume_command,
            model: model.into(),
        })
    }
}

fn verify_opencode(bin: &std::ffi::OsStr, cancel: &AtomicU8) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) == super::CANCELLED {
        return Err("Agent review cancelled.".into());
    }
    let mut child = Command::new(bin)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|_| "Cannot check OpenCode version")?;
    let result = (|| {
        let mut stdout = child.stdout.take().ok_or("Cannot read OpenCode version")?;
        let fd = stdout.as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err("Cannot bound OpenCode version check".into());
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut output = Vec::new();
        let mut eof = false;
        loop {
            if cancel.load(Ordering::Relaxed) == super::CANCELLED {
                return Err("Agent review cancelled.".into());
            }
            if Instant::now() >= deadline {
                return Err("OpenCode version check timed out. Check the CLI in Agent settings; no review was sent.".into());
            }
            let mut bytes = [0; 257];
            match stdout.read(&mut bytes) {
                Ok(0) => eof = true,
                Ok(n) => output.extend_from_slice(&bytes[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => return Err("Cannot read OpenCode version".into()),
            }
            if output.len() > 256 {
                return Err("OpenCode version output exceeds its limit.".into());
            }
            if let Some(status) = child
                .try_wait()
                .map_err(|_| "Cannot check OpenCode version")?
            {
                if eof {
                    let output =
                        String::from_utf8(output).map_err(|_| "Invalid OpenCode version output")?;
                    let version = output
                        .trim()
                        .strip_prefix("opencode v")
                        .unwrap_or(output.trim());
                    return if status.success() && version == "2.0.23" {
                        Ok(())
                    } else {
                        Err("Shelf's private OpenCode adapter requires 2.0.23. This version's policy and configuration schema is not verified. No review was sent; choose another agent in Settings.".into())
                    };
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    })();
    // Descendants can retain stdout after the direct process exits. Never leave a drainer behind.
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.wait();
    result
}

pub(super) fn resume_helper() -> Option<i32> {
    let args = std::env::args_os().collect::<Vec<_>>();
    if args
        .get(1)
        .is_none_or(|arg| arg != "--shelf-opencode-resume")
    {
        return None;
    }
    if args.len() < 4 {
        return Some(2);
    }
    if let Err(error) = verify_opencode(&args[2], &AtomicU8::new(super::RUNNING)) {
        eprintln!("{error}");
        return Some(1);
    }
    let _error = Command::new(&args[2]).args(&args[3..]).exec();
    eprintln!("Cannot resume OpenCode. Check the CLI in Agent settings.");
    Some(1)
}

impl super::Session {
    pub(super) fn wait_for_private_opencode_mode(
        &mut self,
        created: &mut Value,
    ) -> Result<(), String> {
        let ready = |value: &Value| {
            value["configOptions"].as_array().is_some_and(|options| {
                options.iter().any(|option| {
                    option["id"] == "mode"
                        && option["options"].as_array().is_some_and(|modes| {
                            modes.iter().any(|mode| mode["value"] == "shelf-readonly")
                        })
                })
            })
        };
        // OpenCode first advertises built-ins, then publishes config_option_update as its private
        // config plugins finish. Never send a prompt while only the built-in modes are available.
        let deadline = self.deadline;
        self.deadline = deadline.min(Instant::now() + Duration::from_secs(5));
        let result = (|| {
            while !ready(created) {
                let message = self.next_message().map_err(|error| {
                    if Instant::now() >= self.deadline {
                        "OpenCode did not load Shelf's private read-only mode in time. No review was sent. Check the CLI version and retry from Agent settings.".into()
                    } else { error }
                })?;
                if message["method"] == "session/update"
                    && message["params"]["sessionId"] == created["sessionId"]
                    && message["params"]["update"]["sessionUpdate"] == "config_option_update"
                {
                    created["configOptions"] = message["params"]["update"]["configOptions"].clone();
                }
                self.incoming(&message)?;
            }
            Ok(())
        })();
        self.deadline = deadline;
        result
    }
}
