//! inteli.dev.ru — библиотека приложения.
//!
//! Все модули вынесены в библиотеку, чтобы их можно было тестировать
//! интеграционно (каталог `tests/`) и переиспользовать. Точка входа (boot
//! сервера) находится в `main.rs`.

// Макрос `view!` в Leptos — это не строковый шаблон, а вложенные типы: каждый
// узел добавляет уровень в тип всей страницы. Главная (hero с терминалом и
// маскотом плюс пять секций) перебирает дефолтные 128 уровней, и компилятор
// падает с «queries overflow the depth limit» — сообщением, по которому связь
// с разметкой не видна. Запас вдвое: следующая правка hero не должна упираться
// в это же место.
#![recursion_limit = "256"]

pub mod api;
pub mod cache;
pub mod config;
pub mod error;
pub mod limits;
pub mod llm;
pub mod memory;
pub mod notification;
pub mod security;
pub mod services;
pub mod settings;
pub mod state;
pub mod storage;
pub mod ui;
pub mod utils;
