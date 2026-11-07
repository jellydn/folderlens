//! The action list for one row. The window shrinks to the list. Confirm still deletes nothing.

use std::process::{Command, Stdio};

use rune::tok::{type_, TextStyle};
use rune::{col, row, text, El, Key, KeyEvent, Transition, Tx};

use super::{leaf_view, paint, rule, unix_now, State, INK, MUTED, PANEL, TRACK};
use crate::catalog;
use crate::prefs;
use crate::settings::{self, Page};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Group(usize),
    Leaf(usize),
}

struct Row {
    id: &'static str,
    title: String,
    shortcut: &'static str,
    enabled: bool,
}

pub fn open(s: &mut State, target: Option<Target>) {
    s.palette = true;
    s.select_open = false;
    s.query.clear();
    s.highlight = 0;
    s.target = target.or_else(|| s.catalog.groups.first().map(|_| Target::Group(0)));
}

pub fn focus_ask(s: &mut State) {
    if let Some(index) = rows(s).iter().position(|row| row.id == "ask") {
        s.highlight = index;
    }
}

pub const WIDTH: f32 = 360.0;
pub const TRAY_H: f32 = 718.0;

/// Header, rules, rows, and the search line. The list does not grow, so this is the window height.
pub fn height(s: &State) -> f32 {
    if !s.palette {
        return TRAY_H;
    }
    let shown = rows(s);
    let mut rules = 0usize;
    if !s.select_open {
        for action in &shown {
            if action.id == "settings" || action.id == "close" {
                rules += 1;
            }
        }
    }
    let n = shown.len();
    let items = n + rules;
    let gaps = items.saturating_sub(1) as f32 * 2.0;
    let list = 12.0 + n as f32 * 32.0 + rules as f32 * 9.0 + gaps;
    44.0 + 1.0 + list + 1.0 + 40.0
}

pub fn fit(s: &mut State, tx: &mut Tx) {
    let h = height(s);
    if (s.window_h - h).abs() < 0.5 {
        return;
    }
    s.window_h = h;
    tx.resize_window_instant((WIDTH, h));
}

/// The full action list, before a search filter or the Select submenu.
pub fn open_height() -> f32 {
    let n = 9.0;
    let rules = 2.0;
    let gaps = (n + rules - 1.0) * 2.0;
    let list = 12.0 + n * 32.0 + rules * 9.0 + gaps;
    44.0 + 1.0 + list + 1.0 + 40.0
}

pub fn on_key(s: &mut State, ev: &KeyEvent, _tx: &mut Tx) -> bool {
    if !s.palette {
        if ev.is_command(',') {
            settings::open(Page::General);
            return true;
        }
        if ev.is_command('j') {
            _tx.instant();
            open(s, s.target);
            focus_ask(s);
            fit(s, _tx);
            return true;
        }
        return false;
    }
    _tx.instant();
    let command = if ev.is_command('d') {
        Some("keep")
    } else if ev.is_command('u') {
        Some("unlock")
    } else if ev.is_command('r') {
        Some("analyse")
    } else if ev.is_command('j') {
        Some("ask")
    } else if ev.is_command('p') {
        Some("pause")
    } else if ev.is_command(',') {
        Some("settings")
    } else {
        None
    };
    if let Some(id) = command {
        act(s, id);
        fit(s, _tx);
        return true;
    }
    match &ev.key {
        Key::Escape => close(s),
        Key::Enter if ev.mods.cmd => {
            act(s, "finder");
        }
        Key::Enter => activate(s),
        Key::Up => move_highlight(s, -1),
        Key::Down => move_highlight(s, 1),
        Key::Backspace => {
            s.query.pop();
            s.highlight = 0;
        }
        Key::Text(typed) if !ev.mods.cmd && !ev.mods.ctrl => {
            if typed.chars().all(|c| !c.is_control()) && s.query.len() < 40 {
                s.query.push_str(typed);
                s.highlight = 0;
            }
        }
        _ => {}
    }
    fit(s, _tx);
    true
}

pub fn panel(s: &State) -> El {
    let shown = rows(s);
    let mut list = col().w_full().px(8.0).py(6.0).gap(2.0);
    for (index, action) in shown.iter().enumerate() {
        if !s.select_open && (action.id == "settings" || action.id == "close") {
            list = list.child(rule().my(4.0));
        }
        list = list.child(action_row(s, action, index));
    }
    let query = if s.query.is_empty() {
        text("Search for actions...")
            .font(type_::BODY)
            .color(paint(MUTED))
    } else {
        text(s.query.clone()).font(type_::BODY).color(paint(INK))
    };
    col()
        .key("palette")
        .w_full()
        .fill(paint(PANEL))
        .radius(14.0)
        .transition(Transition::none())
        .child(
            row()
                .w_full()
                .h(44.0)
                .px(16.0)
                .center_y()
                .window_drag()
                .child(
                    text(target_name(s))
                        .font(TextStyle::new(15.0, 600.0))
                        .color(paint(INK)),
                ),
        )
        .child(rule())
        .child(list)
        .child(rule())
        .child(
            row()
                .key("action-search")
                .w_full()
                .h(40.0)
                .px(16.0)
                .center_y()
                .child(query),
        )
}

fn action_row(s: &State, action: &Row, index: usize) -> El {
    let id = action.id;
    let on = index == s.highlight;
    let ink = if action.enabled { INK } else { MUTED };
    let mut line = row()
        .key(format!("action-{id}"))
        .w_full()
        .h(32.0)
        .px(8.0)
        .radius(8.0)
        .center_y()
        .transition(Transition::none());
    if on {
        line = line.fill(paint(TRACK));
    }
    // A press that swaps the tray fades the new rows in unless this commit is instant.
    let mut line = line
        .on_press_tx(move |state: &mut State, tx: &mut Tx| {
            tx.instant();
            state.highlight = index;
            run(state, id);
            fit(state, tx);
        })
        .child(
            text(action.title.clone())
                .font(type_::BODY)
                .color(paint(ink))
                .grow(1.0),
        );
    if show_shortcut(s, action.shortcut) {
        line = line.child(text(action.shortcut).font(type_::LABEL).color(paint(MUTED)));
    }
    line
}

fn show_shortcut(s: &State, shortcut: &str) -> bool {
    if shortcut.is_empty() {
        return false;
    }
    shortcut == "›" || s.prefs.shortcut_hints
}

fn rows(s: &State) -> Vec<Row> {
    let query = s.query.trim().to_lowercase();
    let all = if s.select_open {
        select_rows(s)
    } else {
        action_rows(s)
    };
    if query.is_empty() {
        all
    } else {
        all.into_iter()
            .filter(|row| row.title.to_lowercase().contains(&query))
            .collect()
    }
}

fn action_rows(s: &State) -> Vec<Row> {
    let path = !target_indexes(s).is_empty();
    vec![
        item("finder", "Show in Finder", "⌘↩", path),
        item("keep", "Always keep", "⌘D", path),
        item("unlock", "Unlock", "⌘U", path),
        item("select", "Select", "›", s.target.is_some()),
        item("analyse", "Analyse now", "⌘R", path),
        Row {
            id: "ask",
            title: ask_title(s),
            shortcut: "⌘J",
            enabled: true,
        },
        item("pause", "Pause alerts for 1 hour", "⌘P", true),
        item("settings", "Settings…", "⌘,", true),
        item("close", "Close", "esc", true),
    ]
}

fn select_rows(s: &State) -> Vec<Row> {
    let item_on = target_indexes(s)
        .iter()
        .any(|&index| catalog::can_tick(&leaf_view(s, index)));
    let group_on = group_indexes(s)
        .iter()
        .any(|&index| catalog::can_tick(&leaf_view(s, index)));
    vec![
        item("tick-item", "Tick this item", "", item_on),
        item("tick-group", "Tick the group", "", group_on),
        item("clear", "Clear ticks", "", true),
        item("back", "Back", "", true),
    ]
}

fn item(id: &'static str, title: &str, shortcut: &'static str, enabled: bool) -> Row {
    Row {
        id,
        title: title.to_string(),
        shortcut,
        enabled,
    }
}

fn ask_title(s: &State) -> String {
    if s.prefs.agent_bin.is_empty() {
        return "Ask your agent".to_string();
    }
    match prefs::agent_name(&s.prefs.agent_id) {
        Some(name) if s.prefs.agent_id != "custom" => format!("Ask {name}"),
        _ => "Ask your agent".to_string(),
    }
}

fn act(s: &mut State, id: &str) -> bool {
    run(s, id);
    true
}

fn activate(s: &mut State) {
    let shown = rows(s);
    if shown.is_empty() {
        return;
    }
    let index = s.highlight.min(shown.len() - 1);
    run(s, shown[index].id);
}

fn move_highlight(s: &mut State, delta: isize) {
    let count = rows(s).len();
    if count == 0 {
        s.highlight = 0;
        return;
    }
    let next = s.highlight as isize + delta;
    let wrapped = if next < 0 {
        count - 1
    } else {
        next as usize % count
    };
    s.highlight = wrapped;
}

pub fn run(s: &mut State, id: &str) {
    let enabled = if s.select_open {
        select_rows(s)
    } else {
        action_rows(s)
    }
    .iter()
    .any(|row| row.id == id && row.enabled);
    if !enabled {
        return;
    }
    match id {
        "finder" => reveal(s),
        "keep" => keep(s),
        "unlock" => unlock(s),
        "select" => {
            s.select_open = true;
            s.highlight = 0;
            s.query.clear();
        }
        "analyse" => analyse(s),
        "ask" => ask(s),
        "pause" => {
            s.paused_until = unix_now() + 3600.0;
            s.note = "Alerts paused for 1 hour.".to_string();
            close(s);
        }
        "settings" => {
            settings::open(Page::General);
            close(s);
        }
        "close" | "back" => {
            if id == "back" {
                s.select_open = false;
                s.highlight = 0;
            } else {
                close(s);
            }
        }
        "tick-item" => tick(s, target_indexes(s)),
        "tick-group" => tick(s, group_indexes(s)),
        "clear" => {
            for index in group_indexes(s) {
                s.ticks[index] = Some(false);
            }
            s.confirmed = false;
            close(s);
        }
        _ => {}
    }
}

fn reveal(s: &mut State) {
    if let Some(path) = first_path(s) {
        let _ = Command::new("open")
            .args(["-R", &path])
            .stdin(Stdio::null())
            .spawn();
        s.note = "Opened in Finder.".to_string();
    }
    close(s);
}

fn keep(s: &mut State) {
    for path in paths(s) {
        if !s.prefs.kept.iter().any(|kept| kept == &path) {
            s.prefs.kept.push(path);
        }
    }
    prefs::save_to(&s.prefs_path, &s.prefs);
    s.note = "Shelf keeps this.".to_string();
    close(s);
}

fn unlock(s: &mut State) {
    for path in paths(s) {
        s.prefs.kept.retain(|kept| kept != &path);
        if !s.unlocked.iter().any(|item| item == &path) {
            s.unlocked.push(path);
        }
    }
    prefs::save_to(&s.prefs_path, &s.prefs);
    s.note = "Unlocked.".to_string();
    close(s);
}

fn analyse(s: &mut State) {
    let name = target_name(s);
    for index in target_indexes(s) {
        catalog::remeasure(&mut s.catalog.leaves[index]);
    }
    s.note = format!("Measured {name}.");
    close(s);
}

fn ask(s: &mut State) {
    let name = target_name(s);
    let path = first_path(s).unwrap_or_default();
    let detail = match s.target {
        Some(Target::Leaf(index)) if index < s.catalog.leaves.len() => leaf_view(s, index).detail,
        Some(Target::Group(index)) => s
            .catalog
            .groups
            .get(index)
            .map(|group| group.detail.clone())
            .unwrap_or_default(),
        _ => String::new(),
    };
    match prefs::prepare_ask(&s.prefs, &name, &path, &detail) {
        Ok(prompt) => match prefs::launch(&s.prefs.agent_bin, &prompt) {
            Ok(()) => {
                let who = prefs::agent_name(&s.prefs.agent_id).unwrap_or("your agent");
                s.note = format!("Asked {who}.");
            }
            Err(err) => s.note = err,
        },
        Err(message) => {
            s.note = message.to_string();
            let page = if message.starts_with("Choose") {
                Page::Agent
            } else {
                Page::Access
            };
            settings::open(page);
        }
    }
    close(s);
}

fn tick(s: &mut State, indexes: Vec<usize>) {
    for index in indexes {
        if catalog::can_tick(&leaf_view(s, index)) {
            s.ticks[index] = Some(true);
        }
    }
    s.confirmed = false;
    close(s);
}

fn close(s: &mut State) {
    s.palette = false;
    s.select_open = false;
    s.query.clear();
}

fn target_name(s: &State) -> String {
    match s.target {
        Some(Target::Group(index)) => s
            .catalog
            .groups
            .get(index)
            .map(|group| group.name.clone())
            .unwrap_or_else(|| "Shelf".to_string()),
        Some(Target::Leaf(index)) => s
            .catalog
            .leaves
            .get(index)
            .map(|leaf| leaf.name.clone())
            .unwrap_or_else(|| "Shelf".to_string()),
        None => "Shelf".to_string(),
    }
}

fn first_path(s: &State) -> Option<String> {
    target_indexes(s)
        .first()
        .map(|index| s.catalog.leaves[*index].path.clone())
}

fn paths(s: &State) -> Vec<String> {
    target_indexes(s)
        .into_iter()
        .map(|index| s.catalog.leaves[index].path.clone())
        .collect()
}

fn target_indexes(s: &State) -> Vec<usize> {
    match s.target {
        Some(Target::Leaf(index)) if index < s.catalog.leaves.len() => vec![index],
        Some(Target::Group(index)) => {
            let Some(group) = s.catalog.groups.get(index) else {
                return Vec::new();
            };
            (group.first..group.first + group.count)
                .filter(|index| *index < s.catalog.leaves.len())
                .collect()
        }
        _ => Vec::new(),
    }
}

/// The group that owns the target. Clear ticks uses this whole group.
fn group_indexes(s: &State) -> Vec<usize> {
    let group_index = match s.target {
        Some(Target::Group(index)) => Some(index),
        Some(Target::Leaf(leaf)) => s
            .catalog
            .groups
            .iter()
            .position(|group| (group.first..group.first + group.count).contains(&leaf)),
        None => None,
    };
    let Some(index) = group_index else {
        return Vec::new();
    };
    let Some(group) = s.catalog.groups.get(index) else {
        return Vec::new();
    };
    (group.first..group.first + group.count)
        .filter(|index| *index < s.catalog.leaves.len())
        .collect()
}
