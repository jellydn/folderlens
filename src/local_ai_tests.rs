use super::*;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

#[test]
fn without_the_opt_in_only_this_mac_is_accepted() {
    assert_eq!(
        validate_endpoint("http://127.0.0.1:11434/v1/", false),
        Ok("http://127.0.0.1:11434/v1".into())
    );
    for good in [
        "http://localhost:1234/v1",
        "http://LOCALHOST/v1",
        "http://[::1]:8080/v1",
        "https://127.0.0.1:8443/v1",
    ] {
        assert!(
            validate_endpoint(good, false).is_ok(),
            "{good} must be accepted"
        );
    }
    for bad in [
        "http://192.168.1.5:11434/v1",
        "http://100.101.1.2:11434/v1",
        "http://box.tailnet.ts.net:11434/v1",
        "http://127.0.0.1@example.com/v1",
        "http://localhost.example.com/v1",
        "127.0.0.1:11434/v1",
        "ftp://127.0.0.1/v1",
        "http://127.0.0.1:99999/v1",
    ] {
        assert!(
            validate_endpoint(bad, false).is_err(),
            "{bad} must be refused"
        );
    }
}

#[test]
fn with_the_opt_in_private_addresses_and_names_are_accepted_but_public_ones_never() {
    for good in [
        "http://10.0.0.2:11434/v1",
        "http://172.31.255.255/v1",
        "http://192.168.1.5:11434/v1",
        "http://100.64.0.1/v1",
        "http://100.127.255.254/v1",
        "http://169.254.10.10/v1",
        "http://[fd7a:115c:a1e0::1]:11434/v1",
        "http://[fe80::1]/v1",
        "https://box.tailnet.ts.net/v1",
        "http://studio.local:1234/v1",
    ] {
        assert!(
            validate_endpoint(good, true).is_ok(),
            "{good} must be accepted"
        );
    }
    // Each just outside a private range, and the user-name trick that still points elsewhere.
    for bad in [
        "http://8.8.8.8/v1",
        "http://172.32.0.1/v1",
        "http://100.63.255.255/v1",
        "http://100.128.0.1/v1",
        "http://[2001:4860::8888]/v1",
        "http://[::ffff:8.8.8.8]/v1",
        "http://10.0.0.2@example.com/v1",
        "http://-bad.local/v1",
    ] {
        assert!(
            validate_endpoint(bad, true).is_err(),
            "{bad} must be refused"
        );
    }
}

#[test]
fn the_resolver_policy_follows_the_endpoint_host() {
    let ip = |s: &str| s.parse::<IpAddr>().expect("ip");
    // A loopback endpoint (`localhost`) may only reach loopback, even with the opt-in on.
    assert!(allowed(ip("127.0.0.1"), true));
    assert!(allowed(ip("::ffff:127.0.0.1"), true));
    assert!(!allowed(ip("192.168.1.5"), true));
    // A network endpoint (a name) may reach private addresses, never public ones.
    assert!(allowed(ip("100.101.1.2"), false));
    assert!(allowed(ip("fd7a:115c:a1e0::1"), false));
    assert!(!allowed(ip("93.184.216.34"), false));
    assert!(!allowed(ip("::ffff:93.184.216.34"), false));
}

#[test]
fn network_host_names_servers_outside_this_mac_only() {
    assert_eq!(network_host(OLLAMA), None);
    assert_eq!(network_host("http://localhost:1234/v1"), None);
    assert_eq!(network_host("http://[::1]:8080/v1"), None);
    assert_eq!(
        network_host("http://100.101.1.2:11434/v1"),
        Some("100.101.1.2:11434")
    );
    assert_eq!(
        network_host("https://box.tailnet.ts.net/v1"),
        Some("box.tailnet.ts.net")
    );
}

/// A one-shot loopback server: answers one request with `status` and `body`, and returns the
/// request it got.
fn stub(status: &str, body: &str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (tx, rx) = mpsc::channel();
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    );
    thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream);
        let mut head = String::new();
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("read");
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().unwrap_or(0);
            }
            head.push_str(&line);
            if line == "\r\n" {
                break;
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).expect("body");
        reader
            .get_mut()
            .write_all(response.as_bytes())
            .expect("write");
        tx.send(head + &String::from_utf8_lossy(&body)).ok();
    });
    (format!("http://127.0.0.1:{port}/v1"), rx)
}

#[test]
fn a_review_posts_the_exact_model_and_the_delimited_selection_without_a_key() {
    let (endpoint, request) = stub(
        "200 OK",
        r#"{"choices":[{"message":{"content":"  Caches are safe to review.  "}}]}"#,
    );
    let answer = review(&endpoint, false, "qwen2.5:3b", None, "- Caches | 2 GB");
    assert_eq!(answer, Ok("Caches are safe to review.".into()));
    let request = request.recv().expect("request");
    assert!(
        request.starts_with("POST /v1/chat/completions"),
        "{request}"
    );
    assert!(!request.to_ascii_lowercase().contains("authorization"));
    let body: Value =
        serde_json::from_str(request.split("\r\n\r\n").nth(1).expect("body")).expect("json");
    assert_eq!(body["model"], "qwen2.5:3b");
    assert_eq!(body["stream"], false);
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(
        body["messages"][1]["content"],
        "<selection>\n- Caches | 2 GB\n</selection>"
    );
}

#[test]
fn the_key_is_a_bearer_token_and_401_and_503_say_what_to_do() {
    let key = ApiKey("s3cret".into());
    let (endpoint, request) = stub("200 OK", r#"{"choices":[{"message":{"content":"Done."}}]}"#);
    assert_eq!(
        review(&endpoint, false, "m", Some(&key), "x"),
        Ok("Done.".into())
    );
    assert!(request
        .recv()
        .expect("request")
        .contains("authorization: Bearer s3cret\r\n"));
    assert_eq!(
        format!("{key:?}"),
        "ApiKey(***)",
        "Debug never shows the key"
    );

    let (endpoint, _) = stub(
        "401 Unauthorized",
        r#"{"error":{"message":"Invalid API Key","code":401}}"#,
    );
    let error = list_models(&endpoint, false, None).unwrap_err();
    assert!(error.contains("needs an API key"), "{error}");
    // llama-server answers 503 with a plain-text or JSON body while it loads the model.
    let (endpoint, _) = stub(
        "503 Service Unavailable",
        r#"{"error":{"message":"Loading model","code":503}}"#,
    );
    let error = review(&endpoint, false, "m", None, "x").unwrap_err();
    assert!(error.contains("still loading"), "{error}");
    assert!(error.contains("Loading model"), "{error}");
    let (endpoint, _) = stub("503 Service Unavailable", "not json");
    let error = review(&endpoint, false, "m", None, "x").unwrap_err();
    assert!(error.contains("still loading"), "{error}");
}

#[test]
fn empty_and_malformed_answers_are_failures() {
    let (endpoint, _) = stub("200 OK", r#"{"choices":[{"message":{"content":"   "}}]}"#);
    assert!(review(&endpoint, false, "m", None, "x").is_err());
    let (endpoint, _) = stub("200 OK", r#"{"ok":true}"#);
    let error = review(&endpoint, false, "m", None, "x").unwrap_err();
    assert!(error.contains("OpenAI-compatible"), "{error}");
}

#[test]
fn models_are_listed_by_exact_id() {
    let (endpoint, request) = stub(
        "200 OK",
        r#"{"object":"list","data":[{"id":"qwen2.5:3b"},{"id":"/models/llama-3.2-3b.gguf"}]}"#,
    );
    assert_eq!(
        list_models(&endpoint, false, None),
        Ok(vec![
            "qwen2.5:3b".into(),
            "/models/llama-3.2-3b.gguf".into()
        ])
    );
    assert!(request
        .recv()
        .expect("request")
        .starts_with("GET /v1/models"));
    assert_eq!(
        short_model_name("/models/llama-3.2-3b.gguf"),
        "llama-3.2-3b.gguf"
    );
    assert_eq!(
        short_model_name("hf.co/user/model:q4"),
        "hf.co/user/model:q4"
    );
}

#[test]
fn a_closed_port_says_the_server_is_not_running_and_a_lan_address_needs_the_opt_in() {
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("bind")
        .local_addr()
        .expect("addr")
        .port();
    let error = review(
        &format!("http://127.0.0.1:{port}/v1"),
        false,
        "m",
        None,
        "x",
    )
    .unwrap_err();
    assert!(error.contains("Cannot reach"), "{error}");
    let error = review("http://192.168.1.5:11434/v1", false, "m", None, "x").unwrap_err();
    assert!(error.contains("Allow LAN and Tailscale"), "{error}");
}

#[test]
fn the_key_never_travels_in_plain_http_to_a_lan_host() {
    let key = ApiKey::new("s3cret".into());
    // Refused before any connection, so the stub-free address is never contacted.
    let error = review("http://192.168.1.20:8080/v1", true, "m", Some(&key), "x").unwrap_err();
    assert!(error.contains("only over https://"), "{error}");
    let error = list_models("http://studio.local:1234/v1", true, Some(&key)).unwrap_err();
    assert!(error.contains("only over https://"), "{error}");
    // 100.64/10 is also carrier-grade NAT, so a Tailscale-looking address needs https too.
    assert!(!key_may_travel("http://100.101.1.2:9931/v1"));
    assert!(!key_may_travel("http://[fd7a:115c:a1e0::1]:9931/v1"));
    assert!(key_may_travel("https://100.101.1.2:9931/v1"));
    assert!(key_may_travel("https://studio.local:1234/v1"));
    assert!(key_may_travel("http://127.0.0.1:8080/v1"));
}
