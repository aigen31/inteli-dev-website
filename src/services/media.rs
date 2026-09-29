//! Изображения статей: приём, проверка и отдача.
//!
//! # Что здесь решается
//!
//! Автор перетаскивает картинку в редактор статьи — браузер отправляет её
//! сюда. Сайт обязан ответить на три вопроса: это точно изображение, сколько
//! оно весит и по какому адресу его теперь показывать.
//!
//! # Тип определяется по содержимому, а не по заголовку запроса
//!
//! `Content-Type` из запроса подделывается тривиально, а последствия ошибки
//! серьёзные: файл с расширением `.png` и содержимым HTML, отданный с нашего
//! домена, — это XSS на своём origin. Поэтому тип распознаётся по «магическим»
//! байтам, и всё, что не распозналось, отклоняется. `Content-Type` в ответе
//! берётся из результата распознавания, а не из запроса.
//!
//! SVG отклоняется отдельно и с понятным текстом: это изображение по смыслу, но
//! оно умеет нести `<script>`, и отдавать его с нашего домена нельзя.
//!
//! # Почему Raw body, а не multipart
//!
//! Загружается ровно один файл и никаких метаданных к нему не прилагается (имя
//! собирается из хеша содержимого, alt-текст автор пишет в markdown). Multipart
//! при таком запросе — только лишний парсер и лишняя зависимость.

use sha2::{Digest, Sha256};
use sqlx::sqlite::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::storage::media as repo;

/// Максимальный размер одной картинки.
///
/// 8 МБ — это фотография с телефона в приличном качестве. Всё, что больше,
/// читатель всё равно не увидит без потери смысла, а сайт платит за это местом
/// в базе и трафиком.
pub const MEDIA_MAX_BYTES: usize = 8 * 1024 * 1024;

/// Сколько hex-символов sha256 попадает в имя файла (96 бит).
///
/// Полный хеш в адресе выглядит пугающе и ничего не добавляет: коллизия на 96
/// битах не встречается на практике, а от одинакового содержимого защищает
/// уникальный индекс по полному хешу.
const NAME_HEX_CHARS: usize = 24;

/// Сколько байт начала файла смотрим, чтобы распознать «похоже на SVG».
const SNIFF_HEAD_BYTES: usize = 512;

/// Форматы, которые сайт принимает.
///
/// Список — это одновременно белый список расширений, `Content-Type` ответа и
/// набор сигнатур. Один источник правды: добавить формат можно только здесь.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Png,
    Jpeg,
    Gif,
    WebP,
    Avif,
}

impl ImageKind {
    /// Расширение файла без точки.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Gif => "gif",
            Self::WebP => "webp",
            Self::Avif => "avif",
        }
    }

    /// `Content-Type`, с которым файл отдаётся.
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::WebP => "image/webp",
            Self::Avif => "image/avif",
        }
    }

    /// Человекочитаемое название для сообщений об ошибке.
    fn title(self) -> &'static str {
        match self {
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
            Self::Gif => "GIF",
            Self::WebP => "WebP",
            Self::Avif => "AVIF",
        }
    }

    /// Все форматы — для текста подсказки.
    pub fn supported_list() -> String {
        [Self::Png, Self::Jpeg, Self::WebP, Self::Gif, Self::Avif]
            .iter()
            .map(|k| k.title())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Распознаёт формат по сигнатуре файла.
    ///
    /// Проверки намеренно строгие: `RIFF` без `WEBP` — это WAV, `ftyp` без
    /// нужного бренда — видео или HEIC, а HEIC браузеры не показывают.
    fn from_magic(bytes: &[u8]) -> Option<Self> {
        const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        const GIF87: &[u8] = b"GIF87a";
        const GIF89: &[u8] = b"GIF89a";

        if bytes.starts_with(PNG) {
            return Some(Self::Png);
        }
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Some(Self::Jpeg);
        }
        if bytes.starts_with(GIF87) || bytes.starts_with(GIF89) {
            return Some(Self::Gif);
        }
        if is_riff_webp(bytes) {
            return Some(Self::WebP);
        }
        if is_iso_bmff_avif(bytes) {
            return Some(Self::Avif);
        }
        None
    }
}

/// Распознаёт формат изображения по содержимому.
pub fn detect_image(bytes: &[u8]) -> Option<ImageKind> {
    ImageKind::from_magic(bytes)
}

/// Проверяет загруженный файл и возвращает его формат.
///
/// Тексты ошибок адресованы автору статьи и объясняют, что делать: это
/// единственный человек, который увидит их в интерфейсе.
pub fn validate_upload(bytes: &[u8]) -> AppResult<ImageKind> {
    if bytes.is_empty() {
        return Err(AppError::Validation("файл пустой".into()));
    }
    if bytes.len() > MEDIA_MAX_BYTES {
        return Err(AppError::Validation(format!(
            "изображение больше {} МБ — сожмите его перед загрузкой",
            MEDIA_MAX_BYTES / 1024 / 1024
        )));
    }

    if let Some(kind) = detect_image(bytes) {
        return Ok(kind);
    }

    // Дальше — только объяснения, почему файл не принят. Порядок важен: HEIC и
    // SVG дают самый частый и самый непонятный для автора отказ.
    if looks_like_heic(bytes) {
        return Err(AppError::Validation(
            "iPhone сохранил фото в HEIC, а браузеры его не показывают. Включите в \
             настройках камеры «Наиболее совместимые» или сохраните картинку как JPEG"
                .into(),
        ));
    }
    if looks_like_svg(bytes) {
        return Err(AppError::Validation(
            "SVG не принимаем: в нём может быть скрипт, а картинки отдаются с того же \
             домена, что и сайт. Сохраните изображение как PNG или WebP"
                .into(),
        ));
    }

    Err(AppError::Validation(format!(
        "это не изображение. Поддерживаются {}",
        ImageKind::supported_list()
    )))
}

/// Имя файла в URL: `<24 hex от sha256>.<расширение>`.
pub fn media_name(sha256_hex: &str, kind: ImageKind) -> String {
    // `get` вместо среза: если функция когда-нибудь получит не-ASCII строку,
    // срез по байтам запаникует, а `get` вернёт всю строку.
    let stem = sha256_hex.get(..NAME_HEX_CHARS).unwrap_or(sha256_hex);
    format!("{stem}.{}", kind.extension())
}

/// Проверяет имя файла из URL.
///
/// Имя попадает в SQL как параметр, поэтому обход каталога здесь невозможен в
/// принципе; проверка нужна, чтобы мусорные запросы не доходили до базы, а
/// список расширений не разошёлся с [`ImageKind`].
pub fn is_valid_media_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 96 {
        return false;
    }
    let Some((stem, extension)) = name.rsplit_once('.') else {
        return false;
    };

    let known_extension = [
        ImageKind::Png,
        ImageKind::Jpeg,
        ImageKind::Gif,
        ImageKind::WebP,
        ImageKind::Avif,
    ]
    .iter()
    .any(|k| k.extension() == extension);

    known_extension
        && stem.len() == NAME_HEX_CHARS
        && stem.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Хеш содержимого — он же ключ дедупликации.
pub fn content_hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Результат сохранения картинки.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredMedia {
    pub name: String,
    /// Относительный адрес: он же вставляется в markdown.
    pub url_path: String,
    pub content_type: String,
    pub size_bytes: i64,
    /// `true` — такой файл уже был в медиатеке, новая копия не создавалась.
    pub deduplicated: bool,
}

/// Файл, готовый к отдаче в HTTP-ответе.
#[derive(Debug, Clone)]
pub struct MediaFile {
    pub content_type: String,
    pub bytes: Vec<u8>,
    /// Используется как `ETag`: содержимое неизменяемо, поэтому хеш — идеальный
    /// валидатор для условных запросов.
    pub sha256: String,
}

/// Сервис изображений: единственная точка записи и чтения медиатеки.
#[derive(Clone)]
pub struct MediaService {
    db: SqlitePool,
}

impl MediaService {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Принимает файл: проверяет, считает хеш, сохраняет и возвращает адрес.
    ///
    /// Повторная загрузка того же файла не создаёт дубль, а возвращает тот же
    /// адрес — вставленная в статью картинка не «переезжает».
    pub async fn store(&self, bytes: &[u8]) -> AppResult<StoredMedia> {
        let kind = validate_upload(bytes)?;
        let hash = content_hash(bytes);
        let name = media_name(&hash, kind);

        let (row, created) = repo::insert(
            &self.db,
            &name,
            kind.content_type(),
            bytes,
            &hash,
        )
        .await?;

        if !created {
            tracing::debug!("медиа: {} уже в медиатеке, дубль не создаём", row.name);
        }

        Ok(StoredMedia {
            url_path: media_url(&row.name),
            name: row.name,
            content_type: row.content_type,
            size_bytes: row.size_bytes,
            deduplicated: !created,
        })
    }

    /// Читает файл по имени из URL. Неизвестное имя — `None` (отдаём 404).
    pub async fn load(&self, name: &str) -> AppResult<Option<MediaFile>> {
        if !is_valid_media_name(name) {
            return Ok(None);
        }

        Ok(repo::find_blob(&self.db, name)
            .await?
            .map(|blob| MediaFile {
                content_type: blob.content_type,
                bytes: blob.bytes,
                sha256: blob.sha256,
            }))
    }

    /// Сколько файлов и байт в медиатеке.
    pub async fn stats(&self) -> AppResult<(i64, i64)> {
        repo::stats(&self.db).await
    }
}

/// Относительный адрес картинки на сайте.
pub fn media_url(name: &str) -> String {
    format!("/media/{name}")
}

/// RIFF-контейнер с брендом WEBP.
fn is_riff_webp(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP"
}

/// ISO-BMFF (`ftyp`) с брендом AVIF.
fn is_iso_bmff_avif(bytes: &[u8]) -> bool {
    if bytes.len() < 12 || &bytes[4..8] != b"ftyp" {
        return false;
    }
    let brand = &bytes[8..12];
    brand == b"avif" || brand == b"avis"
}

/// HEIC/HEIF из iPhone: тот же `ftyp`, но бренды другие.
fn looks_like_heic(bytes: &[u8]) -> bool {
    if bytes.len() < 12 || &bytes[4..8] != b"ftyp" {
        return false;
    }
    const HEIC_BRANDS: [&[u8]; 6] = [b"heic", b"heix", b"hevc", b"hevx", b"mif1", b"msf1"];
    HEIC_BRANDS.contains(&&bytes[8..12])
}

/// «Похоже на SVG» — по началу файла, без полноценного XML-парсера.
fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(SNIFF_HEAD_BYTES)];
    let text = String::from_utf8_lossy(head).to_lowercase();
    text.contains("<svg")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::init_memory_pool;

    /// Минимальные валидные начала файлов нужных форматов.
    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];
    const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0, 0];
    const GIF: &[u8] = b"GIF89a\x01\x00\x01\x00";
    const WEBP: &[u8] = b"RIFF\x24\x00\x00\x00WEBPVP8 ";
    const AVIF: &[u8] = b"\x00\x00\x00\x20ftypavifavif";
    const HEIC: &[u8] = b"\x00\x00\x00\x18ftypheic";

    fn service(pool: SqlitePool) -> MediaService {
        MediaService::new(pool)
    }

    #[test]
    fn detects_every_supported_format() {
        assert_eq!(detect_image(PNG), Some(ImageKind::Png));
        assert_eq!(detect_image(JPEG), Some(ImageKind::Jpeg));
        assert_eq!(detect_image(GIF), Some(ImageKind::Gif));
        assert_eq!(detect_image(WEBP), Some(ImageKind::WebP));
        assert_eq!(detect_image(AVIF), Some(ImageKind::Avif));
    }

    #[test]
    fn rejects_things_that_only_look_like_images() {
        // RIFF без бренда WEBP — это WAV, а `ftyp` без avif — видео.
        assert_eq!(detect_image(b"RIFF\x24\x00\x00\x00WAVEfmt "), None);
        assert_eq!(detect_image(b"\x00\x00\x00\x20ftypisom...."), None);
        assert_eq!(detect_image(b"<html><body>hi</body></html>"), None);
        assert_eq!(detect_image(b""), None);
    }

    #[test]
    fn svg_and_heic_get_their_own_explanations() {
        let svg = br#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg"></svg>"#;
        let err = validate_upload(svg).unwrap_err().to_string();
        assert!(err.contains("SVG"), "сообщение про SVG: {err}");
        // Регистр не важен: `<SVG>` — тот же файл.
        assert!(validate_upload(b"<SVG></SVG>").unwrap_err().to_string().contains("SVG"));

        let err = validate_upload(HEIC).unwrap_err().to_string();
        assert!(err.contains("HEIC"), "сообщение про HEIC: {err}");

        let err = validate_upload("просто текст".as_bytes())
            .unwrap_err()
            .to_string();
        assert!(err.contains("не изображение"), "общее сообщение: {err}");
    }

    #[test]
    fn upload_size_is_capped() {
        let mut huge = PNG.to_vec();
        huge.resize(MEDIA_MAX_BYTES + 1, 0);
        assert!(validate_upload(&huge).unwrap_err().to_string().contains("МБ"));
        assert!(validate_upload(&[]).is_err());
    }

    #[test]
    fn names_are_derived_from_hash_and_stay_valid() {
        let hash = content_hash(PNG);
        let name = media_name(&hash, ImageKind::Png);
        assert_eq!(name.len(), NAME_HEX_CHARS + 4);
        assert!(name.ends_with(".png"));
        assert!(is_valid_media_name(&name));
    }

    #[test]
    fn media_name_validation_rejects_paths_and_unknown_extensions() {
        assert!(!is_valid_media_name(""));
        assert!(!is_valid_media_name("../../etc/passwd"));
        assert!(!is_valid_media_name("index.html"));
        assert!(!is_valid_media_name("0123456789abcdef01234567.svg"));
        // Верхний регистр не принимаем: имена генерируем мы, а не автор.
        assert!(!is_valid_media_name("0123456789ABCDEF01234567.png"));
        // Не тот размер стема.
        assert!(!is_valid_media_name("abc.png"));
    }

    #[tokio::test]
    async fn storing_twice_returns_the_same_url() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool.clone());

        let first = svc.store(PNG).await.unwrap();
        assert!(!first.deduplicated);
        assert_eq!(first.content_type, "image/png");
        assert_eq!(first.url_path, format!("/media/{}", first.name));
        assert_eq!(first.size_bytes, PNG.len() as i64);

        let second = svc.store(PNG).await.unwrap();
        assert!(second.deduplicated);
        assert_eq!(second.name, first.name);

        let (count, _) = svc.stats().await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn different_images_get_different_urls() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool);

        let png = svc.store(PNG).await.unwrap();
        let jpeg = svc.store(JPEG).await.unwrap();
        assert_ne!(png.name, jpeg.name);
        assert_eq!(jpeg.content_type, "image/jpeg");
    }

    #[tokio::test]
    async fn load_returns_bytes_and_ignores_garbage_names() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool);

        let stored = svc.store(WEBP).await.unwrap();
        let file = svc.load(&stored.name).await.unwrap().expect("файл найден");
        assert_eq!(file.bytes, WEBP);
        assert_eq!(file.content_type, "image/webp");

        // Мусорное имя отсекается до запроса в базу.
        assert!(svc.load("../../etc/passwd").await.unwrap().is_none());
        assert!(svc
            .load("000000000000000000000000.png")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn store_rejects_non_images_without_touching_the_database() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool);

        assert!(svc.store(b"<script>alert(1)</script>").await.is_err());
        assert_eq!(svc.stats().await.unwrap().0, 0);
    }
}
