pub const BIND_ADDR_ENV: &str = "FUDOKI_BIND_ADDR";
pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8000";

pub fn bind_addr_from_env() -> String {
    std::env::var(BIND_ADDR_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_BIND_ADDR.to_string())
}
