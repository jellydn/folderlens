//! Settings window: a sidebar of pages, each a card of rows after Hex's settings.

use std::path::PathBuf;
use std::sync::OnceLock;

use rune::tok::{type_, TextStyle};
use rune::{
    col, el, icon, row, text, text_input, AppHandle, AppSpec, Color, Cx, El, Image, ImageCache,
    ScrollSpec, TextEdit, Tx,
};

use crate::prefs::{self, Agent, NotchPosition, Prefs};
use crate::theme::{CHECK_BORDER, WARNING};
use crate::{button, paint, ButtonStyle, BLUE, INK, MUTED, PANEL, TRACK};

#[path = "settings_icons.rs"]
mod icons;
#[path = "settings_rows.rs"]
mod rows;
#[path = "settings_updates.rs"]
mod updates;

use rows::{
    back, card, dropdown, page_title, setting, setting_tinted, switch, value_link, Options,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    General,
    Agent,
    Protected,
    Access,
}

/// A dropdown; its value's key is the anchor its list draws under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum List {
    Theme,
    Display,
    Position,
}

impl List {
    pub fn anchor(self) -> &'static str {
        match self {
            Self::Theme => "theme",
            Self::Display => "notch-display",
            Self::Position => "notch-position",
        }
    }
}

/// A value that needs its own editor opens a sub-page with a back link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubPage {
    Agent,
    /// The Local AI server: address, network opt-in, API key and model list.
    Server,
    Model,
    Kept,
}

pub struct Settings {
    pub prefs: Prefs,
    pub displays: Vec<crate::status::Display>,
    pub path: PathBuf,
    pub page: Page,
    pub custom: TextEdit,
    pub model: TextEdit,
    pub model_error: String,
    pub update_note: String,
    pub sub: Option<SubPage>,
    pub advanced_open: bool,
    pub open_list: Option<List>,
    pub logo: Option<Image>,
    pub endpoint: TextEdit,
    pub api_key: TextEdit,
    /// The last List models, key or address result, or an error, shown on the Server page.
    pub local_note: String,
    /// A List models or Keychain request is running on a worker.
    pub local_busy: bool,
}

pub fn spec() -> AppSpec<Settings> {
    AppSpec {
        scenes: &[
            ("general", |state| state.page = Page::General),
            ("displays-example", preview_displays),
            ("display-missing-example", |state| {
                preview_displays(state);
                state.displays.truncate(1);
            }),
            ("agent", |state| state.page = Page::Agent),
            ("agent-copilot", |state| {
                preview_agent(state, "copilot", "auto")
            }),
            ("agent-pi", |state| {
                preview_agent(state, "pi", "openrouter/openrouter/free")
            }),
            ("agent-advanced", |state| {
                preview_agent(state, "pi", "openrouter/openrouter/free");
                state.sub = Some(SubPage::Agent);
            }),
            ("agent-model", |state| {
                preview_agent(state, "pi", "openrouter/openrouter/free");
                state.sub = Some(SubPage::Model);
            }),
            ("agent-opencode", |state| {
                preview_agent(state, "opencode", "opencode/glm-5.3-flash-free")
            }),
            ("agent-model-error", |state| {
                preview_agent(state, "pi", "unknown");
                state.sub = Some(SubPage::Model);
                save_model(state);
            }),
            ("access", |state| state.page = Page::Access),
            ("protected", |state| state.page = Page::Protected),
            ("protected-example", |state| {
                state.page = Page::Protected;
                state.prefs.kept = vec!["~/Projects/retained-work".into()];
                state.prefs.keep_hours = 24;
            }),
            ("protected-advanced", |state| {
                state.page = Page::Protected;
                state.advanced_open = true;
            }),
            ("kept-example", |state| {
                state.page = Page::Protected;
                state.sub = Some(SubPage::Kept);
                state.prefs.kept = vec![
                    "~/Projects/retained-work".into(),
                    "~/Sites/client-archive".into(),
                ];
            }),
            ("theme-open", |state| {
                preview_displays(state);
                state.prefs.theme = crate::theme::Theme::Lotus;
                state.open_list = Some(List::Theme);
            }),
            ("agent-local", |state| {
                preview_agent(state, prefs::LOCAL, "qwen2.5:3b");
                state.prefs.local_models = vec!["qwen2.5:3b".into(), "llama3.2:1b".into()];
            }),
            ("local-server", |state| {
                preview_agent(state, prefs::LOCAL, "qwen2.5:3b");
                state.prefs.local_endpoint = "http://100.101.1.2:9931/v1".into();
                state.prefs.local_allow_network = true;
                load_endpoint(state);
                state.prefs.local_models = vec!["qwen2.5:3b".into(), "llama3.2:1b".into()];
                state.sub = Some(SubPage::Server);
                state.local_note = "2 models listed.".into();
            }),
            ("local-models", |state| {
                preview_agent(state, prefs::LOCAL, "qwen2.5:3b");
                state.prefs.local_models = vec![
                    "qwen2.5:3b".into(),
                    "/Users/me/models/llama-3.2-3b-q4_k_m.gguf".into(),
                ];
                state.sub = Some(SubPage::Model);
            }),
            ("consent-example", |state| {
                state.page = Page::Access;
                state.prefs.consent = ["local:qwen2.5:3b".into(), "pi:openrouter/x".into()].into();
            }),
            ("updates", |state| state.advanced_open = true),
            ("updates-preview", |state| {
                preview_displays(state);
                state.advanced_open = true;
                state.prefs.update_downloads = true;
                state.prefs.update_prereleases = true;
            }),
        ],
        setup: Some(|state, waker| {
            let logo = ImageCache::new(waker).load(LOGO);
            // A headless snapshot draws one frame, so it waits for the decode the window does not.
            if rune::is_headless() {
                let _ = logo.wait();
            }
            state.logo = Some(logo);
        }),
        poll: Some(|state, _, tx| {
            tx.wake_after(std::time::Duration::from_secs(1));
            if rune::is_headless() {
                return false;
            }
            let displays = crate::status::displays();
            if state.displays == displays {
                return false;
            }
            state.displays = displays;
            true
        }),
        on_key: Some(|state, event, tx| {
            if event.is_command('q') {
                tx.quit();
                return true;
            }
            if matches!(event.key, rune::Key::Escape) {
                if state.open_list.take().is_some() || state.sub.take().is_some() {
                    return true;
                }
                hide();
                crate::notch::show();
                return true;
            }
            false
        }),
        ..AppSpec::new("Settings", (820.0, 580.0), state, view)
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
        displays: crate::status::displays(),
        path,
        page: Page::General,
        custom,
        model: TextEdit::default(),
        model_error: String::new(),
        update_note: String::new(),
        sub: None,
        advanced_open: false,
        open_list: None,
        logo: None,
        endpoint: TextEdit::default(),
        api_key: TextEdit::default().secure(),
        local_note: String::new(),
        local_busy: false,
    };
    load_model(&mut state);
    load_endpoint(&mut state);
    state
}

fn preview_displays(state: &mut Settings) {
    state.page = Page::General;
    state.displays = [
        ("example-built-in", "Built-in Retina Display", 0.0),
        ("example-external", "Studio Display", -2560.0),
    ]
    .into_iter()
    .map(|(id, name, x)| crate::status::Display {
        id: id.into(),
        name: name.into(),
        frame: crate::status::Rect {
            x,
            y: 0.0,
            w: 2560.0,
            h: 1440.0,
        },
        visible: crate::status::Rect {
            x,
            y: 40.0,
            w: 2560.0,
            h: 1375.0,
        },
        safe_area_top: if id == "example-built-in" { 32.0 } else { 0.0 },
    })
    .collect();
    state.prefs.notch_display = Some("example-external".into());
}

fn load_endpoint(state: &mut Settings) {
    state.endpoint = TextEdit::default();
    state.endpoint.text = state.prefs.local_endpoint.clone();
    state.endpoint.anchor = state.endpoint.text.len();
    state.endpoint.focus = state.endpoint.anchor;
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
    crate::notch::hide();
    handle.update(move |state, tx: &mut Tx| {
        tx.instant();
        select_page(state, page);
        state.prefs = prefs::load_from(&state.path);
        load_model(state);
        load_endpoint(state);
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

const RELEASES: &str = "https://github.com/jellydn/folderlens/releases";
const SIDEBAR_W: f32 = 184.0;
const LOGO: &[u8] = include_bytes!("../assets/icon-192.png");

fn view(state: &Settings, _: &mut Cx<Settings>) -> El {
    let _theme = crate::theme::scope(state.prefs.theme);
    row()
        .key("settings")
        .w_full()
        .h_full()
        .fill(paint(PANEL))
        .child(sidebar(state))
        .child(
            page(state)
                .key("settings-page")
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .grow(1.0)
                .min_w(0.0)
                .h_full(),
        )
        .child(open_list(state))
}

fn sidebar(state: &Settings) -> El {
    let mut bar = col()
        .key("sidebar")
        .w(SIDEBAR_W)
        .h_full()
        .px(8.0)
        // Clears the traffic lights, which macOS draws over the full-size content view.
        .pt(44.0)
        .pb(14.0)
        .gap(2.0)
        .fill(paint(INK).alpha(0.03))
        .child(brand(state));
    for (page, key, glyph, label) in [
        (Page::General, "nav-general", &icons::GENERAL, "General"),
        (Page::Agent, "nav-agent", &icons::AGENT, "Agent"),
        (Page::Protected, "nav-protected", &icons::KEEP, "Keep safe"),
        (Page::Access, "nav-access", &icons::PRIVACY, "Privacy"),
    ] {
        let on = state.page == page;
        bar = bar.child(
            row()
                .key(key)
                .role("tab")
                .label(format!("{label}{}", if on { ", selected" } else { "" }))
                .focusable(true)
                .focus_ring(2.0, 0.0, paint(BLUE))
                .w_full()
                .h(32.0)
                .px(10.0)
                .gap(10.0)
                .center_y()
                .radius(7.0)
                .fill(if on {
                    paint(INK).alpha(0.08)
                } else {
                    Color::TRANSPARENT
                })
                .hover(paint(INK).alpha(0.05))
                .on_press_tx(move |s: &mut Settings, tx: &mut Tx| {
                    tx.instant();
                    select_page(s, page);
                })
                .child(
                    icon(glyph, 16.0)
                        .color(paint(if on { INK } else { MUTED }))
                        .a11y_hidden(),
                )
                .child(
                    text(label)
                        .font(TextStyle::new(14.0, 500.0))
                        .color(paint(if on { INK } else { MUTED })),
                ),
        );
    }
    bar.child(el().grow(1.0)).child(footer(state))
}

/// The app icon and name above the pages, as in Hex's sidebar header.
fn brand(state: &Settings) -> El {
    let mut mark = el().w(22.0).h(22.0).a11y_hidden();
    if let Some(logo) = &state.logo {
        mark = mark.pixels(logo.pixels());
    }
    row()
        .key("brand")
        .h(32.0)
        .px(10.0)
        .mb(10.0)
        .gap(8.0)
        .center_y()
        .child(mark)
        .child(
            text("FolderLens")
                .font(TextStyle::new(15.0, 600.0))
                .color(paint(INK)),
        )
}

/// What's new, the version, and Check for updates, as in Hex's sidebar footer.
fn footer(state: &Settings) -> El {
    let version = env!("CARGO_PKG_VERSION");
    let link = |key: &'static str, label: &'static str, ink: &'static str| {
        text(label)
            .key(key)
            .role("button")
            .focusable(true)
            .focus_ring(2.0, 2.0, paint(BLUE))
            .font(type_::BODY)
            .color(paint(ink))
    };
    let mut foot = col()
        .key("footer")
        .w_full()
        .px(10.0)
        .gap(2.0)
        .child(
            row()
                .h(26.0)
                .gap(8.0)
                .center_y()
                .child(icon(&icons::BOOK, 16.0).color(paint(MUTED)).a11y_hidden())
                .child(
                    link("whats-new", "What's new", MUTED).on_press(|_: &mut Settings| {
                        let _ =
                            prefs::open([format!("{RELEASES}/tag/v{}", env!("CARGO_PKG_VERSION"))]);
                    }),
                ),
        )
        .child(
            text(format!("Version {version}"))
                .font(type_::LABEL)
                .color(paint(MUTED))
                .mt(6.0),
        )
        .child(
            link("check-updates", "Check for updates", INK)
                .h(26.0)
                .on_press(|s: &mut Settings| {
                    s.update_note = match crate::update::check() {
                        Ok(()) => updates::CHECKING_NOTE.into(),
                        Err(error) => error.into(),
                    };
                }),
        );
    if !state.update_note.is_empty() {
        foot = foot.child(
            text(state.update_note.clone())
                .key("update-note")
                .font(type_::LABEL)
                .color(paint(MUTED))
                .wrap()
                .w_full(),
        );
    }
    foot
}

fn select_page(state: &mut Settings, page: Page) {
    state.page = page;
    state.sub = None;
    state.advanced_open = false;
    state.open_list = None;
}

fn page(state: &Settings) -> El {
    let body = match (state.page, state.sub) {
        (_, Some(SubPage::Agent)) => agent_choice(state),
        (_, Some(SubPage::Server)) => server(state),
        (_, Some(SubPage::Model)) => model(state),
        (_, Some(SubPage::Kept)) => kept(state),
        (Page::General, None) => general(state),
        (Page::Agent, None) => agent(state),
        (Page::Protected, None) => protected(state),
        (Page::Access, None) => access(state),
    };
    col()
        .w_full()
        .px(24.0)
        .pt(22.0)
        .pb(24.0)
        .gap(14.0)
        .child(body)
}

/// A page: title, one card of rows and, when given, an Advanced fold.
fn sheet(state: &Settings, title: &str, rows: Vec<El>, advanced: Option<Vec<El>>) -> El {
    let mut sheet = col()
        .w_full()
        .gap(14.0)
        .child(page_title(title))
        .child(card("card", rows));
    if let Some(rows) = advanced {
        sheet = sheet.child(rows::advanced(state.advanced_open));
        if state.advanced_open {
            sheet = sheet.child(card("advanced-card", rows));
        }
    }
    sheet
}

/// A wrapped, muted explanation.
fn note(content: impl Into<String>) -> El {
    text(content.into())
        .font(type_::LABEL)
        .color(paint(MUTED))
        .wrap()
        .w_full()
        .px(4.0)
}

const DISPLAY_MISSING_NOTE: &str = "Saved display disconnected. Using primary until it reconnects. Choose Primary to forget the saved display. The tray remains available.";
const DISPLAY_NOTE: &str =
    "A disconnected choice falls back to primary and returns when reconnected";
const PLACEMENT_NOTE: &str = "Top center uses the camera row on notched displays";
const THEME_NOTE: &str = "Notch, Review and Settings";
const READINGS_NOTE: &str = "Shown with your selection. Estimates are not reclaimed space.";
const RECORD_CHOICE_NOTE: &str =
    "Record choice keeps the selection in memory and deletes nothing. Changes save automatically.";

const POSITIONS: [(&str, NotchPosition); 4] = [
    ("Top left", NotchPosition::Left),
    ("Top right", NotchPosition::Right),
    ("Top center", NotchPosition::Center),
    ("Lower center", NotchPosition::Below),
];

fn display_missing(state: &Settings) -> bool {
    state
        .prefs
        .notch_display
        .as_ref()
        .is_some_and(|id| !state.displays.iter().any(|display| &display.id == id))
}

fn display_label(state: &Settings) -> String {
    if display_missing(state) {
        return "Primary (fallback)".into();
    }
    state
        .prefs
        .notch_display
        .as_ref()
        .and_then(|id| state.displays.iter().find(|display| &display.id == id))
        .map_or_else(|| "Primary display".into(), |display| display.name.clone())
}

fn general(state: &Settings) -> El {
    let p = &state.prefs;
    let position = POSITIONS
        .iter()
        .find(|(_, position)| *position == p.notch_position)
        .map_or("Lower center", |(label, _)| label);
    let display_note = if display_missing(state) {
        setting_tinted(
            &icons::DISPLAY,
            "Notch display",
            DISPLAY_MISSING_NOTE,
            WARNING,
            dropdown(List::Display, &display_label(state)),
        )
    } else {
        setting(
            &icons::DISPLAY,
            "Notch display",
            DISPLAY_NOTE,
            dropdown(List::Display, &display_label(state)),
        )
    };
    col()
        .w_full()
        .gap(14.0)
        .child(sheet(
            state,
            "General",
            vec![
                setting(
                    &icons::THEME,
                    "Theme",
                    THEME_NOTE,
                    dropdown(List::Theme, p.theme.label()),
                ),
                display_note.key("notch-display-note"),
                setting(
                    &icons::PLACEMENT,
                    "Notch placement",
                    PLACEMENT_NOTE,
                    dropdown(List::Position, position),
                ),
                setting(
                    &icons::KEYBOARD,
                    "Show shortcut hints",
                    "",
                    switch(
                        "hint-toggle",
                        "Show shortcut hints",
                        p.shortcut_hints,
                        |s| {
                            s.prefs.shortcut_hints = !s.prefs.shortcut_hints;
                            persist(s);
                        },
                    ),
                ),
                setting(
                    &icons::MEMORY,
                    "Memory usage",
                    READINGS_NOTE,
                    switch("gauge-memory", "Memory usage", p.gauge_memory, |s| {
                        s.prefs.gauge_memory = !s.prefs.gauge_memory;
                        persist(s);
                    }),
                ),
                setting(
                    &icons::CPU,
                    "CPU usage",
                    "",
                    switch("gauge-cpu", "CPU usage", p.gauge_cpu, |s| {
                        s.prefs.gauge_cpu = !s.prefs.gauge_cpu;
                        persist(s);
                    }),
                ),
            ],
            Some(updates::rows(state)),
        ))
        .child(if state.advanced_open {
            updates::notes()
        } else {
            note(RECORD_CHOICE_NOTE)
        })
}

/// The open dropdown's options, drawn at the root so they cover later rows.
fn open_list(state: &Settings) -> El {
    let p = &state.prefs;
    let options: Options = match state.open_list {
        Some(List::Theme) => crate::theme::Theme::ALL
            .into_iter()
            .map(|theme| {
                let press: Box<dyn Fn(&mut Settings)> = Box::new(move |s| {
                    s.prefs.theme = theme;
                    persist(s);
                });
                (
                    format!("theme-{}", theme.as_str()),
                    theme.label().to_string(),
                    p.theme == theme,
                    press,
                )
            })
            .collect(),
        Some(List::Display) => {
            let missing = display_missing(state);
            let primary = p.notch_display.is_none() || missing;
            let mut options: Options = vec![(
                "notch-display-primary".into(),
                if missing {
                    "Primary display (fallback)".into()
                } else {
                    "Primary display (automatic)".into()
                },
                primary,
                Box::new(|s| {
                    s.prefs.notch_display = None;
                    persist(s);
                }),
            )];
            for (index, display) in state.displays.iter().enumerate() {
                let id = display.id.clone();
                options.push((
                    format!("notch-display-{id}"),
                    format!("{} (display {})", display.name, index + 1),
                    p.notch_display.as_ref() == Some(&display.id),
                    Box::new(move |s| {
                        s.prefs.notch_display = Some(id.clone());
                        persist(s);
                    }),
                ));
            }
            options
        }
        Some(List::Position) => POSITIONS
            .into_iter()
            .map(|(label, position)| {
                let press: Box<dyn Fn(&mut Settings)> = Box::new(move |s| {
                    s.prefs.notch_position = position;
                    persist(s);
                });
                (
                    format!("notch-position-{}", position.as_str()),
                    label.to_string(),
                    p.notch_position == position,
                    press,
                )
            })
            .collect(),
        None => return el(),
    };
    rows::list(state.open_list.map_or("theme", List::anchor), options)
}

const ADAPTERS_NOTE: &str = "Adapters: Grok, Antigravity, Copilot, Pi and OpenCode 2.0.23. Other detected commands cannot run a FolderLens review; detection does not verify login, runtime dependencies, model access or native analysis.";
const RUN_FILES_NOTE: &str = "Each manual run saves private context and a continuation launcher. Shared paths may remain in run files until you remove them. Analysis and an exact saved session are available only after success. Credentials belong in the provider's setup, not the model field. Copilot, Pi and OpenCode use FolderLens's bounded reader; Grok/Antigravity Terminal tools rely on their native controls, not FolderLens's live read boundary.";

fn agent_label(state: &Settings) -> &'static str {
    prefs::agent_name(&state.prefs.agent_id).unwrap_or(if state.prefs.agent_bin.is_empty() {
        "Not set"
    } else {
        "Custom"
    })
}

fn agent(state: &Settings) -> El {
    let p = &state.prefs;
    let command = if p.agent_bin.is_empty() {
        "Selected command: not set".to_string()
    } else {
        format!("Selected command: {}", p.agent_bin)
    };
    let model = match (p.model().is_empty(), p.is_local()) {
        (true, true) => "Not set",
        (true, false) => "CLI default",
        (false, _) => crate::local_ai::short_model_name(p.model()),
    };
    let command = if p.is_local() {
        format!("Server: {}", p.local_endpoint)
    } else {
        command
    };
    let mut rows = vec![];
    if p.is_local() {
        rows.push(setting(
            &icons::SERVER,
            "Server",
            "Address, network access and API key",
            value_link(
                "local-server-link",
                crate::local_ai::network_host(&p.local_endpoint).unwrap_or("This Mac"),
                |s| s.sub = Some(SubPage::Server),
            ),
        ));
    }
    sheet(
        state,
        "Agent",
        vec![
            setting(
                &icons::TERMINAL,
                "Agent",
                &command,
                value_link("agent-choice", agent_label(state), |s| {
                    s.sub = Some(SubPage::Agent)
                }),
            )
            .key("selected-command"),
            setting(
                &icons::MODEL,
                "Model",
                if p.is_local() {
                    "Exact ID from the server"
                } else {
                    "Passed with --model where supported"
                },
                value_link("agent-model-link", model, |s| s.sub = Some(SubPage::Model)),
            ),
            setting(
                &icons::PLAY,
                "Allow manual agent requests",
                "Manual requests only. Provider costs and data policies apply.",
                switch(
                    "allow-start",
                    "Allow manual agent requests",
                    p.allow_start,
                    |s| {
                        s.prefs.allow_start = !s.prefs.allow_start;
                        persist(s);
                    },
                ),
            ),
        ]
        .into_iter()
        .chain(rows)
        .collect(),
        Some(vec![setting(
            &icons::NOTE,
            "Run files",
            RUN_FILES_NOTE,
            el(),
        )]),
    )
}

fn agent_choice(state: &Settings) -> El {
    let local = state.prefs.is_local();
    let mut rows: Vec<El> = vec![setting(
        &icons::SERVER,
        "Local AI",
        "Ollama, LM Studio or llama.cpp; on this Mac unless you allow your network",
        radio(local),
    )
    .key("agent-local")
    .role("radio")
    .label("Local AI")
    .checked(local)
    .focusable(true)
    .on_press(|state: &mut Settings| {
        state.prefs.agent_id = prefs::LOCAL.into();
        state.prefs.agent_bin = prefs::LOCAL.into();
        load_model(state);
        persist(state);
    })];
    rows.extend(prefs::agents().iter().map(|agent| agent_row(state, agent)));
    if rows.len() == 1 {
        rows.push(setting(
            &icons::TERMINAL,
            "No agent found on PATH.",
            "",
            el(),
        ));
    }
    rows.push(setting(
        &icons::TERMINAL,
        "Custom executable",
        "Same adapter rules apply",
        row()
            .gap(8.0)
            .center_y()
            .child(
                text_input(&state.custom, TextStyle::new(13.0, 400.0))
                    .key("agent-command")
                    .w(200.0)
                    .h(28.0)
                    .px(8.0)
                    .radius(6.0)
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
    ));
    col()
        .key("agent-commands")
        .w_full()
        .gap(14.0)
        .child(back("Agent"))
        .child(page_title("Choose agent"))
        .child(card("card", rows))
        .child(note(ADAPTERS_NOTE))
}

fn agent_row(state: &Settings, agent: &Agent) -> El {
    let id = agent.id.to_string();
    let bin = agent.bin.to_string_lossy().into_owned();
    let on = state.prefs.agent_id == agent.id;
    setting(&icons::TERMINAL, agent.name, &bin, radio(on))
        .key(format!("agent-{}", agent.id))
        .role("radio")
        .label(agent.name)
        .checked(on)
        .focusable(true)
        .on_press(move |state: &mut Settings| {
            state.prefs.agent_id = id.clone();
            state.prefs.agent_bin = bin.clone();
            load_model(state);
            persist(state);
        })
}

fn radio(on: bool) -> El {
    let mut mark = el()
        .size(16.0, 16.0)
        .radius(8.0)
        .border(1.4, paint(if on { BLUE } else { CHECK_BORDER }));
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

fn model(state: &Settings) -> El {
    let name = prefs::agent_name(&state.prefs.agent_id).unwrap_or("the selected command");
    let mut rows = vec![setting(
        &icons::MODEL,
        "Model ID",
        if state.prefs.is_local() {
            "Or type an exact ID"
        } else {
            "Leave empty for the CLI default"
        },
        row()
            .gap(8.0)
            .center_y()
            .child(
                text_input(&state.model, TextStyle::new(13.0, 400.0))
                    .key("agent-model")
                    .w(240.0)
                    .h(28.0)
                    .px(8.0)
                    .radius(6.0)
                    .border(1.0, paint(TRACK))
                    .fill(paint(PANEL))
                    .single_line()
                    .placeholder("Model ID (or CLI default)", paint(MUTED))
                    .color(paint(INK))
                    .on_edit(|state: &mut Settings| &mut state.model),
            )
            .child(
                button("Apply", ButtonStyle::Primary)
                    .key("apply-model")
                    .on_press_tx(|state: &mut Settings, tx: &mut Tx| {
                        save_model(state);
                        if !state.model_error.is_empty() {
                            tx.scroll_into_view("model-error");
                        }
                    }),
            ),
    )];
    if !state.model_error.is_empty() {
        rows.push(
            setting_tinted(&icons::NOTE, "Not saved", &state.model_error, WARNING, el())
                .key("model-error"),
        );
    }
    let mut page = col()
        .w_full()
        .gap(14.0)
        .child(back("Agent"))
        .child(page_title(&format!("Model for {name}")));
    if state.prefs.is_local() {
        page = page.child(card("listed-models", listed_models(state)));
    }
    page.child(card("card", rows))
        .child(note(crate::agent::model_note(&state.prefs.agent_bin)))
}

fn listed_models(state: &Settings) -> Vec<El> {
    let p = &state.prefs;
    let mut rows: Vec<El> = p
        .local_models
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let on = p.model() == id;
            let shown = crate::local_ai::short_model_name(id);
            let help = if shown == id { "" } else { id.as_str() };
            let chosen = id.clone();
            setting(&icons::MODEL, shown, help, radio(on))
                .key(format!("local-model-{index}"))
                .role("radio")
                .label(shown.to_string())
                .checked(on)
                .focusable(true)
                .on_press(move |state: &mut Settings| {
                    state
                        .prefs
                        .models
                        .insert(prefs::LOCAL.into(), chosen.clone());
                    load_model(state);
                    persist(state);
                })
        })
        .collect();
    let help = if state.local_note.is_empty() {
        "Models the server reports at /v1/models"
    } else {
        state.local_note.as_str()
    };
    rows.push(setting(
        &icons::SERVER,
        "Server models",
        help,
        list_button(state),
    ));
    rows
}

fn list_button(state: &Settings) -> El {
    let style = if state.local_busy {
        ButtonStyle::Disabled
    } else {
        ButtonStyle::Secondary
    };
    button("List models", style)
        .key("list-models")
        .on_press(start_list_models)
}

const NETWORK_NOTE: &str = "Off: only servers on this Mac. On: also private LAN and Tailscale addresses and names. Public addresses are always refused, also when a name resolves to one.";
const KEY_NOTE: &str =
    "Optional, for llama-server --api-key. Kept in the Keychain and sent as a Bearer token.";

fn server(state: &Settings) -> El {
    let p = &state.prefs;
    let preset = |key: &'static str, label: &'static str, url: &'static str| {
        button(label, ButtonStyle::Secondary)
            .key(key)
            .on_press(move |s: &mut Settings| {
                s.endpoint.text = url.into();
                apply_endpoint(s);
            })
    };
    let key_control = if p.local_key_saved {
        button("Remove", ButtonStyle::Secondary)
            .key("remove-key")
            .on_press(remove_key)
    } else {
        row()
            .gap(8.0)
            .center_y()
            .child(
                text_input(&state.api_key, TextStyle::new(13.0, 400.0))
                    .key("api-key")
                    .w(140.0)
                    .h(28.0)
                    .px(8.0)
                    .radius(6.0)
                    .border(1.0, paint(TRACK))
                    .fill(paint(PANEL))
                    .single_line()
                    .placeholder("API key", paint(MUTED))
                    .color(paint(INK))
                    .on_edit(|state: &mut Settings| &mut state.api_key),
            )
            .child(
                button("Save", ButtonStyle::Secondary)
                    .key("save-key")
                    .on_press(save_key),
            )
    };
    let mut rows = vec![
        setting(
            &icons::SERVER,
            "Address",
            "Ends in /v1",
            row()
                .gap(8.0)
                .center_y()
                .child(
                    text_input(&state.endpoint, TextStyle::new(13.0, 400.0))
                        .key("local-endpoint")
                        .w(200.0)
                        .h(28.0)
                        .px(8.0)
                        .radius(6.0)
                        .border(1.0, paint(TRACK))
                        .fill(paint(PANEL))
                        .single_line()
                        .placeholder(crate::local_ai::OLLAMA, paint(MUTED))
                        .color(paint(INK))
                        .on_edit(|state: &mut Settings| &mut state.endpoint),
                )
                .child(
                    button("Apply", ButtonStyle::Primary)
                        .key("apply-endpoint")
                        .on_press(apply_endpoint),
                ),
        ),
        setting(
            &icons::TERMINAL,
            "On this Mac",
            "",
            row()
                .gap(6.0)
                .child(preset("preset-ollama", "Ollama", crate::local_ai::OLLAMA))
                .child(preset(
                    "preset-lm-studio",
                    "LM Studio",
                    crate::local_ai::LM_STUDIO,
                ))
                .child(preset(
                    "preset-llama-cpp",
                    "llama.cpp",
                    crate::local_ai::LLAMA_CPP,
                )),
        ),
        setting(
            &icons::SHARE,
            "Allow LAN and Tailscale servers",
            NETWORK_NOTE,
            switch(
                "local-network",
                "Allow LAN and Tailscale servers",
                p.local_allow_network,
                |s| {
                    s.prefs.local_allow_network = !s.prefs.local_allow_network;
                    persist(s);
                },
            ),
        ),
        setting(
            &icons::KEY,
            "API key",
            if p.local_key_saved {
                "Saved in the Keychain. Sent as a Bearer token."
            } else {
                KEY_NOTE
            },
            key_control,
        ),
        setting(
            &icons::MODEL,
            "Models",
            if p.local_models.is_empty() {
                "None listed yet"
            } else {
                "Choose one in Agent › Model"
            },
            list_button(state),
        ),
    ];
    if !state.local_note.is_empty() {
        rows.push(setting(&icons::NOTE, &state.local_note, "", el()).key("local-note"));
    }
    col()
        .w_full()
        .gap(14.0)
        .child(back("Agent"))
        .child(page_title("Local AI server"))
        .child(card("card", rows))
        .child(note("FolderLens sends item names and estimated sizes, and full paths only with path sharing on. It never sends file contents to Local AI and writes no run folder for it. The server may keep its own logs."))
}

fn apply_endpoint(state: &mut Settings) {
    match crate::local_ai::validate_endpoint(&state.endpoint.text, state.prefs.local_allow_network)
    {
        Ok(url) => {
            if url != state.prefs.local_endpoint {
                // Models belong to a server; a new address lists its own.
                state.prefs.local_models.clear();
            }
            state.prefs.local_endpoint = url;
            load_endpoint(state);
            state.local_note = "Address saved. Use List models to check the server.".into();
            persist(state);
        }
        Err(error) => state.local_note = error,
    }
}

/// Runs `work` on a worker and applies its result through the Settings handle. The Keychain
/// can prompt and the network can be slow, so neither runs in the frame loop.
fn in_background<T: Send + 'static>(
    state: &mut Settings,
    note: &str,
    work: impl FnOnce() -> T + Send + 'static,
    finish: fn(&mut Settings, T),
) {
    if state.local_busy {
        return;
    }
    state.local_busy = true;
    state.local_note = note.into();
    std::thread::spawn(move || {
        let result = work();
        if let Some(handle) = HANDLE.get() {
            handle.update(move |state, _tx: &mut Tx| {
                state.local_busy = false;
                finish(state, result);
            });
        }
    });
}

fn start_list_models(state: &mut Settings) {
    let endpoint = state.prefs.local_endpoint.clone();
    let network = state.prefs.local_allow_network;
    let saved = state.prefs.local_key_saved;
    in_background(
        state,
        "Asking the server for its models…",
        move || {
            let key = saved
                .then(crate::keychain::local_api_key)
                .flatten()
                .map(crate::local_ai::ApiKey);
            let models = crate::local_ai::list_models(&endpoint, network, key.as_ref());
            (endpoint, models)
        },
        finish_list_models,
    );
}

fn finish_list_models(
    state: &mut Settings,
    (endpoint, models): (String, Result<Vec<String>, String>),
) {
    if endpoint != state.prefs.local_endpoint {
        state.local_note = "The address changed while listing. List models again.".into();
        return;
    }
    state.local_note = match models {
        Ok(models) if models.is_empty() => {
            "The server reports no models. Load one in the server first.".into()
        }
        Ok(models) => {
            let note = match models.len() {
                1 => "1 model listed.".to_string(),
                n => format!("{n} models listed."),
            };
            state.prefs.local_models = models;
            persist(state);
            note
        }
        Err(error) => error,
    };
}

fn save_key(state: &mut Settings) {
    let key = state.api_key.text.trim().to_string();
    // The field never keeps the key once Save is pressed.
    state.api_key = TextEdit::default().secure();
    if key.is_empty() {
        return;
    }
    in_background(
        state,
        "Saving the key in the Keychain…",
        move || crate::keychain::set_local_api_key(&key),
        |state, result| match result {
            Ok(()) => {
                state.prefs.local_key_saved = true;
                state.local_note = "API key saved in the Keychain.".into();
                persist(state);
            }
            Err(error) => state.local_note = error,
        },
    );
}

fn remove_key(state: &mut Settings) {
    in_background(
        state,
        "Removing the key from the Keychain…",
        crate::keychain::remove_local_api_key,
        |state, result| match result {
            Ok(()) => {
                state.prefs.local_key_saved = false;
                state.local_note = "API key removed.".into();
                persist(state);
            }
            Err(error) => state.local_note = error,
        },
    );
}

const RULES_NOTE: &str = "These rules apply before presets and manual selection. Excluded folders stay out of the selected-folder reader. Removing a rule can select eligible folders through the active preset or a retained manual selection. Nothing is deleted.";
const KEEP_OUT_NOTE: &str = "Keep a folder out of future selections. Session allowance in folder actions can override this rule, but never a built-in or in-use exclusion.";
const KEEP_HOURS_NOTE: &str =
    "0 h turns off the recent-change hold. In-use folders stay unavailable.";

fn protected(state: &Settings) -> El {
    let count = state.prefs.kept.len();
    let kept = match count {
        0 => "None".to_string(),
        1 => "1 folder".to_string(),
        n => format!("{n} folders"),
    };
    let hours = state.prefs.keep_hours;
    let always = prefs::ALWAYS
        .iter()
        .map(|(name, path)| {
            setting(&icons::KEEP, name, path, el()).key(format!("always-{}", always_key(name)))
        })
        .collect();
    col()
        .w_full()
        .gap(14.0)
        .child(sheet(
            state,
            "Keep safe",
            vec![
                setting(
                    &icons::FOLDER,
                    "Your exclusions",
                    "Kept out of future selections",
                    value_link("kept-link", &kept, |s| s.sub = Some(SubPage::Kept)),
                ),
                setting(
                    &icons::CLOCK,
                    "Exclude changes within",
                    KEEP_HOURS_NOTE,
                    row()
                        .gap(6.0)
                        .center_y()
                        .child(hour_step("hour-down", "−", -1))
                        .child(
                            text(format!("{hours} h"))
                                .key("keep-hours")
                                .font(type_::BODY)
                                .color(paint(INK))
                                .min_w(40.0)
                                .center(),
                        )
                        .child(hour_step("hour-up", "+", 1)),
                ),
            ],
            Some(always),
        ))
        .child(note(if state.advanced_open {
            "Built-in exclusions cannot be unlocked."
        } else {
            RULES_NOTE
        }))
}

fn kept(state: &Settings) -> El {
    let mut rows: Vec<El> = state
        .prefs
        .kept
        .iter()
        .map(|path| {
            let removed = path.clone();
            setting(
                &icons::FOLDER,
                path,
                "",
                button("Stop keeping", ButtonStyle::Destructive)
                    .key(format!("unkeep-{path}"))
                    .on_press(move |state: &mut Settings| {
                        state.prefs.kept.retain(|kept| kept != &removed);
                        persist(state);
                    }),
            )
        })
        .collect();
    rows.push(setting(
        &icons::FOLDER,
        "Add folder",
        "Built-in exclusions are already kept",
        button("Add folder…", ButtonStyle::Secondary)
            .key("add-folder")
            .on_press(add_folder),
    ));
    col()
        .w_full()
        .gap(14.0)
        .child(back("Keep safe"))
        .child(page_title("Your exclusions"))
        .child(card("card", rows))
        .child(note(KEEP_OUT_NOTE))
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
    button(mark, ButtonStyle::Secondary)
        .key(key)
        .label(if delta > 0 {
            "One more hour"
        } else {
            "One hour less"
        })
        .w(32.0)
        .on_press(move |state: &mut Settings| {
            let next = state.prefs.keep_hours as i32 + delta;
            state.prefs.keep_hours = next.clamp(0, prefs::MAX_KEEP_HOURS as i32) as u32;
            persist(state);
        })
}

fn always_key(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

const FROZEN_SCOPE_NOTE: &str = "Each request freezes its eligible selection; later changes can reduce, never expand, its live read scope.";
const DISK_ACCESS_NOTE: &str = "macOS hides some folders until this is on. FolderLens measures the rest of your home either way.";

fn access(state: &Settings) -> El {
    sheet(
        state,
        "Privacy",
        vec![
            setting(
                &icons::SHARE,
                "Share eligible selected paths",
                FROZEN_SCOPE_NOTE,
                switch(
                    "send-paths",
                    "Share eligible selected paths",
                    state.prefs.send_paths,
                    |s| {
                        s.prefs.send_paths = !s.prefs.send_paths;
                        persist(s);
                    },
                ),
            ),
            setting(
                &icons::PRIVACY,
                "Allowed providers and models",
                &consent_help(state),
                button(
                    "Revoke all",
                    if state.prefs.consent.is_empty() {
                        ButtonStyle::Disabled
                    } else {
                        ButtonStyle::Secondary
                    },
                )
                .key("revoke-consent")
                .on_press(|s: &mut Settings| {
                    s.prefs.consent.clear();
                    persist(s);
                }),
            ),
            setting(
                &icons::DISK,
                "Full Disk Access",
                DISK_ACCESS_NOTE,
                button("Open", ButtonStyle::Secondary)
                    .key("disk-access")
                    .on_press(|_: &mut Settings| prefs::open_full_disk_access()),
            ),
        ],
        None,
    )
}

fn consent_help(state: &Settings) -> String {
    let what = match state.prefs.consent.len() {
        0 => "None yet. FolderLens asks before the first request to each provider and model."
            .to_string(),
        1 => "1 provider and model may receive your selection.".to_string(),
        n => format!("{n} provider and model choices may receive your selection."),
    };
    if state.prefs.consent.is_empty() {
        what
    } else {
        format!("{what} Revoking asks again before the next request.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    const SIZES: [(f32, f32); 2] = [(820.0, 580.0), (640.0, 460.0)];

    fn runtime(state: Settings, size: (f32, f32)) -> Runtime<Settings> {
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
        rt
    }

    /// Scrolls `key` into view, checks it is inside the window, and clicks it.
    fn press(rt: &mut Runtime<Settings>, key: &str, size: (f32, f32)) {
        rt.update(|_, tx| tx.scroll_into_view(key));
        rt.frame(0.0);
        let (rect, _) = rt
            .presented(key)
            .unwrap_or_else(|| panic!("{size:?}: missing {key}"));
        assert!(
            rect[0] >= 0.0 && rect[0] + rect[2] <= size.0,
            "{size:?}: {key}: {rect:?}"
        );
        assert!(
            rect[1] >= 0.0 && rect[1] + rect[3] <= size.1,
            "{size:?}: {key}: {rect:?}"
        );
        let (x, y) = rt.find(key).unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
    }

    fn selected(rt: &Runtime<Settings>, prefix: &str) -> Vec<String> {
        rt.elements()
            .into_iter()
            .filter(|el| el.key.as_deref().is_some_and(|key| key.starts_with(prefix)))
            .filter(|el| {
                el.label
                    .as_deref()
                    .is_some_and(|label| label.ends_with(", selected"))
            })
            .filter_map(|el| el.key)
            .collect()
    }

    #[test]
    fn display_choices_persist_and_explain_disconnect_without_forgetting_the_choice() {
        for size in SIZES {
            let mut state = state();
            preview_displays(&mut state);
            state.prefs.notch_display = None;
            state.prefs.notch_position = NotchPosition::Right;
            state.prefs.theme = crate::theme::Theme::Lotus;
            let path = state.path.clone();
            let mut rt = runtime(state, size);
            press(&mut rt, "notch-display", size);
            press(&mut rt, "notch-display-example-external", size);
            assert_eq!(rt.state.open_list, None, "a choice closes the list");
            let saved = prefs::load_from(&path);
            assert_eq!(saved.notch_display.as_deref(), Some("example-external"));
            assert_eq!(saved.notch_position, NotchPosition::Right);
            assert_eq!(saved.theme, crate::theme::Theme::Lotus);
            assert!(!saved.allow_start && !saved.send_paths);
            let connected = rt.state.displays.clone();
            rt.update(|s, _| s.displays.truncate(1));
            rt.frame(0.0);
            assert!(rt.elements().iter().any(|el| el
                .text
                .as_deref()
                .is_some_and(|text| text.starts_with("Saved display disconnected"))));
            assert_eq!(prefs::load_from(&path), saved, "disconnect forgets nothing");
            rt.update(|s, _| s.displays = connected);
            rt.frame(0.0);
            press(&mut rt, "notch-display", size);
            assert_eq!(
                selected(&rt, "notch-display-"),
                ["notch-display-example-external"]
            );
            press(&mut rt, "notch-display-primary", size);
            assert_eq!(prefs::load_from(&path).notch_display, None);
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn theme_choices_are_reachable_persist_and_preserve_position_and_permissions() {
        for size in SIZES {
            let mut state = state();
            state.prefs.notch_position = NotchPosition::Right;
            let path = state.path.clone();
            let mut rt = runtime(state, size);
            for (theme, background) in crate::theme::Theme::ALL.into_iter().zip([
                "#010105", "#1f1f28", "#f2ecbc", "#282a36", "#fffbeb", "#222436", "#e1e2e7",
            ]) {
                press(&mut rt, "theme", size);
                let key = format!("theme-{}", theme.as_str());
                press(&mut rt, &key, size);
                let saved = prefs::load_from(&path);
                assert_eq!(saved.theme, theme);
                assert_eq!(saved.notch_position, NotchPosition::Right);
                assert!(!saved.allow_start && !saved.send_paths);
                let root = rt
                    .elements()
                    .into_iter()
                    .find(|el| el.key.as_deref() == Some("settings"))
                    .unwrap();
                assert_eq!(root.fill, rune::color::hex(background).unwrap());
                press(&mut rt, "theme", size);
                assert_eq!(selected(&rt, "theme-"), [key]);
                press(&mut rt, "theme", size);
            }
            // The keyboard reaches the list and picks Default.
            press(&mut rt, "theme", size);
            for _ in 0..40 {
                if rt
                    .elements()
                    .iter()
                    .any(|el| el.key.as_deref() == Some("theme-default") && el.focused)
                {
                    break;
                }
                rt.key(rune::KeyEvent {
                    key: rune::Key::Tab,
                    mods: rune::Mods::default(),
                });
                rt.frame(0.0);
            }
            rt.key(rune::KeyEvent {
                key: rune::Key::Text(" ".into()),
                mods: rune::Mods::default(),
            });
            rt.frame(0.0);
            assert_eq!(prefs::load_from(&path).theme, crate::theme::Theme::Default);
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn gauge_and_position_choices_write_the_temp_file() {
        let state = state();
        assert!(state.path.starts_with(std::env::temp_dir()));
        assert_ne!(state.path, prefs::user_path());
        let path = state.path.clone();
        for size in SIZES {
            let mut rt = runtime(super::state(), size);
            rt.state.path = path.clone();
            rt.state.prefs = prefs::load_from(&path);
            press(&mut rt, "gauge-cpu", size);
            let cpu = prefs::load_from(&path).gauge_cpu;
            for (_, position) in POSITIONS {
                press(&mut rt, "notch-position", size);
                press(
                    &mut rt,
                    &format!("notch-position-{}", position.as_str()),
                    size,
                );
                let saved = prefs::load_from(&path);
                assert_eq!(saved.notch_position, position);
                assert_eq!(saved.gauge_cpu, cpu);
                assert!(saved.gauge_memory);
                assert!(!saved.allow_start && !saved.send_paths);
            }
        }
        assert!(prefs::load_from(&path).gauge_cpu, "two presses restore CPU");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn model_apply_preserves_agent_choices_and_rejects_flags_without_saving() {
        let mut state = state();
        let path = state.path.clone();
        preview_agent(&mut state, "copilot", "auto");
        preview_agent(&mut state, "pi", "");
        let size = SIZES[0];
        let mut rt = runtime(state, size);
        press(&mut rt, "agent-model-link", size);
        assert_eq!(rt.state.sub, Some(SubPage::Model));
        rt.state.model.text = "openrouter/openrouter/free".into();
        press(&mut rt, "apply-model", size);
        assert_eq!(
            prefs::load_from(&path).model(),
            "openrouter/openrouter/free"
        );
        rt.state.model.text = "--allow-all".into();
        press(&mut rt, "apply-model", size);
        assert!(!rt.state.model_error.is_empty());
        assert!(rt.presented("model-error").is_some());
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
        press(&mut rt, "back", size);
        assert_eq!(rt.state.sub, None);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn settings_navigation_keeps_access_and_model_choices_separate() {
        for size in SIZES {
            let mut state = state();
            preview_agent(&mut state, "pi", "openrouter/openrouter/free");
            let path = state.path.clone();
            let mut rt = runtime(state, size);
            for (key, page) in [
                ("nav-general", Page::General),
                ("nav-protected", Page::Protected),
                ("nav-access", Page::Access),
                ("nav-agent", Page::Agent),
            ] {
                press(&mut rt, key, size);
                assert_eq!(rt.state.page, page);
                assert!(rt.presented("page-title").is_some());
            }
            assert!(!path.exists(), "navigation does not change saved settings");
            let label = |rt: &Runtime<Settings>, key: &str| {
                rt.elements()
                    .into_iter()
                    .find(|el| el.key.as_deref() == Some(key))
                    .and_then(|el| el.label)
            };
            assert_eq!(
                label(&rt, "allow-start").as_deref(),
                Some("Allow manual agent requests"),
                "a switch is named by its setting"
            );
            press(&mut rt, "allow-start", size);
            press(&mut rt, "nav-access", size);
            assert_eq!(
                label(&rt, "send-paths").as_deref(),
                Some("Share eligible selected paths")
            );
            for key in ["send-paths", "disk-access"] {
                assert!(rt.presented(key).is_some(), "{size:?}: {key}");
            }
            assert!(rt.find("allow-start").is_none(), "one switch, one page");
            press(&mut rt, "send-paths", size);
            let saved = prefs::load_from(&path);
            assert!(saved.allow_start && saved.send_paths);
            assert_eq!(saved.agent_id, "pi");
            assert_eq!(saved.model(), "openrouter/openrouter/free");
            assert!(!saved.update_downloads && !saved.update_prereleases);

            press(&mut rt, "nav-agent", size);
            assert!(rt.find("agent-command").is_none());
            press(&mut rt, "agent-choice", size);
            assert_eq!(rt.state.sub, Some(SubPage::Agent));
            rt.state.custom.text = "/usr/local/bin/pi".into();
            press(&mut rt, "use-command", size);
            let saved = prefs::load_from(&path);
            assert_eq!(saved.agent_id, "custom");
            assert_eq!(saved.agent_bin, "/usr/local/bin/pi");
            assert_eq!(
                saved.models.get("pi").unwrap(),
                "openrouter/openrouter/free"
            );
            assert!(saved.allow_start && saved.send_paths);
            press(&mut rt, "nav-general", size);
            assert_eq!(rt.state.sub, None, "a page change leaves the sub-page");
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn the_protected_page_steps_the_hold_and_folds_the_locks() {
        let mut state = state();
        state.page = Page::Protected;
        let path = state.path.clone();
        let size = SIZES[1];
        let mut rt = runtime(state, size);
        assert!(rt.find("always-mail").is_none(), "Advanced starts closed");
        press(&mut rt, "hour-up", size);
        assert_eq!(prefs::load_from(&path).keep_hours, 1);
        press(&mut rt, "hour-down", size);
        press(&mut rt, "hour-down", size);
        assert_eq!(prefs::load_from(&path).keep_hours, 0);
        press(&mut rt, "advanced", size);
        for key in ["always-mail", "always-kubernetes"] {
            rt.update(|_, tx| tx.scroll_into_view(key));
            rt.frame(0.0);
            assert!(rt.presented(key).is_some(), "{key}");
        }
        press(&mut rt, "kept-link", size);
        assert_eq!(rt.state.sub, Some(SubPage::Kept));
        assert!(rt.presented("add-folder").is_some());
        let _ = std::fs::remove_file(&path);
    }
}
