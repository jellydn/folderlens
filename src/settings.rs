//! Settings window: the agent Ask starts, and the access that start needs.

use std::path::PathBuf;
use std::sync::OnceLock;

use rune::tok::{type_, TextStyle};
use rune::{
    col, el, row, text, text_input, AppHandle, AppSpec, Color, Cx, El, ScrollSpec, TextEdit, Tx,
};

use crate::prefs::{self, Agent, Prefs};

const PANEL: &str = "#1c1c1e";
const CARD: &str = "#2c2c2e";
const INK: &str = "#f5f5f7";
const MUTED: &str = "#98989d";
const BLUE: &str = "#0a84ff";
const GREEN: &str = "#30d158";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    General,
    Agent,
    Protected,
    Access,
}

pub struct Settings {
    pub prefs: Prefs,
    pub path: PathBuf,
    pub page: Page,
    pub custom: TextEdit,
}

pub fn spec() -> AppSpec<Settings> {
    AppSpec {
        scenes: &[
            ("general", |state| state.page = Page::General),
            ("agent", |state| state.page = Page::Agent),
            ("access", |state| state.page = Page::Access),
            ("protected", |state| state.page = Page::Protected),
        ],
        on_key: Some(|_, event, tx| {
            if event.is_command('q') {
                tx.quit();
                return true;
            }
            false
        }),
        ..AppSpec::new("Settings", (760.0, 560.0), state, view)
    }
}

pub fn state() -> Settings {
    let path = prefs::path_for_state();
    let prefs = prefs::load_from(&path);
    let mut custom = TextEdit::default();
    if prefs.agent_id == "custom" {
        custom.text = prefs.agent_bin.clone();
        custom.anchor = custom.text.len();
        custom.focus = custom.anchor;
    }
    Settings {
        prefs,
        path,
        page: Page::General,
        custom,
    }
}

static HANDLE: OnceLock<AppHandle<Settings>> = OnceLock::new();

pub fn install(handle: AppHandle<Settings>) {
    let _ = HANDLE.set(handle);
}

pub fn open(page: Page) {
    let Some(handle) = HANDLE.get() else {
        return;
    };
    // The panel and this window are never on screen together.
    crate::status::conceal();
    handle.update(move |state, _tx: &mut Tx| {
        state.page = page;
        state.prefs = prefs::load_from(&state.path);
    });
    handle.open();
}

pub fn hide() {
    if let Some(handle) = HANDLE.get() {
        handle.hide();
    }
}

fn persist(state: &Settings) {
    prefs::save_to(&state.path, &state.prefs);
}

fn paint(hex: &str) -> Color {
    rune::color::hex(hex).unwrap_or(Color::WHITE)
}

fn view(state: &Settings, _: &mut Cx<Settings>) -> El {
    row()
        .w_full()
        .h_full()
        .fill(paint(PANEL))
        .child(sidebar(state))
        .child(
            col()
                .grow(1.0)
                .shrink()
                .h_full()
                .min_w(0.0)
                .fill(paint("#232325"))
                .child(
                    page(state)
                        .scroll(ScrollSpec {
                            stick_bottom: false,
                        })
                        .grow(1.0)
                        .min_h(0.0)
                        .min_w(0.0)
                        .w_full(),
                ),
        )
}

fn sidebar(state: &Settings) -> El {
    let mut side = col().w(200.0).h_full().py(16.0).gap(2.0).child(
        text("Shelf")
            .font(TextStyle::new(13.0, 600.0))
            .color(paint(MUTED))
            .px(20.0)
            .h(28.0),
    );
    for (page, label, mark, tint) in [
        (Page::General, "General", "G", "#8e8e93"),
        (Page::Agent, "Agent", "A", BLUE),
        (Page::Protected, "Protected", "P", GREEN),
        (Page::Access, "Access", "K", GREEN),
    ] {
        let on = state.page == page;
        side = side.child(side_row(page, label, mark, tint, on));
    }
    side
}

fn side_row(
    page: Page,
    label: &'static str,
    mark: &'static str,
    tint: &'static str,
    on: bool,
) -> El {
    let key = match page {
        Page::General => "nav-general",
        Page::Agent => "nav-agent",
        Page::Protected => "nav-protected",
        Page::Access => "nav-access",
    };
    let mut line = row()
        .key(key)
        .h(32.0)
        .mx(8.0)
        .px(8.0)
        .gap(8.0)
        .radius(8.0)
        .center_y();
    if on {
        line = line.fill(paint(BLUE));
    }
    line.on_press(move |state: &mut Settings| state.page = page)
        .child(letter(mark, if on { "#ffffff" } else { tint }))
        .child(
            text(label)
                .font(type_::BODY)
                .color(paint(if on { "#ffffff" } else { INK })),
        )
}

fn letter(mark: &str, tint: &str) -> El {
    el().size(18.0, 18.0)
        .radius(5.0)
        .fill(paint(tint))
        .center()
        .child(
            text(mark)
                .font(TextStyle::new(11.0, 700.0))
                .color(if tint == "#ffffff" {
                    paint(BLUE)
                } else {
                    Color::WHITE
                }),
        )
}

fn page(state: &Settings) -> El {
    let body = match state.page {
        Page::General => general(state),
        Page::Agent => agent(state),
        Page::Protected => protected(state),
        Page::Access => access(state),
    };
    col().w_full().padding(24.0).gap(16.0).child(body)
}

fn heading(title: &str) -> El {
    text(title)
        .font(TextStyle::new(22.0, 650.0))
        .color(paint(INK))
}

fn section(title: &str) -> El {
    text(title).font(type_::LABEL).color(paint(MUTED)).px(4.0)
}

fn card(rows: El) -> El {
    col()
        .w_full()
        .radius(12.0)
        .fill(paint(CARD))
        .px(4.0)
        .py(4.0)
        .child(rows)
}

fn setting_row(label: &str, trailing: El) -> El {
    row()
        .w_full()
        .h(40.0)
        .px(12.0)
        .center_y()
        .child(text(label).font(type_::BODY).color(paint(INK)).grow(1.0))
        .child(trailing)
}

fn toggle(key: &'static str, on: bool, press: impl Fn(&mut Settings) + 'static) -> El {
    let knob = if on { 18.0 } else { 2.0 };
    el().key(key)
        .size(36.0, 22.0)
        .radius(11.0)
        .fill(paint(if on { GREEN } else { "#3a3a3c" }))
        .on_press(press)
        .child(
            el().size(18.0, 18.0)
                .radius(9.0)
                .fill(Color::WHITE)
                .absolute(knob, 2.0),
        )
}

fn general(state: &Settings) -> El {
    col()
        .w_full()
        .gap(14.0)
        .child(heading("General"))
        .child(section("Tray"))
        .child(card(
            col()
                .w_full()
                .child(setting_row(
                    "Show shortcut hints",
                    toggle("hint-toggle", state.prefs.shortcut_hints, |state| {
                        state.prefs.shortcut_hints = !state.prefs.shortcut_hints;
                        persist(state);
                    }),
                ))
                .child(setting_row(
                    "Memory gauge",
                    toggle("gauge-memory", state.prefs.gauge_memory, |state| {
                        state.prefs.gauge_memory = !state.prefs.gauge_memory;
                        persist(state);
                    }),
                ))
                .child(setting_row(
                    "CPU gauge",
                    toggle("gauge-cpu", state.prefs.gauge_cpu, |state| {
                        state.prefs.gauge_cpu = !state.prefs.gauge_cpu;
                        persist(state);
                    }),
                ))
                .child(setting_row(
                    "GPU gauge",
                    toggle("gauge-gpu", state.prefs.gauge_gpu, |state| {
                        state.prefs.gauge_gpu = !state.prefs.gauge_gpu;
                        persist(state);
                    }),
                )),
        ))
        .child(
            text("Confirm records the choice and deletes nothing. The tray stays dark.")
                .font(type_::LABEL)
                .color(paint(MUTED))
                .wrap()
                .w_full(),
        )
}

fn agent(state: &Settings) -> El {
    let found = prefs::agents();
    let mut list = col().w_full();
    if found.is_empty() {
        list = list.child(
            text("Shelf found no agent on PATH.")
                .font(type_::BODY)
                .color(paint(MUTED))
                .px(12.0)
                .h(40.0),
        );
    }
    for agent in found {
        list = list.child(agent_row(state, agent));
    }
    col()
        .w_full()
        .gap(14.0)
        .child(heading("Agent"))
        .child(
            text("Ask your agent starts this command.")
                .font(type_::LABEL)
                .color(paint(MUTED))
                .wrap()
                .w_full(),
        )
        .child(section("On this Mac"))
        .child(card(list))
        .child(section("Other command"))
        .child(
            row()
                .w_full()
                .gap(8.0)
                .center_y()
                .child(
                    text_input(&state.custom, TextStyle::new(13.0, 400.0))
                        .key("agent-command")
                        .grow(1.0)
                        .h(32.0)
                        .single_line()
                        .placeholder("Command on PATH", paint(MUTED))
                        .color(paint(INK))
                        .on_edit(|state: &mut Settings| &mut state.custom),
                )
                .child(
                    text("Use")
                        .key("use-command")
                        .font(TextStyle::new(13.0, 600.0))
                        .color(paint(BLUE))
                        .on_press(|state: &mut Settings| {
                            let bin = state.custom.text.trim().to_string();
                            if bin.is_empty() {
                                return;
                            }
                            state.prefs.agent_id = "custom".into();
                            state.prefs.agent_bin = bin;
                            persist(state);
                        }),
                ),
        )
}

fn agent_row(state: &Settings, agent: &Agent) -> El {
    let id = agent.id.to_string();
    let bin = agent.bin.to_string_lossy().into_owned();
    let on = state.prefs.agent_id == agent.id;
    let key = format!("agent-{}", agent.id);
    row()
        .key(key)
        .w_full()
        .h(40.0)
        .px(12.0)
        .gap(8.0)
        .center_y()
        .on_press(move |state: &mut Settings| {
            state.prefs.agent_id = id.clone();
            state.prefs.agent_bin = bin.clone();
            persist(state);
        })
        .child(radio(on))
        .child(
            text(agent.name)
                .font(type_::BODY)
                .color(paint(INK))
                .grow(1.0),
        )
        .child(
            text(agent.bin.to_string_lossy().into_owned())
                .font(type_::LABEL)
                .color(paint(MUTED))
                .truncate_middle(),
        )
}

fn radio(on: bool) -> El {
    let mut mark = el()
        .size(16.0, 16.0)
        .radius(8.0)
        .border(1.4, paint(if on { BLUE } else { "#636366" }));
    if on {
        mark = mark.child(
            el().size(8.0, 8.0)
                .radius(4.0)
                .fill(paint(BLUE))
                .absolute(4.0, 4.0),
        );
    }
    mark
}

fn protected(state: &Settings) -> El {
    let mut folders = col().w_full().child(
        row()
            .w_full()
            .h(40.0)
            .px(12.0)
            .center_y()
            .child(
                text("Protected folders")
                    .font(type_::BODY)
                    .color(paint(INK))
                    .grow(1.0),
            )
            .child(
                el().key("add-folder")
                    .size(28.0, 28.0)
                    .radius(6.0)
                    .center()
                    .on_press(add_folder)
                    .child(
                        text("+")
                            .font(TextStyle::new(18.0, 500.0))
                            .color(paint(INK)),
                    ),
            ),
    );
    let mut paths = col().w_full();
    for path in state.prefs.kept.clone() {
        let shown = path.clone();
        paths = paths.child(
            row()
                .w_full()
                .h(32.0)
                .px(12.0)
                .gap(8.0)
                .center_y()
                .child(
                    text(shown)
                        .font(type_::LABEL)
                        .color(paint(INK))
                        .grow(1.0)
                        .shrink()
                        .min_w(0.0)
                        .truncate_middle(),
                )
                .child(
                    text("Remove")
                        .key(format!("unkeep-{path}"))
                        .font(type_::LABEL)
                        .color(paint(BLUE))
                        .on_press(move |state: &mut Settings| {
                            state.prefs.kept.retain(|kept| kept != &path);
                            persist(state);
                        }),
                ),
        );
    }
    if !state.prefs.kept.is_empty() {
        folders = folders.child(
            paths
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .max_h(96.0)
                .w_full(),
        );
    }

    let hours = state.prefs.keep_hours;
    let mut always = col().w_full().child(
        text("Always protected")
            .font(TextStyle::new(13.0, 600.0))
            .color(paint(INK))
            .px(12.0)
            .h(32.0),
    );
    for (name, path) in prefs::ALWAYS {
        let key = format!("always-{}", always_key(name));
        always = always.child(
            row()
                .key(key)
                .w_full()
                .h(32.0)
                .px(12.0)
                .gap(8.0)
                .center_y()
                .child(lock_mark())
                .child(
                    text(*name)
                        .font(type_::BODY)
                        .color(paint(INK))
                        .grow(1.0)
                        .shrink()
                        .min_w(0.0),
                )
                .child(
                    text(*path)
                        .font(type_::LABEL)
                        .color(paint(MUTED))
                        .truncate_middle(),
                ),
        );
    }

    col()
        .w_full()
        .gap(14.0)
        .child(
            col()
                .w_full()
                .gap(10.0)
                .items_center()
                .child(shield())
                .child(
                    text("Protected")
                        .font(TextStyle::new(28.0, 650.0))
                        .color(paint(INK)),
                ),
        )
        .child(card(folders))
        .child(card(
            row()
                .w_full()
                .h(44.0)
                .px(12.0)
                .gap(8.0)
                .center_y()
                .child(
                    text("Keep what changed within")
                        .font(type_::BODY)
                        .color(paint(INK)),
                )
                .child(info_mark())
                .child(el().grow(1.0))
                .child(
                    text(format!("{hours} h"))
                        .key("keep-hours")
                        .font(type_::BODY)
                        .color(paint(INK)),
                )
                .child(col().child(hour_step("hour-up", "▴", 1)).child(hour_step(
                    "hour-down",
                    "▾",
                    -1,
                ))),
        ))
        .child(card(always))
}

fn add_folder(state: &mut Settings) {
    let Some(path) = prefs::choose_folder() else {
        return;
    };
    if prefs::always_name(&path).is_some() {
        return;
    }
    if state.prefs.kept.iter().any(|kept| kept == &path) {
        return;
    }
    state.prefs.kept.push(path);
    persist(state);
}

fn hour_step(key: &'static str, mark: &'static str, delta: i32) -> El {
    el().key(key)
        .size(22.0, 16.0)
        .center()
        .on_press(move |state: &mut Settings| {
            let next = state.prefs.keep_hours as i32 + delta;
            state.prefs.keep_hours = next.clamp(0, prefs::MAX_KEEP_HOURS as i32) as u32;
            persist(state);
        })
        .child(
            text(mark)
                .font(TextStyle::new(9.0, 700.0))
                .color(paint(MUTED)),
        )
}

fn always_key(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn shield() -> El {
    el().size(64.0, 64.0)
        .radius(16.0)
        .fill(paint(GREEN))
        .center()
        .child(lock_mark_on(Color::WHITE, paint(GREEN), 26.0))
}

fn lock_mark() -> El {
    lock_mark_on(paint(MUTED), paint(CARD), 14.0)
}

/// A ring with its lower half covered, so the mark reads as a lock shackle.
fn lock_mark_on(ink: Color, cover: Color, size: f32) -> El {
    let shackle_w = size * 0.48;
    let shackle_h = size * 0.46;
    el().size(size, size)
        .child(
            el().size(shackle_w, shackle_h)
                .radius(shackle_w * 0.5)
                .border(1.6, ink)
                .absolute((size - shackle_w) / 2.0, 0.0),
        )
        .child(
            el().size(shackle_w + 4.0, shackle_h * 0.55)
                .fill(cover)
                .absolute((size - shackle_w) / 2.0 - 2.0, shackle_h * 0.48),
        )
        .child(
            el().size(size * 0.7, size * 0.46)
                .radius(2.0)
                .fill(ink)
                .absolute(size * 0.15, size * 0.5),
        )
}

fn info_mark() -> El {
    el().size(14.0, 14.0)
        .radius(7.0)
        .border(1.0, paint(MUTED))
        .center()
        .child(
            text("i")
                .font(TextStyle::new(9.0, 700.0))
                .color(paint(MUTED)),
        )
}

fn access(state: &Settings) -> El {
    col()
        .w_full()
        .gap(14.0)
        .child(heading("Access"))
        .child(
            text("Shelf asks before it starts an agent or sends a folder path.")
                .font(type_::LABEL)
                .color(paint(MUTED))
                .wrap()
                .w_full(),
        )
        .child(section("Agent"))
        .child(card(
            col()
                .w_full()
                .child(setting_row(
                    "Shelf may start the agent",
                    toggle("allow-start", state.prefs.allow_start, |state| {
                        state.prefs.allow_start = !state.prefs.allow_start;
                        persist(state);
                    }),
                ))
                .child(setting_row(
                    "Send the folder path",
                    toggle("send-paths", state.prefs.send_paths, |state| {
                        state.prefs.send_paths = !state.prefs.send_paths;
                        persist(state);
                    }),
                )),
        ))
        .child(section("Folders"))
        .child(card(setting_row(
            "Full Disk Access",
            text("Open")
                .key("disk-access")
                .font(TextStyle::new(13.0, 600.0))
                .color(paint(BLUE))
                .on_press(|_state: &mut Settings| prefs::open_full_disk_access()),
        )))
        .child(
            text("macOS hides some folders until Full Disk Access is on. Shelf measures the rest of your home either way.")
                .font(type_::LABEL)
                .color(paint(MUTED))
                .wrap()
                .w_full(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gauge_toggle_writes_the_temp_file() {
        let state = state();
        assert!(state.path.starts_with(std::env::temp_dir()));
        assert_ne!(state.path, prefs::user_path());
        let mut state = state;
        state.prefs.gauge_cpu = false;
        persist(&state);
        let loaded = prefs::load_from(&state.path);
        assert!(!loaded.gauge_cpu);
        assert!(loaded.gauge_memory);
        let _ = std::fs::remove_file(&state.path);
    }

    #[test]
    fn access_controls_stay_inside_the_window() {
        use rune::{Env, Runtime};

        let mut state = state();
        state.page = Page::Access;
        let mut rt = Runtime::new(
            state,
            view,
            (760.0, 560.0),
            Env {
                system_dark: true,
                scale: 2.0,
            },
        );
        rt.frame(0.0);
        for key in ["allow-start", "send-paths", "disk-access"] {
            let (rect, opacity) = rt.presented(key).unwrap_or_else(|| panic!("missing {key}"));
            assert!(opacity > 0.5, "{key} opacity {opacity}");
            assert!(
                rect[0] >= 200.0 && rect[0] + rect[2] <= 760.0,
                "{key} {rect:?}"
            );
        }
    }

    #[test]
    fn the_protected_page_steps_the_hold_and_lists_the_locks() {
        use rune::{Env, Runtime};

        let mut state = state();
        state.page = Page::Protected;
        let path = state.path.clone();
        let mut rt = Runtime::new(
            state,
            view,
            (760.0, 560.0),
            Env {
                system_dark: true,
                scale: 2.0,
            },
        );
        rt.set_system_reduced_motion(true);
        rt.frame(0.0);
        rt.frame(0.3);
        for key in [
            "add-folder",
            "hour-up",
            "hour-down",
            "always-mail",
            "always-kubernetes",
        ] {
            let (rect, opacity) = rt.presented(key).unwrap_or_else(|| panic!("missing {key}"));
            assert!(opacity > 0.5, "{key} opacity {opacity}");
            assert!(
                rect[0] >= 200.0 && rect[0] + rect[2] <= 760.0,
                "{key} {rect:?}"
            );
        }
        let (x, y) = rt.find("hour-up").expect("hour-up");
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
        assert_eq!(rt.state.prefs.keep_hours, 1);
        assert_eq!(prefs::load_from(&path).keep_hours, 1);
        let (x, y) = rt.find("hour-down").expect("hour-down");
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
        assert_eq!(rt.state.prefs.keep_hours, 0);
        assert_eq!(prefs::load_from(&path).keep_hours, 0);
        let _ = std::fs::remove_file(&path);
    }
}
