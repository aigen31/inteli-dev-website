//! Валидация пользовательского ввода (zero-trust к внешним данным).

/// Проверяет базовую валидность email (без внешних крейтов).
pub fn is_valid_email(email: &str) -> bool {
    let email = email.trim();
    if email.is_empty() || email.len() > 254 || email.contains(char::is_whitespace) {
        return false;
    }

    let Some((local, domain)) = email.rsplit_once('@') else {
        return false;
    };

    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !email.contains("..")
}

/// Проверяет, что строка похожа на URL (http/https или голый домен).
pub fn looks_like_url(s: &str) -> bool {
    let s = s.trim();
    if s.starts_with("http://") || s.starts_with("https://") {
        return s.len() > 8;
    }
    // Голый домен: example.com
    s.contains('.') && !s.contains(char::is_whitespace) && !s.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_emails_pass() {
        assert!(is_valid_email("ivan@example.com"));
        assert!(is_valid_email("a.b@sub.domain.ru"));
    }

    #[test]
    fn invalid_emails_rejected() {
        assert!(!is_valid_email(""));
        assert!(!is_valid_email("not-an-email"));
        assert!(!is_valid_email("a@b"));
        assert!(!is_valid_email("a b@c.com"));
        assert!(!is_valid_email("a@.com"));
    }

    #[test]
    fn urls_are_detected() {
        assert!(looks_like_url("https://example.com"));
        assert!(looks_like_url("example.com"));
        assert!(!looks_like_url("hello world"));
        assert!(!looks_like_url("notaurl"));
    }
}
