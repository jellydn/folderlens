//! Shelf's own catalog. It measures folders on this Mac. It does not read Levo.
//!
//! Hints name a group, a tier, and day fields. Careful waits the full day count,
//! Balanced waits at most a day, and Aggressive waits an hour. A project day
//! field waits for Balanced. A regenerable hint with no day field waits a day,
//! then an hour. A parent folder absorbs a hinted child, so a total is counted
//! once. A path in a running command, or a listed tool that is running, stays locked.

#[path = "scan.rs"]
mod scan;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const MIN_GROUP_BYTES: u64 = 50_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    None,
    Careful,
    Balanced,
    Aggressive,
}

#[derive(Clone, Debug)]
pub struct Leaf {
    pub id: String,
    /// Full folder path. The tray shows the short name. Tests and the in-use check read this.
    #[allow(dead_code)]
    pub path: String,
    pub name: String,
    pub detail: String,
    pub size_label: String,
    pub bytes: u64,
    pub idle_h: u32,
    pub careful: Option<u32>,
    pub balanced: Option<u32>,
    pub aggressive: Option<u32>,
    pub locked: bool,
}

#[derive(Clone, Debug)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub detail: String,
    /// The scan's label. The tray sums the leaves, so Analyse can change the total.
    #[allow(dead_code)]
    pub size_label: String,
    pub first: usize,
    pub count: usize,
}

#[derive(Clone, Debug)]
pub struct Catalog {
    pub groups: Vec<Group>,
    pub leaves: Vec<Leaf>,
    pub disk_total: u64,
    pub disk_free: u64,
    pub memory: u32,
    pub cpu: u32,
    pub gpu: u32,
    pub checked: String,
    pub missing: bool,
}

#[derive(Clone, Debug)]
struct Hint {
    id: String,
    group: String,
    label: String,
    tier: String,
    list_only: bool,
    untouched_days: Option<u32>,
    project_days: Option<u32>,
    processes: Vec<String>,
}

#[derive(Clone, Debug)]
struct Row {
    path: String,
    bytes: u64,
    newest: Option<f64>,
    hint_ids: Vec<String>,
}

pub fn can_tick(leaf: &Leaf) -> bool {
    !leaf.locked && (leaf.careful.is_some() || leaf.balanced.is_some() || leaf.aggressive.is_some())
}

pub fn auto_tick(leaf: &Leaf, level: Level) -> bool {
    if !can_tick(leaf) {
        return false;
    }
    let gates = match level {
        Level::None => [None, None, None],
        Level::Careful => [leaf.careful, None, None],
        Level::Balanced => [leaf.careful, leaf.balanced, None],
        Level::Aggressive => [leaf.careful, leaf.balanced, leaf.aggressive],
    };
    gates
        .into_iter()
        .flatten()
        .any(|hours| leaf.idle_h >= hours)
}

pub fn format_bytes(bytes: u64) -> String {
    let gb = bytes as f64 / 1_000_000_000.0;
    if gb >= 10.0 {
        format!("{gb:.1} GB")
    } else if bytes >= 1_000_000_000 {
        format!("{gb:.2} GB")
    } else if bytes >= 1_000_000 {
        format!("{:.0} MB", bytes as f64 / 1_000_000.0)
    } else {
        format!("{bytes} B")
    }
}

#[allow(dead_code)]
pub fn ready_bytes(leaves: &[Leaf]) -> u64 {
    leaves
        .iter()
        .filter(|leaf| can_tick(leaf))
        .map(|leaf| leaf.bytes)
        .sum()
}

pub fn load_live() -> Catalog {
    let Some(home) = std::env::var_os("HOME") else {
        return missing("Home is unset.");
    };
    let home = PathBuf::from(home);
    let (rows, hints) = scan::scan_root(&home, 1_000_000, true);
    let updated = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let comm = run("ps", &["-ax", "-o", "comm="]).unwrap_or_default();
    let commands = run("ps", &["-ax", "-o", "command="]).unwrap_or_default();
    let listing = format!("{comm}\n{commands}");
    let mut catalog = assemble(&rows, &hints, updated, &listing);
    let (total, free) = disk_space();
    catalog.disk_total = total;
    catalog.disk_free = free;
    let (memory, cpu, gpu) = host_gauges();
    catalog.memory = memory;
    catalog.cpu = cpu;
    catalog.gpu = gpu;
    catalog.checked = index_age(updated);
    catalog.missing = false;
    catalog
}

pub fn pending() -> Catalog {
    missing("Scanning this Mac…")
}

fn missing(checked: &str) -> Catalog {
    Catalog {
        groups: Vec::new(),
        leaves: Vec::new(),
        disk_total: 0,
        disk_free: 0,
        memory: 0,
        cpu: 0,
        gpu: 0,
        checked: checked.to_string(),
        missing: true,
    }
}

/// Synthetic data for window tests and explicitly labelled headless examples.
pub fn fixture() -> Catalog {
    let leaves = vec![
        leaf(
            "backups",
            "Backups and archives",
            true,
            0,
            None,
            None,
            None,
            27_000_000_000,
        ),
        leaf(
            "build",
            "frontend-target",
            false,
            0,
            None,
            Some(24),
            Some(1),
            17_480_000_000,
        ),
        leaf(
            "logs",
            "Logs and crash reports",
            false,
            500,
            Some(168),
            Some(24),
            Some(1),
            5_700_000_000,
        ),
        leaf(
            "toolchains",
            "mise node 26.13.0",
            true,
            3,
            None,
            None,
            None,
            10_830_000_000,
        ),
        leaf(
            "deps",
            "acx node_modules",
            false,
            2,
            None,
            Some(24),
            Some(1),
            3_300_000_000,
        ),
        leaf(
            "cache-held",
            "smolvm",
            true,
            1,
            None,
            None,
            None,
            17_000_000_000,
        ),
        leaf(
            "spotify",
            "Spotify cache",
            false,
            48,
            None,
            Some(24),
            Some(1),
            709_000_000,
        ),
    ];
    let groups = vec![
        group("backups", "Backups and archives", 0, 1),
        group("build", "Build output", 1, 1),
        group("logs", "Logs and crash reports", 2, 1),
        group("toolchains", "Toolchains and SDKs", 3, 1),
        group("deps", "Project dependencies", 4, 1),
        group("caches", "App caches", 5, 2),
    ];
    Catalog {
        groups,
        leaves,
        disk_total: 494_350_000_000,
        disk_free: 84_900_000_000,
        memory: 79,
        cpu: 30,
        gpu: 29,
        checked: "Indexed for the test".to_string(),
        missing: false,
    }
}

fn leaf(
    id: &str,
    name: &str,
    locked: bool,
    idle_h: u32,
    careful: Option<u32>,
    balanced: Option<u32>,
    aggressive: Option<u32>,
    bytes: u64,
) -> Leaf {
    Leaf {
        id: id.to_string(),
        path: format!("/fixture/{id}"),
        name: name.to_string(),
        detail: if locked {
            "Shelf leaves this".to_string()
        } else {
            format!("idle {idle_h} h")
        },
        size_label: format_bytes(bytes),
        bytes,
        idle_h,
        careful: if locked { None } else { careful },
        balanced: if locked { None } else { balanced },
        aggressive: if locked { None } else { aggressive },
        locked,
    }
}

fn group(id: &str, name: &str, first: usize, count: usize) -> Group {
    Group {
        id: id.to_string(),
        name: name.to_string(),
        detail: format!("{count} items"),
        size_label: String::new(),
        first,
        count,
    }
}

fn assemble(rows: &[Row], hints: &HashMap<String, Hint>, updated: f64, commands: &str) -> Catalog {
    let mut chosen: Vec<(Row, Hint)> = Vec::new();
    for row in rows {
        if let Some(hint) = choose_hint(&row.hint_ids, hints) {
            chosen.push((row.clone(), hint));
        }
    }
    chosen.sort_by_key(|(row, _)| row.path.matches('/').count());
    let mut kept: Vec<(Row, Hint, bool)> = Vec::new();
    let mut kept_paths: Vec<String> = Vec::new();
    for (row, hint) in chosen {
        if ancestor_kept(&row.path, &kept_paths) {
            continue;
        }
        let held = command_uses(commands, &row.path) || tool_running(&hint, commands);
        kept_paths.push(row.path.clone());
        kept.push((row, hint, held));
    }

    let mut by_group: HashMap<String, Vec<Leaf>> = HashMap::new();
    for (row, hint, held) in kept {
        let (careful, balanced, aggressive, tier_locked) = gates(&hint);
        let locked = held || tier_locked;
        let idle_h = idle_hours(row.newest, updated);
        let name = leaf_name(&hint.label, &row.path);
        let detail = if held {
            format!("In use · {}", idle_phrase(idle_h))
        } else if tier_locked {
            format!("Shelf leaves this · {}", idle_phrase(idle_h))
        } else {
            idle_phrase(idle_h)
        };
        by_group.entry(hint.group.clone()).or_default().push(Leaf {
            id: String::new(),
            path: row.path,
            name,
            detail,
            size_label: format_bytes(row.bytes),
            bytes: row.bytes,
            idle_h,
            careful: if locked { None } else { careful },
            balanced: if locked { None } else { balanced },
            aggressive: if locked { None } else { aggressive },
            locked,
        });
    }

    let mut names: Vec<String> = by_group.keys().cloned().collect();
    names.sort_by_key(|name| {
        let bytes: u64 = by_group[name].iter().map(|leaf| leaf.bytes).sum();
        std::cmp::Reverse(bytes)
    });

    let mut groups = Vec::new();
    let mut leaves = Vec::new();
    for name in names {
        let mut items = by_group.remove(&name).unwrap_or_default();
        items.sort_by_key(|leaf| std::cmp::Reverse(leaf.bytes));
        let bytes: u64 = items.iter().map(|leaf| leaf.bytes).sum();
        if bytes < MIN_GROUP_BYTES {
            continue;
        }
        let first = leaves.len();
        let count = items.len();
        let n = if count == 1 {
            "1 item".to_string()
        } else {
            format!("{count} items")
        };
        let locked_n = items.iter().filter(|leaf| leaf.locked).count();
        let detail = if locked_n == count {
            format!("{n} · Shelf leaves these")
        } else {
            n
        };
        for leaf in &mut items {
            leaf.id = format!("u{}", leaves.len());
            leaves.push(leaf.clone());
        }
        let id = format!("g{}", groups.len());
        groups.push(Group {
            id,
            name: name.clone(),
            detail,
            size_label: format_bytes(bytes),
            first,
            count,
        });
    }
    // ids were assigned with a running length before push; fix them to the final index.
    for (index, leaf) in leaves.iter_mut().enumerate() {
        leaf.id = format!("u{index}");
    }
    Catalog {
        groups,
        leaves,
        disk_total: 0,
        disk_free: 0,
        memory: 0,
        cpu: 0,
        gpu: 0,
        checked: String::new(),
        missing: false,
    }
}

fn choose_hint(ids: &[String], hints: &HashMap<String, Hint>) -> Option<Hint> {
    let known: Vec<&Hint> = ids.iter().filter_map(|id| hints.get(id)).collect();
    if known.is_empty() {
        return None;
    }
    // A non-regenerable or list-only hint wins, so a leftover parent stays locked.
    let locked: Vec<&Hint> = known.iter().copied().filter(|hint| blocks(hint)).collect();
    let pool: Vec<&Hint> = if locked.is_empty() { known } else { locked };
    // Prefer a named hint over the broad "every cache folder" rule when both match.
    let pick = pool
        .iter()
        .copied()
        .find(|hint| hint.id != "user-caches")
        .or_else(|| pool.first().copied())?;
    Some(pick.clone())
}

fn blocks(hint: &Hint) -> bool {
    hint.list_only || hint.tier != "regenerable"
}

fn tool_running(hint: &Hint, listing: &str) -> bool {
    hint.processes
        .iter()
        .any(|name| process_listed(listing, name))
}

fn process_listed(listing: &str, name: &str) -> bool {
    listing.lines().any(|line| {
        let line = line.trim();
        let base = line.rsplit('/').next().unwrap_or(line);
        let base = base.split_whitespace().next().unwrap_or(base);
        base.eq_ignore_ascii_case(name)
    })
}

fn gates(hint: &Hint) -> (Option<u32>, Option<u32>, Option<u32>, bool) {
    if blocks(hint) {
        return (None, None, None, true);
    }
    if let Some(days) = hint.untouched_days {
        let careful = days.saturating_mul(24);
        let balanced = 24.min(careful);
        return (Some(careful), Some(balanced), Some(1), false);
    }
    if let Some(days) = hint.project_days {
        return (None, Some(days.saturating_mul(24)), Some(1), false);
    }
    (None, Some(24), Some(1), false)
}

fn ancestor_kept(path: &str, kept: &[String]) -> bool {
    let mut parent = parent_path(path);
    while let Some(dir) = parent {
        if kept.iter().any(|kept_path| kept_path == &dir) {
            return true;
        }
        parent = parent_path(&dir);
    }
    false
}

fn parent_path(path: &str) -> Option<String> {
    let dir = Path::new(path).parent()?.to_string_lossy().to_string();
    if dir.is_empty() || dir == "/" {
        None
    } else {
        Some(dir)
    }
}

fn command_uses(commands: &str, path: &str) -> bool {
    if path.len() < 8 {
        return false;
    }
    let bytes = commands.as_bytes();
    let mut start = 0;
    while let Some(found) = commands[start..].find(path) {
        let at = start + found;
        let after = at + path.len();
        let before_ok = at == 0 || !bytes[at - 1].is_ascii_alphanumeric();
        let after_ok = after >= bytes.len()
            || matches!(
                bytes[after],
                b'/' | b' ' | b'\n' | b'\t' | b'"' | b'\'' | b'\\' | b')'
            );
        if before_ok && after_ok {
            return true;
        }
        start = at + 1;
    }
    false
}

fn idle_hours(newest: Option<f64>, updated: f64) -> u32 {
    let Some(newest) = newest else { return 0 };
    ((updated - newest).max(0.0) / 3600.0) as u32
}

fn idle_phrase(hours: u32) -> String {
    if hours < 1 {
        "used under an hour ago".to_string()
    } else if hours < 48 {
        format!("used {hours} h ago")
    } else if hours < 24 * 14 {
        format!("used {} d ago", hours / 24)
    } else {
        format!("used {} wk ago", hours / (24 * 7))
    }
}

fn leaf_name(label: &str, path: &str) -> String {
    let base = path.rsplit('/').next().unwrap_or(path);
    if base == "node_modules" || base == "target" || base == "data" || base == "src" {
        path.rsplit('/').nth(1).unwrap_or(base).to_string()
    } else if label == "App caches"
        || label == "Leftovers of removed apps"
        || label == "Containers of removed apps"
    {
        base.to_string()
    } else {
        label.to_string()
    }
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn disk_space() -> (u64, u64) {
    let text = run("diskutil", &["info", "/"]).unwrap_or_default();
    let total = bytes_after(&text, "Container Total Space");
    let free = bytes_after(&text, "Container Free Space");
    (total.unwrap_or(0), free.unwrap_or(0))
}

fn bytes_after(text: &str, label: &str) -> Option<u64> {
    let line = text.lines().find(|line| line.contains(label))?;
    let start = line.find('(')? + 1;
    let end = line[start..].find(" Bytes")?;
    line[start..start + end].trim().parse().ok()
}

fn host_gauges() -> (u32, u32, u32) {
    (memory_percent(), cpu_percent(), gpu_percent())
}

fn memory_percent() -> u32 {
    let total: u64 = run("sysctl", &["-n", "hw.memsize"])
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0);
    let stat = run("vm_stat", &[]).unwrap_or_default();
    let page = stat
        .lines()
        .find_map(|line| line.split("page size of ").nth(1))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|n| n.parse::<u64>().ok())
        .unwrap_or(16384);
    let mut pages = 0_u64;
    for name in [
        "Pages free",
        "Pages inactive",
        "Pages speculative",
        "Pages purgeable",
    ] {
        pages += vm_pages(&stat, name);
    }
    if total == 0 {
        return 0;
    }
    let available = pages.saturating_mul(page).min(total);
    (((total - available) as f64 / total as f64) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u32
}

fn vm_pages(stat: &str, name: &str) -> u64 {
    stat.lines()
        .find(|line| line.trim_start().starts_with(name))
        .and_then(|line| line.split(':').nth(1))
        .map(|rest| rest.trim().trim_end_matches('.').replace('.', ""))
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// Mach host CPU ticks. user, system, idle, nice. A one-second delta is the gauge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CpuTicks {
    pub user: u32,
    pub system: u32,
    pub idle: u32,
    pub nice: u32,
}

pub fn cpu_delta(before: CpuTicks, after: CpuTicks) -> Option<u32> {
    let user = after.user.wrapping_sub(before.user) as u64;
    let system = after.system.wrapping_sub(before.system) as u64;
    let idle = after.idle.wrapping_sub(before.idle) as u64;
    let nice = after.nice.wrapping_sub(before.nice) as u64;
    let total = user + system + idle + nice;
    if total == 0 {
        return None;
    }
    let busy = user + system + nice;
    Some(
        ((busy as f64 / total as f64) * 100.0)
            .round()
            .clamp(0.0, 100.0) as u32,
    )
}

pub fn cpu_ticks() -> Option<CpuTicks> {
    let mut ticks = [0_u32; 4];
    let mut count = 4_u32;
    let rc = unsafe {
        host_statistics(
            host_port(),
            HOST_CPU_LOAD_INFO,
            ticks.as_mut_ptr(),
            &mut count,
        )
    };
    if rc != 0 || count < 4 {
        return None;
    }
    Some(CpuTicks {
        user: ticks[0],
        system: ticks[1],
        idle: ticks[2],
        nice: ticks[3],
    })
}

fn host_port() -> u32 {
    static PORT: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *PORT.get_or_init(|| unsafe { mach_host_self() })
}

const HOST_CPU_LOAD_INFO: i32 = 3;

unsafe extern "C" {
    fn mach_host_self() -> u32;
    fn host_statistics(host: u32, flavor: i32, info: *mut u32, count: *mut u32) -> i32;
}

fn cpu_percent() -> u32 {
    let Some(before) = cpu_ticks() else {
        return 0;
    };
    std::thread::sleep(std::time::Duration::from_millis(200));
    let Some(after) = cpu_ticks() else {
        return 0;
    };
    cpu_delta(before, after).unwrap_or(0)
}

pub fn memory_now() -> u32 {
    memory_percent()
}

pub fn gpu_now() -> u32 {
    gpu_percent()
}

/// Measure one folder again and store the allocated size on the leaf.
pub fn remeasure(leaf: &mut Leaf) {
    let (bytes, _) = scan::measure(Path::new(&leaf.path));
    leaf.bytes = bytes;
    leaf.size_label = format_bytes(bytes);
}

fn gpu_percent() -> u32 {
    crate::gpu::percent().unwrap_or(0)
}

fn index_age(updated: f64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(updated);
    let age = (now - updated).max(0.0);
    if age < 90.0 {
        "Scanned just now".to_string()
    } else if age < 3600.0 {
        format!("Scanned {} min ago", (age / 60.0) as u32)
    } else if age < 86_400.0 {
        format!("Scanned {} h ago", (age / 3600.0) as u32)
    } else {
        format!("Scanned {} d ago", (age / 86_400.0) as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hint(
        id: &str,
        group: &str,
        tier: &str,
        list_only: bool,
        untouched: Option<u32>,
        project: Option<u32>,
    ) -> (String, Hint) {
        (
            id.to_string(),
            Hint {
                id: id.to_string(),
                group: group.to_string(),
                label: id.to_string(),
                tier: tier.to_string(),
                list_only,
                untouched_days: untouched,
                project_days: project,
                processes: Vec::new(),
            },
        )
    }

    #[test]
    fn a_parent_absorbs_its_child_and_a_running_path_stays_locked() {
        let hints = HashMap::from([
            hint(
                "user-caches",
                "App caches",
                "regenerable",
                false,
                None,
                None,
            ),
            hint(
                "logs",
                "Logs and crash reports",
                "regenerable",
                false,
                Some(7),
                None,
            ),
        ]);
        let rows = vec![
            Row {
                path: "/Users/a/Library/Caches/vm".into(),
                bytes: 60_000_000,
                newest: Some(1_000.0),
                hint_ids: vec!["user-caches".into()],
            },
            Row {
                path: "/Users/a/Library/Caches/vm/disk".into(),
                bytes: 50_000_000,
                newest: Some(1_000.0),
                hint_ids: vec!["user-caches".into()],
            },
            Row {
                path: "/Users/a/Library/Logs/app".into(),
                bytes: 50_000_000,
                newest: Some(0.0),
                hint_ids: vec!["logs".into()],
            },
        ];
        // 7 days is 168 h. The index is a week and an hour newer than the log folder.
        let catalog = assemble(
            &rows,
            &hints,
            (168 * 3600 + 3600) as f64,
            "/opt/smolvm /Users/a/Library/Caches/vm/boot\n",
        );
        assert!(catalog
            .leaves
            .iter()
            .all(|leaf| leaf.path != "/Users/a/Library/Caches/vm/disk"));
        let vm = catalog
            .leaves
            .iter()
            .find(|leaf| leaf.path.ends_with("/vm"))
            .unwrap();
        assert!(vm.locked);
        assert!(!can_tick(vm));
        let logs = catalog
            .leaves
            .iter()
            .find(|leaf| leaf.path.ends_with("/app"))
            .unwrap();
        assert_eq!(logs.careful, Some(168));
        assert!(auto_tick(logs, Level::Careful));
        assert!(!auto_tick(logs, Level::None));
    }

    #[test]
    fn cpu_delta_is_the_busy_share_of_new_ticks() {
        let before = CpuTicks {
            user: 10,
            system: 5,
            idle: 80,
            nice: 5,
        };
        let after = CpuTicks {
            user: 20,
            system: 15,
            idle: 150,
            nice: 5,
        };
        // 20 busy ticks out of 90 new ticks.
        assert_eq!(cpu_delta(before, after), Some(22));
        assert_eq!(cpu_delta(before, before), None);
    }

    #[test]
    fn this_mac_reports_cpu_ticks() {
        let before = cpu_ticks().expect("host_statistics");
        std::thread::sleep(std::time::Duration::from_millis(50));
        let after = cpu_ticks().expect("host_statistics");
        assert!(cpu_delta(before, after).is_some());
    }

    #[test]
    fn project_days_wait_for_balanced_and_a_fresh_build_waits() {
        let hints = HashMap::from([
            hint(
                "node-modules",
                "Project dependencies",
                "regenerable",
                false,
                None,
                Some(14),
            ),
            hint(
                "cargo-target",
                "Build output",
                "regenerable",
                false,
                None,
                None,
            ),
            hint("rustup", "Toolchains and SDKs", "costly", true, None, None),
        ]);
        let rows = vec![
            Row {
                path: "/Users/a/app/node_modules".into(),
                bytes: 80_000_000,
                newest: Some(0.0),
                hint_ids: vec!["node-modules".into()],
            },
            Row {
                path: "/Users/a/app/target".into(),
                bytes: 80_000_000,
                newest: Some(9_000.0),
                hint_ids: vec!["cargo-target".into()],
            },
            Row {
                path: "/Users/a/.rustup/toolchains/stable".into(),
                bytes: 80_000_000,
                newest: Some(0.0),
                hint_ids: vec!["rustup".into()],
            },
        ];
        // The project has been idle for 20 days. The build folder was written 2 hours ago.
        let updated = 20.0 * 24.0 * 3600.0;
        let mut rows = rows;
        rows[1].newest = Some(updated - 2.0 * 3600.0);
        let catalog = assemble(&rows, &hints, updated, "");
        let deps = catalog
            .leaves
            .iter()
            .find(|leaf| leaf.path.ends_with("node_modules"))
            .unwrap();
        assert_eq!(deps.balanced, Some(14 * 24));
        assert!(deps.careful.is_none());
        assert!(!auto_tick(deps, Level::Careful));
        assert!(auto_tick(deps, Level::Balanced));
        let build = catalog
            .leaves
            .iter()
            .find(|leaf| leaf.path.ends_with("target"))
            .unwrap();
        // No day field: balanced waits 24 h, aggressive waits 1 h. Idle is 2 h.
        assert!(auto_tick(build, Level::Aggressive));
        assert!(!auto_tick(build, Level::Balanced));
        let toolchain = catalog
            .leaves
            .iter()
            .find(|leaf| leaf.path.contains("toolchains"))
            .unwrap();
        assert!(toolchain.locked);
        assert!(!auto_tick(toolchain, Level::Aggressive));
    }
}
