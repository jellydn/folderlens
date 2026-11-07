//! One private, app-owned reader shared by ACP callbacks and CLI tool bridges.

use std::ffi::{CStr, OsString};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

use super::Access;

#[derive(Clone)]
pub(super) struct Reader {
    initial: Access,
    current: Arc<Mutex<Access>>,
    context: PathBuf,
}

impl Reader {
    pub fn new(initial: Access, current: Arc<Mutex<Access>>, context: PathBuf) -> Self {
        // Freeze the original roots, so replacing a selected symlink cannot widen the grant.
        let freeze = |paths: Vec<PathBuf>| {
            paths
                .into_iter()
                .map(|p| p.canonicalize().unwrap_or(p))
                .collect()
        };
        Self {
            initial: Access {
                selected: freeze(initial.selected),
                blocked: freeze(initial.blocked),
            },
            current,
            context: context.canonicalize().unwrap_or(context),
        }
    }

    pub fn allows(&self, path: &Path) -> bool {
        if self.context == path {
            return true;
        }
        let current = self.current.lock().unwrap();
        let under = |roots: &[PathBuf]| roots.iter().any(|root| path.starts_with(root));
        under(&self.initial.selected) && !under(&self.initial.blocked) && current.allows(path)
    }

    pub fn read(&self, params: &Value) -> Result<String, String> {
        let path = params["path"].as_str().ok_or("Missing path")?;
        let path = Path::new(path)
            .canonicalize()
            .map_err(|_| "Path is unavailable")?;
        if !self.allows(&path) {
            return Err("Shelf permits only currently selected, shared, unprotected paths.".into());
        }
        // Check the opened object, not only a pathname that can change between check and open.
        let file = File::options()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)
            .map_err(|_| "Cannot open selected path")?;
        if !self.allows(&opened_path(&file)?) {
            return Err("Selected path changed before reading.".into());
        }
        let kind = file
            .metadata()
            .map_err(|_| "Cannot inspect selected path")?
            .file_type();
        if kind.is_dir() {
            let mut names = Vec::new();
            for name in directory_names(&file)? {
                if path
                    .join(&name)
                    .canonicalize()
                    .is_ok_and(|p| self.allows(&p))
                {
                    names.push(name.to_string_lossy().into_owned());
                }
            }
            names.sort();
            if !self.allows(&opened_path(&file)?) {
                return Err("Shelf read access was revoked.".into());
            }
            return Ok(format!(
                "Directory entries (up to 1024):\n{}",
                names.join("\n")
            ));
        }
        if !kind.is_file() {
            return Err("Only regular files and directories can be inspected.".into());
        }
        let mut bytes = Vec::new();
        (&file)
            .take(524_289)
            .read_to_end(&mut bytes)
            .map_err(|_| "Cannot read file")?;
        if bytes.len() > 524_288 {
            return Err("File exceeds Shelf's 512 KiB read limit.".into());
        }
        if !self.allows(&opened_path(&file)?) {
            return Err("Shelf read access was revoked.".into());
        }
        let text = String::from_utf8(bytes).map_err(|_| "Only UTF-8 text can be inspected")?;
        let first = params["line"].as_u64().unwrap_or(1).saturating_sub(1) as usize;
        let limit = params["limit"].as_u64().unwrap_or(2000).min(2000) as usize;
        Ok(text.split_inclusive('\n').skip(first).take(limit).collect())
    }
}

fn opened_path(file: &File) -> Result<PathBuf, String> {
    let mut path = [0u8; libc::PATH_MAX as usize];
    // F_GETPATH resolves this live descriptor. Reject removed/unresolvable objects.
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETPATH, path.as_mut_ptr()) } == -1 {
        return Err("Selected path is no longer available.".into());
    }
    let path = unsafe { CStr::from_ptr(path.as_ptr().cast()) };
    Ok(OsString::from_vec(path.to_bytes().to_vec()).into())
}

fn directory_names(file: &File) -> Result<Vec<OsString>, String> {
    let duplicate = unsafe { libc::dup(file.as_raw_fd()) };
    if duplicate == -1 {
        return Err("Cannot open selected directory.".into());
    }
    let dir = unsafe { libc::fdopendir(duplicate) };
    if dir.is_null() {
        unsafe {
            libc::close(duplicate);
        }
        return Err("Cannot list selected directory.".into());
    }
    let mut names = Vec::new();
    let mut error = false;
    for _ in 0..1026 {
        unsafe {
            *libc::__error() = 0;
        }
        let entry = unsafe { libc::readdir(dir) };
        if entry.is_null() {
            error = unsafe { *libc::__error() != 0 };
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if name != b"." && name != b".." {
            names.push(OsString::from_vec(name.to_vec()));
            if names.len() == 1024 {
                break;
            }
        }
    }
    // fdopendir owns the duplicate, never the File used for scope revalidation.
    unsafe {
        libc::closedir(dir);
    }
    if error {
        Err("Cannot list selected directory.".into())
    } else {
        Ok(names)
    }
}

pub(super) struct Broker {
    pub socket: PathBuf,
    stopped: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Broker {
    pub fn start(reader: Reader) -> Result<Self, String> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        // macOS limits Unix socket paths to 104 bytes. Run-folder paths can exceed that.
        let dir = PathBuf::from(format!("/tmp/shelf-read-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&dir).map_err(|e| e.to_string())?;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
        let socket = dir.join("read.sock");
        let listener = UnixListener::bind(&socket).map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = stopped.clone();
        let worker = std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        // BSD accepts inherit O_NONBLOCK. Wait for a complete bounded
                        // request instead of racing the client's first write.
                        if stream.set_nonblocking(false).is_err() {
                            continue;
                        }
                        let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
                        let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
                        let response =
                            packet(&mut BufReader::new(&mut stream)).and_then(|p| reader.read(&p));
                        // Revocation/closing never grants access through another provider's tools.
                        let response = if stop.load(Ordering::Relaxed) {
                            Err("Shelf reader is closed.".into())
                        } else {
                            response
                        };
                        let _ = writeln!(
                            stream,
                            "{}",
                            match response {
                                Ok(text) => json!({"text": text}),
                                Err(error) => json!({"error": error}),
                            }
                        );
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(20))
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            socket,
            stopped,
            worker: Some(worker),
        })
    }

    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Relaxed);
    }
}

impl Drop for Broker {
    fn drop(&mut self) {
        self.stop();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let _ = std::fs::remove_file(&self.socket);
        if let Some(dir) = self.socket.parent() {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

fn packet(input: &mut impl BufRead) -> Result<Value, String> {
    let mut line = String::new();
    input
        .take(16_385)
        .read_line(&mut line)
        .map_err(|_| "Cannot read tool request")?;
    if line.len() > 16_384 || !line.ends_with('\n') {
        return Err("Tool request exceeds its limit or is incomplete.".into());
    }
    serde_json::from_str(&line).map_err(|_| "Invalid tool request".into())
}

fn forward(socket: &Path, params: &Value) -> Value {
    let result = (|| -> Result<Value, String> {
        let mut stream = UnixStream::connect(socket).map_err(|_| {
            "Folder tools require Shelf to remain open. Saved chat context is still available."
        })?;
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|_| "Cannot set reader timeout")?;
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|_| "Cannot set reader timeout")?;
        writeln!(stream, "{params}").map_err(|_| "Cannot send read request")?;
        let mut text = String::new();
        BufReader::new(stream.take(4_194_305))
            .read_line(&mut text)
            .map_err(|_| "Shelf reader did not answer")?;
        if text.len() > 4_194_304 {
            return Err("Shelf reader response exceeds its limit".into());
        }
        serde_json::from_str(&text).map_err(|_| "Invalid Shelf reader response".into())
    })();
    result.unwrap_or_else(|error| json!({"error": error}))
}

/// Headless helpers: no Rune, disk scan, CLI model request, or persisted access grant.
pub(super) fn helper() -> Option<i32> {
    let args: Vec<_> = std::env::args_os().collect();
    let mode = args.get(1)?.to_str()?;
    if !matches!(mode, "--shelf-reader" | "--shelf-read") {
        return None;
    }
    let Some(socket) = args.get(2) else {
        return Some(2);
    };
    let socket = Path::new(socket);
    if mode == "--shelf-read" {
        let response = packet(&mut std::io::stdin().lock())
            .map(|p| forward(socket, &p))
            .unwrap_or_else(|error| json!({"error":error}));
        println!("{response}");
        return Some(0);
    }
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    while let Ok(message) = packet(&mut input) {
        if let Some(response) = mcp_response(&message, |params| forward(socket, params)) {
            println!("{response}");
            let _ = std::io::stdout().flush();
        }
    }
    Some(0)
}

fn mcp_response(message: &Value, read: impl FnOnce(&Value) -> Value) -> Option<Value> {
    let id = message.get("id")?;
    let result = match message["method"].as_str().unwrap_or("") {
        "initialize" => {
            json!({"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"shelf-reader","version":env!("CARGO_PKG_VERSION")}})
        }
        "ping" => json!({}),
        "tools/list" => {
            json!({"tools":[{"name":"read_selected","description":"Read bounded UTF-8 text or list an eligible selected directory through Shelf. Requires Shelf running. Never writes or executes commands.","inputSchema":{"type":"object","properties":{"path":{"type":"string"},"line":{"type":"integer","minimum":1},"limit":{"type":"integer","minimum":1,"maximum":2000}},"required":["path"],"additionalProperties":false},"annotations":{"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false}}]})
        }
        "tools/call" if message["params"]["name"] == "read_selected" => {
            let value = read(&message["params"]["arguments"]);
            let error = value["error"].as_str();
            json!({"isError":error.is_some(),"content":[{"type":"text","text":error.or(value["text"].as_str()).unwrap_or("Reader returned no text")} ]})
        }
        _ => {
            return Some(
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Shelf exposes only read_selected."}}),
            )
        }
    };
    Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_require_original_and_current_selection_and_exclude_blocked_paths() {
        let dir = crate::prefs::path_for_state().with_extension("reader-test");
        std::fs::create_dir_all(dir.join("selected/kept")).unwrap();
        std::fs::write(dir.join("selected/text"), "first\nsecond\nthird\n").unwrap();
        std::fs::write(dir.join("selected/kept/secret"), "private").unwrap();
        std::fs::write(dir.join("outside"), "outside").unwrap();
        std::fs::write(dir.join("context.md"), "saved selection").unwrap();
        std::os::unix::fs::symlink(dir.join("outside"), dir.join("selected/escape")).unwrap();
        let initial = Access {
            selected: vec![dir.join("selected")],
            blocked: vec![dir.join("selected/kept")],
        };
        let current = Arc::new(Mutex::new(initial.clone()));
        let reader = Reader::new(initial, current.clone(), dir.join("context.md"));
        let broker = Broker::start(reader).unwrap();
        let request = |path: &str| json!({"path":dir.join(path), "line":2, "limit":1});
        // Accept can precede the client's first write. A listener's nonblocking flag
        // must not turn this harmless delay into an incomplete read on BSD/macOS.
        let mut delayed = UnixStream::connect(&broker.socket).unwrap();
        std::thread::sleep(Duration::from_millis(80));
        writeln!(delayed, "{}", request("selected/text")).unwrap();
        let mut response = String::new();
        BufReader::new(delayed).read_line(&mut response).unwrap();
        let response: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(response["text"], "second\n", "{response}");
        let response = forward(&broker.socket, &request("selected/text"));
        assert_eq!(response["text"], "second\n", "{response}");
        for path in ["outside", "selected/escape", "selected/kept/secret"] {
            assert!(forward(&broker.socket, &request(path))["error"].is_string());
        }
        let listing = forward(&broker.socket, &json!({"path":dir.join("selected")}));
        assert_eq!(listing["text"], "Directory entries (up to 1024):\ntext");
        std::fs::write(dir.join("selected/large"), vec![b'x'; 524_289]).unwrap();
        std::fs::write(dir.join("selected/binary"), [0xff]).unwrap();
        assert!(forward(&broker.socket, &request("selected/large"))["error"]
            .as_str()
            .unwrap()
            .contains("512 KiB"));
        assert!(
            forward(&broker.socket, &request("selected/binary"))["error"]
                .as_str()
                .unwrap()
                .contains("UTF-8")
        );
        // A later grant cannot widen the scope of an already-created chat.
        *current.lock().unwrap() = Access {
            selected: vec![dir.clone()],
            blocked: vec![],
        };
        assert!(forward(&broker.socket, &request("outside"))["error"].is_string());
        assert!(forward(&broker.socket, &request("selected/kept/secret"))["error"].is_string());
        // Turning sharing off revokes live reads but preserves saved chat context.
        *current.lock().unwrap() = Access::default();
        assert!(forward(&broker.socket, &request("selected/text"))["error"].is_string());
        assert_eq!(
            forward(&broker.socket, &json!({"path":dir.join("context.md")}))["text"],
            "saved selection"
        );
        // Replacing the original context with a symlink cannot grant another path.
        std::fs::remove_file(dir.join("context.md")).unwrap();
        std::os::unix::fs::symlink(dir.join("outside"), dir.join("context.md")).unwrap();
        assert!(forward(&broker.socket, &request("context.md"))["error"].is_string());
        let socket = broker.socket.clone();
        broker.stop();
        assert!(forward(&socket, &request("selected/text"))["error"].is_string());
        drop(broker);
        let closed = forward(&socket, &request("selected/text"));
        assert!(closed["error"]
            .as_str()
            .unwrap()
            .contains("Shelf to remain open"));
        assert!(!socket.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn mcp_exposes_only_reader_and_preserves_consecutive_requests() {
        let mut input = BufReader::new(std::io::Cursor::new(b"{\"id\":1,\"method\":\"tools/list\"}\n{\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"write_file\"}}\n"));
        let tools =
            mcp_response(&packet(&mut input).unwrap(), |_| panic!("must not read")).unwrap();
        assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 1);
        assert_eq!(tools["result"]["tools"][0]["name"], "read_selected");
        let denied =
            mcp_response(&packet(&mut input).unwrap(), |_| panic!("must not write")).unwrap();
        assert_eq!(denied["id"], 2);
        assert_eq!(denied["error"]["code"], -32601);
        let error = mcp_response(
            &json!({"id":3,"method":"tools/call","params":{"name":"read_selected","arguments":{}}}),
            |_| json!({"error":"Shelf is closed"}),
        )
        .unwrap();
        assert_eq!(error["result"]["isError"], true);
        assert_eq!(error["result"]["content"][0]["text"], "Shelf is closed");
    }
}
