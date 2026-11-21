//! Update policy controls, separate from provider and privacy settings.
use super::*;

pub(super) const CHECKING_NOTE: &str = "Checking in a separate native dialog. Download and verify, then Reveal in Finder. Final replacement is manual; Homebrew apps use brew upgrade.";
const STABLE_NOTE: &str = "Stable releases are the default. Check for updates offers a verified download, then Reveal in Finder for manual installation. Automatic downloads need daily checks enabled. FolderLens never replaces your app or installs on quit.";
const SIGNATURE_NOTE: &str = "Publisher signatures and checksums are required. Builds are ad-hoc signed, not notarized. macOS may require Privacy & Security → Open Anyway. FolderLens never bypasses Gatekeeper.";
const HOMEBREW_NOTE: &str = "Homebrew apps are not replaced. Use brew upgrade --cask jellydn/tap/folderlens; the stable cask excludes prereleases.";

/// General › Advanced: the update channel switches.
pub(super) fn rows(state: &Settings) -> Vec<El> {
    let p = &state.prefs;
    vec![
        setting(
            &icons::REFRESH,
            "Check for updates daily",
            "",
            switch("update-checks", p.update_checks, |s| {
                s.prefs.update_checks = !s.prefs.update_checks;
                persist(s);
            }),
        ),
        setting(
            &icons::DOWNLOAD,
            "Download updates automatically",
            "Needs daily checks. Never installs on quit.",
            switch("update-downloads", p.update_downloads, |s| {
                s.prefs.update_downloads = !s.prefs.update_downloads;
                persist(s);
            }),
        ),
        setting(
            &icons::FLASK,
            "Include prereleases",
            "The Homebrew cask stays on stable",
            switch("update-prereleases", p.update_prereleases, |s| {
                s.prefs.update_prereleases = !s.prefs.update_prereleases;
                persist(s);
            }),
        ),
    ]
}

pub(super) fn notes() -> El {
    col()
        .w_full()
        .gap(8.0)
        .child(note(STABLE_NOTE))
        .child(note(SIGNATURE_NOTE))
        .child(note(HOMEBREW_NOTE))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    #[test]
    fn update_controls_keep_stable_defaults_and_persist_explicit_opt_ins() {
        let mut state = state();
        state.advanced_open = true;
        let path = state.path.clone();
        assert!(state.prefs.update_checks);
        assert!(!state.prefs.update_downloads && !state.prefs.update_prereleases);
        let mut runtime = Runtime::new(
            state,
            super::super::view,
            (820.0, 580.0),
            Env {
                system_dark: true,
                scale: 2.0,
            },
        );
        runtime.frame(0.0);
        for key in ["update-downloads", "update-prereleases", "update-checks"] {
            runtime.update(|_, tx| tx.scroll_into_view(key));
            runtime.frame(0.0);
            let (x, y) = runtime.find(key).unwrap();
            runtime.pointer_down(x, y);
            runtime.pointer_up(x, y);
            runtime.frame(0.0);
        }
        let loaded = prefs::load_from(&path);
        assert!(!loaded.update_checks);
        assert!(loaded.update_downloads && loaded.update_prereleases);
        assert!(!loaded.allow_start && !loaded.send_paths);
        std::fs::remove_file(path).unwrap();
    }
}
