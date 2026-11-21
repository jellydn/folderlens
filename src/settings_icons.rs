//! Settings icons: 24-unit stroke drawings in the style of Hex's sidebar icons
//! (github.com/anomalyco/hex, MIT). Rune paints `currentColor` with the element's colour,
//! so one drawing serves every theme.

use rune::Icon;

macro_rules! stroke_icon {
    ($id:ident, $name:literal, $body:literal) => {
        pub static $id: Icon = Icon {
            name: concat!("folderlens-", $name),
            view: 24.0,
            body: concat!(
                r#"<g fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">"#,
                $body,
                "</g>"
            ),
        };
    };
}

stroke_icon!(
    GENERAL,
    "general",
    r#"<path d="M4 6h10M4 12h4M12 12h8M4 18h12"/><circle cx="16" cy="6" r="2"/><circle cx="10" cy="12" r="2"/><circle cx="18" cy="18" r="2"/>"#
);
stroke_icon!(
    AGENT,
    "agent",
    r#"<path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z"/><path d="M18 16l.8 2.2L21 19l-2.2.8L18 22l-.8-2.2L15 19l2.2-.8z"/>"#
);
stroke_icon!(
    KEEP,
    "keep",
    r#"<rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>"#
);
stroke_icon!(
    PRIVACY,
    "privacy",
    r#"<path d="M12 3l7 3v5c0 4.5-3 8-7 10-4-2-7-5.5-7-10V6z"/>"#
);
stroke_icon!(
    THEME,
    "theme",
    r#"<circle cx="12" cy="12" r="9"/><path d="M12 3a9 9 0 0 0 0 18z" fill="currentColor"/>"#
);
stroke_icon!(
    DISPLAY,
    "display",
    r#"<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>"#
);
stroke_icon!(
    PLACEMENT,
    "placement",
    r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 7h6"/>"#
);
stroke_icon!(
    KEYBOARD,
    "keyboard",
    r#"<rect x="3" y="6" width="18" height="12" rx="2"/><path d="M7 10h.01M11 10h.01M15 10h.01M8 14h8"/>"#
);
stroke_icon!(
    MEMORY,
    "memory",
    r#"<rect x="3" y="7" width="18" height="10" rx="1.5"/><path d="M7 7v10M11 7v10M15 7v10M7 20v-3M17 20v-3"/>"#
);
stroke_icon!(
    CPU,
    "cpu",
    r#"<rect x="6" y="6" width="12" height="12" rx="2"/><path d="M9 3v3M15 3v3M9 18v3M15 18v3M3 9h3M3 15h3M18 9h3M18 15h3"/>"#
);
stroke_icon!(
    REFRESH,
    "refresh",
    r#"<path d="M4 12a8 8 0 0 1 14-5.3L20 9"/><path d="M20 4v5h-5"/><path d="M20 12a8 8 0 0 1-14 5.3L4 15"/><path d="M4 20v-5h5"/>"#
);
stroke_icon!(
    DOWNLOAD,
    "download",
    r#"<path d="M12 4v11M7 10l5 5 5-5M5 20h14"/>"#
);
stroke_icon!(
    FLASK,
    "flask",
    r#"<path d="M9 3h6M10 3v6l-5 9a2 2 0 0 0 1.8 3h10.4a2 2 0 0 0 1.8-3l-5-9V3"/><path d="M7.5 15h9"/>"#
);
stroke_icon!(
    TERMINAL,
    "terminal",
    r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 9l3 3-3 3M13 15h4"/>"#
);
stroke_icon!(
    MODEL,
    "model",
    r#"<path d="M12 3l8 4.5v9L12 21l-8-4.5v-9z"/><path d="M4 7.5l8 4.5 8-4.5M12 12v9"/>"#
);
stroke_icon!(PLAY, "play", r#"<path d="M8 5l11 7-11 7z"/>"#);
stroke_icon!(
    NOTE,
    "note",
    r#"<path d="M6 3h9l4 4v14H6z"/><path d="M9 12h7M9 16h5"/>"#
);
stroke_icon!(
    FOLDER,
    "folder",
    r#"<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>"#
);
stroke_icon!(
    CLOCK,
    "clock",
    r#"<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>"#
);
stroke_icon!(
    SHARE,
    "share",
    r#"<path d="M14 4h6v6M20 4l-9 9"/><path d="M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5"/>"#
);
stroke_icon!(
    DISK,
    "disk",
    r#"<rect x="3" y="5" width="18" height="14" rx="2"/><path d="M3 14h18M7 17h.01"/>"#
);
stroke_icon!(
    BOOK,
    "book",
    r#"<path d="M5 4h11a3 3 0 0 1 3 3v13H8a3 3 0 0 1-3-3z"/><path d="M5 17a3 3 0 0 1 3-3h11"/>"#
);
stroke_icon!(
    CHEVRON_RIGHT,
    "chevron-right",
    r#"<path d="M9 6l6 6-6 6"/>"#
);
stroke_icon!(CHEVRON_DOWN, "chevron-down", r#"<path d="M6 9l6 6 6-6"/>"#);
stroke_icon!(
    CHEVRON_LEFT,
    "chevron-left",
    r#"<path d="M15 6l-6 6 6 6"/>"#
);
stroke_icon!(CHECK, "check", r#"<path d="M5 12.5l4.5 4.5L19 7"/>"#);
