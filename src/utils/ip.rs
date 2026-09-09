//! Анонимизация IP-адресов клиентов (P5: конфиденциальность не обсуждается).
//!
//! В БД хранится только хэш IP, чтобы аналитика по-прежнему различала
//! посетителей без хранения персональных данных в открытом виде.

use sha2::{Digest, Sha256};

/// Хэширует IP-адрес (SHA256, первые 32 hex-символа).
pub fn hash_ip(ip: &str) -> String {
    let digest = Sha256::digest(ip.as_bytes());
    hex::encode(digest)[..32].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_and_anonymized() {
        let h1 = hash_ip("192.168.1.10");
        let h2 = hash_ip("192.168.1.10");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 32);
        assert!(!h1.contains("192.168"));
    }

    #[test]
    fn different_ips_have_different_hashes() {
        assert_ne!(hash_ip("1.1.1.1"), hash_ip("2.2.2.2"));
    }
}
