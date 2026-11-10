//! Built-in hints and the folder scan that fills them.
//!
//! Patterns are the well-known places those groups live. A `*` child list stops
//! at one level. A project search looks under source roots and does not enter
//! dependency or build folders. Leftovers are bundle-id folders whose app is
//! not installed. The scan stores sizes in its own cache and does not change
//! the folders it measures.

use std::collections::{HashMap, HashSet};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::{Hint, Row};

const PROJECT_ROOTS: &[&str] = &["src", "Projects", "dev", "code", "work"];
const PRUNE: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "Pods",
    "dist",
    ".next",
    "vendor",
    "Carthage",
    ".build",
    "coverage",
    ".turbo",
    "DerivedData",
    ".venv",
    "venv",
    "__pycache__",
    ".gradle",
    "build",
    ".cache",
];

enum Kind {
    /// Directories, relative to the home directory.
    Paths(&'static [&'static str]),
    /// Immediate child directories. Apple's own caches are skipped.
    Kids { dir: &'static str, skip_apple: bool },
    /// A directory name under the project roots, optional sibling marker file.
    Named {
        name: &'static str,
        near: Option<&'static str>,
    },
    /// Child directories whose bundle id is not installed.
    Leftover(&'static str),
}

struct Rule {
    id: &'static str,
    group: &'static str,
    label: &'static str,
    tier: &'static str,
    list_only: bool,
    untouched_days: Option<u32>,
    project_days: Option<u32>,
    processes: &'static [&'static str],
    kind: Kind,
}

const RULES: &[Rule] = &[
    Rule {
        id: "user-caches",
        group: "App caches",
        label: "App caches",
        tier: "regenerable",
        list_only: false,
        untouched_days: None,
        project_days: None,
        processes: &[],
        kind: Kind::Kids {
            dir: "Library/Caches",
            skip_apple: true,
        },
    },
    Rule {
        id: "spotify-cache",
        group: "App caches",
        label: "Spotify cache",
        tier: "regenerable",
        list_only: false,
        untouched_days: None,
        project_days: None,
        processes: &["Spotify"],
        kind: Kind::Paths(&["Library/Caches/com.spotify.client"]),
    },
    Rule {
        id: "homebrew-cache",
        group: "Package caches",
        label: "Homebrew downloads",
        tier: "regenerable",
        list_only: false,
        untouched_days: None,
        project_days: None,
        processes: &["brew"],
        kind: Kind::Paths(&["Library/Caches/Homebrew"]),
    },
    Rule {
        id: "npm-cache",
        group: "Package caches",
        label: "npm cache",
        tier: "regenerable",
        list_only: false,
        untouched_days: None,
        project_days: None,
        processes: &["npm"],
        kind: Kind::Paths(&[".npm/_cacache"]),
    },
    Rule {
        id: "pnpm-store",
        group: "Package caches",
        label: "pnpm store",
        tier: "regenerable",
        list_only: false,
        untouched_days: None,
        project_days: None,
        processes: &["pnpm"],
        kind: Kind::Kids {
            dir: ".local/share/pnpm/store",
            skip_apple: false,
        },
    },
    Rule {
        id: "bun-cache",
        group: "Package caches",
        label: "Bun cache",
        tier: "regenerable",
        list_only: false,
        untouched_days: None,
        project_days: None,
        processes: &["bun"],
        kind: Kind::Paths(&[".bun/install/cache"]),
    },
    Rule {
        id: "cargo-registry",
        group: "Package caches",
        label: "Cargo registry",
        tier: "regenerable",
        list_only: false,
        untouched_days: None,
        project_days: None,
        processes: &["cargo", "rustc"],
        kind: Kind::Paths(&[".cargo/registry/cache", ".cargo/registry/src"]),
    },
    Rule {
        id: "cargo-git",
        group: "Package caches",
        label: "Cargo git checkouts",
        tier: "regenerable",
        list_only: false,
        untouched_days: None,
        project_days: None,
        processes: &["cargo", "rustc"],
        kind: Kind::Paths(&[".cargo/git/checkouts", ".cargo/git/db"]),
    },
    Rule {
        id: "rustup-toolchains",
        group: "Toolchains and SDKs",
        label: "Rust toolchains",
        tier: "costly",
        list_only: true,
        untouched_days: None,
        project_days: None,
        processes: &["rustup", "cargo", "rustc"],
        kind: Kind::Kids {
            dir: ".rustup/toolchains",
            skip_apple: false,
        },
    },
    Rule {
        id: "mise-installs",
        group: "Toolchains and SDKs",
        label: "mise installs",
        tier: "costly",
        list_only: true,
        untouched_days: None,
        project_days: None,
        processes: &["mise"],
        kind: Kind::Kids {
            dir: ".local/share/mise/installs",
            skip_apple: false,
        },
    },
    Rule {
        id: "orbstack-data",
        group: "Virtual machines and containers",
        label: "OrbStack data",
        tier: "costly",
        list_only: true,
        untouched_days: None,
        project_days: None,
        processes: &["OrbStack"],
        kind: Kind::Paths(&[".orbstack", "OrbStack"]),
    },
    Rule {
        id: "docker-data",
        group: "Virtual machines and containers",
        label: "Docker data",
        tier: "costly",
        list_only: true,
        untouched_days: None,
        project_days: None,
        processes: &["com.docker.backend", "Docker"],
        kind: Kind::Paths(&["Library/Containers/com.docker.docker/Data/vms"]),
    },
    Rule {
        id: "xcode-derived-data",
        group: "Xcode build data",
        label: "Xcode project builds",
        tier: "regenerable",
        list_only: false,
        untouched_days: Some(14),
        project_days: None,
        processes: &["Xcode", "xcodebuild"],
        kind: Kind::Kids {
            dir: "Library/Developer/Xcode/DerivedData",
            skip_apple: false,
        },
    },
    Rule {
        id: "user-logs",
        group: "Logs and crash reports",
        label: "Logs",
        tier: "regenerable",
        list_only: false,
        untouched_days: Some(7),
        project_days: None,
        processes: &[],
        kind: Kind::Kids {
            dir: "Library/Logs",
            skip_apple: true,
        },
    },
    Rule {
        id: "trash",
        group: "Trash",
        label: "Trash",
        tier: "userData",
        list_only: true,
        untouched_days: None,
        project_days: None,
        processes: &[],
        kind: Kind::Paths(&[".Trash"]),
    },
    Rule {
        id: "node-modules",
        group: "Project dependencies",
        label: "node_modules",
        tier: "regenerable",
        list_only: false,
        untouched_days: None,
        project_days: Some(14),
        processes: &[],
        kind: Kind::Named {
            name: "node_modules",
            near: Some("package.json"),
        },
    },
    Rule {
        id: "cargo-target",
        group: "Build output",
        label: "Cargo build output",
        tier: "regenerable",
        list_only: false,
        untouched_days: None,
        project_days: Some(14),
        processes: &["cargo", "rustc"],
        kind: Kind::Named {
            name: "target",
            near: Some("Cargo.toml"),
        },
    },
    Rule {
        id: "app-support-leftovers",
        group: "App leftovers",
        label: "Leftovers of removed apps",
        tier: "userData",
        list_only: true,
        untouched_days: None,
        project_days: None,
        processes: &[],
        kind: Kind::Leftover("Library/Application Support"),
    },
    Rule {
        id: "container-leftovers",
        group: "App leftovers",
        label: "Containers of removed apps",
        tier: "userData",
        list_only: true,
        untouched_days: None,
        project_days: None,
        processes: &[],
        kind: Kind::Leftover("Library/Containers"),
    },
    Rule {
        id: "group-leftovers",
        group: "App leftovers",
        label: "Group containers of removed apps",
        tier: "userData",
        list_only: true,
        untouched_days: None,
        project_days: None,
        processes: &[],
        kind: Kind::Leftover("Library/Group Containers"),
    },
];

pub(super) fn scan_root(
    home: &Path,
    min_bytes: u64,
    system_apps: bool,
) -> (Vec<Row>, HashMap<String, Hint>) {
    let hints = hint_map();
    let mut hits: HashMap<PathBuf, Vec<String>> = HashMap::new();
    let installed = installed_ids(home, system_apps);
    for rule in RULES {
        for path in expand(home, &rule.kind, &installed) {
            hits.entry(path).or_default().push(rule.id.to_string());
        }
    }
    let paths = drop_descendants(hits.keys().cloned().collect());
    let cache_file = home.join("Library/Application Support/FolderLens/sizes.tsv");
    let mut cache = load_cache(&cache_file);
    let mut pending = Vec::new();
    let mut measured = Vec::new();
    for path in &paths {
        if let Some((bytes, newest)) = cached_size(path, &cache) {
            measured.push((path.clone(), bytes, newest));
        } else {
            pending.push(path.clone());
        }
    }
    for (path, bytes, newest) in measure_all(&pending) {
        if let Some(mtime) = dir_mtime(&path) {
            cache.insert(
                path.to_string_lossy().into_owned(),
                Stamp {
                    bytes,
                    newest,
                    mtime,
                },
            );
        }
        measured.push((path, bytes, newest));
    }
    save_cache(&cache_file, &cache);
    let mut rows = Vec::new();
    for (path, bytes, newest) in measured {
        if bytes < min_bytes {
            continue;
        }
        let Some(hint_ids) = hits.get(&path) else {
            continue;
        };
        rows.push(Row {
            path: path.to_string_lossy().into_owned(),
            bytes,
            newest,
            hint_ids: hint_ids.clone(),
        });
    }
    (rows, hints)
}

fn hint_map() -> HashMap<String, Hint> {
    RULES
        .iter()
        .map(|rule| {
            (
                rule.id.to_string(),
                Hint {
                    id: rule.id.to_string(),
                    group: rule.group.to_string(),
                    label: rule.label.to_string(),
                    tier: rule.tier.to_string(),
                    list_only: rule.list_only,
                    untouched_days: rule.untouched_days,
                    project_days: rule.project_days,
                    processes: rule
                        .processes
                        .iter()
                        .map(|name| (*name).to_string())
                        .collect(),
                },
            )
        })
        .collect()
}

fn expand(home: &Path, kind: &Kind, installed: &HashSet<String>) -> Vec<PathBuf> {
    match kind {
        Kind::Paths(rels) => rels
            .iter()
            .map(|rel| home.join(rel))
            .filter(|path| path.is_dir())
            .collect(),
        Kind::Kids { dir, skip_apple } => child_dirs(&home.join(dir))
            .into_iter()
            .filter(|path| {
                let name = file_name(path);
                !name.starts_with('.') && !(*skip_apple && name.starts_with("com.apple."))
            })
            .collect(),
        Kind::Named { name, near } => {
            let mut found = Vec::new();
            for root in PROJECT_ROOTS {
                let root = home.join(root);
                if root.is_dir() {
                    find_named(&root, name, *near, &mut found, 0);
                }
            }
            found
        }
        Kind::Leftover(dir) => child_dirs(&home.join(dir))
            .into_iter()
            .filter(|path| {
                let name = file_name(path);
                name.contains('.')
                    && !name.starts_with('.')
                    && !name.starts_with("com.apple.")
                    && !is_installed(&name, installed)
            })
            .collect(),
    }
}

fn child_dirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    read.flatten()
        .filter_map(|entry| {
            let kind = entry.file_type().ok()?;
            if kind.is_dir() && !kind.is_symlink() {
                Some(entry.path())
            } else {
                None
            }
        })
        .collect()
}

fn find_named(dir: &Path, name: &str, near: Option<&str>, found: &mut Vec<PathBuf>, depth: u32) {
    if depth > 7 {
        return;
    }
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if !kind.is_dir() || kind.is_symlink() {
            continue;
        }
        let fname = entry.file_name();
        let fname = fname.to_string_lossy();
        if fname == name {
            let path = entry.path();
            let marked = near
                .map(|file| {
                    path.parent()
                        .is_some_and(|parent| parent.join(file).is_file())
                })
                .unwrap_or(true);
            if marked {
                found.push(path);
            }
            continue;
        }
        if PRUNE.iter().any(|skip| fname == *skip) {
            continue;
        }
        find_named(&entry.path(), name, near, found, depth + 1);
    }
}

fn drop_descendants(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.sort_by_key(|path| path.components().count());
    let mut kept = Vec::new();
    for path in paths {
        let nested = kept
            .iter()
            .any(|parent: &PathBuf| path.starts_with(parent) && path != *parent);
        if !nested {
            kept.push(path);
        }
    }
    kept
}

struct Stamp {
    bytes: u64,
    newest: Option<f64>,
    mtime: f64,
}

fn load_cache(path: &Path) -> HashMap<String, Stamp> {
    let mut map = HashMap::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return map;
    };
    // v1 stored logical length. A sparse disk image then looked hundreds of
    // gigabytes larger than the space it held.
    let mut lines = text.lines();
    if lines.next() != Some("v2") {
        return map;
    }
    for line in lines {
        let mut parts = line.splitn(4, '\t');
        let (Some(bytes), Some(newest), Some(mtime), Some(folder)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let Ok(bytes) = bytes.parse::<u64>() else {
            continue;
        };
        let Ok(mtime) = mtime.parse::<f64>() else {
            continue;
        };
        let newest = newest.parse::<f64>().ok();
        map.insert(
            folder.to_string(),
            Stamp {
                bytes,
                newest,
                mtime,
            },
        );
    }
    map
}

fn save_cache(path: &Path, cache: &HashMap<String, Stamp>) {
    let Some(parent) = path.parent() else { return };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let mut text = String::from("v2\n");
    for (folder, stamp) in cache {
        let newest = stamp
            .newest
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string());
        text.push_str(&format!(
            "{}\t{newest}\t{}\t{folder}\n",
            stamp.bytes, stamp.mtime
        ));
    }
    let tmp = path.with_extension("tsv.tmp");
    if std::fs::write(&tmp, text).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

fn dir_mtime(path: &Path) -> Option<f64> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    Some(modified.duration_since(UNIX_EPOCH).ok()?.as_secs_f64())
}

fn cached_size(path: &Path, cache: &HashMap<String, Stamp>) -> Option<(u64, Option<f64>)> {
    let mtime = dir_mtime(path)?;
    let saved = cache.get(&path.to_string_lossy().to_string())?;
    if (saved.mtime - mtime).abs() < 1.0 {
        Some((saved.bytes, saved.newest))
    } else {
        None
    }
}

fn measure_all(paths: &[PathBuf]) -> Vec<(PathBuf, u64, Option<f64>)> {
    if paths.is_empty() {
        return Vec::new();
    }
    let threads = 8.min(paths.len());
    let mut chunks: Vec<Vec<PathBuf>> = (0..threads).map(|_| Vec::new()).collect();
    for (index, path) in paths.iter().enumerate() {
        chunks[index % threads].push(path.clone());
    }
    std::thread::scope(|scope| {
        let mut jobs = Vec::new();
        for chunk in &chunks {
            jobs.push(scope.spawn(|| {
                chunk
                    .iter()
                    .map(|path| {
                        let (bytes, newest) = measure(path);
                        (path.clone(), bytes, newest)
                    })
                    .collect::<Vec<_>>()
            }));
        }
        jobs.into_iter()
            .flat_map(|job| job.join().unwrap_or_default())
            .collect()
    })
}

fn note_allocated(
    meta: &std::fs::Metadata,
    bytes: &mut u64,
    newest: &mut f64,
    seen: &mut HashSet<(u64, u64)>,
) {
    if let Ok(modified) = meta.modified() {
        if let Ok(age) = modified.duration_since(UNIX_EPOCH) {
            *newest = (*newest).max(age.as_secs_f64());
        }
    }
    if meta.file_type().is_symlink() {
        return;
    }
    // blocks() is the space held, in 512-byte units. len() is the logical
    // size, which a sparse disk image inflates. One inode is one file.
    if seen.insert((meta.dev(), meta.ino())) {
        *bytes = bytes.saturating_add(meta.blocks().saturating_mul(512));
    }
}

pub(super) fn measure(path: &Path) -> (u64, Option<f64>) {
    let mut bytes = 0_u64;
    let mut newest = 0_f64;
    let mut seen = HashSet::new();
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(meta) = std::fs::symlink_metadata(&dir) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        note_allocated(&meta, &mut bytes, &mut newest, &mut seen);
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.flatten() {
            let child = entry.path();
            let Ok(meta) = std::fs::symlink_metadata(&child) else {
                continue;
            };
            note_allocated(&meta, &mut bytes, &mut newest, &mut seen);
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                stack.push(child);
            }
        }
    }
    let newest = if newest > 0.0 { Some(newest) } else { None };
    (bytes, newest)
}

fn installed_ids(home: &Path, system_apps: bool) -> HashSet<String> {
    let mut ids = HashSet::new();
    let mut roots = vec![home.join("Applications")];
    if system_apps {
        roots.push(PathBuf::from("/Applications"));
        roots.push(PathBuf::from("/System/Applications"));
    }
    for root in roots {
        collect_apps(&root, &mut ids, 0);
    }
    ids
}

fn collect_apps(dir: &Path, ids: &mut HashSet<String>, depth: u32) {
    if depth > 2 {
        return;
    }
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        let name = file_name(&path);
        if name.ends_with(".app") {
            if let Some(id) = bundle_id(&path.join("Contents/Info.plist")) {
                ids.insert(id);
            }
        } else if path.is_dir() {
            collect_apps(&path, ids, depth + 1);
        }
    }
}

fn bundle_id(plist: &Path) -> Option<String> {
    let bytes = std::fs::read(plist).ok()?;
    if bytes.starts_with(b"bplist") {
        let text = super::run(
            "plutil",
            &[
                "-extract",
                "CFBundleIdentifier",
                "raw",
                "-o",
                "-",
                &plist.to_string_lossy(),
            ],
        )?;
        let id = text.trim();
        if id.is_empty() {
            None
        } else {
            Some(id.to_string())
        }
    } else {
        let text = String::from_utf8_lossy(&bytes);
        let key = "<key>CFBundleIdentifier</key>";
        let at = text.find(key)?;
        let rest = &text[at + key.len()..];
        let start = rest.find("<string>")? + "<string>".len();
        let end = rest[start..].find("</string>")?;
        Some(rest[start..start + end].trim().to_string())
    }
}

fn is_installed(name: &str, ids: &HashSet<String>) -> bool {
    if ids.contains(name) {
        return true;
    }
    ids.iter().any(|id| name.ends_with(&format!(".{id}")))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_file(path: &Path, bytes: usize) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, vec![1_u8; bytes]).unwrap();
    }

    #[test]
    fn a_temp_home_finds_a_cache_a_project_and_a_removed_app() {
        let home = std::env::temp_dir().join(format!("folderlens-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        write_file(&home.join("Library/Caches/Demo/blob"), 64);
        write_file(&home.join("Library/Caches/com.apple.bird/blob"), 64);
        write_file(&home.join("src/app/package.json"), 8);
        write_file(&home.join("src/app/node_modules/pkg/index.js"), 64);
        write_file(
            &home.join("src/app/node_modules/pkg/node_modules/inner/index.js"),
            64,
        );
        write_file(&home.join("src/app/target/debug/app"), 64);
        let plist = home.join("Applications/Demo.app/Contents/Info.plist");
        write_file(&plist, 0);
        std::fs::write(
            &plist,
            r#"<?xml version="1.0"?><plist><dict><key>CFBundleIdentifier</key><string>com.demo.app</string></dict></plist>"#,
        )
        .unwrap();
        write_file(
            &home.join("Library/Application Support/com.demo.app/keep"),
            64,
        );
        write_file(
            &home.join("Library/Application Support/com.gone.app/old"),
            64,
        );

        let (rows, _) = scan_root(&home, 1, false);
        let paths: Vec<&str> = rows.iter().map(|row| row.path.as_str()).collect();
        assert!(paths
            .iter()
            .any(|path| path.ends_with("Library/Caches/Demo")));
        assert!(paths.iter().all(|path| !path.contains("com.apple.bird")));
        assert!(paths
            .iter()
            .any(|path| path.ends_with("src/app/node_modules")));
        assert!(paths
            .iter()
            .all(|path| !path.contains("node_modules/pkg/node_modules")));
        assert!(
            paths.iter().all(|path| !path.ends_with("/target")),
            "no Cargo.toml beside target"
        );
        assert!(paths.iter().any(|path| path.ends_with("com.gone.app")));
        assert!(paths.iter().all(|path| !path.ends_with("com.demo.app")));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn allocated_blocks_skip_sparse_holes_and_count_a_hard_link_once() {
        let dir = std::env::temp_dir().join(format!("folderlens-measure-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let hole = dir.join("hole");
        let file = std::fs::File::create(&hole).unwrap();
        file.set_len(32 * 1024 * 1024).unwrap();
        drop(file);
        let (sparse, _) = measure(&dir);
        assert!(
            sparse < 1_000_000,
            "logical size leaked into the measure: {sparse}"
        );

        let linked = dir.join("linked");
        std::fs::create_dir_all(&linked).unwrap();
        let original = linked.join("a");
        std::fs::write(&original, vec![1_u8; 256 * 1024]).unwrap();
        std::fs::hard_link(&original, linked.join("b")).unwrap();
        let (once, _) = measure(&linked);
        let held = std::fs::symlink_metadata(&original).unwrap().blocks() * 512;
        assert!(once >= held, "dropped the file: {once} < {held}");
        assert!(
            once < held + held / 2,
            "hard link counted twice: {once} vs {held}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_cache_without_the_v2_header_is_ignored() {
        let dir = std::env::temp_dir().join(format!("folderlens-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sizes.tsv");
        std::fs::write(&path, "10\t-\t1.0\t/tmp/old\n").unwrap();
        assert!(load_cache(&path).is_empty());
        std::fs::write(&path, "v2\n10\t-\t1.0\t/tmp/old\n").unwrap();
        assert_eq!(
            load_cache(&path).get("/tmp/old").map(|stamp| stamp.bytes),
            Some(10)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
