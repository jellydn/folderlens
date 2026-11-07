//! Shelf is a menu-tray review of what a cleanup level would tick.
//!
//! Rune has no macOS status item, so Shelf installs its own. The panel is an
//! accessory window: no Dock icon. A click on the menu-bar item shows it under
//! that item. The action list lives in the panel. Confirm records the choice
//! and deletes nothing. Settings and the panel are never open together.
//!
//! The list, the free-space line, and the gauges come from this Mac.
//! Shelf ships its own hint catalog and measures the folders itself.
//! Tests click a fixed fixture so they do not depend on the live disk.
//!
//! `CARGO_HOME="$PWD/.cargo-home" cargo run -- --snap shots/tray.png`

mod agent;
mod catalog;
mod gpu;
mod palette;
mod prefs;
mod settings;
mod status;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use catalog::{Catalog, Level};
use rune::tok::{type_, TextStyle};
use rune::{
    canvas, col, el, rich, row, text, Align, App, AppSpec, Color, Cx, El, KeyEvent, LineCap,
    MacWindowOptions, ScrollSpec, Shape, Span, Transition, Tx, WindowOptions, WindowSpec,
};

const PANEL: &str = "#1c1c1e";
const INK: &str = "#f5f5f7";
const MUTED: &str = "#98989d";
const LINE: &str = "#ffffff";
const GREEN: &str = "#30d158";
const BLUE: &str = "#0a84ff";
const RED: &str = "#ff453a";
const YELLOW: &str = "#ffd60a";
const TRACK: &str = "#3a3a3c";

struct State {
    catalog: Catalog,
    level: Level,
    /// None follows the level. Some is a tick the user set.
    ticks: Vec<Option<bool>>,
    /// One flag per group. A group with several leaves can open.
    open: Vec<bool>,
    confirmed: bool,
    prefs: prefs::Prefs,
    prefs_path: std::path::PathBuf,
    palette: bool,
    select_open: bool,
    query: String,
    highlight: usize,
    target: Option<palette::Target>,
    /// Session unlocks. A restart clears them. In-use rows stay locked on disk.
    unlocked: Vec<String>,
    note: String,
    agent: Option<agent::Job>,
    agent_details: bool,
    paused_until: f64,
    /// Last height sent to the window. Arrow keys must not resize again.
    window_h: f32,
}

fn state_from(catalog: Catalog) -> State {
    let mut open = vec![false; catalog.groups.len()];
    if let Some(index) = catalog.groups.iter().position(|group| group.count > 1) {
        open[index] = true;
    }
    let ticks = vec![None; catalog.leaves.len()];
    let prefs_path = prefs::path_for_state();
    let prefs = prefs::load_from(&prefs_path);
    State {
        catalog,
        level: Level::None,
        ticks,
        open,
        confirmed: false,
        prefs,
        prefs_path,
        palette: false,
        select_open: false,
        query: String::new(),
        highlight: 0,
        target: None,
        unlocked: Vec::new(),
        note: String::new(),
        agent: None,
        agent_details: false,
        paused_until: 0.0,
        window_h: palette::TRAY_H,
    }
}

fn live_state() -> State {
    if rune::is_headless() {
        return state_from(catalog::load_live());
    }
    state_from(catalog::pending())
}

/// Workers send results through Rune's UI queue, including while the panel is hidden.
fn start_workers(handle: rune::AppHandle<State>) {
    let scan_handle = handle.clone();
    std::thread::spawn(move || {
        let catalog = catalog::load_live();
        scan_handle.update(move |s, tx| {
            s.ticks = vec![None; catalog.leaves.len()];
            s.open = vec![false; catalog.groups.len()];
            if let Some(index) = catalog.groups.iter().position(|group| group.count > 1) {
                s.open[index] = true;
            }
            s.catalog = catalog;
            s.confirmed = false;
            tx.instant();
        });
    });
    std::thread::spawn(move || {
        let mut before = catalog::cpu_ticks();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let after = catalog::cpu_ticks();
            let cpu = before
                .zip(after)
                .and_then(|(a, b)| catalog::cpu_delta(a, b));
            before = after;
            let memory = catalog::memory_now();
            let gpu = catalog::gpu_now();
            handle.update_if(move |s, tx| {
                let cpu = cpu.unwrap_or(s.catalog.cpu);
                let changed =
                    (s.catalog.cpu, s.catalog.memory, s.catalog.gpu) != (cpu, memory, gpu);
                s.catalog.cpu = cpu;
                s.catalog.memory = memory;
                s.catalog.gpu = gpu;
                tx.instant();
                changed
            });
        }
    });
}

#[cfg(test)]
fn fixture_state() -> State {
    state_from(catalog::fixture())
}

fn apply_level(s: &mut State, level: Level) {
    s.level = level;
    s.ticks.fill(None);
    s.confirmed = false;
    s.note.clear();
}

fn unix_now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// AppKit pointer tracking stays on the UI thread; system commands do not.
fn poll(s: &mut State, _now: f64, tx: &mut Tx) -> bool {
    status::on_poll();
    tx.wake_after(if status::is_visible() {
        Duration::from_millis(100)
    } else {
        Duration::from_secs(1)
    });
    let mut changed = false;
    let prefs = prefs::load_from(&s.prefs_path);
    if prefs != s.prefs {
        s.prefs = prefs;
        changed = true;
    }
    if let Some(job) = &s.agent {
        job.set_access(palette::ask_access(s));
        if !s.prefs.allow_start || s.prefs.agent_bin != job.bin {
            job.cancel();
        }
    }
    if let Some(job) = &mut s.agent {
        changed |= job.poll();
    }
    if s.paused_until > 0.0 && unix_now() >= s.paused_until && s.note.starts_with("Alerts paused") {
        s.paused_until = 0.0;
        s.note.clear();
        changed = true;
    }
    changed
}

fn on_key(s: &mut State, ev: &KeyEvent, tx: &mut Tx) -> bool {
    palette::on_key(s, ev, tx)
}

/// Always-protected roots ignore Unlock. A kept folder and the hour hold can be
/// unlocked for this session. A leaf with no gate stays locked.
fn leaf_view(s: &State, index: usize) -> catalog::Leaf {
    let mut leaf = s.catalog.leaves[index].clone();
    if let Some(name) = prefs::always_name(&leaf.path) {
        leaf.locked = true;
        leaf.detail = format!("Always protected · {name}");
        return leaf;
    }
    if s.unlocked.iter().any(|path| path == &leaf.path) {
        leaf.locked = false;
        if catalog::can_tick(&leaf) {
            leaf.detail = "Unlocked".to_string();
        } else {
            leaf.locked = true;
        }
        return leaf;
    }
    if s.prefs
        .kept
        .iter()
        .any(|root| prefs::path_under(&leaf.path, root))
    {
        leaf.locked = true;
        leaf.detail = "Shelf keeps this".to_string();
        return leaf;
    }
    if s.prefs.keep_hours > 0 && catalog::can_tick(&leaf) && leaf.idle_h < s.prefs.keep_hours {
        leaf.locked = true;
        leaf.detail = format!("Changed within {} h", s.prefs.keep_hours);
    }
    leaf
}

fn viewed_ready(s: &State) -> u64 {
    (0..s.catalog.leaves.len())
        .filter(|&index| catalog::can_tick(&leaf_view(s, index)))
        .map(|index| s.catalog.leaves[index].bytes)
        .sum()
}

fn cycle_level(s: &mut State) {
    let next = match s.level {
        Level::None => Level::Careful,
        Level::Careful => Level::Balanced,
        Level::Balanced => Level::Aggressive,
        Level::Aggressive => Level::None,
    };
    apply_level(s, next);
}

fn is_ticked(s: &State, index: usize) -> bool {
    let leaf = leaf_view(s, index);
    if !catalog::can_tick(&leaf) {
        return false;
    }
    match s.ticks.get(index).copied().flatten() {
        Some(on) => on,
        None => catalog::auto_tick(&leaf, s.level),
    }
}

fn toggle_leaf(s: &mut State, index: usize) {
    if !catalog::can_tick(&leaf_view(s, index)) {
        return;
    }
    let on = is_ticked(s, index);
    s.ticks[index] = Some(!on);
    s.confirmed = false;
}

fn leaf_range(s: &State, index: usize) -> std::ops::Range<usize> {
    let group = &s.catalog.groups[index];
    group.first..group.first + group.count
}

fn toggle_group(s: &mut State, index: usize) {
    let range = leaf_range(s, index);
    let tickable: Vec<usize> = range
        .filter(|&leaf_index| catalog::can_tick(&leaf_view(s, leaf_index)))
        .collect();
    if tickable.is_empty() {
        return;
    }
    let turn_on = !tickable.iter().all(|&leaf_index| is_ticked(s, leaf_index));
    for leaf_index in tickable {
        s.ticks[leaf_index] = Some(turn_on);
    }
    s.confirmed = false;
}

fn group_on(s: &State, index: usize) -> bool {
    let range = leaf_range(s, index);
    let mut any = false;
    for leaf_index in range {
        if catalog::can_tick(&leaf_view(s, leaf_index)) {
            any = true;
            if !is_ticked(s, leaf_index) {
                return false;
            }
        }
    }
    any
}

fn group_locked(s: &State, index: usize) -> bool {
    leaf_range(s, index).all(|leaf_index| !catalog::can_tick(&leaf_view(s, leaf_index)))
}

fn group_bytes(s: &State, index: usize) -> u64 {
    leaf_range(s, index)
        .map(|leaf_index| s.catalog.leaves[leaf_index].bytes)
        .sum()
}

fn ticked_bytes(s: &State) -> u64 {
    s.catalog
        .leaves
        .iter()
        .enumerate()
        .filter(|(index, _)| is_ticked(s, *index))
        .map(|(_, leaf)| leaf.bytes)
        .sum()
}

#[cfg(test)]
fn ticked_ids(s: &State) -> Vec<String> {
    s.catalog
        .leaves
        .iter()
        .enumerate()
        .filter(|(index, _)| is_ticked(s, *index))
        .map(|(_, leaf)| leaf.id.clone())
        .collect()
}

fn level_name(level: Level) -> &'static str {
    match level {
        Level::None => "None",
        Level::Careful => "Careful",
        Level::Balanced => "Balanced",
        Level::Aggressive => "Aggressive",
    }
}

fn level_label(s: &State) -> &'static str {
    if s.ticks.iter().any(|tick| tick.is_some()) {
        "Selected"
    } else {
        level_name(s.level)
    }
}

fn ticked_groups(s: &State) -> Vec<(usize, String)> {
    s.catalog
        .groups
        .iter()
        .enumerate()
        .filter(|(_, group)| {
            (group.first..group.first + group.count).any(|leaf| is_ticked(s, leaf))
        })
        .map(|(index, group)| (index, group.name.clone()))
        .collect()
}

fn cleanup_line(s: &State) -> String {
    let groups = ticked_groups(s);
    if groups.is_empty() {
        return String::new();
    }
    let lead = format!(
        "Your cleanup frees {}",
        catalog::format_bytes(ticked_bytes(s))
    );
    if groups.len() == 1 {
        return format!("{lead}: {}.", groups[0].1);
    }
    if groups.len() == 2 {
        return format!("{lead}: {}, {}.", groups[0].1, groups[1].1);
    }
    let named = [groups[0].0, groups[1].0];
    let extra = (0..s.catalog.leaves.len())
        .filter(|&index| is_ticked(s, index))
        .filter(|&index| {
            let group = s
                .catalog
                .groups
                .iter()
                .position(|group| index >= group.first && index < group.first + group.count);
            group.is_some_and(|group| !named.contains(&group))
        })
        .count();
    if extra == 0 {
        format!("{lead}: {}, {}.", groups[0].1, groups[1].1)
    } else {
        format!("{lead}: {}, {} and {extra} more.", groups[0].1, groups[1].1)
    }
}

fn paint(s: &str) -> Color {
    rune::color::hex(s).unwrap_or(Color::WHITE)
}

fn rule() -> El {
    el().w_full().h(1.0).fill(paint(LINE).alpha(0.08))
}

fn laptop() -> El {
    let ink = paint(MUTED);
    el().size(16.0, 12.0)
        .child(
            el().size(14.0, 9.0)
                .radius(2.0)
                .border(1.2, ink)
                .absolute(1.0, 0.0),
        )
        .child(
            el().size(16.0, 1.5)
                .radius(1.0)
                .fill(ink)
                .absolute(0.0, 10.0),
        )
}

/// 270° of ticks. The gap sits at the bottom. Y grows downward, so angle 0 is to the right.
fn gauge_shape(pct: f32) -> Shape {
    let (cx, cy, r) = (46.0, 38.0, 30.0);
    let ticks = 28;
    let start = 135.0_f32.to_radians();
    let sweep = 270.0_f32.to_radians();
    let cyan = paint("#64d2ff");
    let yellow = paint(YELLOW);
    let orange = paint("#ff9f0a");
    let rest = paint("#545458");
    let filled = (pct / 100.0 * ticks as f32)
        .round()
        .clamp(0.0, ticks as f32) as i32;
    let mut shape = Shape::new();
    for i in 0..ticks {
        let along = i as f32 / (ticks - 1) as f32;
        let ang = start + sweep * along;
        let (s, c) = ang.sin_cos();
        let color = if (i as i32) < filled {
            let u = if filled <= 1 {
                0.0
            } else {
                i as f32 / (filled - 1) as f32
            };
            if u < 0.5 {
                cyan.mix(yellow, u * 2.0)
            } else {
                yellow.mix(orange, (u - 0.5) * 2.0)
            }
        } else {
            rest
        };
        shape = shape.line(
            [cx + (r - 8.0) * c, cy + (r - 8.0) * s],
            [cx + r * c, cy + r * s],
            2.4,
            LineCap::Round,
            color,
        );
    }
    shape
}

fn gauge(name: &str, pct: u32) -> El {
    let w = 92.0;
    let h = 74.0;
    col()
        .grow(1.0)
        .items_center()
        .gap(0.0)
        .child(
            el().size(w, h)
                .child(
                    canvas(gauge_shape(pct as f32))
                        .size(w, h)
                        .absolute(0.0, 0.0),
                )
                .child(
                    text(format!("{pct}%"))
                        .font(TextStyle::new(18.0, 600.0).lh(22.0))
                        .color(paint(INK))
                        .align(Align::Center)
                        .w(w)
                        .absolute(0.0, 26.0),
                ),
        )
        .child(text(name).font(type_::LABEL).color(paint(MUTED)))
}

fn capacity_bar(catalog: &Catalog, ready_bytes: u64) -> El {
    let total = catalog.disk_total as f64 / 1_000_000_000.0;
    let free = catalog.disk_free as f64 / 1_000_000_000.0;
    let ready = ready_bytes as f64 / 1_000_000_000.0;
    let (red, yellow, green, gray) = catalog::bar_counts(total, free, ready);
    let mut bar = row().key("disk-bar").w_full().h(9.0).gap(1.5);
    for (count, color) in [(red, RED), (yellow, YELLOW), (green, GREEN), (gray, TRACK)] {
        for _ in 0..count {
            bar = bar.child(el().grow(1.0).h(9.0).radius(2.0).fill(paint(color)));
        }
    }
    bar
}

fn check_mark() -> Shape {
    let ink = Color::WHITE;
    Shape::new()
        .line([3.5, 8.0], [6.5, 11.0], 1.6, LineCap::Round, ink)
        .line([6.5, 11.0], [12.5, 4.5], 1.6, LineCap::Round, ink)
}

fn checkbox(key: String, on: bool, press: impl Fn(&mut State) + 'static) -> El {
    let mut box_ = el()
        .key(key)
        .size(16.0, 16.0)
        .radius(4.0)
        .border(1.2, if on { paint(BLUE) } else { paint("#636366") })
        .fill(if on { paint(BLUE) } else { paint("#2a2a2c") })
        .on_press(press);
    if on {
        box_ = box_.child(canvas(check_mark()).size(16.0, 16.0).absolute(0.0, 0.0));
    }
    box_
}

fn lock_icon() -> El {
    let ink = paint(MUTED);
    let shackle = Shape::new()
        .line([4.5, 7.0], [4.5, 5.2], 1.3, LineCap::Round, ink)
        .line([4.5, 5.2], [8.0, 3.2], 1.3, LineCap::Round, ink)
        .line([8.0, 3.2], [11.5, 5.2], 1.3, LineCap::Round, ink)
        .line([11.5, 5.2], [11.5, 7.0], 1.3, LineCap::Round, ink);
    el().size(16.0, 16.0)
        .child(canvas(shackle).size(16.0, 16.0).absolute(0.0, 0.0))
        .child(
            el().size(10.0, 7.0)
                .radius(1.5)
                .fill(ink)
                .absolute(3.0, 7.5),
        )
}

fn letter_icon(mark: &str, tint: &str) -> El {
    el().size(22.0, 22.0)
        .radius(6.0)
        .fill(paint(tint))
        .center()
        .child(
            text(mark)
                .font(TextStyle::new(12.0, 700.0))
                .color(Color::WHITE),
        )
}

fn status_line(s: &State) -> El {
    let muted = paint(MUTED);
    let ink = paint(INK);
    if !s.note.is_empty() {
        return text(s.note.clone())
            .key("status")
            .font(TextStyle::new(13.0, 600.0).lh(18.0))
            .color(ink)
            .w_full();
    }
    if s.catalog.missing {
        return text(s.catalog.checked.clone())
            .key("status")
            .font(TextStyle::new(13.0, 600.0).lh(18.0))
            .color(ink)
            .w_full();
    }
    let bytes = ticked_bytes(s);
    if bytes == 0 {
        let line = "Nothing to clean up. More to choose from.";
        let choose = line.find("to choose").unwrap_or(0);
        rich(
            line,
            vec![
                Span::new(0, "Nothing".len()).weight(650.0).color(ink),
                Span::new(choose, choose + "to choose".len())
                    .weight(650.0)
                    .color(ink),
            ],
        )
        .key("status")
        .font(TextStyle::new(13.0, 400.0).lh(18.0))
        .color(muted)
        .w_full()
    } else {
        text(cleanup_line(s))
            .key("status")
            .font(TextStyle::new(13.0, 600.0).lh(18.0))
            .color(ink)
            .wrap()
            .w_full()
    }
}

fn group_row(s: &State, index: usize) -> El {
    let group = &s.catalog.groups[index];
    let id = group.id.clone();
    let name = group.name.clone();
    let detail = group.detail.clone();
    let mark = group.mark.clone();
    let tint = group.tint.clone();
    let opens = group.count > 1;
    let open = s.open.get(index).copied().unwrap_or(false);
    let size = catalog::format_bytes(group_bytes(s, index));
    let on = group_on(s, index);
    let locked = group_locked(s, index);
    let chevron = if opens && open { "⌄" } else { "›" };
    let mut chevron_el = text(chevron)
        .font(TextStyle::new(14.0, 500.0))
        .color(paint(MUTED))
        .w(16.0)
        .align(Align::Center);
    if opens {
        chevron_el = chevron_el
            .key(format!("expand-{id}"))
            .on_press(move |state: &mut State| {
                if let Some(flag) = state.open.get_mut(index) {
                    *flag = !*flag;
                }
            });
    }
    let lead = if locked {
        lock_icon()
    } else {
        checkbox(format!("check-{id}"), on, move |state| {
            toggle_group(state, index)
        })
    };
    row()
        .key(format!("row-{id}"))
        .w_full()
        .h(44.0)
        .px(8.0)
        .gap(8.0)
        .center_y()
        .child(lead)
        .child(letter_icon(&mark, &tint))
        .child(
            col()
                .grow(1.0)
                .gap(1.0)
                .child(text(name).font(type_::BODY).color(paint(INK)))
                .child(text(detail).font(type_::LABEL).color(paint(MUTED))),
        )
        .child(text(size).font(type_::LABEL).color(paint("#d1d1d6")))
        .child(chevron_el)
        .on_secondary_press(move |state: &mut State, _, tx| {
            tx.instant();
            palette::open(state, Some(palette::Target::Group(index)));
            palette::fit(state, tx);
        })
}

fn child_row(s: &State, index: usize) -> El {
    let leaf = leaf_view(s, index);
    let id = leaf.id.clone();
    let name = leaf.name.clone();
    let detail = leaf.detail.clone();
    let size = s.catalog.leaves[index].size_label.clone();
    let mark = leaf.mark.clone();
    let tint = leaf.tint.clone();
    let locked = !catalog::can_tick(&leaf);
    let lead = if locked {
        lock_icon()
    } else {
        let on = is_ticked(s, index);
        checkbox(format!("check-{id}"), on, move |state| {
            toggle_leaf(state, index)
        })
    };
    let mut line = row()
        .key(format!("row-{id}"))
        .w_full()
        .h(36.0)
        .pl(34.0)
        .pr(8.0)
        .gap(8.0)
        .center_y()
        .child(lead);
    if !locked {
        line = line.child(letter_icon(&mark, &tint));
    }
    line.child(
        col()
            .grow(1.0)
            .gap(1.0)
            .child(text(name).font(type_::BODY).color(paint(INK)))
            .child(text(detail).font(type_::LABEL).color(paint(MUTED))),
    )
    .child(text(size).font(type_::LABEL).color(paint("#d1d1d6")))
    .on_secondary_press(move |state: &mut State, _, tx| {
        tx.instant();
        palette::open(state, Some(palette::Target::Leaf(index)));
        palette::fit(state, tx);
    })
}

fn gauge_band(s: &State) -> Option<El> {
    let mut parts = Vec::new();
    if s.prefs.gauge_memory {
        parts.push(("Memory", s.catalog.memory));
    }
    if s.prefs.gauge_cpu {
        parts.push(("CPU", s.catalog.cpu));
    }
    if s.prefs.gauge_gpu {
        parts.push(("GPU", s.catalog.gpu));
    }
    if parts.is_empty() {
        return None;
    }
    let mut band = row().key("gauge").w_full().h(100.0).px(4.0).center_y();
    for (index, (name, pct)) in parts.into_iter().enumerate() {
        if index > 0 {
            band = band.child(el().w(1.0).h(56.0).fill(paint(LINE).alpha(0.08)));
        }
        band = band.child(gauge(name, pct));
    }
    Some(band)
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    if s.palette {
        return palette::panel(s);
    }
    let (free_n, free_unit) = if s.catalog.disk_total == 0 {
        ("—".to_string(), "")
    } else {
        catalog::free_parts(s.catalog.disk_free)
    };
    let ready_bytes = viewed_ready(s);
    let ready = if s.catalog.missing {
        String::new()
    } else {
        format!("{} ready", catalog::format_bytes(ready_bytes))
    };
    let checked = s.catalog.checked.clone();
    let mut list = col().key("list").w_full().py(4.0);
    for index in 0..s.catalog.groups.len() {
        list = list.child(group_row(s, index));
        if s.catalog.groups[index].count > 1 && s.open.get(index) == Some(&true) {
            let first = s.catalog.groups[index].first;
            let count = s.catalog.groups[index].count;
            for leaf_index in first..first + count {
                list = list.child(child_row(s, leaf_index));
            }
        }
    }
    let confirm = if s.confirmed {
        "Nothing deleted"
    } else {
        "Confirm"
    };
    let confirm_ink = if s.confirmed {
        paint(MUTED)
    } else {
        paint(INK)
    };

    let mut body = col()
        .w_full()
        .h_full()
        .fill(paint(PANEL))
        .radius(14.0)
        .transition(Transition::none())
        .child(
            row()
                .w_full()
                .h(44.0)
                .px(14.0)
                .gap(8.0)
                .center_y()
                .window_drag()
                .child(laptop())
                .child(
                    text("Shelf")
                        .font(TextStyle::new(15.0, 600.0))
                        .color(paint(INK)),
                ),
        )
        .child(rule());
    if let Some(band) = gauge_band(s) {
        body = body.child(band).child(rule());
    }
    body.child(
        col()
            .w_full()
            .px(16.0)
            .py(10.0)
            .gap(10.0)
            .child(
                row()
                    .w_full()
                    .items_end()
                    .child(
                        text(free_n)
                            .font(TextStyle::new(28.0, 640.0).lh(32.0))
                            .color(paint(INK)),
                    )
                    .child(
                        text(free_unit)
                            .font(TextStyle::new(15.0, 500.0))
                            .color(paint(INK))
                            .mb(4.0)
                            .ml(4.0),
                    )
                    .child(
                        text("free")
                            .font(type_::BODY)
                            .color(paint(MUTED))
                            .mb(4.0)
                            .ml(6.0),
                    )
                    .child(el().grow(1.0))
                    .child(
                        text(ready)
                            .font(TextStyle::new(15.0, 600.0))
                            .color(paint(GREEN))
                            .mb(4.0),
                    ),
            )
            .child(capacity_bar(&s.catalog, ready_bytes)),
    )
    .child(rule())
    .child(col().w_full().px(16.0).py(10.0).child(status_line(s)))
    .child(rule())
    .child(
        row()
            .w_full()
            .h(36.0)
            .px(16.0)
            .center_y()
            .child(
                text("Ready to free")
                    .font(TextStyle::new(13.0, 600.0))
                    .color(paint(INK))
                    .grow(1.0),
            )
            .child(if s.agent.is_some() {
                text("Terminal")
                    .key("agent-terminal")
                    .font(type_::LABEL)
                    .color(paint(
                        if s.agent
                            .as_ref()
                            .is_some_and(|job| matches!(job.result, Some(Ok(_))))
                        {
                            BLUE
                        } else {
                            MUTED
                        },
                    ))
                    .mr(8.0)
                    .on_press(|state: &mut State| {
                        if let Some(job) = &state.agent {
                            if let Err(error) = job.open_chat(&state.prefs) {
                                state.note = error;
                            }
                        }
                    })
            } else {
                el()
            })
            .child(
                row()
                    .key("level-menu")
                    .h(24.0)
                    .px(8.0)
                    .gap(4.0)
                    .center_y()
                    .radius(6.0)
                    .fill(paint("#2a2a2c"))
                    .on_press(|state: &mut State| cycle_level(state))
                    .child(
                        text(level_label(s))
                            .key("level-label")
                            .font(type_::LABEL)
                            .color(paint(INK)),
                    )
                    .child(text("⌄").font(type_::LABEL).color(paint(MUTED))),
            ),
    )
    .child(
        list.scroll(ScrollSpec {
            stick_bottom: false,
        })
        .grow(1.0)
        .min_h(0.0),
    )
    .child(if s.agent.is_some() { rule() } else { el() })
    .child(agent_review(s))
    .child(rule())
    .child(
        row()
            .w_full()
            .h(36.0)
            .px(16.0)
            .gap(16.0)
            .center_y()
            .child(
                text(checked)
                    .font(type_::LABEL)
                    .color(paint(MUTED))
                    .grow(1.0),
            )
            .child(
                row()
                    .key("actions")
                    .h(24.0)
                    .px(4.0)
                    .center_y()
                    .on_press_tx(|state: &mut State, tx: &mut Tx| {
                        tx.instant();
                        palette::open(state, None);
                        palette::fit(state, tx);
                    })
                    .child(
                        text("Actions")
                            .font(TextStyle::new(13.0, 600.0))
                            .color(paint(INK)),
                    ),
            )
            .child(
                text(confirm)
                    .key("confirm")
                    .font(TextStyle::new(13.0, 600.0))
                    .color(confirm_ink)
                    .on_press(|state: &mut State| state.confirmed = true),
            ),
    )
}

fn agent_review(s: &State) -> El {
    let Some(job) = &s.agent else {
        return el();
    };
    let mut panel = col()
        .key("agent-review")
        .w_full()
        .px(16.0)
        .py(8.0)
        .gap(4.0)
        .child(
            row()
                .w_full()
                .center_y()
                .child(
                    text(job.status())
                        .key("agent-progress")
                        .font(type_::LABEL)
                        .color(paint(MUTED))
                        .grow(1.0),
                )
                .child(if job.result.is_none() {
                    text("Cancel")
                        .key("agent-cancel")
                        .font(type_::LABEL)
                        .color(paint(BLUE))
                        .on_press(|state: &mut State| {
                            if let Some(job) = &state.agent {
                                job.cancel();
                            }
                        })
                } else {
                    el()
                }),
        );
    if let Some(result) = &job.result {
        let text = match result {
            Ok(report) => report,
            Err(error) => error,
        };
        let summary: String = if result.is_err() {
            text.lines().next().unwrap_or(text)
        } else {
            text
        }
        .chars()
        .take(280)
        .collect();
        panel = panel.child(
            rune::text(summary)
                .key("agent-summary")
                .font(type_::LABEL)
                .color(paint(if result.is_err() { RED } else { INK }))
                .wrap()
                .w_full()
                .max_h(90.0),
        );
        if result.is_err() {
            panel = panel.child(
                row()
                    .w_full()
                    .gap(16.0)
                    .child(
                        rune::text(if s.agent_details {
                            "Hide details"
                        } else {
                            "Details"
                        })
                        .key("agent-error-toggle")
                        .font(type_::LABEL)
                        .color(paint(BLUE))
                        .on_press(|state: &mut State| state.agent_details = !state.agent_details),
                    )
                    .child(
                        rune::text("Agent settings")
                            .key("agent-error-settings")
                            .font(type_::LABEL)
                            .color(paint(BLUE))
                            .on_press(|_: &mut State| settings::open(settings::Page::Agent)),
                    )
                    .child(
                        rune::text("Retry")
                            .key("agent-retry")
                            .font(type_::LABEL)
                            .color(paint(BLUE))
                            .on_press(palette::ask),
                    ),
            );
            if s.agent_details {
                panel = panel.child(
                    col()
                        .w_full()
                        .max_h(110.0)
                        .child(
                            rune::text(text.clone())
                                .key("agent-error-details")
                                .font(type_::LABEL)
                                .color(paint(MUTED))
                                .wrap()
                                .w_full(),
                        )
                        .scroll(ScrollSpec {
                            stick_bottom: false,
                        }),
                );
            }
        }
    }
    panel
}

fn launch_size() -> (f32, f32) {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let scene = if arg == "--scene" {
            args.next()
        } else {
            arg.strip_prefix("--scene=").map(str::to_string)
        };
        if scene.as_deref() == Some("menu") {
            return (palette::WIDTH, palette::open_height());
        }
    }
    (palette::WIDTH, palette::TRAY_H)
}

fn main() {
    let size = launch_size();
    let spec = AppSpec {
        scenes: &[
            ("scanning", |s: &mut State| {
                s.catalog = catalog::pending();
                s.ticks.clear();
                s.open.clear();
            }),
            ("careful", |s| apply_level(s, Level::Careful)),
            ("balanced", |s| apply_level(s, Level::Balanced)),
            ("aggressive", |s| apply_level(s, Level::Aggressive)),
            ("asking", |s| {
                apply_level(s, Level::Balanced);
                s.agent = Some(agent::Job::preview(None));
            }),
            ("reviewed", |s| {
                apply_level(s, Level::Balanced);
                s.agent = Some(agent::Job::preview(Some(Ok("Review the selected caches first. Keep protected and in-use folders. Check the saved context and risks in Terminal before taking action.".into()))));
            }),
            ("agent-failed", |s| {
                apply_level(s, Level::Balanced);
                s.agent = Some(agent::Job::failed("Grok", "preview-only", "Agent ended without an answer (exit status: 2).\nunknown flag: --silent\nCheck the installed CLI version and login in Agent settings, then retry. Shelf did not change your selection."));
            }),
            ("agent-error-details", |s| {
                apply_level(s, Level::Balanced);
                s.agent = Some(agent::Job::failed("Grok", "preview-only", "Agent ended without an answer (exit status: 2).\nunknown flag: --silent\nCheck the installed CLI version and login in Agent settings, then retry. Shelf did not change your selection."));
                s.agent_details = true;
            }),
            ("confirmed", |s| {
                apply_level(s, Level::Aggressive);
                s.confirmed = true;
            }),
            ("menu", |s| {
                palette::open(s, None);
                palette::focus_ask(s);
            }),
            ("selected", |s| {
                apply_level(s, Level::Aggressive);
                if !s.ticks.is_empty() {
                    s.ticks[0] = Some(false);
                }
            }),
        ],
        poll: Some(poll),
        on_key: Some(on_key),
        ..AppSpec::new("Shelf", size, live_state, view)
    };
    let mut app = App::new(spec).accessory().configured_options(
        WindowOptions {
            transparent: true,
            borderless: true,
            always_on_top: true,
            resizable: false,
            min_size: Some((palette::WIDTH, 120.0)),
            max_size: Some((palette::WIDTH, palette::TRAY_H)),
            ..WindowOptions::default()
        }
        .macos(
            MacWindowOptions::default()
                .corner_radius(14.0)
                .shadow(true)
                .level(3),
        ),
    );
    let settings = WindowSpec::new("settings", settings::spec()).options(WindowOptions {
        visible: false,
        resizable: true,
        min_size: Some((640.0, 420.0)),
        ..WindowOptions::default()
    });
    let panel = app.handle();
    let (handle, _) = app.window(settings);
    settings::install(handle);
    status::arm(panel.clone(), palette::WIDTH);
    let mut started = false;
    app = app.on_ready(move |_| {
        if !started {
            started = true;
            start_workers(panel.clone());
        }
    });
    app.run();
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    fn ids(level: Level) -> Vec<String> {
        let mut state = fixture_state();
        apply_level(&mut state, level);
        ticked_ids(&state)
    }

    #[test]
    fn levels_tick_more_and_skip_the_safety_floor() {
        assert!(ids(Level::None).is_empty());
        assert_eq!(ids(Level::Careful), vec!["logs"]);
        assert_eq!(ids(Level::Balanced), vec!["logs", "spotify"]);
        // Build output was used under an hour ago, so Aggressive leaves it unticked.
        assert_eq!(ids(Level::Aggressive), vec!["logs", "deps", "spotify"]);
    }

    #[test]
    fn the_disk_bar_is_mostly_space_in_use() {
        let (red, yellow, green, gray) = catalog::bar_counts(494.35, 89.0, 70.0);
        assert_eq!(red + yellow + green + gray, catalog::SEGMENTS);
        assert!(red > yellow + green + gray);
        assert_eq!((red, yellow, green, gray), (28, 3, 3, 8));
    }

    fn runtime() -> Runtime<State> {
        let env = Env {
            system_dark: true,
            scale: 2.0,
        };
        let mut rt = Runtime::new(fixture_state(), view, (360.0, 980.0), env);
        rt.set_system_reduced_motion(true);
        rt.frame(0.0);
        rt
    }

    fn click(rt: &mut Runtime<State>, key: &str) {
        let (x, y) = rt.find(key).unwrap_or_else(|| panic!("missing {key}"));
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
    }

    fn leaf_at(state: &State, id: &str) -> usize {
        state
            .catalog
            .leaves
            .iter()
            .position(|leaf| leaf.id == id)
            .unwrap()
    }

    #[test]
    fn a_locked_row_stays_unticked_and_confirm_deletes_nothing() {
        let mut rt = runtime();
        assert!(rt.find("check-cache-held").is_none());
        assert!(rt.find("check-backups").is_none());
        assert!(rt.find("row-cache-held").is_some());
        assert!(rt.find("row-spotify").is_some());
        assert!(!is_ticked(&rt.state, leaf_at(&rt.state, "backups")));
        assert!(!is_ticked(&rt.state, leaf_at(&rt.state, "toolchains")));

        click(&mut rt, "level-menu");
        click(&mut rt, "level-menu");
        click(&mut rt, "level-menu");
        assert_eq!(rt.state.level, Level::Aggressive);
        assert_eq!(ticked_ids(&rt.state), vec!["logs", "deps", "spotify"]);
        assert!(!is_ticked(&rt.state, leaf_at(&rt.state, "build")));

        let leaves = rt.state.catalog.leaves.len();
        let groups = rt.state.catalog.groups.len();
        click(&mut rt, "confirm");
        assert!(rt.state.confirmed);
        assert_eq!(rt.state.catalog.leaves.len(), leaves);
        assert_eq!(rt.state.catalog.groups.len(), groups);
        assert_eq!(leaves, 7);
        assert_eq!(groups, 6);
        assert!(rt.find("row-backups").is_some());
        assert!(rt.find("row-cache-held").is_some());
        assert!(rt.find("row-spotify").is_some());
    }

    #[test]
    fn the_caches_check_ticks_only_spotify() {
        let mut rt = runtime();
        click(&mut rt, "check-caches");
        assert_eq!(ticked_ids(&rt.state), vec!["spotify"]);
        click(&mut rt, "expand-caches");
        assert!(rt.state.open.iter().all(|open| !*open));
        assert!(rt.find("row-spotify").is_none());
        assert!(rt.find("row-caches").is_some());
    }

    #[test]
    fn untick_holds_until_the_level_changes() {
        let mut rt = runtime();
        click(&mut rt, "level-menu");
        let logs = leaf_at(&rt.state, "logs");
        assert!(is_ticked(&rt.state, logs));
        click(&mut rt, "check-logs");
        assert!(!is_ticked(&rt.state, logs));
        click(&mut rt, "level-menu");
        assert_eq!(rt.state.level, Level::Balanced);
        assert!(is_ticked(&rt.state, logs));
    }

    #[test]
    fn the_live_index_feeds_this_mac() {
        let catalog = catalog::load_live();
        assert!(!catalog.missing, "{}", catalog.checked);
        assert!(catalog.disk_total > 0, "disk {}", catalog.disk_total);
        assert!(catalog.disk_free > 0, "free {}", catalog.disk_free);
        assert!(catalog.disk_free <= catalog.disk_total);
        assert!(catalog.memory <= 100 && catalog.cpu <= 100 && catalog.gpu <= 100);
        let commands = std::process::Command::new("ps")
            .args(["-ax", "-o", "command="])
            .output()
            .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
            .unwrap_or_default();
        for leaf in catalog
            .leaves
            .iter()
            .filter(|leaf| leaf.path.contains("smolvm"))
        {
            if commands.contains(&leaf.path) {
                assert!(leaf.locked, "{}", leaf.path);
            }
        }
    }

    #[test]
    fn ask_stays_closed_until_an_agent_is_chosen() {
        let mut rt = runtime();
        click(&mut rt, "actions");
        assert!(rt.find("action-ask").is_some());
        assert!(rt.find("action-unlock").is_some());
        assert!(rt.find("action-settings").is_some());
        click(&mut rt, "action-ask");
        assert!(!rt.state.palette);
        assert_eq!(rt.state.note, "Choose an agent in Settings.");
        assert!(rt.find("confirm").is_some());
        assert!(rt.find("action-ask").is_none());
    }

    #[test]
    fn agent_progress_and_results_leave_ticks_and_estimates_unchanged() {
        let mut rt = runtime();
        rt.update(|state, _| {
            apply_level(state, Level::Aggressive);
            let logs = leaf_at(state, "logs");
            state.ticks[logs] = Some(false);
            state.agent = Some(agent::Job::preview(None));
        });
        rt.frame(0.1);
        let before = ticked_ids(&rt.state);
        let bytes = ticked_bytes(&rt.state);
        assert!(rt.find("agent-progress").is_some());
        assert!(rt.find("agent-cancel").is_some());
        rt.update(|state, _| {
            state.agent = Some(agent::Job::preview(Some(Ok("Keep the held items.".into()))))
        });
        rt.frame(0.2);
        assert!(rt.find("agent-summary").is_some());
        assert!(rt.find("agent-terminal").is_some());
        assert!(rt.find("agent-cancel").is_none());
        assert_eq!(ticked_ids(&rt.state), before);
        assert_eq!(ticked_bytes(&rt.state), bytes);
        rt.update(|state, _| {
            state.agent = Some(agent::Job::preview(Some(Err(
                "Agent ended without an answer.\nunknown flag: --silent".into(),
            ))));
            state.prefs.agent_bin = "unsupported-test-agent".into();
            state.prefs.allow_start = true;
        });
        rt.frame(0.3);
        assert!(rt.find("agent-cancel").is_none());
        assert!(rt.find("agent-error-details").is_none());
        click(&mut rt, "agent-error-toggle");
        assert!(rt.find("agent-error-details").is_some());
        click(&mut rt, "agent-error-settings");
        assert_eq!(ticked_ids(&rt.state), before);
        click(&mut rt, "agent-retry");
        assert!(rt.find("agent-error-details").is_none());
        assert!(rt
            .state
            .agent
            .as_ref()
            .unwrap()
            .result
            .as_ref()
            .unwrap()
            .is_err());
        assert_eq!(ticked_ids(&rt.state), before);
        assert_eq!(ticked_bytes(&rt.state), bytes);
        click(&mut rt, "confirm");
        assert!(rt.state.confirmed);
        assert_eq!(ticked_ids(&rt.state), before);
    }

    #[test]
    fn ask_agent_uses_the_current_ticks_not_the_hovered_target() {
        let mut state = fixture_state();
        state.prefs.agent_bin = "grok".into();
        state.prefs.allow_start = true;
        apply_level(&mut state, Level::Aggressive);
        let logs = leaf_at(&state, "logs");
        let backups = leaf_at(&state, "backups");
        let deps = leaf_at(&state, "deps");
        state.ticks[logs] = Some(false);
        state.ticks[backups] = Some(true);
        state.target = Some(palette::Target::Leaf(backups));

        let prompt = palette::ask_prompt(&state).unwrap();
        assert!(prompt.contains("Selection mode: Selected"));
        assert!(prompt.contains("Selected items: 2, 4.01 GB total"));
        assert!(prompt.contains("acx node_modules | 3.30 GB | idle 2 h"));
        assert!(prompt.contains("Spotify cache | 709 MB | idle 48 h"));
        assert!(!prompt.contains("Logs and crash reports"));
        assert!(!prompt.contains("Backups and archives"));
        assert!(!prompt.contains("/fixture/"));
        assert!(prompt.contains("Ask for approval before deleting"));
        assert!(palette::ask_access(&state).selected.is_empty());

        state.prefs.send_paths = true;
        let prompt = palette::ask_prompt(&state).unwrap();
        assert!(prompt.contains("/fixture/deps\n/fixture/spotify"));
        assert!(!prompt.contains("/fixture/logs"));
        assert!(!prompt.contains("/fixture/backups"));
        assert_eq!(
            palette::ask_access(&state).selected,
            vec![
                std::path::PathBuf::from("/fixture/deps"),
                std::path::PathBuf::from("/fixture/spotify")
            ]
        );

        state
            .prefs
            .kept
            .push(state.catalog.leaves[deps].path.clone());
        let prompt = palette::ask_prompt(&state).unwrap();
        assert!(prompt.contains("Selected items: 1, 709 MB total"));
        assert!(!prompt.contains("acx node_modules"));
        assert_eq!(
            palette::ask_access(&state).selected,
            vec![std::path::PathBuf::from("/fixture/spotify")]
        );
        apply_level(&mut state, Level::None);
        assert_eq!(
            palette::ask_prompt(&state),
            Err("Select items to clean up first.")
        );
    }

    #[test]
    fn the_action_list_height_matches_its_rows() {
        let mut rt = runtime();
        rt.update(|state, _| palette::open(state, None));
        rt.frame(0.0);
        let search = rt.presented("action-search").expect("search").0;
        let bottom = search[1] + search[3];
        let want = palette::height(&rt.state);
        assert!((bottom - want).abs() < 1.0, "drawn {bottom} formula {want}");
        assert!((want - palette::open_height()).abs() < 1.0);
    }

    #[test]
    fn quit_requests_app_exit_from_actions_and_the_keyboard() {
        let mut rt = runtime();
        click(&mut rt, "actions");
        click(&mut rt, "action-quit");
        assert_eq!(rt.shell_request, Some(rune::Shell::Quit));

        for palette_open in [false, true] {
            let mut rt = runtime();
            rt.update(|state, _| state.palette = palette_open);
            rt.frame(0.0);
            rt.update(|state, tx| {
                on_key(
                    state,
                    &KeyEvent {
                        key: rune::Key::Command('q'),
                        mods: rune::Mods::default(),
                    },
                    tx,
                );
            });
            assert_eq!(rt.shell_request, Some(rune::Shell::Quit));
        }
    }

    #[test]
    fn a_secondary_click_opens_that_rows_menu() {
        let mut rt = runtime();
        let (x, y) = rt.find("row-caches").expect("caches row");
        rt.secondary_down(x, y);
        rt.secondary_up(x, y);
        rt.frame(0.0);
        assert!(rt.state.palette);
        assert_eq!(rt.state.target, Some(palette::Target::Group(5)));
        assert!(rt.find("action-finder").is_some());
    }

    #[test]
    fn unlock_then_tick_needs_a_gate_and_keep_holds_the_row() {
        let user = prefs::user_path();
        let before = std::fs::metadata(&user)
            .and_then(|meta| meta.modified())
            .ok();
        let mut state = fixture_state();
        {
            let leaf = &mut state.catalog.leaves[5];
            leaf.locked = true;
            leaf.balanced = Some(24);
            leaf.aggressive = Some(1);
            leaf.idle_h = 48;
        }
        palette::open(&mut state, Some(palette::Target::Leaf(5)));
        palette::run(&mut state, "unlock");
        assert!(catalog::can_tick(&leaf_view(&state, 5)));
        assert_eq!(leaf_view(&state, 5).detail, "Unlocked");
        palette::open(&mut state, Some(palette::Target::Leaf(5)));
        palette::run(&mut state, "select");
        palette::run(&mut state, "tick-item");
        assert!(is_ticked(&state, 5));

        palette::open(&mut state, Some(palette::Target::Leaf(3)));
        palette::run(&mut state, "unlock");
        assert!(!catalog::can_tick(&leaf_view(&state, 3)));
        assert!(!is_ticked(&state, 3));

        palette::open(&mut state, Some(palette::Target::Leaf(2)));
        palette::run(&mut state, "keep");
        assert_eq!(leaf_view(&state, 2).detail, "Shelf keeps this");
        assert!(!catalog::can_tick(&leaf_view(&state, 2)));
        let saved = prefs::load_from(&state.prefs_path);
        assert!(saved.kept.iter().any(|path| path == "/fixture/logs"));
        let after = std::fs::metadata(&user)
            .and_then(|meta| meta.modified())
            .ok();
        assert_eq!(before, after);
        let _ = std::fs::remove_file(&state.prefs_path);
    }

    #[test]
    fn the_cleanup_line_names_two_groups_and_counts_the_rest() {
        let mut state = fixture_state();
        apply_level(&mut state, Level::Careful);
        assert_eq!(
            cleanup_line(&state),
            "Your cleanup frees 5.70 GB: Logs and crash reports."
        );
        assert_eq!(level_label(&state), "Careful");
        apply_level(&mut state, Level::Aggressive);
        assert_eq!(
            cleanup_line(&state),
            "Your cleanup frees 9.71 GB: Logs and crash reports, Project dependencies and 1 more."
        );
        state.ticks[2] = Some(false);
        assert_eq!(level_label(&state), "Selected");
        apply_level(&mut state, Level::None);
        assert_eq!(level_label(&state), "None");
        assert!(cleanup_line(&state).is_empty());
    }

    #[test]
    fn always_protected_ignores_unlock_and_a_recent_hold_can_be_unlocked() {
        let mut state = fixture_state();
        state.catalog.leaves[2].path = prefs::expand_home("~/Library/Mail/V10");
        palette::open(&mut state, Some(palette::Target::Leaf(2)));
        palette::run(&mut state, "unlock");
        let leaf = leaf_view(&state, 2);
        assert!(!catalog::can_tick(&leaf), "{}", leaf.detail);
        assert_eq!(leaf.detail, "Always protected · Mail");
        let _ = std::fs::remove_file(&state.prefs_path);

        let mut state = fixture_state();
        state.prefs.keep_hours = 3;
        let held = leaf_view(&state, 1);
        assert_eq!(held.detail, "Changed within 3 h");
        assert!(!catalog::can_tick(&held));
        palette::open(&mut state, Some(palette::Target::Leaf(1)));
        palette::run(&mut state, "unlock");
        assert!(catalog::can_tick(&leaf_view(&state, 1)));
        assert_eq!(leaf_view(&state, 1).detail, "Unlocked");

        state.catalog.leaves[4].path = "/work/app/node_modules".into();
        state.prefs.kept = vec!["/work/app".into()];
        assert_eq!(leaf_view(&state, 4).detail, "Shelf keeps this");
        assert!(!catalog::can_tick(&leaf_view(&state, 4)));
        let _ = std::fs::remove_file(&state.prefs_path);
    }
}
