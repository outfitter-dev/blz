#![allow(dead_code)]

use std::sync::{Mutex, OnceLock};

#[allow(clippy::expect_used)]
pub(crate) fn env_mutex() -> &'static Mutex<()> {
    static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_MUTEX.get_or_init(Mutex::default)
}
