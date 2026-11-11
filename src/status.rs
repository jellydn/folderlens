//! The menu-bar item that shows and hides the panel.
//!
//! Rune has no macOS status item. FolderLens installs its own. A headless snap and a
//! unit test leave the panel where it is. The coordinate helper is pure so the
//! position can be checked without AppKit.

use std::sync::{Mutex, OnceLock};

use crate::prefs::NotchPosition;

// Native frame callbacks have no Rune state. Review polling publishes this preference.
static NOTCH_POSITION: Mutex<NotchPosition> = Mutex::new(NotchPosition::Below);

pub fn set_notch_position(position: NotchPosition) {
    *NOTCH_POSITION.lock().unwrap() = position;
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    /// Height in AppKit points.
    pub h: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Screen {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub scale: f64,
}

/// Only an explicit status-button press toggles the persistent panel.
fn button_pressed(button: Rect, x: f64, y: f64, mouse_down: bool) -> bool {
    mouse_down
        && x >= button.x
        && x <= button.x + button.w
        && y >= button.y
        && y <= button.y + button.h
}

/// Button frame is points, origin at the bottom left, y up.
/// The result is the panel's top-left in physical pixels, y down.
pub fn popover_origin(button: Rect, screen: Screen, window_w: f64) -> (i32, i32) {
    let scale = if screen.scale <= 0.0 {
        1.0
    } else {
        screen.scale
    };
    let mut x = button.x + button.w * 0.5 - window_w * 0.5;
    let min_x = screen.x + 8.0;
    let max_x = (screen.x + screen.w - window_w - 8.0).max(min_x);
    x = x.clamp(min_x, max_x);
    let y = screen.y + screen.h - button.y;
    ((x * scale).round() as i32, (y * scale).round() as i32)
}

pub fn arm<S: 'static>(handle: rune::AppHandle<S>, window_w: f32) {
    #[cfg(target_os = "macos")]
    {
        let _ = START.set(Box::new(move || {
            macos::install(handle.clone(), window_w);
        }));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (handle, window_w);
    }
}

/// Creates the status item on the first live frame. A snap does not call it.
pub fn on_poll() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if rune::is_headless() {
            return;
        }
        if let Some(start) = START.get() {
            start();
        }
    });
}

pub fn is_visible() -> bool {
    #[cfg(target_os = "macos")]
    return macos::is_visible();
    #[cfg(not(target_os = "macos"))]
    false
}

static START: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

/// Hides the panel and clears the menu-bar highlight.
/// Settings calls this so the panel is gone before Settings opens.
pub fn conceal() {
    #[cfg(target_os = "macos")]
    macos::conceal();
}

/// Opens the full review from the notch, with the tray position as fallback.
pub fn reveal() {
    #[cfg(target_os = "macos")]
    macos::apply(true);
}

/// Screen-relative placement, independent of other apps and their window sizes.
fn notch_origin(
    visible: Rect,
    center: f64,
    width: f64,
    height: f64,
    position: NotchPosition,
) -> Option<(f64, f64)> {
    if width > visible.w || height > visible.h {
        return None;
    }
    let centered = center - width * 0.5;
    let x = match position {
        NotchPosition::Left => visible.x + 8.0,
        NotchPosition::Right => visible.x + visible.w - width - 8.0,
        _ => centered,
    };
    // Preserve the lower-row spacing of existing 0.3.1 settings.
    let inset = if position == NotchPosition::Below {
        41.0
    } else {
        0.0
    };
    let y = visible.y + visible.h - height - inset;
    Some((
        x.clamp(visible.x, visible.x + visible.w - width),
        y.clamp(visible.y, visible.y + visible.h - height),
    ))
}

pub fn position_notch() {
    #[cfg(target_os = "macos")]
    macos::position_notch();
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
    use std::sync::OnceLock;

    use super::{button_pressed, popover_origin, Rect, Screen};

    static ITEM: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
    static VISIBLE: AtomicBool = AtomicBool::new(true);
    static PANEL: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
    static EXPANDED: AtomicBool = AtomicBool::new(false);
    static SESSION_STARTED: AtomicU64 = AtomicU64::new(0);
    static APPLY: OnceLock<Box<dyn Fn(bool) + Send + Sync>> = OnceLock::new();

    pub fn install<S: 'static>(handle: rune::AppHandle<S>, window_w: f32) {
        unsafe {
            install_item();
        }
        let handle_for_click = handle.clone();
        let _ = APPLY.set(Box::new(move |show| {
            VISIBLE.store(show, Ordering::SeqCst);
            if show {
                crate::notch::hide();
                crate::settings::hide();
                set_highlight(true);
                if let Some(origin) = current_origin(window_w) {
                    handle_for_click.set_position(origin);
                }
                // Focus too, so the first action click and shortcuts reach Rune.
                handle_for_click.open();
            } else {
                set_highlight(false);
                handle_for_click.hide();
                crate::notch::show();
            }
        }));
        VISIBLE.store(false, Ordering::SeqCst);
        handle.hide();
        crate::notch::show();
    }

    pub(super) fn conceal() {
        let item = ITEM.load(Ordering::Acquire);
        if EXPANDED.load(Ordering::Relaxed) {
            let session = msg0(item, sel(c"expandedInterfaceSession"));
            if !session.is_null() {
                msg_void(session, sel(c"cancel"));
            }
        }
        apply(false);
    }

    pub(super) fn is_visible() -> bool {
        VISIBLE.load(Ordering::Relaxed)
    }

    pub(super) fn apply(show: bool) {
        if let Some(apply) = APPLY.get() {
            apply(show);
        }
    }

    pub(super) fn position_notch() {
        // The first NSScreen owns the menu bar; mainScreen follows keyboard focus.
        let screen = msg_index(
            msg0(class(c"NSScreen"), sel(c"screens")),
            sel(c"objectAtIndex:"),
            0,
        );
        if screen.is_null() {
            return;
        }
        let area = msg_rect(screen, sel(c"visibleFrame"));
        let display = msg_rect(screen, sel(c"frame"));
        let windows = msg0(
            msg0(class(c"NSApplication"), sel(c"sharedApplication")),
            sel(c"windows"),
        );
        for index in 0..msg_usize(windows, sel(c"count")) {
            let window = msg_index(windows, sel(c"objectAtIndex:"), index);
            if !msg_ptr_bool(
                msg0(window, sel(c"title")),
                sel(c"isEqualToString:"),
                ns_string("FolderLens Notch"),
            ) {
                continue;
            }
            let frame = msg_rect(window, sel(c"frame"));
            let Some((x, y)) = super::notch_origin(
                Rect {
                    x: area.origin.x,
                    y: area.origin.y,
                    w: area.size.w,
                    h: area.size.h,
                },
                display.origin.x + display.size.w * 0.5,
                frame.size.w,
                frame.size.h,
                *super::NOTCH_POSITION.lock().unwrap(),
            ) else {
                crate::notch::fit_work_area((area.size.w as f32, area.size.h as f32));
                break;
            };
            if (frame.origin.x - x).abs() > 0.5 || (frame.origin.y - y).abs() > 0.5 {
                unsafe {
                    let set: unsafe extern "C" fn(*mut c_void, *mut c_void, NsPoint) =
                        std::mem::transmute(objc_msgSend as *const ());
                    set(window, sel(c"setFrameOrigin:"), NsPoint { x, y });
                }
            }
            break;
        }
    }

    /// Returns false when AppKit did not hand back a button.
    ///
    /// The item stays retained for the process. A local autorelease pool here
    /// frees it before the status bar keeps it.
    unsafe fn install_item() -> bool {
        let app = msg0(class(c"NSApplication"), sel(c"sharedApplication"));
        let bar = msg0(class(c"NSStatusBar"), sel(c"systemStatusBar"));
        if bar.is_null() {
            return false;
        }
        let item = msg_f64(bar, sel(c"statusItemWithLength:"), -1.0);
        if item.is_null() {
            return false;
        }
        let item = msg0(item, sel(c"retain"));
        // A stable name. An automatic Item-N can come back hidden.
        msg_void_ptr(item, sel(c"setAutosaveName:"), ns_string("FolderLens"));
        let button = msg0(item, sel(c"button"));
        if button.is_null() {
            msg_void_ptr(bar, sel(c"removeStatusItem:"), item);
            return false;
        }
        set_icon(button);
        let target = make_target();
        if target.is_null() {
            msg_void_ptr(bar, sel(c"removeStatusItem:"), item);
            return false;
        }
        let supports_expanded = msg_ptr_bool(
            item,
            sel(c"respondsToSelector:"),
            sel(c"setExpandedInterfaceDelegate:"),
        );
        EXPANDED.store(supports_expanded, Ordering::Relaxed);
        if supports_expanded {
            msg_void_ptr(item, sel(c"setExpandedInterfaceDelegate:"), target);
        } else {
            msg_void_ptr(button, sel(c"setTarget:"), target);
            msg_void_ptr(button, sel(c"setAction:"), sel(c"click:"));
        }
        monitor_clicks();
        let windows = msg0(app, sel(c"windows"));
        for index in 0..msg_usize(windows, sel(c"count")) {
            let window = msg_index(windows, sel(c"objectAtIndex:"), index);
            if msg_ptr_bool(
                msg0(window, sel(c"title")),
                sel(c"isEqualToString:"),
                ns_string("FolderLens"),
            ) {
                PANEL.store(window, Ordering::Release);
                break;
            }
        }
        ITEM.store(item, Ordering::Release);
        true
    }

    // macOS 27 needs a second-button-click toggle while a session is active.
    // Other apps, including FolderLensUpdater's dialog, must not dismiss FolderLens.
    unsafe fn monitor_clicks() {
        let handler = block2::RcBlock::new(|event: *mut c_void| {
            if !is_visible()
                || msg_f64_ret(event, sel(c"timestamp"))
                    <= f64::from_bits(SESSION_STARTED.load(Ordering::Relaxed))
            {
                return;
            }
            let item = ITEM.load(Ordering::Acquire);
            // A new session's initiating press belongs to begin(), not this monitor.
            if !EXPANDED.load(Ordering::Relaxed)
                || msg0(item, sel(c"expandedInterfaceSession")).is_null()
            {
                return;
            }
            let window = msg0(msg0(item, sel(c"button")), sel(c"window"));
            let rect = msg_rect(window, sel(c"frame"));
            let point = msg_point(class(c"NSEvent"), sel(c"mouseLocation"));
            let button = Rect {
                x: rect.origin.x,
                y: rect.origin.y,
                w: rect.size.w,
                h: rect.size.h,
            };
            let mouse_down = matches!(msg_usize(event, sel(c"type")), 1 | 3 | 25);
            if button_pressed(button, point.x, point.y, mouse_down) {
                conceal();
            }
        });
        let add: unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const c_void,
        ) -> *mut c_void = std::mem::transmute(objc_msgSend as *const ());
        // AppKit copies the block and owns the monitor for the process lifetime.
        add(
            class(c"NSEvent"),
            sel(c"addGlobalMonitorForEventsMatchingMask:handler:"),
            (1 << 1) | (1 << 3) | (1 << 25), // Mouse-down only; no key-event permission.
            (&*handler as *const block2::Block<dyn Fn(*mut c_void)>).cast(),
        );
    }

    fn set_icon(button: *mut c_void) {
        let name = ns_string("archivebox");
        let image = msg_ptr2(
            class(c"NSImage"),
            sel(c"imageWithSystemSymbolName:accessibilityDescription:"),
            name,
            ns_string("FolderLens"),
        );
        if image.is_null() {
            msg_void_ptr(button, sel(c"setTitle:"), ns_string("FolderLens"));
            return;
        }
        msg_bool(image, sel(c"setTemplate:"), true);
        msg_void_ptr(button, sel(c"setImage:"), image);
        msg_void_ptr(button, sel(c"setToolTip:"), ns_string("FolderLens"));
    }

    fn make_target() -> *mut c_void {
        static CLASS: OnceLock<usize> = OnceLock::new();
        let cls = *CLASS.get_or_init(|| unsafe {
            let super_cls = objc_getClass(c"NSObject".as_ptr());
            let cls = objc_allocateClassPair(super_cls, c"FolderLensStatusTarget".as_ptr(), 0);
            if cls.is_null() {
                return objc_getClass(c"FolderLensStatusTarget".as_ptr()) as usize;
            }
            let protocol = objc_getProtocol(c"NSStatusItemExpandedInterfaceDelegate".as_ptr());
            if !protocol.is_null() {
                class_addProtocol(cls, protocol);
            }
            add_method(cls, c"click:", click as *const (), c"v@:@");
            add_method(
                cls,
                c"statusItem:didBeginExpandedInterfaceSession:",
                begin as *const (),
                c"v@:@@",
            );
            add_method(
                cls,
                c"statusItemDidEndExpandedInterfaceSession:animated:",
                end as *const (),
                c"v@:@c",
            );
            objc_registerClassPair(cls);
            cls as usize
        });
        if cls == 0 {
            return std::ptr::null_mut();
        }
        let obj = msg0(cls as *mut c_void, sel(c"alloc"));
        msg0(obj, sel(c"init"))
    }

    unsafe extern "C" fn click(_this: *mut c_void, _cmd: *mut c_void, _sender: *mut c_void) {
        if is_visible() {
            conceal();
        } else {
            apply(true);
        }
    }

    unsafe extern "C" fn begin(
        _this: *mut c_void,
        _cmd: *mut c_void,
        _item: *mut c_void,
        _session: *mut c_void,
    ) {
        let process = msg0(class(c"NSProcessInfo"), sel(c"processInfo"));
        SESSION_STARTED.store(
            msg_f64_ret(process, sel(c"systemUptime")).to_bits(),
            Ordering::Relaxed,
        );
        // A previous session can end while the persistent panel stays visible.
        // Treat the next initiating tray press as a toggle, not another open.
        if is_visible() {
            conceal();
        } else {
            apply(true);
        }
    }

    unsafe extern "C" fn end(
        _this: *mut c_void,
        _cmd: *mut c_void,
        _item: *mut c_void,
        _animated: i8,
    ) {
        // A helper dialog or another app can end AppKit's transient session.
        // FolderLens stays visible until Close, Escape, or an explicit tray toggle.
        set_highlight(is_visible());
    }

    fn current_origin(window_w: f32) -> Option<(i32, i32)> {
        let item = ITEM.load(Ordering::Acquire);
        if item.is_null() {
            return None;
        }
        let button = msg0(item, sel(c"button"));
        if button.is_null() {
            return None;
        }
        let window = msg0(button, sel(c"window"));
        if window.is_null() {
            return None;
        }
        let frame = msg_rect(window, sel(c"frame"));
        // The menu bar can sit on a screen that is not `mainScreen`.
        let screen = {
            let of_button = msg0(window, sel(c"screen"));
            if of_button.is_null() {
                msg0(class(c"NSScreen"), sel(c"mainScreen"))
            } else {
                of_button
            }
        };
        if screen.is_null() {
            return None;
        }
        let bounds = msg_rect(screen, sel(c"frame"));
        let scale = msg_f64_ret(screen, sel(c"backingScaleFactor"));
        let panel = PANEL.load(Ordering::Acquire);
        let width = if panel.is_null() {
            f64::from(window_w)
        } else {
            msg_rect(panel, sel(c"frame")).size.w
        };
        Some(popover_origin(
            Rect {
                x: frame.origin.x,
                y: frame.origin.y,
                w: frame.size.w,
                h: frame.size.h,
            },
            Screen {
                x: bounds.origin.x,
                y: bounds.origin.y,
                w: bounds.size.w,
                h: bounds.size.h,
                scale,
            },
            width,
        ))
    }

    fn set_highlight(on: bool) {
        let item = ITEM.load(Ordering::Acquire);
        if item.is_null() {
            return;
        }
        let button = msg0(item, sel(c"button"));
        if !button.is_null() {
            msg_bool(button, sel(c"setHighlighted:"), on);
        }
    }

    fn ns_string(text: &str) -> *mut c_void {
        let c = std::ffi::CString::new(text)
            .unwrap_or_else(|_| std::ffi::CString::new("FolderLens").unwrap());
        msg_cstr(
            class(c"NSString"),
            sel(c"stringWithUTF8String:"),
            c.as_ptr(),
        )
    }

    fn class(name: &std::ffi::CStr) -> *mut c_void {
        unsafe { objc_getClass(name.as_ptr()) }
    }

    fn sel(name: &std::ffi::CStr) -> *mut c_void {
        unsafe { sel_registerName(name.as_ptr()) }
    }

    fn msg0(obj: *mut c_void, sel: *mut c_void) -> *mut c_void {
        if obj.is_null() {
            return std::ptr::null_mut();
        }
        unsafe {
            let f = std::mem::transmute::<
                usize,
                unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void,
            >(objc_msgSend as *const () as usize);
            f(obj, sel)
        }
    }

    fn msg_void(obj: *mut c_void, selector: *mut c_void) {
        unsafe {
            let f: unsafe extern "C" fn(*mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());
            f(obj, selector);
        }
    }

    fn msg_void_ptr(obj: *mut c_void, sel: *mut c_void, arg: *mut c_void) {
        if obj.is_null() {
            return;
        }
        unsafe {
            let f = std::mem::transmute::<
                usize,
                unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void),
            >(objc_msgSend as *const () as usize);
            f(obj, sel, arg);
        }
    }

    unsafe fn add_method(
        cls: *mut c_void,
        name: &std::ffi::CStr,
        imp: *const (),
        types: &std::ffi::CStr,
    ) {
        class_addMethod(cls, sel_registerName(name.as_ptr()), imp, types.as_ptr());
    }

    fn msg_bool(obj: *mut c_void, sel: *mut c_void, on: bool) {
        if obj.is_null() {
            return;
        }
        unsafe {
            let f = std::mem::transmute::<usize, unsafe extern "C" fn(*mut c_void, *mut c_void, i8)>(
                objc_msgSend as *const () as usize,
            );
            f(obj, sel, i8::from(on));
        }
    }

    fn msg_f64(obj: *mut c_void, sel: *mut c_void, value: f64) -> *mut c_void {
        if obj.is_null() {
            return std::ptr::null_mut();
        }
        unsafe {
            let f = std::mem::transmute::<
                usize,
                unsafe extern "C" fn(*mut c_void, *mut c_void, f64) -> *mut c_void,
            >(objc_msgSend as *const () as usize);
            f(obj, sel, value)
        }
    }

    fn msg_f64_ret(obj: *mut c_void, sel: *mut c_void) -> f64 {
        if obj.is_null() {
            return 1.0;
        }
        unsafe {
            let f = std::mem::transmute::<
                usize,
                unsafe extern "C" fn(*mut c_void, *mut c_void) -> f64,
            >(objc_msgSend as *const () as usize);
            f(obj, sel)
        }
    }

    fn msg_cstr(obj: *mut c_void, sel: *mut c_void, text: *const i8) -> *mut c_void {
        if obj.is_null() {
            return std::ptr::null_mut();
        }
        unsafe {
            let f = std::mem::transmute::<
                usize,
                unsafe extern "C" fn(*mut c_void, *mut c_void, *const i8) -> *mut c_void,
            >(objc_msgSend as *const () as usize);
            f(obj, sel, text)
        }
    }

    fn msg_ptr2(obj: *mut c_void, sel: *mut c_void, a: *mut c_void, b: *mut c_void) -> *mut c_void {
        if obj.is_null() {
            return std::ptr::null_mut();
        }
        unsafe {
            let f = std::mem::transmute::<
                usize,
                unsafe extern "C" fn(
                    *mut c_void,
                    *mut c_void,
                    *mut c_void,
                    *mut c_void,
                ) -> *mut c_void,
            >(objc_msgSend as *const () as usize);
            f(obj, sel, a, b)
        }
    }

    fn msg_rect(obj: *mut c_void, sel: *mut c_void) -> NsRect {
        if obj.is_null() {
            return NsRect::default();
        }
        unsafe {
            #[cfg(target_arch = "aarch64")]
            let raw = objc_msgSend as *const () as usize;
            #[cfg(target_arch = "x86_64")]
            let raw = objc_msgSend_stret as *const () as usize;
            let f = std::mem::transmute::<
                usize,
                unsafe extern "C" fn(*mut c_void, *mut c_void) -> NsRect,
            >(raw);
            f(obj, sel)
        }
    }

    fn msg_point(obj: *mut c_void, selector: *mut c_void) -> NsPoint {
        unsafe {
            let f: unsafe extern "C" fn(*mut c_void, *mut c_void) -> NsPoint =
                std::mem::transmute(objc_msgSend as *const ());
            f(obj, selector)
        }
    }

    fn msg_ptr_bool(obj: *mut c_void, selector: *mut c_void, arg: *mut c_void) -> bool {
        unsafe {
            let f: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i8 =
                std::mem::transmute(objc_msgSend as *const ());
            f(obj, selector, arg) != 0
        }
    }

    fn msg_usize(obj: *mut c_void, selector: *mut c_void) -> usize {
        unsafe {
            let f: unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize =
                std::mem::transmute(objc_msgSend as *const ());
            f(obj, selector)
        }
    }

    fn msg_index(obj: *mut c_void, selector: *mut c_void, index: usize) -> *mut c_void {
        unsafe {
            let f: unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            f(obj, selector, index)
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct NsPoint {
        x: f64,
        y: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct NsSize {
        w: f64,
        h: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct NsRect {
        origin: NsPoint,
        size: NsSize,
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn ending_an_update_popup_session_preserves_the_panel_until_explicit_close() {
            // The status item owns process-wide callbacks; isolate from headless UI tests.
            if std::env::var_os("FOLDERLENS_STATUS_TEST_CHILD").is_none() {
                let result = std::process::Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", std::thread::current().name().unwrap()])
                    .env("FOLDERLENS_STATUS_TEST_CHILD", "1")
                    .output()
                    .unwrap();
                assert!(
                    result.status.success(),
                    "{}",
                    String::from_utf8_lossy(&result.stdout)
                );
                return;
            }
            APPLY
                .set(Box::new(|show| VISIBLE.store(show, Ordering::SeqCst)))
                .ok()
                .unwrap();
            VISIBLE.store(true, Ordering::SeqCst);
            unsafe {
                end(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                );
            }
            assert!(
                is_visible(),
                "Ending a transient session must not hide FolderLens"
            );
            unsafe {
                begin(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
            assert!(
                !is_visible(),
                "A new tray session must toggle the persistent panel closed"
            );
            unsafe {
                begin(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
            assert!(is_visible(), "The next tray session must reopen FolderLens");
            conceal();
            assert!(
                !is_visible(),
                "Explicit dismissal must still hide FolderLens"
            );
            unsafe {
                end(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                );
            }
            assert!(
                !is_visible(),
                "A late popup callback must not reopen FolderLens"
            );
            unsafe {
                click(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
            assert!(
                is_visible(),
                "The explicit tray callback still opens FolderLens"
            );
            unsafe {
                click(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
            assert!(
                !is_visible(),
                "The explicit tray callback still closes FolderLens"
            );
        }
    }

    #[link(name = "objc", kind = "dylib")]
    #[link(name = "AppKit", kind = "framework")]
    unsafe extern "C" {
        fn objc_getClass(name: *const i8) -> *mut c_void;
        fn objc_getProtocol(name: *const i8) -> *mut c_void;
        fn class_addProtocol(cls: *mut c_void, protocol: *mut c_void) -> u8;
        fn sel_registerName(name: *const i8) -> *mut c_void;
        fn objc_msgSend();
        #[cfg(target_arch = "x86_64")]
        fn objc_msgSend_stret();
        fn objc_allocateClassPair(
            superclass: *mut c_void,
            name: *const i8,
            extra_bytes: usize,
        ) -> *mut c_void;
        fn objc_registerClassPair(cls: *mut c_void);
        fn class_addMethod(
            cls: *mut c_void,
            name: *mut c_void,
            imp: *const (),
            types: *const i8,
        ) -> u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_notch_preserves_lower_center_placement() {
        let visible = Rect {
            x: -1440.0,
            y: 80.0,
            w: 1440.0,
            h: 790.0,
        };
        for (width, height) in [crate::notch::COMPACT, crate::notch::EXPANDED] {
            let (x, y) = notch_origin(
                visible,
                -720.0,
                width.into(),
                height.into(),
                crate::prefs::Prefs::default().notch_position,
            )
            .unwrap();
            assert_eq!(y + f64::from(height), 829.0);
            assert!(x >= visible.x && x + f64::from(width) <= visible.x + visible.w);
        }
        assert!(crate::notch::COMPACT.0 <= 300.0);
    }

    #[test]
    fn notch_expansion_keeps_center_and_clears_camera_on_offset_displays() {
        let visible = Rect {
            x: -1440.0,
            y: 80.0,
            w: 1440.0,
            h: 790.0,
        };
        let (width, height) = crate::notch::COMPACT;
        assert_eq!(
            notch_origin(
                visible,
                -720.0,
                width.into(),
                height.into(),
                NotchPosition::Center
            ),
            Some((-870.0, 834.0))
        );
        assert_eq!(
            notch_origin(visible, -720.0, 480.0, 230.0, NotchPosition::Center),
            Some((-960.0, 640.0))
        );
        assert_eq!(
            notch_origin(
                Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 1280.0,
                    h: 770.0
                },
                640.0,
                width.into(),
                height.into(),
                NotchPosition::Center
            ),
            Some((490.0, 734.0))
        );
    }

    #[test]
    fn positions_fit_compact_and_expanded_on_offset_and_tight_screens() {
        for (width, height) in [(300.0, 36.0), (480.0, 230.0)] {
            for screen_width in [width, width + 15.0, 640.0, 1920.0] {
                let visible = Rect {
                    x: -1730.0,
                    y: 45.0,
                    w: screen_width,
                    h: 900.0,
                };
                let center = visible.x + visible.w * 0.5;
                for position in [
                    NotchPosition::Left,
                    NotchPosition::Right,
                    NotchPosition::Center,
                    NotchPosition::Below,
                ] {
                    let (x, y) = notch_origin(visible, center, width, height, position).unwrap();
                    assert!(x >= visible.x && x + width <= visible.x + visible.w);
                    assert!(y >= visible.y && y + height <= visible.y + visible.h);
                    assert_eq!(
                        y + height,
                        if position == NotchPosition::Below {
                            904.0
                        } else {
                            945.0
                        }
                    );
                    if screen_width >= width + 16.0 {
                        match position {
                            NotchPosition::Left => assert_eq!(x, visible.x + 8.0),
                            NotchPosition::Right => {
                                assert_eq!(x + width, visible.x + visible.w - 8.0)
                            }
                            _ => assert_eq!(x + width * 0.5, center),
                        }
                    }
                }
            }
        }
        // A left Dock changes edge positions, not the full display's center anchor.
        let dock_area = Rect {
            x: 80.0,
            y: 0.0,
            w: 1520.0,
            h: 900.0,
        };
        assert_eq!(
            notch_origin(dock_area, 800.0, 480.0, 230.0, NotchPosition::Right),
            Some((1112.0, 670.0))
        );
        assert_eq!(
            notch_origin(dock_area, 800.0, 480.0, 230.0, NotchPosition::Left),
            Some((88.0, 670.0))
        );
        assert_eq!(
            notch_origin(dock_area, 800.0, 480.0, 230.0, NotchPosition::Center),
            Some((560.0, 670.0))
        );
        assert_eq!(
            notch_origin(
                Rect {
                    x: 0.0,
                    y: 50.0,
                    w: 480.0,
                    h: 230.0
                },
                240.0,
                480.0,
                230.0,
                NotchPosition::Below
            ),
            Some((0.0, 50.0))
        );
        for (w, h) in [(479.0, 230.0), (480.0, 229.0), (299.0, 36.0), (300.0, 35.0)] {
            let area = Rect {
                x: -800.0,
                y: 40.0,
                w,
                h,
            };
            assert_eq!(
                notch_origin(area, -560.0, 480.0, 230.0, NotchPosition::Right),
                None
            );
            assert_eq!(
                notch_origin(area, -560.0, 300.0, 36.0, NotchPosition::Right).is_some(),
                w >= 300.0 && h >= 36.0
            );
        }
    }

    #[test]
    fn only_status_button_presses_dismiss_not_update_popup_clicks() {
        let button = Rect {
            x: -220.0,
            y: 605.0,
            w: 24.0,
            h: 22.0,
        };
        assert!(button_pressed(button, -210.0, 615.0, true));
        assert!(!button_pressed(button, -210.0, 615.0, false));
        assert!(!button_pressed(button, -300.0, 350.0, true));
        for (x, y) in [
            (-501.0, 350.0),
            (-139.0, 350.0),
            (-300.0, 199.0),
            (-300.0, 601.0),
            (-195.0, 602.0),
        ] {
            assert!(
                !button_pressed(button, x, y, false),
                "Hover must not dismiss: {x}, {y}"
            );
            assert!(
                !button_pressed(button, x, y, true),
                "Closing a separate update popup must leave FolderLens visible: {x}, {y}"
            );
        }
    }

    #[test]
    fn the_panel_sits_under_the_status_button() {
        let (x, y) = popover_origin(
            Rect {
                x: 1000.0,
                y: 800.0,
                w: 24.0,
                h: 22.0,
            },
            Screen {
                x: 0.0,
                y: 0.0,
                w: 1400.0,
                h: 900.0,
                scale: 2.0,
            },
            360.0,
        );
        assert_eq!((x, y), (1664, 200));

        let (x, y) = popover_origin(
            Rect {
                x: 0.0,
                y: 878.0,
                w: 20.0,
                h: 22.0,
            },
            Screen {
                x: 0.0,
                y: 0.0,
                w: 400.0,
                h: 900.0,
                scale: 2.0,
            },
            360.0,
        );
        assert_eq!((x, y), (16, 44));
    }
}
