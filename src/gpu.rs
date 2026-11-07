//! Read the driver's typed PerformanceStatistics instead of parsing ioreg's display text.

#[cfg(target_os = "macos")]
pub fn percent() -> Option<u32> {
    use std::ffi::c_void;
    use std::ptr;

    type CFRef = *const c_void;
    #[link(name = "IOKit", kind = "framework")]
    unsafe extern "C" {
        fn IOServiceMatching(name: *const i8) -> CFRef;
        fn IOServiceGetMatchingServices(port: u32, matching: CFRef, iterator: *mut u32) -> i32;
        fn IOIteratorNext(iterator: u32) -> u32;
        fn IOObjectRelease(object: u32) -> i32;
        fn IORegistryEntryCreateCFProperty(
            entry: u32,
            key: CFRef,
            allocator: CFRef,
            options: u32,
        ) -> CFRef;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFStringCreateWithCString(allocator: CFRef, string: *const i8, encoding: u32) -> CFRef;
        fn CFDictionaryGetValue(dictionary: CFRef, key: CFRef) -> CFRef;
        fn CFGetTypeID(value: CFRef) -> usize;
        fn CFDictionaryGetTypeID() -> usize;
        fn CFNumberGetTypeID() -> usize;
        fn CFNumberGetValue(number: CFRef, kind: isize, value: *mut f64) -> bool;
        fn CFRelease(value: CFRef);
    }

    unsafe {
        let mut iterator = 0;
        if IOServiceGetMatchingServices(
            0,
            IOServiceMatching(c"IOAccelerator".as_ptr()),
            &mut iterator,
        ) != 0
        {
            return None;
        }
        let key =
            CFStringCreateWithCString(ptr::null(), c"PerformanceStatistics".as_ptr(), 0x08000100);
        let names = [
            c"Device Utilization %",
            c"Renderer Utilization %",
            c"Tiler Utilization %",
        ];
        let keys =
            names.map(|name| CFStringCreateWithCString(ptr::null(), name.as_ptr(), 0x08000100));
        let mut best = None;
        loop {
            let service = IOIteratorNext(iterator);
            if service == 0 {
                break;
            }
            let stats = IORegistryEntryCreateCFProperty(service, key, ptr::null(), 0);
            if !stats.is_null() {
                if CFGetTypeID(stats) == CFDictionaryGetTypeID() {
                    // Device load is authoritative, even at zero. Older drivers may
                    // expose only renderer/tiler load; use their maximum as a fallback.
                    let mut values = [None; 3];
                    for (index, key) in keys.iter().enumerate() {
                        let number = CFDictionaryGetValue(stats, *key);
                        let mut value = 0.0_f64;
                        if !number.is_null()
                            && CFGetTypeID(number) == CFNumberGetTypeID()
                            && CFNumberGetValue(number, 6, &mut value)
                            && value.is_finite()
                        {
                            values[index] = Some(value.round().clamp(0.0, 100.0) as u32);
                        }
                    }
                    if let Some(value) =
                        values[0].or_else(|| values[1..].iter().flatten().copied().max())
                    {
                        best = Some(best.unwrap_or(0).max(value));
                    }
                }
                CFRelease(stats);
            }
            IOObjectRelease(service);
        }
        for key in keys {
            CFRelease(key);
        }
        CFRelease(key);
        IOObjectRelease(iterator);
        best
    }
}

#[cfg(not(target_os = "macos"))]
pub fn percent() -> Option<u32> {
    None
}
