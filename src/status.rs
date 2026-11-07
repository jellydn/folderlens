//! The menu-bar item that shows and hides the panel.
//!
//! Rune has no macOS status item. Shelf installs its own. A headless snap and a
//! unit test leave the panel where it is. The coordinate helper is pure so the
//! position can be checked without AppKit.

use std::sync::OnceLock;

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    /// The button's height. The panel uses `y`, the bottom edge, as its top.
    #[allow(dead_code)]
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

static START: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

/// Hides the panel and clears the menu-bar highlight.
/// Settings calls this so the panel is gone before Settings opens.
pub fn conceal() {
    #[cfg(target_os = "macos")]
    macos::conceal();
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
    use std::sync::OnceLock;

    use super::{popover_origin, Rect, Screen};

    static ITEM: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
    static VISIBLE: AtomicBool = AtomicBool::new(true);
    static APPLY: OnceLock<Box<dyn Fn(bool) + Send + Sync>> = OnceLock::new();

    pub fn install<S: 'static>(handle: rune::AppHandle<S>, window_w: f32) {
        let installed = unsafe { install_item() };
        if !installed {
            return;
        }
        let handle_for_click = handle.clone();
        let _ = APPLY.set(Box::new(move |show| {
            VISIBLE.store(show, Ordering::SeqCst);
            if show {
                crate::settings::hide();
                set_highlight(true);
                if let Some(origin) = current_origin(window_w) {
                    handle_for_click.set_position(origin);
                }
                handle_for_click.show();
            } else {
                set_highlight(false);
                handle_for_click.hide();
            }
        }));
        VISIBLE.store(false, Ordering::SeqCst);
        handle.hide();
    }

    pub(super) fn conceal() {
        apply(false);
    }

    fn apply(show: bool) {
        if let Some(apply) = APPLY.get() {
            apply(show);
        }
    }

    /// Returns false when AppKit did not hand back a button.
    ///
    /// The item stays retained for the process. A local autorelease pool here
    /// frees it before the status bar keeps it.
    unsafe fn install_item() -> bool {
        let _app = msg0(class(c"NSApplication"), sel(c"sharedApplication"));
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
        msg_void_ptr(item, sel(c"setAutosaveName:"), ns_string("Shelf"));
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
        msg_void_ptr(button, sel(c"setTarget:"), target);
        msg_void_ptr(button, sel(c"setAction:"), sel(c"click:"));
        ITEM.store(item, Ordering::Release);
        true
    }

    fn set_icon(button: *mut c_void) {
        let name = ns_string("archivebox");
        let image = msg_ptr2(
            class(c"NSImage"),
            sel(c"imageWithSystemSymbolName:accessibilityDescription:"),
            name,
            ns_string("Shelf"),
        );
        if image.is_null() {
            msg_void_ptr(button, sel(c"setTitle:"), ns_string("Shelf"));
            return;
        }
        msg_bool(image, sel(c"setTemplate:"), true);
        msg_void_ptr(button, sel(c"setImage:"), image);
        msg_void_ptr(button, sel(c"setToolTip:"), ns_string("Shelf"));
    }

    fn make_target() -> *mut c_void {
        static CLASS: OnceLock<usize> = OnceLock::new();
        let cls = *CLASS.get_or_init(|| unsafe {
            let super_cls = objc_getClass(c"NSObject".as_ptr());
            let cls = objc_allocateClassPair(super_cls, c"ShelfStatusTarget".as_ptr(), 0);
            if cls.is_null() {
                return objc_getClass(c"ShelfStatusTarget".as_ptr()) as usize;
            }
            add_method(cls, c"click:", click as *const (), c"v@:@");
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
        apply(!VISIBLE.load(Ordering::SeqCst));
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
            f64::from(window_w),
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
            .unwrap_or_else(|_| std::ffi::CString::new("Shelf").unwrap());
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

    #[link(name = "objc", kind = "dylib")]
    #[link(name = "AppKit", kind = "framework")]
    unsafe extern "C" {
        fn objc_getClass(name: *const i8) -> *mut c_void;
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
