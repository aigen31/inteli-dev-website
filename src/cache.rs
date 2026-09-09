//! Кэш уровня приложения (L2): in-memory с TTL.
//!
//! Для MVP используется встроенный in-memory кэш вместо Redis (Redis — этап P7).
//! Интерфейс изолирован, чтобы позже подставить Redis-реализацию без изменения
//! бизнес-логики. Плюс простой fixed-window rate limiter для `/api/chat`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Простой in-memory кэш с TTL (запись живёт `ttl` секунд).
#[derive(Debug, Clone)]
pub struct InMemoryCache {
    ttl: Duration,
    store: Arc<Mutex<HashMap<String, (String, Instant)>>>,
}

impl InMemoryCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Возвращает значение, если оно есть и не протухло.
    pub fn get(&self, key: &str) -> Option<String> {
        let mut store = self.store.lock().expect("cache mutex poisoned");
        match store.get(key) {
            Some((value, inserted)) if inserted.elapsed() < self.ttl => Some(value.clone()),
            Some(_) => {
                store.remove(key);
                None
            }
            None => None,
        }
    }

    /// Записывает значение в кэш.
    pub fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        let mut store = self.store.lock().expect("cache mutex poisoned");
        store.insert(key.into(), (value.into(), Instant::now()));
    }
}

/// Fixed-window rate limiter по строковому ключу (обычно хэш IP).
#[derive(Debug, Clone)]
pub struct RateLimiter {
    max_requests: u32,
    window: Duration,
    state: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            state: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Проверяет, разрешён ли запрос по ключу. `true` = можно выполнять.
    pub fn check(&self, key: &str) -> bool {
        let mut state = self.state.lock().expect("rate limiter mutex poisoned");
        let now = Instant::now();

        let count = match state.get(key) {
            Some((count, window_start)) if now.duration_since(*window_start) < self.window => {
                *count
            }
            _ => 0,
        };

        if count >= self.max_requests {
            false
        } else {
            state.insert(key.to_string(), (count + 1, now));
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_returns_value_within_ttl() {
        let cache = InMemoryCache::new(Duration::from_secs(60));
        cache.set("k", "v");
        assert_eq!(cache.get("k").as_deref(), Some("v"));
    }

    #[test]
    fn cache_expires_after_ttl() {
        let cache = InMemoryCache::new(Duration::from_millis(1));
        cache.set("k", "v");
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(cache.get("k"), None);
    }

    #[test]
    fn rate_limiter_blocks_after_limit() {
        let rl = RateLimiter::new(3, Duration::from_secs(60));
        assert!(rl.check("ip"));
        assert!(rl.check("ip"));
        assert!(rl.check("ip"));
        assert!(!rl.check("ip"));
    }

    #[test]
    fn rate_limiter_isolation_between_keys() {
        let rl = RateLimiter::new(1, Duration::from_secs(60));
        assert!(rl.check("a"));
        assert!(!rl.check("a"));
        assert!(rl.check("b"));
    }
}
