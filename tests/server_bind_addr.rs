use std::sync::{Mutex, OnceLock};

use fudoki_backend::server::{bind_addr_from_env, BIND_ADDR_ENV, DEFAULT_BIND_ADDR};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvGuard {
    _guard: std::sync::MutexGuard<'static, ()>,
    original: Option<String>,
}

impl EnvGuard {
    fn new() -> Self {
        let guard = env_lock().lock().unwrap();
        let original = std::env::var(BIND_ADDR_ENV).ok();
        std::env::remove_var(BIND_ADDR_ENV);
        Self {
            _guard: guard,
            original,
        }
    }

    fn set(&self, value: &str) {
        std::env::set_var(BIND_ADDR_ENV, value);
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => std::env::set_var(BIND_ADDR_ENV, value),
            None => std::env::remove_var(BIND_ADDR_ENV),
        }
    }
}

#[test]
fn bind_addr_defaults_to_localhost_port_8000() {
    let _env = EnvGuard::new();

    assert_eq!(bind_addr_from_env(), DEFAULT_BIND_ADDR);
}

#[test]
fn bind_addr_uses_trimmed_env_value_for_lan_sharing() {
    let env = EnvGuard::new();
    env.set("  0.0.0.0:8000  ");

    assert_eq!(bind_addr_from_env(), "0.0.0.0:8000");
}

#[test]
fn bind_addr_ignores_blank_env_value() {
    let env = EnvGuard::new();
    env.set("   ");

    assert_eq!(bind_addr_from_env(), DEFAULT_BIND_ADDR);
}
