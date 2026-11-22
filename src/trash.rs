//! Move to Trash (ADR 4): the only path in FolderLens that moves user files.
//!
//! Only a press on the confirmation sheet's destructive button calls [`run`]. Each item is
//! rechecked just before it moves: the caller confirms it is still an eligible ticked leaf
//! with the same size class, then the item is opened with `O_NOFOLLOW` and its `F_GETPATH`
//! must equal the reviewed path. A failed check skips that item. The first refused move
//! stops the run. Items go to the Trash with `NSFileManager trashItemAtURL`; nothing is
//! deleted permanently and the Trash is never emptied.

use std::ffi::{c_void, CStr, CString};
use std::fs::File;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// One reviewed item, as the confirmation sheet listed it.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub path: String,
    pub name: String,
    pub bytes: u64,
}

#[derive(Debug, Default, PartialEq)]
pub struct Report {
    pub moved: Vec<Item>,
    /// Items that failed a recheck, with the reason. The run went on without them.
    pub skipped: Vec<(Item, String)>,
    /// The item whose move failed, with the reason. The run stopped there.
    pub stopped: Option<(Item, String)>,
    /// Items after the stop that were not tried.
    pub not_tried: usize,
}

/// A size estimate "changes class" when its decimal order of magnitude changes.
pub fn size_class(bytes: u64) -> u32 {
    bytes.checked_ilog10().unwrap_or(0)
}

/// Moves `items` in order. `eligible` rechecks the item against the current selection;
/// `trash` performs the move ([`to_trash`] outside tests).
pub fn run(
    items: &[Item],
    eligible: impl Fn(&Item) -> Result<(), String>,
    mut trash: impl FnMut(&Path) -> Result<(), String>,
) -> Report {
    let mut report = Report::default();
    for (index, item) in items.iter().enumerate() {
        let path = Path::new(&item.path);
        if let Err(reason) = eligible(item).and_then(|()| unchanged(path)) {
            report.skipped.push((item.clone(), reason));
            continue;
        }
        if let Err(reason) = trash(path) {
            report.stopped = Some((item.clone(), reason));
            report.not_tried = items.len() - index - 1;
            break;
        }
        report.moved.push(item.clone());
    }
    report
}

/// The reviewed path still names the same kind of object: no symlink in its place and no
/// moved parent folder.
fn unchanged(path: &Path) -> Result<(), String> {
    let file = File::options()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|error| match error.raw_os_error() {
            Some(libc::ENOENT) => "it is missing".to_string(),
            Some(libc::ELOOP) => "it became a link".to_string(),
            _ => "macOS denied access to it".to_string(),
        })?;
    if crate::agent::opened_path(&file)? != path {
        return Err("its location changed after review".into());
    }
    Ok(())
}

impl Report {
    pub fn summary(&self) -> String {
        let bytes: u64 = self.moved.iter().map(|item| item.bytes).sum();
        let mut parts = vec![match self.moved.len() {
            0 => "Nothing moved to the Trash.".to_string(),
            n => format!(
                "Moved {n} {} to the Trash (estimated {}).",
                if n == 1 { "item" } else { "items" },
                crate::catalog::format_bytes(bytes)
            ),
        }];
        for (item, reason) in &self.skipped {
            parts.push(format!("Skipped {}: {reason}.", item.name));
        }
        if let Some((item, reason)) = &self.stopped {
            parts.push(format!("Stopped at {}: {reason}.", item.name));
            if self.not_tried > 0 {
                parts.push(format!("{} not tried.", self.not_tried));
            }
        }
        parts.join(" ")
    }
}

/// Moves one item to the Trash of its volume. `trashItemAtURL` takes a path, not the
/// descriptor `unchanged` checked, so a parent folder swapped between the two by a process of
/// this user is not caught; such a process could already change the files itself.
pub fn to_trash(path: &Path) -> Result<(), String> {
    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| "invalid path")?;
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let result = trash_in_pool(&path);
        objc_autoreleasePoolPop(pool);
        result
    }
}

unsafe fn trash_in_pool(path: &CStr) -> Result<(), String> {
    let send1: unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const ());
    let send0: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const ());
    let trash_item: unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        *mut c_void,
        *mut *mut c_void,
        *mut *mut c_void,
    ) -> i8 = std::mem::transmute(objc_msgSend as *const ());
    let string = send1(
        class(c"NSString"),
        sel(c"stringWithUTF8String:"),
        path.as_ptr().cast(),
    );
    if string.is_null() {
        return Err("macOS could not prepare the move".into());
    }
    let url = send1(class(c"NSURL"), sel(c"fileURLWithPath:"), string.cast());
    let manager = send0(class(c"NSFileManager"), sel(c"defaultManager"));
    if url.is_null() || manager.is_null() {
        return Err("macOS could not prepare the move".into());
    }
    let mut resulting = std::ptr::null_mut();
    let mut error = std::ptr::null_mut();
    let selector = sel(c"trashItemAtURL:resultingItemURL:error:");
    if trash_item(manager, selector, url, &mut resulting, &mut error) != 0 {
        return Ok(());
    }
    if error.is_null() {
        return Err("macOS refused the move".into());
    }
    let text = send0(
        send0(error, sel(c"localizedDescription")),
        sel(c"UTF8String"),
    );
    if text.is_null() {
        return Err("macOS refused the move".into());
    }
    Err(CStr::from_ptr(text.cast()).to_string_lossy().into_owned())
}

fn class(name: &CStr) -> *mut c_void {
    unsafe { objc_getClass(name.as_ptr()) }
}

fn sel(name: &CStr) -> *mut c_void {
    unsafe { sel_registerName(name.as_ptr()) }
}

#[link(name = "objc", kind = "dylib")]
#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {
    fn objc_getClass(name: *const std::ffi::c_char) -> *mut c_void;
    fn sel_registerName(name: *const std::ffi::c_char) -> *mut c_void;
    fn objc_msgSend();
    fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A fresh folder under the temp dir, with symlinks resolved so `F_GETPATH` matches.
    fn fixture(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("folderlens-trash-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root.canonicalize().unwrap()
    }

    fn item(root: &Path, name: &str) -> Item {
        let path = root.join(name);
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("data"), b"x").unwrap();
        Item {
            path: path.display().to_string(),
            name: name.into(),
            bytes: 2_000_000,
        }
    }

    #[test]
    fn a_symlink_swapped_in_after_review_never_reaches_the_trash_call() {
        let root = fixture("swap");
        let reviewed = item(&root, "cache");
        let elsewhere = item(&root, "documents");
        std::fs::remove_dir_all(&reviewed.path).unwrap();
        std::os::unix::fs::symlink(&elsewhere.path, &reviewed.path).unwrap();
        let mut calls = Vec::new();
        let report = run(
            std::slice::from_ref(&reviewed),
            |_| Ok(()),
            |path| {
                calls.push(path.to_path_buf());
                Ok(())
            },
        );
        assert!(calls.is_empty());
        assert!(report.moved.is_empty());
        assert_eq!(report.skipped.len(), 1);
        assert!(Path::new(&elsewhere.path).join("data").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_moved_parent_folder_is_refused() {
        let root = fixture("parent");
        let real = root.join("real");
        std::fs::create_dir(&real).unwrap();
        let inner = item(&real, "logs");
        let linked = root.join("linked");
        std::os::unix::fs::symlink(&real, &linked).unwrap();
        let reviewed = Item {
            path: linked.join("logs").display().to_string(),
            ..inner
        };
        let report = run(&[reviewed], |_| Ok(()), |_| panic!("must not move"));
        assert_eq!(report.skipped[0].1, "its location changed after review");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_failure_partway_stops_and_reports_what_moved_and_what_did_not() {
        let root = fixture("partial");
        let items = ["a", "b", "c", "d"].map(|name| item(&root, name));
        let mut calls = Vec::new();
        let report = run(
            &items,
            |item| {
                if item.name == "b" {
                    Err("it is protected now".into())
                } else {
                    Ok(())
                }
            },
            |path| {
                calls.push(path.to_path_buf());
                if path.ends_with("c") {
                    Err("volume has no Trash".into())
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(calls, [root.join("a"), root.join("c")]);
        assert_eq!(report.moved, [items[0].clone()]);
        assert_eq!(
            report.skipped,
            [(items[1].clone(), "it is protected now".into())]
        );
        assert_eq!(
            report.stopped,
            Some((items[2].clone(), "volume has no Trash".into()))
        );
        assert_eq!(report.not_tried, 1);
        assert_eq!(
            report.summary(),
            "Moved 1 item to the Trash (estimated 2 MB). Skipped b: it is protected now. Stopped at c: volume has no Trash. 1 not tried."
        );
        // The fake moved nothing; every fixture is still in place.
        assert!(items.iter().all(|item| Path::new(&item.path).exists()));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn size_class_changes_only_with_the_order_of_magnitude() {
        assert_eq!(size_class(5_700_000_000), size_class(9_900_000_000));
        assert_ne!(size_class(9_900_000_000), size_class(10_100_000_000));
        assert_eq!(size_class(0), size_class(9));
    }

    /// Moves a temp folder to the user's real Trash. Run by hand only:
    /// `cargo test --locked real_trash -- --ignored`.
    #[test]
    #[ignore]
    fn real_trash_moves_a_temp_folder() {
        let root = fixture("real");
        let target = item(&root, "folderlens-trash-check");
        to_trash(Path::new(&target.path)).unwrap();
        assert!(!Path::new(&target.path).exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
