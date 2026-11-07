use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn saved_continuation_rechecks_version_and_bounds_inherited_stdout() {
    let root = std::env::temp_dir().join(format!("shelf-continuation-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let bin = root.join("opencode");
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_shelf"))
            .args([
                "--shelf-opencode-resume",
                bin.to_str().unwrap(),
                "--standalone",
                "--session",
                "exact-chat",
            ])
            .current_dir(&root)
            .output()
            .unwrap()
    };
    for (version, accepted) in [("2.0.23", true), ("2.2.0", false)] {
        std::fs::write(&bin, format!("#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'opencode v{version}'; exit; fi\nprintf '%s\\n' \"$*\"\n")).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o700)).unwrap();
        let output = run();
        assert_eq!(output.status.success(), accepted);
        if accepted {
            assert_eq!(
                String::from_utf8(output.stdout).unwrap().trim(),
                "--standalone --session exact-chat"
            );
        } else {
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8(output.stderr)
                .unwrap()
                .contains("requires 2.0.23"));
        }
    }
    // The direct process exits, but its descendant retains stdout. No drain may wait forever.
    std::fs::write(
        &bin,
        "#!/bin/sh\nsleep 30 &\necho 'opencode v2.0.23'\nexit 0\n",
    )
    .unwrap();
    let start = Instant::now();
    let output = run();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("timed out"));
    assert!(start.elapsed() < Duration::from_secs(6));
    std::fs::remove_dir_all(root).unwrap();
}
