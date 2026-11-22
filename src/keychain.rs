//! The Local AI API key in the login Keychain, never in `settings.txt`.

use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

const SERVICE: &str = "io.github.jellydn.folderlens";
const ACCOUNT: &str = "local-api-key";

/// Can show a Keychain prompt (an ad-hoc signed rebuild is a new app to the Keychain), so call
/// it from a worker, never the frame loop.
pub fn local_api_key() -> Option<String> {
    let bytes = get_generic_password(SERVICE, ACCOUNT).ok()?;
    String::from_utf8(bytes).ok().filter(|key| !key.is_empty())
}

pub fn set_local_api_key(key: &str) -> Result<(), String> {
    set_generic_password(SERVICE, ACCOUNT, key.as_bytes()).map_err(|e| format!("Keychain: {e}"))
}

pub fn remove_local_api_key() -> Result<(), String> {
    match delete_generic_password(SERVICE, ACCOUNT) {
        // errSecItemNotFound: nothing to remove is not an error.
        Err(e) if e.code() != -25300 => Err(format!("Keychain: {e}")),
        _ => Ok(()),
    }
}
