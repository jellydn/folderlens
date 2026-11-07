//! Native update helper: no network or process waits on Rune's UI thread.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

fn helper(executable: &Path) -> Option<(PathBuf, PathBuf)> {
    let macos = executable.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    if macos.file_name()? != "MacOS"
        || contents.file_name()? != "Contents"
        || bundle.extension()? != "app"
    {
        return None;
    }
    Some((macos.join("ShelfUpdater"), bundle.into()))
}

fn command(manual: bool) -> Result<Command, &'static str> {
    let executable = std::env::current_exe().map_err(|_| "Cannot locate Shelf.app")?;
    let (helper, bundle) = helper(&executable)
        .ok_or("Update checks need the bundled Shelf.app. Build with sh scripts/bundle.sh.")?;
    if !helper.is_file() {
        return Err("This bundle has no update helper. Download a newer verified release.");
    }
    let mut command = Command::new(helper);
    command
        .arg(if manual { "--manual" } else { "--automatic" })
        .arg(bundle)
        .arg(crate::prefs::user_path())
        .arg(std::process::id().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    Ok(command)
}

pub fn check() -> Result<(), &'static str> {
    let mut command = command(true)?;
    std::thread::spawn(move || {
        let _ = command.status();
    });
    Ok(())
}

pub fn start() {
    if rune::is_headless() {
        return;
    }
    std::thread::spawn(|| loop {
        if let Ok(mut command) = command(false) {
            let _ = command.status();
        }
        // The native helper persists a 24-hour throttle, including across app restarts.
        std::thread::sleep(Duration::from_secs(3600));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_app_bundles_can_start_the_installer() {
        assert!(helper(Path::new("/tmp/shelf")).is_none());
        assert!(helper(Path::new("/tmp/Shelf.app/Other/Shelf")).is_none());
        assert_eq!(
            helper(Path::new("/Applications/Shelf.app/Contents/MacOS/Shelf")),
            Some((
                PathBuf::from("/Applications/Shelf.app/Contents/MacOS/ShelfUpdater"),
                PathBuf::from("/Applications/Shelf.app")
            ))
        );
    }
}
