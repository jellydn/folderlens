//! Notch-first controls. The review window remains the sole owner of scans and AI jobs.

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use rune::tok::TextStyle;
use rune::{col, el, icon, row, text, AppHandle, AppSpec, Cx, El, Icon, Key, Tx};

use crate::{button, catalog, paint, ButtonStyle, BLUE, INK, MUTED, PANEL, TRACK};

pub const COMPACT: (f32, f32) = (300.0, 32.0);
pub const EXPANDED: (f32, f32) = (480.0, 230.0);

const FOLDER: Icon = Icon {
    name: "candidate-folders",
    view: 24.0,
    body: r#"<path d="M3 6h7l2 3h9v11H3Z" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"/>"#,
};
const DISK: Icon = Icon {
    name: "disk-space",
    view: 24.0,
    body: r#"<rect x="3" y="5" width="18" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="2"/><path d="M3 14h18M16 17h2" stroke="currentColor" stroke-width="2"/>"#,
};
const CPU: Icon = Icon {
    name: "cpu-usage",
    view: 24.0,
    body: r#"<rect x="6" y="6" width="12" height="12" rx="2" fill="none" stroke="currentColor" stroke-width="2"/><path d="M9 2v4m6-4v4M9 18v4m6-4v4M2 9h4m-4 6h4m12-6h4m-4 6h4" stroke="currentColor" stroke-width="2"/>"#,
};
const MEMORY: Icon = Icon {
    name: "memory-usage",
    view: 24.0,
    body: r#"<rect x="2" y="6" width="20" height="11" rx="1" fill="none" stroke="currentColor" stroke-width="2"/><path d="M6 9v5m6-5v5m6-5v5M5 17v3m5-3v3m4-3v3m5-3v3" stroke="currentColor" stroke-width="2"/>"#,
};

#[derive(Clone, PartialEq, Eq)]
struct Readings {
    theme: crate::theme::Theme,
    candidates: String,
    free: String,
    compact_disk: String,
    cpu: Option<String>,
    memory: Option<String>,
    scanning: bool,
    running: bool,
    minutes: u64,
}

impl Readings {
    fn from_review(s: &crate::State) -> Self {
        let bytes: u64 = (0..s.catalog.leaves.len())
            .filter(|&i| catalog::can_tick(&crate::leaf_view(s, i)))
            .map(|i| s.catalog.leaves[i].bytes)
            .sum();
        Self {
            theme: s.prefs.theme,
            candidates: if s.catalog.missing {
                "—".into()
            } else {
                catalog::format_bytes(bytes)
            },
            free: if s.catalog.disk_total == 0 {
                "Disk space unavailable".into()
            } else {
                format!(
                    "Disk {} free of {}",
                    catalog::format_bytes(s.catalog.disk_free),
                    catalog::format_bytes(s.catalog.disk_total)
                )
            },
            cpu: s.prefs.gauge_cpu.then(|| {
                if s.catalog.missing {
                    "CPU —".into()
                } else {
                    format!("CPU {}%", s.catalog.cpu)
                }
            }),
            memory: s.prefs.gauge_memory.then(|| {
                if s.catalog.missing {
                    "Memory —".into()
                } else {
                    format!("Memory {}%", s.catalog.memory)
                }
            }),
            compact_disk: if s.catalog.disk_total == 0 {
                "—".into()
            } else {
                format!(
                    "{}/{}",
                    compact_bytes(s.catalog.disk_free),
                    compact_bytes(s.catalog.disk_total)
                )
            },
            scanning: s.scanning,
            running: s.scans_running,
            minutes: s.interval_minutes,
        }
    }
}

pub struct Notch {
    expanded: bool,
    readings: Readings,
}

static HANDLE: OnceLock<AppHandle<Notch>> = OnceLock::new();
static REVIEW: OnceLock<AppHandle<crate::State>> = OnceLock::new();
// Explicit hiding (tray, Settings, Review or Escape) must cancel automatic restoration.
static VISIBILITY: Mutex<Visibility> = Mutex::new(Visibility::Visible);

#[derive(Clone, Copy, Debug, PartialEq)]
enum Visibility {
    Visible,
    Hidden,
    SpaceHidden,
}

impl Visibility {
    fn fit(&mut self, fits: bool) -> Option<bool> {
        match (*self, fits) {
            (Self::Visible, false) => {
                *self = Self::SpaceHidden;
                Some(false)
            }
            (Self::SpaceHidden, true) => {
                *self = Self::Visible;
                Some(true)
            }
            _ => None,
        }
    }
}

pub fn install(handle: AppHandle<Notch>, review: AppHandle<crate::State>) {
    let _ = HANDLE.set(handle);
    let _ = REVIEW.set(review);
}

pub fn sync(s: &crate::State) {
    crate::status::set_notch_placement(s.prefs.notch_position, s.prefs.notch_display.as_deref());
    if let Some(handle) = HANDLE.get() {
        let readings = Readings::from_review(s);
        handle.update_if(move |state, tx| {
            if state.readings == readings {
                return false;
            }
            state.readings = readings;
            tx.instant();
            true
        });
    }
}

fn send(action: impl FnOnce(&mut crate::State) + Send + 'static) {
    if let Some(handle) = REVIEW.get() {
        handle.update(move |state, tx| {
            action(state);
            sync(state);
            tx.instant();
        });
    }
}

pub fn show() {
    *VISIBILITY.lock().unwrap() = Visibility::Visible;
    if let Some(handle) = HANDLE.get() {
        handle.show();
    }
}

pub fn hide() {
    *VISIBILITY.lock().unwrap() = Visibility::Hidden;
    if let Some(handle) = HANDLE.get() {
        handle.hide();
    }
}

/// Restore only a notch hidden by insufficient space, never an explicit dismissal.
pub fn restore_work_area() {
    if VISIBILITY.lock().unwrap().fit(true) == Some(true) {
        if let Some(handle) = HANDLE.get() {
            handle.show();
        }
    }
}

/// Native placement cannot clamp a window larger than the work area.
pub fn fit_work_area(area: (f32, f32)) {
    if area.0 < COMPACT.0 || area.1 < COMPACT.1 {
        if VISIBILITY.lock().unwrap().fit(false) == Some(false) {
            if let Some(handle) = HANDLE.get() {
                handle.hide();
            }
        }
        return;
    }
    if let Some(handle) = HANDLE.get() {
        handle.update_if(|state, tx| {
            if !state.expanded {
                return false;
            }
            expand(state, tx);
            true
        });
    }
}

fn expand(s: &mut Notch, tx: &mut Tx) {
    s.expanded = !s.expanded;
    tx.instant();
    tx.resize_window_instant(if s.expanded { EXPANDED } else { COMPACT });
}

pub fn spec() -> AppSpec<Notch> {
    AppSpec {
        scenes: &[
            ("compact-example", |_| {}),
            ("expanded-example", |s| s.expanded = true),
            ("running-example", |s| {
                s.expanded = true;
                s.readings.running = true;
                s.readings.minutes = 60;
            }),
            ("scanning-example", |s| {
                s.expanded = true;
                s.readings.scanning = true;
                s.readings.cpu = Some("CPU 100%".into());
                s.readings.memory = Some("Memory 100%".into());
            }),
        ],
        poll: Some(|_, _, tx| {
            tx.wake_after(Duration::from_secs(1));
            if !rune::is_headless() {
                crate::status::position_notch();
            }
            false
        }),
        on_key: Some(|s, event, tx| {
            if event.is_command('q') {
                tx.quit();
                return true;
            }
            if event.is_command(',') {
                crate::settings::open(crate::settings::Page::General);
                return true;
            }
            if matches!(event.key, Key::Escape) {
                if s.expanded {
                    expand(s, tx);
                } else {
                    hide();
                }
                return true;
            }
            false
        }),
        ..AppSpec::new(
            "FolderLens Notch",
            COMPACT,
            || Notch {
                expanded: false,
                readings: Readings::from_review(&crate::state_from(if rune::is_headless() {
                    catalog::fixture()
                } else {
                    catalog::pending()
                })),
            },
            view,
        )
    }
}

fn label(value: impl Into<String>, size: f32, color: &str) -> El {
    text(value)
        .font(TextStyle::new(size, 500.0))
        .color(paint(color))
}

fn compact_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000_000 {
        format!("{:.1}TB", bytes as f64 / 1_000_000_000_000.0)
    } else {
        catalog::format_bytes(bytes).replace(' ', "")
    }
}

fn compact_reading(
    symbol: &'static Icon,
    value: String,
    key: &'static str,
    description: String,
) -> El {
    row()
        .gap(3.0)
        .center_y()
        .help(description.clone())
        .child(icon(symbol, 13.0).color(paint(MUTED)).a11y_hidden())
        .child(label(value, 10.0, INK).key(key).label(description))
}

fn view(s: &Notch, _: &mut Cx<Notch>) -> El {
    let _theme = crate::theme::scope(s.readings.theme);
    let r = &s.readings;
    let status = if r.scanning {
        "Scanning…"
    } else if r.running {
        "Running"
    } else {
        "Paused"
    };
    let mut header = row()
        .w_full()
        .h(if s.expanded { 36.0 } else { COMPACT.1 })
        .px(if s.expanded { 12.0 } else { 8.0 })
        .gap(if s.expanded { 8.0 } else { 4.0 })
        .center_y()
        .child(
            el().size(6.0, 6.0)
                .key("notch-status")
                .role("status")
                .label(status)
                .help(status)
                .radius(3.0)
                .fill(paint(if r.running || r.scanning { BLUE } else { MUTED })),
        )
        .child(if s.expanded {
            label("FolderLens", 12.0, INK).grow(1.0)
        } else {
            compact_reading(
                &FOLDER,
                r.candidates.replace(' ', ""),
                "notch-candidates",
                format!(
                    "Estimated cleanup candidates {}, not reclaimed space",
                    r.candidates
                ),
            )
            .grow(1.0)
        });
    if !s.expanded {
        header = header.child(compact_reading(
            &DISK,
            r.compact_disk.clone(),
            "notch-disk",
            r.free.clone(),
        ));
    }
    if let Some(cpu) = &r.cpu {
        header = header.child(if s.expanded {
            label(cpu.clone(), 12.0, MUTED).key("notch-cpu")
        } else {
            compact_reading(
                &CPU,
                cpu.trim_start_matches("CPU ").into(),
                "notch-cpu",
                cpu.clone(),
            )
        });
    }
    if let Some(memory) = &r.memory {
        header = header.child(if s.expanded {
            label(memory.clone(), 12.0, MUTED).key("notch-memory")
        } else {
            compact_reading(
                &MEMORY,
                memory.trim_start_matches("Memory ").into(),
                "notch-memory",
                memory.clone(),
            )
        });
    }
    header = header.child(
        row()
            .key("notch-expand")
            .role("button")
            .label(if s.expanded {
                "Collapse notch"
            } else {
                "Expand notch"
            })
            .size(if s.expanded { 28.0 } else { 22.0 }, 28.0)
            .center()
            .radius(6.0)
            .fill(paint(TRACK))
            .hover(paint(BLUE).alpha(0.25))
            .focusable(true)
            .focus_ring(2.0, 1.0, paint(BLUE))
            .on_press_tx(expand)
            .child(label(if s.expanded { "⌃" } else { "⌄" }, 15.0, INK)),
    );
    let mut root = col()
        .key("notch")
        .w_full()
        .h_full()
        .radius(12.0)
        .fill(paint(PANEL))
        .child(header);
    if !s.expanded {
        return root;
    }
    let mut intervals = row().gap(6.0);
    for (minutes, title, key) in [
        (30, "30 min", "notch-half-hour"),
        (60, "1 hour", "notch-hour"),
    ] {
        intervals = intervals.child(
            button(
                title,
                if r.minutes == minutes {
                    ButtonStyle::Primary
                } else {
                    ButtonStyle::Secondary
                },
            )
            .key(key)
            .on_press(move |_: &mut Notch| {
                send(move |state| {
                    state.interval_minutes = minutes;
                    crate::schedule_scan(state, std::time::Instant::now());
                })
            }),
        );
    }
    root = root.child(
        col()
            .w_full()
            .px(12.0)
            .pb(12.0)
            .gap(10.0)
            .child(
                row()
                    .w_full()
                    .center_y()
                    .gap(12.0)
                    .child(
                        col()
                            .grow(1.0)
                            .gap(4.0)
                            .child(label("Cleanup candidates · estimated", 12.0, MUTED))
                            .child(label(r.candidates.clone(), 25.0, INK).key("notch-candidates"))
                            .child(label(r.free.clone(), 12.0, MUTED)),
                    )
                    .child(
                        button("Review / AI", ButtonStyle::Secondary)
                            .key("notch-review")
                            .on_press(|_: &mut Notch| crate::status::reveal()),
                    ),
            )
            .child(
                row()
                    .w_full()
                    .gap(6.0)
                    .center_y()
                    .child(intervals.grow(1.0))
                    .child(
                        button(
                            if r.running { "Pause" } else { "Run" },
                            ButtonStyle::Secondary,
                        )
                        .key("notch-run")
                        .on_press(|_: &mut Notch| send(crate::toggle_scans)),
                    )
                    .child({
                        let scan = button(
                            if r.scanning {
                                "Scanning…"
                            } else {
                                "Scan now"
                            },
                            if r.scanning {
                                ButtonStyle::Disabled
                            } else {
                                ButtonStyle::Primary
                            },
                        )
                        .key("notch-scan");
                        if r.scanning {
                            scan
                        } else {
                            scan.on_press(|_: &mut Notch| send(crate::request_scan))
                        }
                    }),
            )
            .child(
                row()
                    .w_full()
                    .gap(6.0)
                    .center_y()
                    .child(
                        label(
                            format!("{status} · {} min · local only", r.minutes),
                            12.0,
                            MUTED,
                        )
                        .grow(1.0),
                    )
                    .child(
                        button("Settings", ButtonStyle::Quiet)
                            .key("notch-settings")
                            .on_press(|_: &mut Notch| {
                                crate::settings::open(crate::settings::Page::General)
                            }),
                    )
                    .child(
                        button("Use tray", ButtonStyle::Quiet)
                            .key("notch-tray")
                            .on_press(|_: &mut Notch| hide()),
                    )
                    .child(
                        button("Quit", ButtonStyle::Quiet)
                            .key("notch-quit")
                            .help("Quit FolderLens · ⌘Q")
                            .on_press_tx(|_: &mut Notch, tx| tx.quit()),
                    ),
            )
            .child(label(
                "Review only. Nothing deleted. AI requests stay manual.",
                11.0,
                MUTED,
            )),
    );
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnect_restores_space_hidden_notches_but_not_explicitly_hidden_ones() {
        let mut visibility = Visibility::Visible;
        // Disconnect onto a work area too small for compact. Repeated polls stay quiet.
        assert_eq!(visibility.fit(false), Some(false));
        assert_eq!(visibility.fit(false), None);
        assert_eq!(visibility.fit(true), Some(true));
        assert_eq!(visibility.fit(true), None);
        // Opening Settings/Review or choosing tray while space-hidden cancels restore.
        assert_eq!(visibility.fit(false), Some(false));
        visibility = Visibility::Hidden;
        assert_eq!(visibility.fit(false), None);
        assert_eq!(visibility.fit(true), None);
        // An explicit show permits normal work-area handling again.
        visibility = Visibility::Visible;
        assert_eq!(visibility.fit(false), Some(false));
        assert_eq!(visibility.fit(true), Some(true));
    }

    #[test]
    fn notch_controls_fit_expand_collapse_and_disable_an_active_scan() {
        let state = Notch {
            expanded: false,
            readings: Readings::from_review(&crate::fixture_state()),
        };
        let mut rt = rune::Runtime::new(
            state,
            view,
            COMPACT,
            rune::Env {
                system_dark: true,
                scale: 2.0,
            },
        );
        rt.set_system_reduced_motion(true);
        rt.frame(0.0);
        for (theme, background) in crate::theme::Theme::ALL.into_iter().zip([
            "#010105", "#1f1f28", "#f2ecbc", "#282a36", "#fffbeb", "#222436", "#e1e2e7",
        ]) {
            rt.update(|s, tx| {
                s.readings.theme = theme;
                tx.instant();
            });
            rt.frame(0.0);
            let root = rt
                .elements()
                .into_iter()
                .find(|el| el.key.as_deref() == Some("notch"))
                .unwrap();
            assert_eq!(root.fill, rune::color::hex(background).unwrap());
            let button = rt.presented("notch-expand").unwrap().0;
            assert_eq!(button[2..], [22.0, 28.0]);
            assert!(button[1] >= 0.0 && button[1] + button[3] <= 32.0);
        }
        assert!(rt.find("notch-scan").is_none());
        for (key, expected) in [
            ("notch-candidates", "27.2GB"),
            ("notch-disk", "84.9GB/494.4GB"),
            ("notch-cpu", "30%"),
            ("notch-memory", "79%"),
        ] {
            let element = rt
                .elements()
                .into_iter()
                .find(|el| el.key.as_deref() == Some(key))
                .unwrap_or_else(|| panic!("missing {key}"));
            assert_eq!(element.text.as_deref(), Some(expected));
            assert!(element.rect[0] >= 0.0 && element.rect[0] + element.rect[2] <= COMPACT.0);
            assert!(element.rect[1] >= 0.0 && element.rect[1] + element.rect[3] <= COMPACT.1);
            assert_eq!(element.rect[1] + element.rect[3] * 0.5, 16.0);
        }
        let mut busy = crate::fixture_state();
        busy.catalog.cpu = 100;
        busy.catalog.memory = 100;
        busy.scanning = true;
        rt.update(move |s, _| s.readings = Readings::from_review(&busy));
        rt.frame(0.0);
        let mut right = 0.0;
        for (key, expected) in [
            ("notch-candidates", "27.2GB"),
            ("notch-disk", "84.9GB/494.4GB"),
            ("notch-cpu", "100%"),
            ("notch-memory", "100%"),
        ] {
            let element = rt
                .elements()
                .into_iter()
                .find(|el| el.key.as_deref() == Some(key))
                .unwrap();
            assert_eq!(element.text.as_deref(), Some(expected));
            assert!(element.rect[0] >= right && element.rect[0] + element.rect[2] <= COMPACT.0);
            right = element.rect[0] + element.rect[2];
        }
        let mut mixed = crate::fixture_state();
        mixed.catalog.disk_free = 84_900_000;
        mixed.catalog.disk_total = 4_000_000_000_000;
        mixed.catalog.cpu = 100;
        mixed.catalog.memory = 100;
        rt.update(move |s, _| s.readings = Readings::from_review(&mixed));
        rt.frame(0.0);
        let disk = rt
            .elements()
            .into_iter()
            .find(|el| el.key.as_deref() == Some("notch-disk"))
            .unwrap();
        assert_eq!(disk.text.as_deref(), Some("85MB/4.0TB"));
        assert!(disk.rect[0] + disk.rect[2] < COMPACT.0);
        rt.update(|s, _| s.readings.scanning = true);
        rt.frame(0.0);
        let status = rt
            .elements()
            .into_iter()
            .find(|el| el.key.as_deref() == Some("notch-status"))
            .unwrap();
        assert_eq!(status.label.as_deref(), Some("Scanning…"));
        assert_eq!(status.help.as_deref(), Some("Scanning…"));
        let (x, y) = rt.find("notch-expand").expect("compact expand control");
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.set_viewport(EXPANDED, 2.0);
        rt.frame(0.0);
        for key in [
            "notch-candidates",
            "notch-hour",
            "notch-run",
            "notch-scan",
            "notch-review",
            "notch-tray",
            "notch-quit",
        ] {
            let rect = rt
                .presented(key)
                .unwrap_or_else(|| panic!("missing {key}"))
                .0;
            assert!(
                rect[0] >= 0.0
                    && rect[1] >= 0.0
                    && rect[0] + rect[2] <= EXPANDED.0
                    && rect[1] + rect[3] <= EXPANDED.1,
                "{key}: {rect:?}"
            );
        }
        rt.update(|s, _| {
            s.readings.scanning = true;
            s.readings.cpu = None;
            s.readings.memory = None;
        });
        rt.frame(0.0);
        assert!(rt.find("notch-cpu").is_none());
        assert!(rt.find("notch-memory").is_none());
        let scan = rt
            .elements()
            .into_iter()
            .find(|el| el.key.as_deref() == Some("notch-scan"))
            .unwrap();
        assert!(!scan.focusable);
        assert_eq!(scan.label.as_deref(), Some("Scanning…"));
        let (x, y) = rt.find("notch-expand").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.set_viewport(COMPACT, 2.0);
        rt.frame(0.0);
        assert!(rt.find("notch-review").is_none());
        assert!(rt.find("notch-expand").is_some());
        let mut review = crate::state_from(catalog::pending());
        review.prefs.gauge_cpu = false;
        review.prefs.gauge_memory = false;
        rt.update(move |s, _| s.readings = Readings::from_review(&review));
        rt.frame(0.0);
        assert!(rt.find("notch-cpu").is_none());
        assert!(rt.find("notch-memory").is_none());
        for (key, expected) in [("notch-candidates", "—"), ("notch-disk", "—")] {
            let element = rt
                .elements()
                .into_iter()
                .find(|el| el.key.as_deref() == Some(key))
                .unwrap();
            assert_eq!(element.text.as_deref(), Some(expected));
        }
        let (x, y) = rt.find("notch-expand").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.set_viewport(EXPANDED, 2.0);
        rt.frame(0.0);
        let (x, y) = rt.find("notch-quit").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
        assert_eq!(rt.shell_request, Some(rune::Shell::Quit));
    }
}
