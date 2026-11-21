//! Settings page parts, after Hex's settings (github.com/anomalyco/hex, MIT): a large page
//! title, one rounded card of rows with thin separators, each row an icon, a title, an
//! optional grey help line and a control on the right. Colours come from FolderLens's theme
//! tokens, so every palette (including Kanagawa Lotus) applies.

use rune::tok::{type_, TextStyle};
use rune::{col, el, icon, row, text, El, Icon, Tx};

use super::icons;
use super::Settings;
use crate::theme::{CARD, TOGGLE_OFF};
use crate::{paint, BLUE, INK, MUTED, PANEL};

/// A row's inset inside its card.
const ROW_PX: f32 = 16.0;

pub fn page_title(title: &str) -> El {
    text(title.to_string())
        .key("page-title")
        .role("heading")
        .level(1)
        .font(TextStyle::new(20.0, 600.0).lh(28.0))
        .color(paint(INK))
}

/// A rounded card of rows with a thin line between each pair.
pub fn card(key: &str, rows: Vec<El>) -> El {
    let mut card = col()
        .key(key.to_string())
        .w_full()
        .radius(10.0)
        .fill(paint(CARD))
        .border(1.0, paint(INK).alpha(0.06));
    for (index, item) in rows.into_iter().enumerate() {
        if index > 0 {
            card = card.child(
                el().h(1.0)
                    .ml(ROW_PX)
                    .mr(ROW_PX)
                    .fill(paint(INK).alpha(0.08))
                    .a11y_hidden(),
            );
        }
        card = card.child(item);
    }
    card
}

/// One setting: icon, title and optional help line on the left, `control` on the right.
pub fn setting(glyph: &'static Icon, title: &str, help: &str, control: El) -> El {
    setting_tinted(glyph, title, help, MUTED, control)
}

pub fn setting_tinted(
    glyph: &'static Icon,
    title: &str,
    help: &str,
    help_color: &str,
    control: El,
) -> El {
    let mut copy = col().grow(1.0).shrink().min_w(0.0).gap(2.0).child(
        text(title.to_string())
            .font(type_::BODY)
            .color(paint(INK))
            .wrap(),
    );
    if !help.is_empty() {
        copy = copy.child(
            text(help.to_string())
                .font(type_::LABEL)
                .color(paint(help_color))
                .wrap(),
        );
    }
    row()
        .w_full()
        .min_h(if help.is_empty() { 48.0 } else { 60.0 })
        .px(ROW_PX)
        .py(10.0)
        .gap(12.0)
        .center_y()
        .child(icon(glyph, 16.0).color(paint(MUTED)).a11y_hidden())
        .child(copy)
        .child(control)
}

/// A pressable value with a trailing glyph: › for a sub-page, ⌄ for a dropdown.
fn value(key: &str, shown: &str, glyph: &'static Icon) -> El {
    row()
        .key(key.to_string())
        .role("button")
        .label(shown.to_string())
        .focusable(true)
        .focus_ring(2.0, 0.0, paint(BLUE))
        .h(28.0)
        .pl(8.0)
        .pr(4.0)
        .gap(4.0)
        .center_y()
        .radius(6.0)
        .hover(paint(INK).alpha(0.07))
        .press(paint(INK).alpha(0.14))
        .child(
            text(shown.to_string())
                .font(TextStyle::new(13.0, 500.0))
                .color(paint(INK))
                .truncate()
                .max_w(240.0),
        )
        .child(icon(glyph, 16.0).color(paint(MUTED)).a11y_hidden())
}

/// A value that opens a sub-page.
pub fn value_link(key: &str, shown: &str, press: impl Fn(&mut Settings) + 'static) -> El {
    value(key, shown, &icons::CHEVRON_RIGHT).on_press_tx(move |s: &mut Settings, tx: &mut Tx| {
        tx.instant();
        press(s);
    })
}

/// A value that opens a dropdown list; `list` names it in [`Settings::open_list`].
pub fn dropdown(key: &'static str, shown: &str, list: &'static str) -> El {
    value(key, shown, &icons::CHEVRON_DOWN).on_press_tx(move |s: &mut Settings, tx: &mut Tx| {
        tx.instant();
        s.open_list = if s.open_list == Some(list) {
            None
        } else {
            Some(list)
        };
    })
}

/// Dropdown options: key, label, selected, and what choosing it does.
pub type Options = Vec<(String, String, bool, Box<dyn Fn(&mut Settings)>)>;

/// A dropdown's options, drawn at the root under the value with `anchor` as its key.
pub fn list(anchor: &str, options: Options) -> El {
    let mut menu = col()
        .key(format!("{anchor}-list"))
        .min_w(200.0)
        .padding(4.0)
        .radius(8.0)
        .fill(paint(PANEL))
        .border(1.0, paint(INK).alpha(0.12))
        .box_shadow(rune::Shadow {
            x: 0.0,
            y: 8.0,
            blur: 24.0,
            spread: 0.0,
            color: rune::Color::BLACK.alpha(0.3),
            inset: false,
        });
    for (key, label, selected, press) in options {
        menu = menu.child(
            row()
                .key(key)
                .role("option")
                .label(format!(
                    "{label}{}",
                    if selected { ", selected" } else { "" }
                ))
                .focusable(true)
                .focus_ring(2.0, 0.0, paint(BLUE))
                .h(28.0)
                .px(10.0)
                .gap(16.0)
                .center_y()
                .radius(5.0)
                .hover(paint(INK).alpha(0.07))
                .on_press_tx(move |s: &mut Settings, tx: &mut Tx| {
                    tx.instant();
                    press(s);
                    s.open_list = None;
                })
                .child(text(label).font(type_::BODY).color(paint(INK)).grow(1.0))
                .child(if selected {
                    icon(&icons::CHECK, 14.0).color(paint(BLUE)).a11y_hidden()
                } else {
                    el().size(14.0, 14.0)
                }),
        );
    }
    menu.anchored(
        anchor.to_string(),
        rune::Side::Bottom,
        rune::Align::End,
        4.0,
    )
}

/// An on/off switch.
pub fn switch(key: &'static str, on: bool, press: impl Fn(&mut Settings) + 'static) -> El {
    el().key(key)
        .role("switch")
        .a11y_value(if on { "on" } else { "off" })
        .focusable(true)
        .focus_ring(2.0, 2.0, paint(BLUE))
        .size(36.0, 22.0)
        .radius(11.0)
        .fill(paint(if on { BLUE } else { TOGGLE_OFF }))
        .on_press(press)
        .child(
            el().size(18.0, 18.0)
                .radius(9.0)
                .fill(if on { paint(PANEL) } else { paint(INK) })
                .absolute(if on { 16.0 } else { 2.0 }, 2.0),
        )
}

/// The collapsed "Advanced ▸" header under a page's card.
pub fn advanced(open: bool) -> El {
    row()
        .key("advanced")
        .role("button")
        .label(if open { "Advanced, open" } else { "Advanced" })
        .focusable(true)
        .focus_ring(2.0, 2.0, paint(BLUE))
        .mt(8.0)
        .px(ROW_PX)
        .h(28.0)
        .gap(4.0)
        .center_y()
        .on_press_tx(|s: &mut Settings, tx: &mut Tx| {
            tx.instant();
            s.advanced_open = !s.advanced_open;
            if s.advanced_open {
                tx.scroll_into_view("advanced-card");
            }
        })
        .child(text("Advanced").font(type_::BODY).color(paint(MUTED)))
        .child(
            icon(
                if open {
                    &icons::CHEVRON_DOWN
                } else {
                    &icons::CHEVRON_RIGHT
                },
                12.0,
            )
            .color(paint(MUTED))
            .a11y_hidden(),
        )
}

/// The ‹ link back from a sub-page.
pub fn back(label: &str) -> El {
    row()
        .key("back")
        .role("button")
        .label(format!("Back to {label}"))
        .focusable(true)
        .focus_ring(2.0, 2.0, paint(BLUE))
        .h(24.0)
        .gap(2.0)
        .center_y()
        .on_press_tx(|s: &mut Settings, tx: &mut Tx| {
            tx.instant();
            s.sub = None;
        })
        .child(
            icon(&icons::CHEVRON_LEFT, 14.0)
                .color(paint(BLUE))
                .a11y_hidden(),
        )
        .child(
            text(label.to_string())
                .font(type_::LABEL)
                .color(paint(BLUE)),
        )
}
