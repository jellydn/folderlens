//! Private CLI runs (ADR 5): Pi or OpenCode answers once, without tools, sessions or a run
//! folder. Ported from Retone's `provider/cli.rs`, `pi.rs` and `opencode.rs`.
//!
//! Each run: a private `0700` temp folder as the working directory (removed afterwards), the
//! prompt on stdin (argv shows in `ps`), only verified CLI versions, a deadline, cancel that kills
//! the whole process group, and a guard that compares the CLI's data folders before and after.
//! When something changed, the caller turns the adapter off. FolderLens never deletes the CLI's
//! own files.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Read;
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::thread::{self, sleep, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::provider::Provider;
use super::CANCELLED;

/// Versions whose runs left the data folders unchanged. Add one only after that check passes.
const PI_VERIFIED: &[&str] = &["1.5.2"];
const OPENCODE_VERIFIED: &[&str] = &["2.3.0"];
const VERSION_DEADLINE: Duration = Duration::from_secs(5);
const DEADLINE: Duration = Duration::from_secs(300);
/// Same report limit as the chat adapters.
const ANSWER_LIMIT: usize = 256 * 1024;
const STDERR_LIMIT: usize = 16 * 1024;

/// OpenCode's only agent has every tool and permission denied; sharing, snapshots and updates
/// are off.
const OPENCODE_CONFIG: &str = r#"{
  "$schema": "https://opencode.ai/config.json",
  "share": "disabled",
  "autoupdate": false,
  "snapshot": false,
  "default_agent": "folderlens",
  "agent": {
    "folderlens": {
      "mode": "primary",
      "description": "Review a FolderLens selection; no tools",
      "tools": { "*": false },
      "permission": { "*": "deny" }
    }
  }
}"#;

pub struct Outcome {
    pub answer: Result<String, String>,
    /// The data folder in which a file appeared, changed or went away during the run.
    pub wrote: Option<PathBuf>,
}

/// Whether `bin` is a CLI with a private mode.
pub fn supports(bin: &str) -> bool {
    matches!(
        Provider::for_bin(bin),
        Ok(Provider::Pi | Provider::OpenCode)
    )
}

/// One private review: `prompt` goes on stdin, the answer is the CLI's stdout.
pub fn review(bin: &str, model: &str, prompt: &str, cancel: &AtomicU8, home: &Path) -> Outcome {
    let fail = |error: &str| Outcome {
        answer: Err(error.into()),
        wrote: None,
    };
    let provider = match Provider::for_bin(bin) {
        Ok(provider @ (Provider::Pi | Provider::OpenCode)) => provider,
        _ => return fail("Private runs work with Pi and OpenCode only."),
    };
    if let Err(error) = provider.validate_model(model) {
        return fail(&error);
    }
    let Ok(workdir) = WorkDir::new() else {
        return fail("FolderLens could not create its private temp folder.");
    };
    let run = match provider {
        Provider::Pi => pi(bin, model, home),
        _ => match opencode(bin, model, &workdir, home) {
            Ok(run) => run,
            Err(error) => return fail(error),
        },
    };
    execute(run, &workdir, prompt, cancel)
}

fn pi(bin: &str, model: &str, home: &Path) -> Run {
    let mut args: Vec<OsString> = [
        "-p",
        "--no-session",
        "--no-tools",
        "--no-extensions",
        "--no-skills",
        "--no-prompt-templates",
        "--no-context-files",
        "--offline",
        "--thinking",
        "off",
    ]
    .map(OsString::from)
    .into();
    // Blank keeps Pi's own default model, as in the chat adapter.
    if !model.is_empty() {
        args.extend(["--model".into(), model.into()]);
    }
    Run {
        name: "Pi",
        program: bin.into(),
        verified: PI_VERIFIED,
        args,
        envs: vec![("PI_OFFLINE", "1".into())],
        data_dirs: vec![home.join(".pi/agent/sessions")],
    }
}

/// `OPENCODE_DB=:memory:` alone still appends to `~/.local/share/opencode/log/opencode.log`
/// (Retone's check on 2.3.0), so the whole data folder moves into the temp folder.
fn opencode(bin: &str, model: &str, workdir: &WorkDir, home: &Path) -> Result<Run, &'static str> {
    let config = workdir.0.join("opencode.json");
    let data = workdir.0.join("data");
    std::fs::write(&config, OPENCODE_CONFIG)
        .and_then(|()| std::fs::create_dir_all(data.join("opencode")))
        .map_err(|_| "FolderLens could not prepare OpenCode's private configuration.")?;
    // Moving the data folder also hides OpenCode's login file; a copy keeps connected providers
    // working and is removed with the temp folder.
    let auth = home.join(".local/share/opencode/auth.json");
    if auth.is_file() {
        let _ = std::fs::copy(&auth, data.join("opencode/auth.json"));
    }
    Ok(Run {
        name: "OpenCode",
        program: bin.into(),
        verified: OPENCODE_VERIFIED,
        args: ["run", "--standalone", "--model", model]
            .map(OsString::from)
            .into(),
        envs: vec![
            ("OPENCODE_DB", ":memory:".into()),
            ("XDG_DATA_HOME", data.into_os_string()),
            ("OPENCODE_CONFIG", config.into_os_string()),
            ("OPENCODE_CONFIG_DIR", workdir.0.clone().into_os_string()),
            ("OPENCODE_CONFIG_PROJECT_DISABLE", "1".into()),
            ("OPENCODE_DISABLE_PROJECT_CONFIG", "1".into()),
            ("OPENCODE_DISABLE_FILEWATCHER", "1".into()),
            ("OPENCODE_DISABLE_CLAUDE_CODE", "1".into()),
            ("OPENCODE_DISABLE_AUTOUPDATE", "1".into()),
        ],
        data_dirs: [
            ".local/share/opencode",
            ".local/state/opencode",
            ".cache/opencode",
            ".config/opencode",
        ]
        .iter()
        .map(|dir| home.join(dir))
        .collect(),
    })
}

/// FolderLens's scratch folder for one run, removed on drop (success, failure or cancel).
struct WorkDir(PathBuf);

impl WorkDir {
    fn new() -> std::io::Result<Self> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        loop {
            let name = format!(
                "folderlens-private-{}-{nanos}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            );
            let path = std::env::temp_dir().join(name);
            // `create` fails if the path exists, so nobody can plant the folder first.
            match std::fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Run {
    /// Shown in messages: "Pi", "OpenCode".
    name: &'static str,
    program: PathBuf,
    verified: &'static [&'static str],
    args: Vec<OsString>,
    envs: Vec<(&'static str, OsString)>,
    data_dirs: Vec<PathBuf>,
}

fn execute(run: Run, workdir: &WorkDir, prompt: &str, cancel: &AtomicU8) -> Outcome {
    // Taken before `--version` too, so a CLI that writes even when asked its version is caught.
    let before = snapshot(&run.data_dirs);
    let answer = version_then_review(&run, workdir, prompt, cancel);
    Outcome {
        answer,
        wrote: changed_dir(&before, &snapshot(&run.data_dirs)),
    }
}

fn version_then_review(
    run: &Run,
    workdir: &WorkDir,
    prompt: &str,
    cancel: &AtomicU8,
) -> Result<String, String> {
    let not_verified = |found: &str| {
        format!(
            "{} {found} is not verified for private runs. FolderLens supports {} {}. Nothing was sent.",
            run.name,
            run.name,
            run.verified.join(", ")
        )
    };
    let version = match spawn_and_wait(
        run,
        workdir,
        &["--version".into()],
        "",
        cancel,
        VERSION_DEADLINE,
        4096,
    ) {
        Ok(output) => parse_version(&output).ok_or_else(|| not_verified("(unknown version)"))?,
        Err(Failure::Cancelled) => return Err("Analysis cancelled.".into()),
        Err(_) => {
            return Err(format!(
                "Cannot run {}. Check the CLI in Agent settings.",
                run.name
            ))
        }
    };
    if !run.verified.contains(&version.as_str()) {
        return Err(not_verified(&version));
    }
    match spawn_and_wait(
        run,
        workdir,
        &run.args,
        prompt,
        cancel,
        DEADLINE,
        ANSWER_LIMIT + 1,
    ) {
        Ok(output) => {
            let answer = strip_ansi(&output).trim().to_string();
            if answer.is_empty() {
                Err(format!("{} returned no text.", run.name))
            } else if answer.len() > ANSWER_LIMIT {
                Err("The model's answer was too long, so FolderLens dropped it.".into())
            } else {
                Ok(answer)
            }
        }
        Err(Failure::Cancelled) => Err("Analysis cancelled.".into()),
        Err(Failure::TimedOut) => Err(format!("No answer within {} s.", DEADLINE.as_secs())),
        Err(Failure::Exited(stderr)) => {
            Err(format!("{} failed: {}", run.name, last_error_line(&stderr)))
        }
        Err(Failure::Spawn) => Err(format!(
            "Cannot run {}. Check the CLI in Agent settings.",
            run.name
        )),
    }
}

enum Failure {
    Spawn,
    Cancelled,
    TimedOut,
    Exited(String),
}

fn spawn_and_wait(
    run: &Run,
    workdir: &WorkDir,
    args: &[OsString],
    stdin: &str,
    cancel: &AtomicU8,
    deadline: Duration,
    stdout_limit: usize,
) -> Result<String, Failure> {
    let mut child = Command::new(&run.program)
        .args(args)
        .current_dir(&workdir.0)
        .env("NO_COLOR", "1")
        .envs(run.envs.iter().map(|(key, value)| (key, value)))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Own process group, so cancel also stops helpers the CLI starts (OpenCode's server).
        .process_group(0)
        .spawn()
        .map_err(|_| Failure::Spawn)?;
    let stdout = reader(child.stdout.take(), stdout_limit);
    let stderr = reader(child.stderr.take(), STDERR_LIMIT);
    if let Some(mut pipe) = child.stdin.take() {
        let input = stdin.to_string();
        // A writer thread, so a CLI that does not read stdin cannot block FolderLens.
        thread::spawn(move || std::io::Write::write_all(&mut pipe, input.as_bytes()));
    }
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if cancel.load(Ordering::SeqCst) == CANCELLED => break None,
            Ok(None) if started.elapsed() > deadline => {
                kill_group(&mut child);
                return Err(Failure::TimedOut);
            }
            Ok(None) => sleep(Duration::from_millis(20)),
            Err(_) => break None,
        }
    };
    kill_group(&mut child);
    let Some(status) = status else {
        return Err(Failure::Cancelled);
    };
    let out = stdout.join().unwrap_or_default();
    let err = stderr.join().unwrap_or_default();
    if status.success() {
        Ok(out)
    } else {
        Err(Failure::Exited(err))
    }
}

/// Reads a pipe to the end, keeping at most `limit` bytes; the rest is drained so the child
/// never blocks on a full pipe.
fn reader(pipe: Option<impl Read + Send + 'static>, limit: usize) -> JoinHandle<String> {
    thread::spawn(move || {
        let Some(mut pipe) = pipe else {
            return String::new();
        };
        let mut kept = Vec::new();
        let mut chunk = [0u8; 8192];
        while let Ok(read) = pipe.read(&mut chunk) {
            if read == 0 {
                break;
            }
            let room = limit.saturating_sub(kept.len());
            kept.extend_from_slice(&chunk[..read.min(room)]);
        }
        String::from_utf8_lossy(&kept).into_owned()
    })
}

/// Stops the child and everything in its process group, and reaps the child.
fn kill_group(child: &mut Child) {
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// The first `x.y.z` in `--version` output, such as "opencode v2.3.0" → "2.3.0".
fn parse_version(output: &str) -> Option<String> {
    output
        .split_whitespace()
        .map(|word| word.trim_start_matches('v'))
        .find_map(|word| {
            let parts: Vec<&str> = word.split('.').collect();
            let numeric = parts.len() == 3
                && parts
                    .iter()
                    .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
            numeric.then(|| word.to_string())
        })
}

/// Removes terminal escape sequences (`ESC [ … letter`) that some CLIs print despite `NO_COLOR`.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        if chars.peek() == Some(&'[') {
            chars.next();
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        }
    }
    out
}

/// The line that names the problem: the last one with "error", else the last non-empty one.
/// The caller redacts it.
fn last_error_line(stderr: &str) -> String {
    let clean = strip_ansi(stderr);
    let lines: Vec<&str> = clean
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let line = lines
        .iter()
        .rev()
        .find(|line| line.to_ascii_lowercase().contains("error"))
        .or(lines.last());
    line.map_or("no details".into(), |line| {
        line.trim_start_matches("Error:").trim().to_string()
    })
}

type Snapshot = BTreeMap<PathBuf, Option<(u64, SystemTime)>>;

/// Size and modification time of each data folder and its top-level entries. A new file inside
/// a subfolder changes that subfolder's time, so one level covers new sessions cheaply.
fn snapshot(dirs: &[PathBuf]) -> Snapshot {
    let stamp = |path: &Path| {
        std::fs::symlink_metadata(path)
            .ok()
            .and_then(|m| Some((m.len(), m.modified().ok()?)))
    };
    let mut entries = Snapshot::new();
    for dir in dirs {
        entries.insert(dir.clone(), stamp(dir));
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            entries.insert(entry.path(), stamp(&entry.path()));
        }
    }
    entries
}

/// The data folder in which something changed, if any.
fn changed_dir(before: &Snapshot, after: &Snapshot) -> Option<PathBuf> {
    let changed = after
        .iter()
        .find(|(path, stamp)| before.get(*path) != Some(stamp))
        .map(|(path, _)| path);
    let removed = before.keys().find(|path| !after.contains_key(*path));
    let path = changed.or(removed)?;
    Some(
        after
            .keys()
            .chain(before.keys())
            .find(|dir| path.starts_with(dir))
            .unwrap_or(path)
            .clone(),
    )
}

#[cfg(test)]
#[path = "private_cli_tests.rs"]
mod tests;
