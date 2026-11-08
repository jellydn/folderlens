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
mod update;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use catalog::{Catalog, Level};
use rune::tok::{type_, TextStyle};
use rune::{
    canvas, col, el, row, text, Align, App, AppSpec, Color, Cx, El, KeyEvent, LineCap,
    MacWindowOptions, ScrollSpec, Shape, Transition, Tx, WindowOptions, WindowSpec,
};

const PANEL: &str = "#17242b";
const INK: &str = "#edf4f3";
const MUTED: &str = "#a6bbc2";
const LINE: &str = "#ffffff";
const BLUE: &str = "#79d8cb";
const RED: &str = "#ff453a";
const TRACK: &str = "#2b4049";
const MIN_PANEL_HEIGHT: f32 = 280.0;

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
    access_details: bool,
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
        access_details: false,
        paused_until: 0.0,
        window_h: palette::TRAY_H,
    }
}

fn live_state() -> State {
    if rune::is_headless() {
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            let scene = if arg == "--scene" {
                args.next()
            } else {
                arg.strip_prefix("--scene=").map(str::to_string)
            };
            if scene.is_some_and(|name| name.ends_with("-example") || name == "agent-error-details")
            {
                return state_from(catalog::fixture());
            }
        }
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
        s.confirmed = false;
        changed = true;
    }
    if let Some(job) = &s.agent {
        job.set_access(palette::ask_access(s));
        if !s.prefs.allow_start || s.prefs.agent_bin != job.bin || s.prefs.model() != job.model {
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
        "Selected footprint {}",
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

fn check_mark() -> Shape {
    let ink = paint(PANEL);
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

fn status_line(s: &State) -> El {
    let ink = paint(INK);
    if !s.note.is_empty() {
        return text(s.note.clone())
            .key("status")
            .font(TextStyle::new(13.0, 600.0).lh(18.0))
            .color(ink)
            .wrap()
            .w_full();
    }
    if s.catalog.missing {
        return text(s.catalog.checked.clone())
            .key("status")
            .font(TextStyle::new(13.0, 600.0).lh(18.0))
            .color(ink)
            .wrap()
            .w_full();
    }
    let bytes = ticked_bytes(s);
    if bytes == 0 {
        text("Nothing selected. Choose a preset or select an eligible folder.")
            .key("status")
            .font(TextStyle::new(13.0, 400.0).lh(18.0))
            .color(paint(MUTED))
            .wrap()
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
    let detail = group_reason(s, index);
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
        .min_h(52.0)
        .px(8.0)
        .py(8.0)
        .gap(8.0)
        .center_y()
        .child(lead)
        .child(
            col()
                .grow(1.0)
                .gap(1.0)
                .min_w(0.0)
                .child(text(name).font(type_::BODY).color(paint(INK)).truncate())
                .child(
                    text(detail)
                        .font(type_::LABEL)
                        .color(paint(MUTED))
                        .wrap()
                        .w_full(),
                ),
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
    let detail = selection_reason(s, index);
    let size = s.catalog.leaves[index].size_label.clone();
    let locked = !catalog::can_tick(&leaf);
    let lead = if locked {
        lock_icon()
    } else {
        let on = is_ticked(s, index);
        checkbox(format!("check-{id}"), on, move |state| {
            toggle_leaf(state, index)
        })
    };
    let line = row()
        .key(format!("row-{id}"))
        .w_full()
        .min_h(58.0)
        .py(6.0)
        .pl(18.0)
        .pr(8.0)
        .gap(8.0)
        .center_y()
        .child(lead);
    line.child(
        col()
            .grow(1.0)
            .min_w(0.0)
            .gap(1.0)
            .child(text(name).font(type_::BODY).color(paint(INK)).truncate())
            .child(
                text(detail)
                    .font(type_::LABEL)
                    .color(paint(MUTED))
                    .wrap()
                    .w_full(),
            ),
    )
    .child(text(size).font(type_::LABEL).color(paint("#d1d1d6")))
    .on_secondary_press(move |state: &mut State, _, tx| {
        tx.instant();
        palette::open(state, Some(palette::Target::Leaf(index)));
        palette::fit(state, tx);
    })
}

fn group_reason(s: &State, index: usize) -> String {
    let group = &s.catalog.groups[index];
    if group.count == 1 {
        return selection_reason(s, group.first);
    }
    let excluded = leaf_range(s, index)
        .filter(|&leaf| !catalog::can_tick(&leaf_view(s, leaf)))
        .count();
    format!("{} folders · {excluded} excluded", group.count)
}

fn selection_reason(s: &State, index: usize) -> String {
    let leaf = leaf_view(s, index);
    if !catalog::can_tick(&leaf) {
        return format!("Excluded · {}", leaf.detail);
    }
    if let Some(on) = s.ticks[index] {
        return if on {
            "Selected by you"
        } else {
            "Excluded by you"
        }
        .into();
    }
    if is_ticked(s, index) {
        format!("{} preset · {}", level_name(s.level), leaf.detail)
    } else {
        format!("Not in this preset · {}", leaf.detail)
    }
}

fn view(s: &State, cx: &mut Cx<State>) -> El {
    // Publish each rendered selection change without waiting for the next polling interval.
    if let Some(job) = &s.agent {
        job.set_access(palette::ask_access(s));
    }
    if s.palette {
        return palette::panel(s);
    }
    let compact = cx.layout_size("ledger").is_some_and(|size| size[0] < 620.0);
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
    let candidates = col()
        .key("candidates")
        .grow(1.0)
        .min_w(0.0)
        .gap(8.0)
        .child(
            row()
                .w_full()
                .h(36.0)
                .center_y()
                .child(
                    text("Folders")
                        .font(TextStyle::new(13.0, 600.0))
                        .color(paint(INK))
                        .grow(1.0),
                )
                .child(
                    row()
                        .key("level-menu")
                        .h(24.0)
                        .px(8.0)
                        .gap(4.0)
                        .center_y()
                        .radius(6.0)
                        .fill(paint(TRACK))
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
        .child(list);
    let receipt = agent_help(s).key("receipt").min_w(0.0);
    let workspace = if compact {
        col().w_full().gap(20.0).child(receipt).child(candidates)
    } else {
        row()
            .w_full()
            .gap(24.0)
            .child(candidates)
            .child(receipt.w(260.0))
    };
    col()
        .key("ledger")
        .w_full()
        .h_full()
        .fill(paint(PANEL))
        .radius(14.0)
        .transition(Transition::none())
        .child(
            row()
                .w_full()
                .h(56.0)
                .px(18.0)
                .gap(12.0)
                .center_y()
                .window_drag()
                .child(
                    text("Shelf")
                        .font(TextStyle::new(16.0, 500.0))
                        .color(paint(INK))
                        .grow(1.0)
                        .min_w(0.0)
                        .truncate(),
                )
                .child(
                    text("Agent")
                        .key("agent-reveal")
                        .font(type_::BODY)
                        .color(paint(BLUE))
                        .on_press_tx(|state: &mut State, tx: &mut Tx| {
                            tx.scroll_into_view(if state.agent.is_some() {
                                "agent-review"
                            } else {
                                "ask-agent"
                            });
                        }),
                )
                .child(
                    text("Settings")
                        .key("open-settings")
                        .font(type_::BODY)
                        .color(paint(BLUE))
                        .on_press(|_: &mut State| settings::open(settings::Page::Agent)),
                )
                .child(
                    text("Actions")
                        .key("actions")
                        .font(type_::BODY)
                        .color(paint(BLUE))
                        .on_press_tx(|state: &mut State, tx: &mut Tx| {
                            tx.instant();
                            palette::open(state, None);
                            palette::fit(state, tx);
                        }),
                )
                .child(
                    text("Close")
                        .key("close-panel")
                        .font(type_::LABEL)
                        .color(paint(MUTED))
                        .h(32.0)
                        .center_y()
                        .on_press(|_: &mut State| status::conceal()),
                ),
        )
        .child(rule())
        .child(
            col()
                .w_full()
                .padding(18.0)
                .gap(16.0)
                .child(status_line(s))
                .child(workspace)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .grow(1.0)
                .min_h(0.0),
        )
        .child(rule())
        .child(record_choice(s))
        .child(
            text(system_readings(s))
                .key("system-readings")
                .font(type_::LABEL)
                .color(paint(MUTED))
                .px(18.0)
                .py(10.0)
                .wrap()
                .w_full(),
        )
}

fn agent_help(s: &State) -> El {
    col().w_full().gap(10.0)
        .child(text(if s.agent.is_some() { "Agent review" } else { "Agent help" }).font(TextStyle::new(14.0, 500.0)).color(paint(INK)))
        .child(text(if !s.prefs.allow_start { "Set up an agent to request a review." }
            else if s.prefs.send_paths { "Eligible selected folder paths may be shared." }
            else { "Saved context only. Folder paths are not shared." })
            .key("access-summary").font(type_::LABEL).color(paint(MUTED)).wrap().w_full())
        .child(if s.agent.as_ref().is_some_and(|job| job.result.is_none()) { el() } else {
            text("Ask Agent").key("ask-agent").font(type_::BODY).color(paint(BLUE))
                .disabled(!(0..s.catalog.leaves.len()).any(|i| is_ticked(s, i))).on_press(palette::ask)
        })
        .child(text("Manual request. Provider costs may apply.").font(type_::LABEL).color(paint(MUTED)).wrap().w_full())
        .child(text("Agent setup").key("agent-configure").font(type_::LABEL).color(paint(BLUE))
            .on_press(|_: &mut State| settings::open(settings::Page::Agent)))
        .child(agent_review(s))
        .child(if s.agent.as_ref().is_some_and(|job| matches!(job.result, Some(Ok(_)))) { text("Continue saved chat in Terminal").key("agent-terminal").font(type_::LABEL).color(paint(BLUE)).wrap().w_full()
            .on_press(|state: &mut State| { if let Some(job) = &state.agent { if let Err(error) = job.open_chat(&state.prefs) { state.note = error; } } }) } else { el() })
        .child(if s.agent.is_some() { text("Run files in Finder").key("agent-files").font(type_::LABEL).color(paint(BLUE))
            .on_press(|state: &mut State| {
                if let Some(folder) = state.agent.as_ref().and_then(|job| job.script.parent()) {
                    state.note = match std::process::Command::new("open").arg(folder).spawn() {
                        Ok(_) => "Requested run folder in Finder. Shared paths may be saved there.".into(),
                        Err(_) => "Could not open the run folder in Finder.".into(),
                    };
                }
            }) } else { el() })
        .child(text(if s.access_details { "Hide access and limits" } else { "Access and limits" }).key("access-details-toggle").font(type_::LABEL).color(paint(BLUE))
            .on_press(|state: &mut State| state.access_details = !state.access_details))
        .child(if s.access_details { col().w_full().gap(8.0)
            .child(text(format!("{} · {}", prefs::agent_name(&s.prefs.agent_id).unwrap_or("Custom / unconfigured agent"), if s.prefs.model().is_empty() { "CLI default model" } else { s.prefs.model() }))
                .key("agent-configuration").font(type_::LABEL).color(paint(INK)).wrap().w_full())
            .child(text("Manual request; provider costs and data policies apply. An answer does not prove cleanup is safe. Live reads require the original and current eligible selection. Grok/Antigravity Terminal tools use their native controls, not Shelf's live reader. Provider controls are not an OS sandbox.")
                .font(type_::LABEL).color(paint(MUTED)).wrap().w_full())
            .child(text("Review access settings").key("review-access").font(type_::LABEL).color(paint(BLUE))
                .on_press(|_: &mut State| settings::open(settings::Page::Access)))
        } else { el() })
}

fn record_choice(s: &State) -> El {
    let selected = (0..s.catalog.leaves.len())
        .filter(|&i| is_ticked(s, i))
        .count();
    let excluded = (0..s.catalog.leaves.len())
        .filter(|&i| !catalog::can_tick(&leaf_view(s, i)))
        .count();
    col()
        .key("record-choice")
        .w_full()
        .px(18.0)
        .pt(10.0)
        .gap(6.0)
        .child(
            row()
                .w_full()
                .gap(12.0)
                .center_y()
                .child(
                    text(format!(
                        "{selected} selected · {} estimated",
                        catalog::format_bytes(ticked_bytes(s))
                    ))
                    .font(type_::LABEL)
                    .color(paint(INK))
                    .wrap()
                    .grow(1.0)
                    .min_w(0.0)
                    .key("receipt-summary"),
                )
                .child(
                    text(if s.confirmed {
                        "Choice recorded"
                    } else {
                        "Record choice"
                    })
                    .key("confirm")
                    .font(TextStyle::new(13.0, 500.0))
                    .color(paint(if selected > 0 { BLUE } else { MUTED }))
                    .h(32.0)
                    .px(10.0)
                    .radius(6.0)
                    .fill(paint(TRACK))
                    .disabled(selected == 0)
                    .on_press(|state: &mut State| {
                        state.confirmed =
                            (0..state.catalog.leaves.len()).any(|i| is_ticked(state, i))
                    }),
                ),
        )
        .child(
            text(format!(
                "Nothing deleted · in memory only · {excluded} protected / unavailable"
            ))
            .font(type_::LABEL)
            .color(paint(MUTED))
            .wrap()
            .w_full(),
        )
}

fn system_readings(s: &State) -> String {
    let mut readings = vec![if s.catalog.disk_total == 0 {
        "Free space unavailable".into()
    } else {
        format!("{} free", catalog::format_bytes(s.catalog.disk_free))
    }];
    for (show, name, value) in [
        (s.prefs.gauge_memory, "Memory", s.catalog.memory),
        (s.prefs.gauge_cpu, "CPU", s.catalog.cpu),
        (s.prefs.gauge_gpu, "GPU", s.catalog.gpu),
    ] {
        if show {
            readings.push(format!("{name} {value}%"));
        }
    }
    readings.join(" · ")
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
                        .grow(1.0)
                        .min_w(0.0)
                        .wrap(),
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
        if matches!(scene.as_deref(), Some("menu" | "menu-example")) {
            return (palette::WIDTH, palette::open_height());
        }
    }
    (palette::WIDTH, palette::TRAY_H)
}

fn main() {
    if let Some(code) = agent::reader_helper() {
        std::process::exit(code);
    }
    let size = launch_size();
    let spec = AppSpec {
        scenes: &[
            ("ledger-example", |s: &mut State| {
                apply_level(s, Level::Balanced);
                s.note = "Example data · no disk scan or agent request".into();
            }),
            ("recorded-example", |s| {
                apply_level(s, Level::Balanced);
                s.note = "Example data · no disk scan or agent request".into();
                s.confirmed = true;
            }),
            ("menu-example", |s| palette::open(s, None)),
            ("asking-example", |s| {
                apply_level(s, Level::Balanced);
                s.agent = Some(agent::Job::preview(None));
            }),
            ("reviewed-example", |s| {
                apply_level(s, Level::Balanced);
                s.agent = Some(agent::Job::preview(Some(Ok("Example review: check rebuild costs first. Nothing was deleted. A model answer is not proof that cleanup is safe.".into()))));
            }),
            ("scanning", |s| {
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
            resizable: true,
            min_size: Some((360.0, MIN_PANEL_HEIGHT)),
            max_size: Some((960.0, 1000.0)),
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
        min_size: Some((360.0, 420.0)),
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
            update::start();
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

    fn runtime() -> Runtime<State> {
        let env = Env {
            system_dark: true,
            scale: 2.0,
        };
        let mut rt = Runtime::new(fixture_state(), view, (720.0, 718.0), env);
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
    fn quick_review_keeps_recording_visible_and_non_destructive() {
        for width in [360.0, 619.0, 620.0, 720.0] {
            let mut rt = runtime();
            rt.set_viewport((width, 718.0), 2.0);
            rt.frame(0.0);
            let (candidates, _) = rt.presented("candidates").unwrap();
            let (receipt, _) = rt.presented("receipt").unwrap();
            if width < 620.0 {
                assert!((receipt[0] - candidates[0]).abs() < 1.0);
                assert!(candidates[1] >= receipt[1] + receipt[3]);
            } else {
                assert!(receipt[0] >= candidates[0] + candidates[2]);
            }
            for key in ["receipt-summary", "access-summary", "confirm"] {
                let (rect, _) = rt.presented(key).unwrap();
                assert!(
                    rect[0] >= 0.0 && rect[0] + rect[2] <= width,
                    "{width}: {key}: {rect:?}"
                );
            }
            let (record_before, _) = rt.presented("confirm").unwrap();
            assert!(record_before[1] >= 56.0 && record_before[1] + record_before[3] <= 718.0);
            rt.wheel(width / 2.0, 350.0, -2000.0);
            rt.frame(0.0);
            assert_eq!(rt.presented("confirm").unwrap().0, record_before);
            click(&mut rt, "confirm");
            assert!(
                !rt.state.confirmed,
                "empty selection is not a recorded choice"
            );
            rt.update(|s, _| apply_level(s, Level::Balanced));
            rt.frame(0.0);
            let before = ticked_ids(&rt.state);
            click(&mut rt, "confirm");
            assert!(rt.state.confirmed);
            assert_eq!(ticked_ids(&rt.state), before);
            assert_eq!(rt.state.catalog.leaves.len(), 7);
            assert_eq!(ticked_bytes(&rt.state), 6_409_000_000);
            rt.wheel(width / 2.0, 350.0, 2000.0);
            rt.frame(0.0);
            click(&mut rt, "check-logs");
            assert!(!rt.state.confirmed);
            assert_eq!(ticked_bytes(&rt.state), 709_000_000);
            rt.update(|s, _| {
                s.agent = Some(agent::Job::failed(
                    "Example",
                    "preview-only",
                    &"Agent failed.\n".repeat(50),
                ));
                s.agent_details = true;
            });
            rt.frame(0.0);
            rt.wheel(width / 2.0, 350.0, -2000.0);
            rt.frame(0.0);
            click(&mut rt, "confirm");
            assert!(
                rt.state.confirmed,
                "expanded errors must not hide recording"
            );
            assert_eq!(ticked_bytes(&rt.state), 709_000_000);
        }
    }

    #[test]
    fn minimum_panel_height_keeps_recording_and_readings_visible() {
        for width in [360.0, 960.0] {
            let mut rt = runtime();
            rt.set_viewport((width, MIN_PANEL_HEIGHT), 2.0);
            rt.frame(0.0);
            for key in ["confirm", "system-readings"] {
                let rect = rt.presented(key).unwrap().0;
                assert!(
                    rect[1] >= 56.0 && rect[1] + rect[3] <= MIN_PANEL_HEIGHT,
                    "{width}: {key}: {rect:?}"
                );
            }
        }
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
        assert!(
            rt.find("agent-terminal").is_none(),
            "no saved chat before success"
        );
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
        assert!(
            rt.find("agent-terminal").is_none(),
            "a failed review has no resumable chat"
        );
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
    fn wrapped_group_reasons_fit_between_adjacent_rows() {
        let mut rt = runtime();
        rt.update(|s, _| {
            for leaf in &mut s.catalog.leaves {
                leaf.bytes = u64::MAX / 7;
                leaf.locked = true;
                leaf.detail = "Always protected · Cloud storage".into();
            }
        });
        for width in [619.0, 620.0, 720.0] {
            rt.set_viewport((width, 718.0), 2.0);
            rt.frame(0.0);
            rt.frame(0.0);
            let elements = rt.elements();
            for (i, group) in rt
                .state
                .catalog
                .groups
                .iter()
                .enumerate()
                .filter(|(_, g)| g.count == 1)
            {
                let row_key = format!("row-{}", group.id);
                let row = rt.presented(&row_key).unwrap().0;
                let reason = group_reason(&rt.state, i);
                let text = elements
                    .iter()
                    .find(|el| {
                        if el.text.as_deref() != Some(&reason) {
                            return false;
                        }
                        let mut parent = el.parent;
                        while let Some(index) = parent {
                            if elements[index].key.as_deref() == Some(&row_key) {
                                return true;
                            }
                            parent = elements[index].parent;
                        }
                        false
                    })
                    .unwrap()
                    .rect;
                assert!(
                    text[1] >= row[1] && text[1] + text[3] <= row[1] + row[3],
                    "{width}: {reason}: text {text:?}, row {row:?}"
                );
            }
        }
    }

    #[test]
    fn agent_controls_can_be_reached_without_scrolling_through_candidates() {
        for width in [360.0, 720.0] {
            let mut rt = runtime();
            rt.set_viewport((width, 718.0), 2.0);
            rt.update(|s, _| {
                s.open.fill(true);
                s.agent = Some(agent::Job::preview(None));
            });
            rt.frame(0.0);
            let (entry, _) = rt.presented("agent-reveal").expect("visible agent entry");
            assert!(entry[1] >= 0.0 && entry[1] + entry[3] < 718.0);
            click(&mut rt, "agent-reveal");
            let (progress, _) = rt.presented("agent-progress").unwrap();
            let (cancel, _) = rt.presented("agent-cancel").unwrap();
            let (footer, _) = rt.presented("system-readings").unwrap();
            assert!(
                progress[1] >= 56.0 && progress[1] + progress[3] <= footer[1],
                "{width}: {progress:?}"
            );
            assert!(
                cancel[1] >= 56.0 && cancel[1] + cancel[3] <= footer[1],
                "{width}: {cancel:?}"
            );
        }
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
    fn resized_actions_reveal_the_filter_and_keyboard_selection() {
        let mut rt = runtime();
        click(&mut rt, "actions");
        rt.set_viewport((360.0, 120.0), 2.0);
        rt.frame(0.0);
        rt.wheel(180.0, 60.0, -2000.0);
        rt.frame(0.0);
        let search = rt.presented("action-search").unwrap().0;
        assert!(
            search[1] >= 0.0 && search[1] + search[3] <= 120.0,
            "{search:?}"
        );
        for id in [
            "keep", "unlock", "select", "analyse", "ask", "pause", "settings", "close", "quit",
            "finder",
        ] {
            rt.update(|state, tx| {
                on_key(
                    state,
                    &KeyEvent {
                        key: rune::Key::Down,
                        mods: rune::Mods::default(),
                    },
                    tx,
                );
            });
            rt.frame(0.0);
            let rect = rt.presented(&format!("action-{id}")).unwrap().0;
            assert!(
                rect[1] >= 0.0 && rect[1] + rect[3] <= 120.0,
                "{id}: {rect:?}"
            );
        }
    }

    #[test]
    fn ledger_and_folder_actions_explain_current_selection() {
        let mut rt = runtime();
        rt.update(|state, _| {
            apply_level(state, Level::Balanced);
            state.ticks[2] = Some(false);
            state.ticks[4] = Some(true);
        });
        rt.frame(0.0);
        for reason in [
            "Excluded · Shelf leaves this",
            "Selected by you",
            "Excluded by you",
            "Balanced preset · idle 48 h",
            "Not in this preset · idle 0 h",
        ] {
            assert!(
                rt.elements()
                    .iter()
                    .any(|element| element.text.as_deref() == Some(reason)),
                "{reason}"
            );
        }
        rt.update(|state, _| palette::open(state, Some(palette::Target::Group(2))));
        rt.frame(0.0);
        assert!(rt
            .elements()
            .iter()
            .any(|element| element.text.as_deref() == Some("Excluded by you")));
        for (keep_spotify, reason) in [
            (false, "2 folders · 1 excluded"),
            (true, "2 folders · 2 excluded"),
        ] {
            rt.update(|state, _| {
                if keep_spotify {
                    state.prefs.kept.push("/fixture/spotify".into());
                }
                palette::open(state, Some(palette::Target::Group(5)));
            });
            rt.frame(0.0);
            assert!(
                rt.elements()
                    .iter()
                    .any(|element| element.text.as_deref() == Some(reason)),
                "{reason}"
            );
        }
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
            "Selected footprint 5.70 GB: Logs and crash reports."
        );
        assert_eq!(level_label(&state), "Careful");
        apply_level(&mut state, Level::Aggressive);
        assert_eq!(
            cleanup_line(&state),
            "Selected footprint 9.71 GB: Logs and crash reports, Project dependencies and 1 more."
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
