use super::*;

#[test]
fn local_ai_settings_and_consent_round_trip_but_the_key_never_does() {
    let path = path_for_state();
    let prefs = Prefs {
        agent_id: LOCAL.into(),
        agent_bin: LOCAL.into(),
        models: BTreeMap::from([(LOCAL.into(), "/models/llama 3.2.gguf".into())]),
        local_endpoint: "http://100.101.1.2:9931/v1".into(),
        local_allow_network: true,
        local_key_saved: true,
        local_models: vec!["qwen2.5:3b".into()],
        consent: BTreeSet::from([r#"["local@100.101.1.2:9931","qwen2.5:3b"]"#.into()]),
        private_cli: true,
        adapter_disabled: BTreeMap::from([("pi".into(), "Pi wrote to ~/.pi".into())]),
        ..Prefs::default()
    };
    save_to(&path, &prefs);
    assert_eq!(load_from(&path), prefs);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.to_ascii_lowercase().contains("bearer"));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn consent_covers_one_provider_model_and_host() {
    let mut prefs = Prefs {
        agent_id: LOCAL.into(),
        agent_bin: LOCAL.into(),
        models: BTreeMap::from([(LOCAL.into(), "qwen2.5:3b".into())]),
        ..Prefs::default()
    };
    assert_eq!(prefs.consent_key(), r#"["local","qwen2.5:3b"]"#);
    prefs.consent.insert(prefs.consent_key());
    assert!(prefs.has_consent());
    // Another model, or the same model on another server, asks again.
    prefs.models.insert(LOCAL.into(), "qwen2.5:7b".into());
    assert!(!prefs.has_consent());
    prefs.models.insert(LOCAL.into(), "qwen2.5:3b".into());
    prefs.local_endpoint = "http://studio.local:1234/v1".into();
    assert_eq!(
        prefs.consent_key(),
        r#"["local@studio.local:1234","qwen2.5:3b"]"#
    );
    assert!(!prefs.has_consent());
    assert!(prefs
        .consent_question()
        .contains("Plain http is not encrypted"));
    prefs.local_endpoint = "https://studio.local:1234/v1".into();
    assert!(!prefs.consent_question().contains("Plain http"));
    // A CLI key uses its agent and model, so Pi's consent never covers Local AI.
    prefs.agent_id = "pi".into();
    prefs.agent_bin = "/opt/bin/pi".into();
    prefs.models.insert("pi".into(), "qwen2.5:3b".into());
    assert_eq!(prefs.consent_key(), r#"["pi","qwen2.5:3b"]"#);
}

#[test]
fn a_port_and_a_model_tag_never_make_the_same_consent() {
    let local = |endpoint: &str, model: &str| Prefs {
        agent_id: LOCAL.into(),
        agent_bin: LOCAL.into(),
        models: BTreeMap::from([(LOCAL.into(), model.into())]),
        local_endpoint: endpoint.into(),
        ..Prefs::default()
    };
    assert_ne!(
        local("http://192.168.1.20:8080/v1", "latest").consent_key(),
        local("http://192.168.1.20/v1", "8080:latest").consent_key()
    );
}

#[test]
fn the_quick_switch_offers_adapted_clis_and_each_local_model_once() {
    let agents = vec![
        Agent {
            id: "pi",
            name: "Pi",
            bin: "/opt/bin/pi".into(),
        },
        Agent {
            id: "claude",
            name: "Claude",
            bin: "/opt/bin/claude".into(),
        },
    ];
    let mut prefs = Prefs {
        models: BTreeMap::from([
            ("pi".into(), "openrouter/free".into()),
            (LOCAL.into(), "typed:1b".into()),
        ]),
        local_models: vec!["qwen2.5:3b".into(), "/m/llama.gguf".into()],
        ..Prefs::default()
    };
    let choices = quick_choices(&prefs, &agents);
    let labels: Vec<_> = choices.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Pi · openrouter/free",
            "Local AI · typed:1b",
            "Local AI · qwen2.5:3b",
            "Local AI · llama.gguf"
        ]
    );
    prefs.choose(&choices[3]);
    assert_eq!(
        (prefs.agent_id.as_str(), prefs.model()),
        (LOCAL, "/m/llama.gguf")
    );
    assert!(prefs.is_chosen(&choices[3]) && !prefs.is_chosen(&choices[2]));
    // The saved model is now listed, so it is not added twice.
    assert_eq!(quick_choices(&prefs, &agents).len(), 3);
    prefs.choose(&choices[0]);
    assert_eq!(
        (prefs.agent_bin.as_str(), prefs.model()),
        ("/opt/bin/pi", "openrouter/free")
    );
}

#[test]
fn legacy_settings_are_imported_once_without_changing_the_original() {
    let root = path_for_state().with_extension("migration");
    let path = root.join("FolderLens/settings.txt");
    let legacy = root.join("Shelf/settings.txt");
    let prefs = Prefs {
        agent_id: "pi".into(),
        models: BTreeMap::from([("pi".into(), "provider/exact-model".into())]),
        keep_hours: 72,
        kept: vec!["/work/important".into()],
        update_checks: false,
        ..Prefs::default()
    };
    save_to(&legacy, &prefs);
    let original = std::fs::read(&legacy).unwrap();
    assert_eq!(load_from(&path), prefs);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(std::fs::read(&legacy).unwrap(), original);

    save_to(&legacy, &Prefs::default());
    assert_eq!(load_from(&path), prefs);
    save_to(&path, &Prefs::default());
    save_to(&legacy, &prefs);
    assert_eq!(load_from(&path), Prefs::default());
    // An empty new file is authoritative too; do not revive old grants.
    std::fs::write(&path, "").unwrap();
    assert_eq!(load_from(&path), Prefs::default());
    assert_eq!(
        load_from(&root.join("Other/settings.txt")),
        Prefs::default()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_round_trip_keeps_the_agent_and_the_grant() {
    let path = std::env::temp_dir().join(format!("folderlens-prefs-round-{}", std::process::id()));
    let prefs = Prefs {
        agent_id: "claude".into(),
        agent_bin: "/usr/local/bin/claude".into(),
        models: BTreeMap::from([
            ("copilot".into(), "auto".into()),
            ("pi".into(), "openrouter/openrouter/free".into()),
        ]),
        allow_start: true,
        send_paths: true,
        notch_position: NotchPosition::Right,
        notch_display: Some("AABBCCDD-1234-5678-ABCD-123456789ABC".into()),
        theme: crate::theme::Theme::Lotus,
        keep_hours: 36,
        kept: vec!["/tmp/keep".into()],
        ..Prefs::default()
    };
    save_to(&path, &prefs);
    assert_eq!(load_from(&path), prefs);
    let mut switched = load_from(&path);
    switched.agent_id = "pi".into();
    assert_eq!(switched.model(), "openrouter/openrouter/free");
    switched.agent_id = "copilot".into();
    assert_eq!(switched.model(), "auto");
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
        Err("Grant access in Settings before FolderLens starts an agent.")
    );
    prefs.allow_start = true;
    let prompt = prepare_ask(&prefs, "smolvm", "/tmp/smolvm", "In use").unwrap();
    assert!(!prompt.contains("/tmp/smolvm"));
    prefs.send_paths = true;
    let prompt = prepare_ask(&prefs, "smolvm", "/tmp/smolvm", "In use").unwrap();
    assert!(prompt.contains("/tmp/smolvm"));
}

#[test]
#[cfg(unix)]
fn ask_runs_keep_private_snapshots_and_start_the_configured_agent() {
    use std::os::unix::fs::PermissionsExt;

    let root = path_for_state().with_extension("run test's $folder");
    std::fs::create_dir(&root).unwrap();
    let bin = root.join("grok");
    std::fs::write(
        &bin,
        "#!/bin/sh\nprintf '%s\\n' \"$PWD\" \"$#\" \"$GROK_FOLDER_TRUST\" \"$1\" \"$2\"\ncat context.md\n",
    )
    .unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o700)).unwrap();
    let prefs = Prefs {
        agent_bin: bin.to_string_lossy().into_owned(),
        allow_start: true,
        ..Prefs::default()
    };
    let context = prepare_ask(
        &prefs,
        "Selected",
        "/private/cache",
        "Selected items: 1 (42 KB)\n- cache's $(echo unsafe): 42 KB",
    )
    .unwrap();
    let runs = root.join("Runs");
    let first = prepare_run(&runs, &context).unwrap();
    let second = prepare_run(&runs, "A different selection").unwrap();
    assert_ne!(first, second);
    let dir = first.parent().unwrap();
    assert_eq!(std::fs::read_dir(dir).unwrap().count(), 2);
    assert_eq!(
        std::fs::read_to_string(dir.join("context.md")).unwrap(),
        context
    );
    assert_eq!(
        std::fs::metadata(dir).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert!(!Command::new(&first).output().unwrap().status.success());
    let mut command = Command::new(&prefs.agent_bin);
    command.arg("--resume").env("GROK_FOLDER_TRUST", "0");
    resume_run(&first, &command, "session's $23").unwrap();
    let output = Command::new(&first).output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parts: Vec<_> = stdout.splitn(6, '\n').collect();
    assert_eq!(Path::new(parts[0]), dir);
    assert_eq!(parts[1], "2");
    assert_eq!(parts[2], "0");
    assert_eq!(parts[3], "--resume");
    assert_eq!(parts[4], "session's $23");
    assert_eq!(parts[5], context);
    assert!(!stdout.contains("/private/cache"));
    assert_eq!(
        std::fs::read_to_string(second.parent().unwrap().join("context.md")).unwrap(),
        "A different selection"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn an_old_file_leaves_the_hour_hold_off() {
    let path = std::env::temp_dir().join(format!("folderlens-prefs-old-{}", std::process::id()));
    std::fs::write(&path, "agent_id=grok\nkeep_hours=nope\n").unwrap();
    let prefs = load_from(&path);
    assert_eq!(prefs.agent_id, "grok");
    assert_eq!(prefs.keep_hours, 0);
    assert!(prefs.gauge_cpu);
    assert_eq!(prefs.notch_display, None);
    assert_eq!(prefs.theme, crate::theme::Theme::Default);
    std::fs::write(&path, "theme=unknown\nnotch_position=right\n").unwrap();
    let loaded = load_from(&path);
    assert_eq!(loaded.theme, crate::theme::Theme::Default);
    assert_eq!(loaded.notch_position, NotchPosition::Right);
    assert_eq!(prefs.notch_position, NotchPosition::Below);
    for (text, expected) in [
        ("notch_below=0\n", NotchPosition::Center),
        ("notch_below=1\n", NotchPosition::Below),
        (
            "notch_below=1\nnotch_position=right\n",
            NotchPosition::Right,
        ),
        ("notch_position=left\nnotch_below=0\n", NotchPosition::Left),
        ("notch_position=unknown\n", NotchPosition::Below),
        (
            "notch_below=0\nnotch_position=unknown\n",
            NotchPosition::Center,
        ),
    ] {
        std::fs::write(&path, text).unwrap();
        let loaded = load_from(&path);
        assert_eq!(loaded.notch_position, expected);
        assert!(!loaded.allow_start && !loaded.send_paths);
        save_to(&path, &loaded);
        assert_eq!(load_from(&path), loaded);
    }
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
    let dir = std::env::temp_dir().join(format!("folderlens-agents-{}", std::process::id()));
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
