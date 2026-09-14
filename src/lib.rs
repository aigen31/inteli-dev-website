//! inteli.dev.ru — библиотека приложения.
//!
//! Все модули вынесены в библиотеку, чтобы их можно было тестировать
//! интеграционно (каталог `tests/`) и переиспользовать. Точка входа (boot
//! сервера) находится в `main.rs`.

pub mod api;
pub mod cache;
pub mod config;
pub mod error;
pub mod limits;
pub mod llm;
pub mod memory;
pub mod notification;
pub mod services;
pub mod state;
pub mod storage;
pub mod ui;
pub mod utils;
