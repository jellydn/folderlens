//! Provider-specific CLI arguments and private run configuration, shared by analysis and resume.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
    pub fn prepare(bin: &str, model: &str, dir: &Path, socket: &Path) -> Result<Self, String> {
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
                let mut version = Command::new(bin)
                    .arg("--version")
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|_| "Cannot check OpenCode version")?;
                let deadline = Instant::now() + Duration::from_secs(3);
                let status = loop {
                    if let Ok(Some(status)) = version.try_wait() {
                        break status;
                    }
                    if Instant::now() >= deadline {
                        let _ = version.kill();
                        let _ = version.wait();
                        return Err("OpenCode version check timed out. Check the CLI in Agent settings; no review was sent.".into());
                    }
                    std::thread::sleep(Duration::from_millis(20));
                };
                let mut output = String::new();
                version
                    .stdout
                    .take()
                    .unwrap()
                    .take(256)
                    .read_to_string(&mut output)
                    .map_err(|_| "Cannot read OpenCode version")?;
                let version = output
                    .trim()
                    .strip_prefix("opencode v")
                    .unwrap_or(output.trim());
                if !status.success() || version != "2.0.23" {
                    return Err("Shelf's private OpenCode adapter requires 2.0.23. This version's policy and configuration schema is not verified. No review was sent; choose another agent in Settings.".into());
                }
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
        Ok(Self {
            provider,
            command: make(analysis),
            resume: make(resume),
            model: model.into(),
        })
    }
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
