//! The Local AI server page and its background Keychain and model-list calls.
use super::*;

pub(super) fn listed_models(state: &Settings) -> Vec<El> {
    let p = &state.prefs;
    let mut rows: Vec<El> = p
        .local_models
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let on = p.model() == id;
            let shown = crate::local_ai::short_model_name(id);
            let help = if shown == id { "" } else { id.as_str() };
            let chosen = id.clone();
            setting(&icons::MODEL, shown, help, radio(on))
                .key(format!("local-model-{index}"))
                .role("radio")
                .label(shown.to_string())
                .checked(on)
                .focusable(true)
                .on_press(move |state: &mut Settings| {
                    state
                        .prefs
                        .models
                        .insert(prefs::LOCAL.into(), chosen.clone());
                    load_model(state);
                    persist(state);
                })
        })
        .collect();
    let help = if state.local_note.is_empty() {
        "Models the server reports at /v1/models"
    } else {
        state.local_note.as_str()
    };
    rows.push(setting(
        &icons::SERVER,
        "Server models",
        help,
        list_button(state),
    ));
    rows
}

fn list_button(state: &Settings) -> El {
    let style = if state.local_busy {
        ButtonStyle::Disabled
    } else {
        ButtonStyle::Secondary
    };
    button("List models", style)
        .key("list-models")
        .on_press(start_list_models)
}

const NETWORK_NOTE: &str = "Off: only servers on this Mac. On: also private LAN and Tailscale addresses and names. Public addresses are always refused, also when a name resolves to one.";
const KEY_NOTE: &str =
    "Optional, for llama-server --api-key. Kept in the Keychain and sent as a Bearer token.";

pub(super) fn server(state: &Settings) -> El {
    let p = &state.prefs;
    let preset = |key: &'static str, label: &'static str, url: &'static str| {
        button(label, ButtonStyle::Secondary)
            .key(key)
            .on_press(move |s: &mut Settings| {
                s.endpoint.text = url.into();
                apply_endpoint(s);
            })
    };
    let key_control = if p.local_key_saved {
        button("Remove", ButtonStyle::Secondary)
            .key("remove-key")
            .on_press(remove_key)
    } else {
        row()
            .gap(8.0)
            .center_y()
            .child(
                text_input(&state.api_key, TextStyle::new(13.0, 400.0))
                    .key("api-key")
                    .w(140.0)
                    .h(28.0)
                    .px(8.0)
                    .radius(6.0)
                    .border(1.0, paint(TRACK))
                    .fill(paint(PANEL))
                    .single_line()
                    .placeholder("API key", paint(MUTED))
                    .color(paint(INK))
                    .on_edit(|state: &mut Settings| &mut state.api_key),
            )
            .child(
                button("Save", ButtonStyle::Secondary)
                    .key("save-key")
                    .on_press(save_key),
            )
    };
    let mut rows = vec![
        setting(
            &icons::SERVER,
            "Address",
            "Ends in /v1",
            row()
                .gap(8.0)
                .center_y()
                .child(
                    text_input(&state.endpoint, TextStyle::new(13.0, 400.0))
                        .key("local-endpoint")
                        .w(200.0)
                        .h(28.0)
                        .px(8.0)
                        .radius(6.0)
                        .border(1.0, paint(TRACK))
                        .fill(paint(PANEL))
                        .single_line()
                        .placeholder(crate::local_ai::OLLAMA, paint(MUTED))
                        .color(paint(INK))
                        .on_edit(|state: &mut Settings| &mut state.endpoint),
                )
                .child(
                    button("Apply", ButtonStyle::Primary)
                        .key("apply-endpoint")
                        .on_press(apply_endpoint),
                ),
        ),
        setting(
            &icons::TERMINAL,
            "On this Mac",
            "",
            row()
                .gap(6.0)
                .child(preset("preset-ollama", "Ollama", crate::local_ai::OLLAMA))
                .child(preset(
                    "preset-lm-studio",
                    "LM Studio",
                    crate::local_ai::LM_STUDIO,
                ))
                .child(preset(
                    "preset-llama-cpp",
                    "llama.cpp",
                    crate::local_ai::LLAMA_CPP,
                )),
        ),
        setting(
            &icons::SHARE,
            "Allow LAN and Tailscale servers",
            NETWORK_NOTE,
            switch(
                "local-network",
                "Allow LAN and Tailscale servers",
                p.local_allow_network,
                |s| {
                    s.prefs.local_allow_network = !s.prefs.local_allow_network;
                    persist(s);
                },
            ),
        ),
        setting(
            &icons::KEY,
            "API key",
            if p.local_key_saved {
                "Saved in the Keychain. Sent as a Bearer token."
            } else {
                KEY_NOTE
            },
            key_control,
        ),
        setting(
            &icons::MODEL,
            "Models",
            if p.local_models.is_empty() {
                "None listed yet"
            } else {
                "Choose one in Agent › Model"
            },
            list_button(state),
        ),
    ];
    if !state.local_note.is_empty() {
        rows.push(setting(&icons::NOTE, &state.local_note, "", el()).key("local-note"));
    }
    col()
        .w_full()
        .gap(14.0)
        .child(back("Agent"))
        .child(page_title("Local AI server"))
        .child(card("card", rows))
        .child(note("FolderLens sends item names and estimated sizes, and full paths only with path sharing on. It never sends file contents to Local AI and writes no run folder for it. The server may keep its own logs."))
}

fn apply_endpoint(state: &mut Settings) {
    match crate::local_ai::validate_endpoint(&state.endpoint.text, state.prefs.local_allow_network)
    {
        Ok(url) => {
            if url != state.prefs.local_endpoint {
                // Models belong to a server; a new address lists its own.
                state.prefs.local_models.clear();
            }
            state.prefs.local_endpoint = url;
            load_endpoint(state);
            state.local_note = "Address saved. Use List models to check the server.".into();
            persist(state);
        }
        Err(error) => state.local_note = error,
    }
}

/// Runs `work` on a worker and applies its result through the Settings handle. The Keychain
/// can prompt and the network can be slow, so neither runs in the frame loop.
fn in_background<T: Send + 'static>(
    state: &mut Settings,
    note: &str,
    work: impl FnOnce() -> T + Send + 'static,
    finish: fn(&mut Settings, T),
) {
    if state.local_busy {
        return;
    }
    state.local_busy = true;
    state.local_note = note.into();
    std::thread::spawn(move || {
        let result = work();
        if let Some(handle) = HANDLE.get() {
            handle.update(move |state, _tx: &mut Tx| {
                state.local_busy = false;
                finish(state, result);
            });
        }
    });
}

fn start_list_models(state: &mut Settings) {
    let endpoint = state.prefs.local_endpoint.clone();
    let network = state.prefs.local_allow_network;
    let saved = state.prefs.local_key_saved;
    in_background(
        state,
        "Asking the server for its models…",
        move || {
            let key = saved
                .then(crate::keychain::local_api_key)
                .flatten()
                .map(crate::local_ai::ApiKey);
            let models = crate::local_ai::list_models(&endpoint, network, key.as_ref());
            (endpoint, models)
        },
        finish_list_models,
    );
}

fn finish_list_models(
    state: &mut Settings,
    (endpoint, models): (String, Result<Vec<String>, String>),
) {
    if endpoint != state.prefs.local_endpoint {
        state.local_note = "The address changed while listing. List models again.".into();
        return;
    }
    state.local_note = match models {
        Ok(models) if models.is_empty() => {
            "The server reports no models. Load one in the server first.".into()
        }
        Ok(models) => {
            let note = match models.len() {
                1 => "1 model listed.".to_string(),
                n => format!("{n} models listed."),
            };
            state.prefs.local_models = models;
            persist(state);
            note
        }
        Err(error) => error,
    };
}

fn save_key(state: &mut Settings) {
    let key = state.api_key.text.trim().to_string();
    // The field never keeps the key once Save is pressed.
    state.api_key = TextEdit::default().secure();
    if key.is_empty() {
        return;
    }
    in_background(
        state,
        "Saving the key in the Keychain…",
        move || crate::keychain::set_local_api_key(&key),
        |state, result| match result {
            Ok(()) => {
                state.prefs.local_key_saved = true;
                state.local_note = "API key saved in the Keychain.".into();
                persist(state);
            }
            Err(error) => state.local_note = error,
        },
    );
}

fn remove_key(state: &mut Settings) {
    in_background(
        state,
        "Removing the key from the Keychain…",
        crate::keychain::remove_local_api_key,
        |state, result| match result {
            Ok(()) => {
                state.prefs.local_key_saved = false;
                state.local_note = "API key removed.".into();
                persist(state);
            }
            Err(error) => state.local_note = error,
        },
    );
}
