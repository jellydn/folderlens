use super::*;
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::AtomicU8;
use std::sync::Arc;

/// A fake home and a stub CLI that records what it got into `<home>/record/`, then runs `body`.
/// The stub is named `pi` or `opencode`, because adapters are chosen by file name.
struct Stub {
    _dir: WorkDir,
    home: PathBuf,
    program: PathBuf,
}

impl Stub {
    fn new(name: &str, version: &str, body: &str) -> Stub {
        let dir = WorkDir::new().expect("dir");
        let home = dir.0.join("home");
        std::fs::create_dir_all(home.join("record")).expect("record");
        for data in [
            ".pi/agent/sessions",
            ".local/share/opencode/log",
            ".local/state/opencode",
            ".config/opencode",
        ] {
            std::fs::create_dir_all(home.join(data)).expect("data");
        }
        let bin = dir.0.join("bin");
        std::fs::create_dir_all(&bin).expect("bin");
        let program = bin.join(name);
        let record = home.join("record");
        let script = format!(
            "#!/bin/sh\nexport HOME='{h}'\nif [ \"$1\" = --version ]; then echo 'tool v{version}'; exit 0; fi\n\
             printf '%s\\n' \"$@\" > '{r}/argv'\npwd > '{r}/cwd'\nenv > '{r}/env'\ncat > '{r}/stdin'\n{body}\n",
            h = home.display(),
            r = record.display()
        );
        std::fs::write(&program, script).expect("write");
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        Stub {
            _dir: dir,
            home,
            program,
        }
    }

    fn bin(&self) -> String {
        self.program.to_string_lossy().into_owned()
    }

    fn record(&self, name: &str) -> String {
        std::fs::read_to_string(self.home.join("record").join(name)).unwrap_or_default()
    }
}

const PROMPT: &str = "- Caches | 2 GB | secret-project-name";

fn running() -> AtomicU8 {
    AtomicU8::new(super::super::RUNNING)
}

#[test]
fn pi_runs_without_session_or_tools_and_gets_the_selection_only_on_stdin() {
    let stub = Stub::new("pi", "1.5.2", "echo 'Caches look safe to review.'");
    let out = review(&stub.bin(), "ollama/qwen", PROMPT, &running(), &stub.home);
    assert_eq!(out.answer, Ok("Caches look safe to review.".into()));
    assert_eq!(out.wrote, None);
    let argv = stub.record("argv");
    for flag in [
        "-p",
        "--no-session",
        "--no-tools",
        "--no-extensions",
        "--no-skills",
        "--no-context-files",
        "--offline",
    ] {
        assert!(
            argv.lines().any(|line| line == flag),
            "{flag} missing: {argv}"
        );
    }
    assert!(argv.ends_with("--model\nollama/qwen\n"), "{argv}");
    assert!(
        !argv.contains("secret-project-name"),
        "the selection must not be in argv"
    );
    assert_eq!(stub.record("stdin"), PROMPT);
    let cwd = PathBuf::from(stub.record("cwd").trim());
    assert!(
        cwd.file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("folderlens-private-")),
        "{cwd:?}"
    );
    assert!(!cwd.exists(), "the temp folder is removed after the run");
}

#[test]
fn opencode_keeps_its_data_in_memory_and_in_the_temp_folder() {
    let stub = Stub::new("opencode", "2.3.0", "echo 'Done.'");
    std::fs::write(stub.home.join(".local/share/opencode/auth.json"), "{}").unwrap();
    let out = review(
        &stub.bin(),
        "opencode/beam-free",
        PROMPT,
        &running(),
        &stub.home,
    );
    assert_eq!(out.answer, Ok("Done.".into()));
    assert_eq!(out.wrote, None, "reading auth.json is not a write");
    let env = stub.record("env");
    let cwd = stub.record("cwd");
    let folder = Path::new(cwd.trim())
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let var = |key: &str| {
        env.lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .unwrap_or_default()
    };
    assert_eq!(var("OPENCODE_DB"), ":memory:");
    assert!(var("XDG_DATA_HOME").ends_with(&format!("{folder}/data")));
    assert!(var("OPENCODE_CONFIG").ends_with(&format!("{folder}/opencode.json")));
    assert_eq!(var("OPENCODE_CONFIG_PROJECT_DISABLE"), "1");
    assert_eq!(
        stub.record("argv"),
        "run\n--standalone\n--model\nopencode/beam-free\n"
    );
}

#[test]
fn an_unverified_version_is_refused_before_anything_is_sent() {
    let stub = Stub::new("opencode", "2.0.23", "echo never");
    let out = review(&stub.bin(), "opencode/x", PROMPT, &running(), &stub.home);
    let error = out.answer.unwrap_err();
    assert!(
        error.contains("2.0.23 is not verified for private runs"),
        "{error}"
    );
    assert!(error.contains("2.3.0"), "{error}");
    assert_eq!(stub.record("stdin"), "", "the stub never got the selection");
}

#[test]
fn a_cli_that_writes_into_its_data_folder_is_reported_and_nothing_is_deleted() {
    let stub = Stub::new(
        "pi",
        "1.5.2",
        "echo session > \"$HOME/.pi/agent/sessions/new.jsonl\"; echo ok",
    );
    let out = review(&stub.bin(), "a/m", PROMPT, &running(), &stub.home);
    assert_eq!(out.answer, Ok("ok".into()));
    assert_eq!(out.wrote, Some(stub.home.join(".pi/agent/sessions")));
    assert!(
        stub.home.join(".pi/agent/sessions/new.jsonl").exists(),
        "FolderLens never deletes CLI files"
    );
}

#[test]
fn cancel_kills_the_child_quickly() {
    let stub = Stub::new("pi", "1.5.2", "sleep 30");
    let cancel = Arc::new(running());
    let flag = cancel.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        flag.store(CANCELLED, Ordering::SeqCst);
    });
    let started = Instant::now();
    let out = review(&stub.bin(), "a/m", PROMPT, &cancel, &stub.home);
    assert_eq!(out.answer, Err("Analysis cancelled.".into()));
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn flags_as_models_and_other_clis_are_refused_before_any_run() {
    let stub = Stub::new("pi", "1.5.2", "echo never");
    for model in ["--tools", "a b", "x\ny", "no-prefix"] {
        let out = review(&stub.bin(), model, PROMPT, &running(), &stub.home);
        assert!(out.answer.is_err(), "{model:?}");
    }
    assert_eq!(stub.record("stdin"), "");
    let out = review("/opt/bin/grok", "", PROMPT, &running(), &stub.home);
    assert!(out.answer.is_err());
    assert!(supports("/x/pi") && supports("/x/opencode") && !supports("/x/copilot"));
}

#[test]
fn versions_and_escape_codes_are_parsed() {
    assert_eq!(parse_version("opencode v2.3.0\n").as_deref(), Some("2.3.0"));
    assert_eq!(parse_version("1.5.2").as_deref(), Some("1.5.2"));
    assert_eq!(parse_version("version 2.3"), None);
    assert_eq!(strip_ansi("\u{1b}[1mok\u{1b}[0m"), "ok");
    assert_eq!(last_error_line("info\nError: bad model\n\n"), "bad model");
}
