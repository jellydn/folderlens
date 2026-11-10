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
    Some((macos.join("FolderLensUpdater"), bundle.into()))
}

fn command(manual: bool) -> Result<Command, &'static str> {
    let executable = std::env::current_exe().map_err(|_| "Cannot locate FolderLens.app")?;
    let (helper, bundle) = helper(&executable)
        .ok_or("Update checks need the bundled FolderLens.app. Build with sh scripts/bundle.sh.")?;
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
        if !command.status().is_ok_and(|status| status.success()) {
            crate::settings::update_status("The update helper could not start or stopped unexpectedly. Check the installed app before retrying, or download a verified release manually.");
        }
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
        assert!(helper(Path::new("/tmp/folderlens")).is_none());
        assert!(helper(Path::new("/tmp/FolderLens.app/Other/FolderLens")).is_none());
        assert_eq!(
            helper(Path::new("/Applications/FolderLens.app/Contents/MacOS/FolderLens")),
            Some((
                PathBuf::from("/Applications/FolderLens.app/Contents/MacOS/FolderLensUpdater"),
                PathBuf::from("/Applications/FolderLens.app")
            ))
        );
    }
}
