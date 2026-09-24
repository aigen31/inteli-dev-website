//! Services Layer — бизнес-логика (orchestration).
//!
//! Слои ниже (memory/storage/llm) не знают про HTTP и друг про друга; этот слой
//! связывает их: чатбот, статус занятости, приём заявок, статьи блога и
//! SEO-поверхность (robots.txt, IndexNow).

pub mod article;
pub mod chat;
pub mod feed;
pub mod github;
pub mod lead;
pub mod seo;
pub mod settings_bot;
pub mod status;

pub use article::{Article, ArticleHead, ArticleService, ArticleStatus, PublishedArticles};
pub use chat::ChatService;
pub use github::{GitHubProfile, GitHubService, GitHubStats};
pub use lead::LeadService;
pub use seo::IndexNow;
pub use settings_bot::SettingsBot;
