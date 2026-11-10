//! Settings window: the agent Ask starts, and the access that start needs.

use std::path::PathBuf;
use std::sync::OnceLock;

use rune::tok::{type_, TextStyle};
use rune::{
    col, el, row, text, text_input, AppHandle, AppSpec, Color, Cx, El, ScrollSpec, TextEdit, Tx,
};

use crate::prefs::{self, Agent, Prefs};
use crate::{button, ButtonStyle, BLUE, INK, MUTED, PANEL, TRACK};

#[path = "settings_updates.rs"]
mod updates;

const CARD: &str = "#243740";
const GREEN: &str = BLUE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    General,
    Agent,
    Protected,
    Access,
    Updates,
}

pub struct Settings {
    pub prefs: Prefs,
    pub path: PathBuf,
    pub page: Page,
    pub custom: TextEdit,
    pub model: TextEdit,
    pub model_error: String,
    pub update_note: String,
    pub commands_open: bool,
}

pub fn spec() -> AppSpec<Settings> {
    AppSpec {
        scenes: &[
            ("general", |state| state.page = Page::General),
            ("agent", |state| state.page = Page::Agent),
            ("agent-copilot", |state| {
                preview_agent(state, "copilot", "auto")
            }),
            ("agent-pi", |state| {
                preview_agent(state, "pi", "openrouter/openrouter/free")
            }),
            ("agent-advanced", |state| {
                preview_agent(state, "pi", "openrouter/openrouter/free");
                state.commands_open = true;
            }),
            ("agent-opencode", |state| {
                preview_agent(state, "opencode", "opencode/glm-5.3-flash-free")
            }),
            ("agent-model-error", |state| {
                preview_agent(state, "pi", "unknown");
                save_model(state);
            }),
            ("access", |state| state.page = Page::Access),
            ("protected", |state| state.page = Page::Protected),
            ("protected-example", |state| {
                state.page = Page::Protected;
                state.prefs.kept = vec!["~/Projects/retained-work".into()];
                state.prefs.keep_hours = 24;
            }),
            ("updates", |state| state.page = Page::Updates),
            ("updates-preview", |state| {
                state.page = Page::Updates;
                state.prefs.update_downloads = true;
                state.prefs.update_prereleases = true;
            }),
        ],
        on_key: Some(|_, event, tx| {
            if event.is_command('q') {
                tx.quit();
                return true;
            }
            if matches!(event.key, rune::Key::Escape) {
                hide();
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
    let mut state = Settings {
        prefs,
        path,
        page: Page::General,
        custom,
        model: TextEdit::default(),
        model_error: String::new(),
        update_note: String::new(),
        commands_open: false,
    };
    load_model(&mut state);
    state
}

fn load_model(state: &mut Settings) {
    state.model = TextEdit::default();
    state.model.text = state.prefs.model().into();
    state.model.anchor = state.model.text.len();
    state.model.focus = state.model.anchor;
    state.model_error.clear();
}

fn preview_agent(state: &mut Settings, id: &str, model: &str) {
    state.page = Page::Agent;
    state.prefs.agent_id = id.into();
    state.prefs.agent_bin = id.into();
    state.prefs.models.insert(id.into(), model.into());
    load_model(state);
}

fn save_model(state: &mut Settings) {
    let model = state.model.text.trim();
    if let Err(error) = crate::agent::validate_model(&state.prefs.agent_bin, model) {
        state.model_error = error;
        return;
    }
    state
        .prefs
        .models
        .insert(state.prefs.model_key(), model.into());
    state.model_error.clear();
    persist(state);
}

static HANDLE: OnceLock<AppHandle<Settings>> = OnceLock::new();

pub fn install(handle: AppHandle<Settings>) {
    let _ = HANDLE.set(handle);
}

pub fn update_status(message: &'static str) {
    if let Some(handle) = HANDLE.get() {
        handle.update(move |state, _tx: &mut Tx| state.update_note = message.into());
    }
}

pub fn open(page: Page) {
    let Some(handle) = HANDLE.get() else {
        return;
    };
    // The panel and this window are never on screen together.
    crate::status::conceal();
    handle.update(move |state, tx: &mut Tx| {
        tx.instant();
        state.page = page;
        state.prefs = prefs::load_from(&state.path);
        load_model(state);
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
    col()
        .w_full()
        .h_full()
        .fill(paint(PANEL))
        .child(
            row()
                .w_full()
                .h(56.0)
                .px(24.0)
                .center_y()
                .child(
                    text("FolderLens settings")
                        .font(TextStyle::new(16.0, 500.0))
                        .color(paint(INK))
                        .grow(1.0),
                )
                .child(
                    button("Close", ButtonStyle::Quiet)
                        .key("close-settings")
                        .on_press(|_: &mut Settings| hide()),
                ),
        )
        .child(navigation(state))
        .child(
            page(state)
                .key("settings-page")
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .grow(1.0)
                .min_h(0.0)
                .min_w(0.0)
                .w_full(),
        )
        .child(crate::rule())
        .child(
            row()
                .w_full()
                .gap(12.0)
                .center_y()
                .px(24.0)
                .py(10.0)
                .child(
                    text(if state.page == Page::Agent {
                        "Switches save now. Model edits need Apply."
                    } else {
                        "Changes save automatically."
                    })
                    .font(type_::LABEL)
                    .color(paint(MUTED))
                    .wrap()
                    .grow(1.0)
                    .min_w(0.0),
                )
                .child(if state.page == Page::Agent {
                    button("Apply model", ButtonStyle::Primary)
                        .key("apply-model")
                        .on_press_tx(|state: &mut Settings, tx: &mut Tx| {
                            save_model(state);
                            if !state.model_error.is_empty() {
                                tx.scroll_into_view("model-error");
                            }
                        })
                } else {
                    el()
                }),
        )
}

fn navigation(state: &Settings) -> El {
    let mut tabs = row().w_full().px(16.0).gap(4.0);
    for (page, label) in [
        (Page::General, "General"),
        (Page::Agent, "Agent"),
        (Page::Protected, "Keep safe"),
        (Page::Access, "Access"),
        (Page::Updates, "Updates"),
    ] {
        let on = state.page == page;
        tabs = tabs.child(tab(page, label, on));
    }
    col().w_full().child(tabs).child(crate::rule())
}

fn tab(page: Page, label: &'static str, on: bool) -> El {
    let key = match page {
        Page::General => "nav-general",
        Page::Agent => "nav-agent",
        Page::Protected => "nav-protected",
        Page::Access => "nav-access",
        Page::Updates => "nav-updates",
    };
    col()
        .key(key)
        .grow(1.0)
        .min_w(0.0)
        .on_press_tx(move |state: &mut Settings, tx: &mut Tx| {
            tx.instant();
            state.page = page;
        })
        .child(
            text(label)
                .font(TextStyle::new(13.0, 600.0))
                .color(paint(if on { BLUE } else { MUTED }))
                .h(38.0)
                .center(),
        )
        .child(
            el().w_full()
                .h(2.0)
                .fill(paint(if on { BLUE } else { PANEL })),
        )
}

fn page(state: &Settings) -> El {
    let body = match state.page {
        Page::General => general(state),
        Page::Agent => agent(state),
        Page::Protected => protected(state),
        Page::Access => access(state),
        Page::Updates => updates::view(state),
    };
    col().w_full().padding(24.0).gap(16.0).child(body)
}

fn heading(title: &str) -> El {
    text(title)
        .font(TextStyle::new(18.0, 500.0))
        .color(paint(INK))
}

fn section(title: &str) -> El {
    text(title).font(type_::LABEL).color(paint(MUTED)).px(4.0)
}

fn card(rows: El) -> El {
    col()
        .w_full()
        .radius(6.0)
        .fill(paint(CARD))
        .px(4.0)
        .py(4.0)
        .child(rows)
}

fn setting_row(label: &str, trailing: El) -> El {
    row()
        .w_full()
        .min_h(44.0)
        .py(8.0)
        .px(12.0)
        .gap(16.0)
        .center_y()
        .child(
            text(label)
                .font(type_::BODY)
                .color(paint(INK))
                .grow(1.0)
                .min_w(0.0)
                .wrap(),
        )
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
                .fill(if on { paint(PANEL) } else { Color::WHITE })
                .absolute(knob, 2.0),
        )
}

fn general(state: &Settings) -> El {
    col()
        .w_full()
        .gap(14.0)
        .child(heading("Only the readings you need"))
        .child(text("Choose the readings shown with your selection. Estimates are not reclaimed space.").font(type_::LABEL).color(paint(MUTED)).wrap().w_full())
        .child(section("Readings and shortcuts"))
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
                    "Memory usage",
                    toggle("gauge-memory", state.prefs.gauge_memory, |state| {
                        state.prefs.gauge_memory = !state.prefs.gauge_memory;
                        persist(state);
                    }),
                ))
                .child(setting_row(
                    "CPU usage",
                    toggle("gauge-cpu", state.prefs.gauge_cpu, |state| {
                        state.prefs.gauge_cpu = !state.prefs.gauge_cpu;
                        persist(state);
                    }),
                ))
                .child(setting_row(
                    "GPU load",
                    toggle("gauge-gpu", state.prefs.gauge_gpu, |state| {
                        state.prefs.gauge_gpu = !state.prefs.gauge_gpu;
                        persist(state);
                    }),
                )),
        ))
        .child(
            text("Record choice keeps the selection in memory and deletes nothing. Readings appear below the review.")
                .font(type_::LABEL)
                .color(paint(MUTED))
                .wrap()
                .w_full(),
        )
}

fn agent(state: &Settings) -> El {
    col()
        .w_full()
        .gap(14.0)
        .child(heading("Set up your agent"))
        .child(
            text("Manual requests only. Provider costs and data policies apply.")
                .font(type_::LABEL)
                .color(paint(MUTED))
                .wrap()
                .w_full(),
        )
        .child(card(
            col()
                .w_full()
                .padding(8.0)
                .gap(6.0)
                .child(
                    text(
                        prefs::agent_name(&state.prefs.agent_id)
                            .unwrap_or("Custom / unconfigured agent"),
                    )
                    .font(type_::BODY)
                    .color(paint(INK)),
                )
                .child(
                    text(format!(
                        "Selected command: {}",
                        if state.prefs.agent_bin.is_empty() {
                            "not set"
                        } else {
                            &state.prefs.agent_bin
                        }
                    ))
                    .key("selected-command")
                    .font(type_::BODY)
                    .color(paint(INK))
                    .wrap()
                    .w_full(),
                )
                .child(
                    text(if state.commands_open {
                        "Hide commands and saved-file details"
                    } else {
                        "Change agent / advanced setup"
                    })
                    .key("agent-commands-toggle")
                    .font(type_::LABEL)
                    .color(paint(BLUE))
                    .wrap()
                    .w_full()
                    .on_press_tx(|s: &mut Settings, tx: &mut Tx| {
                        tx.instant();
                        s.commands_open = !s.commands_open;
                        if s.commands_open {
                            tx.scroll_into_view("agent-commands");
                        }
                    }),
                ),
        ))
        .child(section(&format!(
            "Model for {}",
            prefs::agent_name(&state.prefs.agent_id).unwrap_or("the selected command")
        )))
        .child(card(
            col()
                .w_full()
                .gap(8.0)
                .padding(8.0)
                .child(
                    row().w_full().gap(8.0).center_y().child(
                        text_input(&state.model, TextStyle::new(13.0, 400.0))
                            .key("agent-model")
                            .grow(1.0)
                            .min_w(0.0)
                            .h(32.0)
                            .px(8.0)
                            .radius(4.0)
                            .border(1.0, paint(TRACK))
                            .fill(paint(PANEL))
                            .single_line()
                            .placeholder("Model ID (or CLI default)", paint(MUTED))
                            .color(paint(INK))
                            .on_edit(|state: &mut Settings| &mut state.model),
                    ),
                )
                .child(if state.model_error.is_empty() {
                    el()
                } else {
                    text(state.model_error.clone())
                        .key("model-error")
                        .font(type_::LABEL)
                        .color(paint("#ff9f0a"))
                        .wrap()
                        .w_full()
                }),
        ))
        .child(agent_permissions(state))
        .child(
            text("More access settings")
                .key("agent-access")
                .font(type_::LABEL)
                .color(paint(BLUE))
                .on_press_tx(|s: &mut Settings, tx: &mut Tx| {
                    tx.instant();
                    s.page = Page::Access;
                }),
        )
        .child(if state.commands_open {
            agent_commands(state)
        } else {
            el()
        })
}

fn agent_commands(state: &Settings) -> El {
    let mut list = col().w_full();
    let found = prefs::agents();
    if found.is_empty() {
        list = list.child(
            text("No agent found on PATH.")
                .font(type_::BODY)
                .color(paint(MUTED))
                .wrap()
                .w_full(),
        );
    }
    for agent in found {
        list = list.child(agent_row(state, agent));
    }
    col().key("agent-commands").w_full().gap(12.0)
        .child(section("Model access and cost"))
        .child(text(crate::agent::model_note(&state.prefs.agent_bin)).font(type_::LABEL).color(paint(MUTED)).wrap().w_full())
        .child(section("Commands detected on this Mac"))
        .child(text("Adapters: Grok, Antigravity, Copilot, Pi and OpenCode 2.0.23. Other detected commands cannot run a FolderLens review; detection does not verify login, runtime dependencies, model access or native analysis.").font(type_::LABEL).color(paint(MUTED)).wrap().w_full())
        .child(card(list.max_h(160.0).scroll(ScrollSpec {
            stick_bottom: false,
        })))
        .child(section("Custom executable / same adapter rules"))
        .child(
            row()
                .w_full()
                .gap(8.0)
                .center_y()
                .child(
                    text_input(&state.custom, TextStyle::new(13.0, 400.0))
                        .key("agent-command")
                        .grow(1.0)
                        .min_w(0.0)
                        .h(32.0)
                        .px(8.0)
                        .radius(4.0)
                        .border(1.0, paint(TRACK))
                        .fill(paint(PANEL))
                        .single_line()
                        .placeholder("Command on PATH", paint(MUTED))
                        .color(paint(INK))
                        .on_edit(|state: &mut Settings| &mut state.custom),
                )
                .child(
                    button("Use", ButtonStyle::Secondary)
                        .key("use-command")
                        .on_press(|state: &mut Settings| {
                            let bin = state.custom.text.trim().to_string();
                            if bin.is_empty() {
                                return;
                            }
                            state.prefs.agent_id = "custom".into();
                            state.prefs.agent_bin = bin;
                            load_model(state);
                            persist(state);
                        }),
                ),
        )
        .child(text("Each manual run saves private context and a continuation launcher. Shared paths may remain in run files until you remove them. Analysis and an exact saved session are available only after success. Credentials belong in the provider's setup, not the model field. Copilot, Pi and OpenCode use FolderLens's bounded reader; Grok/Antigravity Terminal tools rely on their native controls, not FolderLens's live read boundary.").font(type_::LABEL).color(paint(MUTED)).wrap().w_full())
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
            load_model(state);
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
                text("Your exclusions")
                    .font(type_::BODY)
                    .color(paint(INK))
                    .grow(1.0),
            )
            .child(
                button("Add folder", ButtonStyle::Secondary)
                    .key("add-folder")
                    .on_press(add_folder),
            ),
    );
    let mut paths = col().w_full();
    for path in state.prefs.kept.clone() {
        let shown = path.clone();
        paths = paths.child(
            row()
                .w_full()
                .min_h(40.0)
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
                    button("Stop keeping", ButtonStyle::Destructive)
                        .key(format!("unkeep-{path}"))
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
        text("Built-in exclusions / cannot be unlocked")
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
        .child(heading("What stays out"))
        .child(text("Why a folder stays outside the review").font(type_::BODY).color(paint(INK)))
        .child(text("These rules apply before presets and manual selection. Excluded folders stay out of the selected-folder reader. Removing a rule can select eligible folders through the active preset or a retained manual selection. Nothing is deleted.")
            .font(type_::LABEL).color(paint(MUTED)).wrap().w_full())
        .child(card(folders))
        .child(text("Keep a folder out of future selections. Session allowance in folder actions can override this rule, but never a built-in or in-use exclusion.")
            .font(type_::LABEL).color(paint(MUTED)).wrap().w_full())
        .child(card(
            row()
                .w_full()
                .h(44.0)
                .px(12.0)
                .gap(8.0)
                .center_y()
                .child(
                    text("Exclude changes within")
                        .font(type_::BODY)
                        .color(paint(INK)),
                )
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
        .child(text("0 h disables the recent-change hold. In-use folders remain unavailable. Nothing on this page authorizes cleanup.")
            .font(type_::LABEL).color(paint(MUTED)).wrap().w_full())
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

fn agent_permissions(state: &Settings) -> El {
    card(
        col()
            .w_full()
            .child(setting_row(
                "Allow manual agent requests",
                toggle("allow-start", state.prefs.allow_start, |state| {
                    state.prefs.allow_start = !state.prefs.allow_start;
                    persist(state);
                }),
            ))
            .child(setting_row(
                "Share eligible selected paths",
                toggle("send-paths", state.prefs.send_paths, |state| {
                    state.prefs.send_paths = !state.prefs.send_paths;
                    persist(state);
                }),
            )),
    )
}

fn access(state: &Settings) -> El {
    col()
        .w_full()
        .gap(14.0)
        .child(heading("Selected-folder access"))
        .child(
            text("Saved permissions apply to manual requests. Each request freezes its eligible selection; later changes can reduce, never expand, its live read scope.")
                .font(type_::LABEL)
                .color(paint(MUTED))
                .wrap()
                .w_full(),
        )
        .child(agent_permissions(state))
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
            text("macOS hides some folders until Full Disk Access is on. FolderLens measures the rest of your home either way.")
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
    fn model_apply_preserves_agent_choices_and_rejects_flags_without_saving() {
        use rune::{Env, Runtime};
        let mut state = state();
        let path = state.path.clone();
        preview_agent(&mut state, "copilot", "auto");
        preview_agent(&mut state, "pi", "");
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
        let (rect, _) = rt.presented("agent-model").unwrap();
        assert!(rect[1] >= 0.0 && rect[1] + rect[3] <= 560.0);
        let (x, y) = rt.find("apply-model").unwrap();
        rt.state.model.text = "openrouter/openrouter/free".into();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
        assert_eq!(
            prefs::load_from(&path).model(),
            "openrouter/openrouter/free"
        );
        rt.state.model.text = "--allow-all".into();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
        assert!(!rt.state.model_error.is_empty());
        assert_eq!(
            prefs::load_from(&path).model(),
            "openrouter/openrouter/free"
        );
        rt.state.prefs.agent_id = "copilot".into();
        rt.state.prefs.agent_bin = "copilot".into();
        load_model(&mut rt.state);
        assert_eq!(rt.state.model.text, "auto");
        rt.state.prefs.agent_id = "pi".into();
        rt.state.prefs.agent_bin = "pi".into();
        load_model(&mut rt.state);
        assert_eq!(rt.state.model.text, "openrouter/openrouter/free");
        std::fs::remove_file(path).unwrap();
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
                rect[0] >= 24.0 && rect[0] + rect[2] <= 736.0,
                "{key} {rect:?}"
            );
        }
    }

    #[test]
    fn settings_navigation_keeps_access_and_model_choices_separate() {
        use rune::{Env, Runtime};

        for size in [(360.0, 420.0), (640.0, 420.0), (960.0, 700.0)] {
            let mut state = state();
            preview_agent(&mut state, "pi", "openrouter/openrouter/free");
            let path = state.path.clone();
            let mut rt = Runtime::new(
                state,
                view,
                size,
                Env {
                    system_dark: true,
                    scale: 2.0,
                },
            );
            rt.set_system_reduced_motion(true);
            rt.frame(0.0);
            for (key, page) in [
                ("nav-general", Page::General),
                ("nav-protected", Page::Protected),
                ("nav-updates", Page::Updates),
                ("nav-agent", Page::Agent),
            ] {
                let (rect, _) = rt.presented(key).unwrap();
                assert!(rect[0] >= 0.0 && rect[0] + rect[2] <= size.0);
                assert!(rect[1] >= 0.0 && rect[1] + rect[3] < size.1);
                let (x, y) = rt.find(key).unwrap();
                rt.pointer_down(x, y);
                rt.pointer_up(x, y);
                rt.frame(0.0);
                assert_eq!(rt.state.page, page);
            }
            assert!(
                rt.find("agent-command").is_none(),
                "advanced setup starts closed"
            );
            let (apply, _) = rt.presented("apply-model").unwrap();
            rt.wheel(size.0 / 2.0, size.1 / 2.0, -2000.0);
            rt.frame(0.0);
            assert_eq!(rt.presented("apply-model").unwrap().0, apply);
            let (x, y) = rt.find("agent-access").unwrap();
            rt.pointer_down(x, y);
            rt.pointer_up(x, y);
            rt.frame(0.0);
            assert_eq!(rt.state.page, Page::Access);
            assert!(
                !path.exists(),
                "navigation does not change saved permissions"
            );
            for key in ["allow-start", "send-paths"] {
                let (rect, _) = rt.presented(key).unwrap();
                assert!(
                    rect[1] >= 97.0 && rect[1] + rect[3] <= size.1 - 34.0,
                    "{size:?}: {key}: {rect:?}"
                );
                let (x, y) = rt.find(key).unwrap();
                rt.pointer_down(x, y);
                rt.pointer_up(x, y);
                rt.frame(0.0);
            }
            let saved = prefs::load_from(&path);
            assert!(saved.allow_start && saved.send_paths);
            assert_eq!(saved.agent_id, "pi");
            assert_eq!(saved.model(), "openrouter/openrouter/free");
            assert!(!saved.update_downloads && !saved.update_prereleases);
            let (x, y) = rt.find("nav-agent").unwrap();
            rt.pointer_down(x, y);
            rt.pointer_up(x, y);
            rt.frame(0.0);
            rt.wheel(size.0 / 2.0, size.1 / 2.0, 2000.0);
            rt.frame(0.0);
            let (x, y) = rt.find("agent-commands-toggle").unwrap();
            rt.pointer_down(x, y);
            rt.pointer_up(x, y);
            rt.frame(0.0);
            assert!(rt.state.commands_open);
            rt.update(|_, tx| tx.scroll_into_view("agent-command"));
            rt.frame(0.0);
            for key in ["agent-command", "use-command"] {
                let (rect, _) = rt.presented(key).unwrap();
                assert!(
                    rect[1] >= 97.0 && rect[1] + rect[3] <= apply[1],
                    "{size:?}: {key}: {rect:?}"
                );
            }
            rt.state.custom.text = "/usr/local/bin/pi".into();
            let (x, y) = rt.find("use-command").unwrap();
            rt.pointer_down(x, y);
            rt.pointer_up(x, y);
            rt.frame(0.0);
            let saved = prefs::load_from(&path);
            assert_eq!(saved.agent_id, "custom");
            assert_eq!(saved.agent_bin, "/usr/local/bin/pi");
            assert_eq!(
                saved.models.get("pi").unwrap(),
                "openrouter/openrouter/free"
            );
            assert!(saved.allow_start && saved.send_paths);
            std::fs::remove_file(path).unwrap();
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
                rect[0] >= 24.0 && rect[0] + rect[2] <= 736.0,
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
