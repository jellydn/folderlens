//! Update policy controls, separate from provider and privacy settings.
use super::*;

const CHECKING_NOTE: &str = "Checking in a separate native dialog. Download and verify, then Reveal in Finder. Final replacement is manual; Homebrew apps use brew upgrade.";
const STABLE_NOTE: &str = "Stable releases are the default. Check for Updates offers a verified download, then Reveal in Finder for manual installation. Automatic downloads need daily checks enabled. FolderLens never replaces your app or installs on quit.";
const SIGNATURE_NOTE: &str = "Publisher signatures and checksums are required. Builds are ad-hoc signed, not notarized. macOS may require Privacy & Security → Open Anyway. FolderLens never bypasses Gatekeeper.";
const HOMEBREW_NOTE: &str = "Homebrew apps are not replaced. Use brew upgrade --cask jellydn/tap/folderlens; the stable cask excludes prereleases.";

pub(super) fn view(state: &Settings) -> El {
    col()
        .w_full()
        .gap(14.0)
        .child(heading("Updates"))
        .child(
            text(format!(
                "FolderLens {} · Apple Silicon",
                env!("CARGO_PKG_VERSION")
            ))
            .font(type_::LABEL)
            .color(paint(MUTED)),
        )
        .child(card(
            col()
                .w_full()
                .child(setting_row(
                    "Check daily",
                    toggle("update-checks", state.prefs.update_checks, |s| {
                        s.prefs.update_checks = !s.prefs.update_checks;
                        persist(s);
                    }),
                ))
                .child(setting_row(
                    "Download automatically",
                    toggle("update-downloads", state.prefs.update_downloads, |s| {
                        s.prefs.update_downloads = !s.prefs.update_downloads;
                        persist(s);
                    }),
                ))
                .child(setting_row(
                    "Include prereleases",
                    toggle("update-prereleases", state.prefs.update_prereleases, |s| {
                        s.prefs.update_prereleases = !s.prefs.update_prereleases;
                        persist(s);
                    }),
                )),
        ))
        .child(note(STABLE_NOTE))
        .child(note(SIGNATURE_NOTE))
        .child(note(HOMEBREW_NOTE))
        .child(
            row().w_full().justify_end().child(
                button("Check for Updates…", ButtonStyle::Primary)
                    .key("check-updates")
                    .on_press(|s: &mut Settings| {
                        s.update_note = match crate::update::check() {
                            Ok(()) => CHECKING_NOTE.into(),
                            Err(error) => error.into(),
                        };
                    }),
            ),
        )
        .child(note(state.update_note.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    #[test]
    fn update_controls_keep_stable_defaults_and_persist_explicit_opt_ins() {
        let mut state = state();
        state.page = Page::Updates;
        let path = state.path.clone();
        assert!(state.prefs.update_checks);
        assert!(!state.prefs.update_downloads && !state.prefs.update_prereleases);
        let mut runtime = Runtime::new(
            state,
            super::super::view,
            (760.0, 560.0),
            Env {
                system_dark: true,
                scale: 2.0,
            },
        );
        runtime.frame(0.0);
        for key in ["update-downloads", "update-prereleases", "update-checks"] {
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
