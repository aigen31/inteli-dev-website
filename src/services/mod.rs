//! Services Layer — бизнес-логика (orchestration).
//!
//! Слои ниже (memory/storage/llm) не знают про HTTP и друг про друга; этот слой
//! связывает их: чатбот, статус занятости, приём заявок.

pub mod chat;
pub mod lead;
pub mod status;

pub use chat::ChatService;
pub use lead::LeadService;
