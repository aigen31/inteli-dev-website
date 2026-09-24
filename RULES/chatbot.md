# RULES: AI-чат — правила проектирования и реализации

## 📋 Содержание
- [Назначение чата](#назначение-чата)
- [Preset-кнопки вопросов](#preset-кнопки-вопросов)
- [Механика работы чата](#механика-работы-чата)
- [Типы ответов и их источники](#типы-ответов-и-их-источники)
- [Промпт для AI-ассистента](#промпт-для-ai-ассистента)
- [Обработка ошибок и крайних случаев](#обработка-ошибок-и-крайних-случаев)
- [UX/UI чата](#uxui-чата)
- [Аналитика чата](#аналитика-чата)

---

## Назначение чата

### Главная цель
AI-чат — это **первый пункт контакта** посетителя. Он должен:
1. Ответить на часто задаваемые вопросы автоматически
2. Помочь посетителю понять, подходит ли ему этот специалист
3. Подвести к оставлению заявки (конверсия)

### Контекст использования
```
Посетитель заходит на главную страницу → видит 2 кнопки:
[💬 Задать вопрос]    [📩 Оставить заявку]

Нажатие "Задать вопрос" → открывается чат с preset-кнопками
Нажатие "Оставить заявку" → форма лида (обходит чат)
```

---

## Preset-кнопки вопросов

### Список preset-кнопок

**Единый источник правды — `PRESETS` в `src/services/chat.rs`.** Страница
`/chat` и hero-терминал на главной рендерят кнопки оттуда, роутер оттуда же
берёт обработку. Свои списки в UI заводить нельзя: раньше подписи и индексы
жили в трёх местах и разошлись — кнопка «Сколько стоит?» на главной отправляла
`preset_index = 2`, который на `/chat` означал «Локальный инференс — что это?».
Клиент спрашивал цену и получал лекцию про инференс.

| # | Текст кнопки | Тип | Действие при нажатии | LLM? |
|---|---|---|---|---|
| 0 | `Кто вы?` | preset | Кто это, опыт, три ключевые компетенции, локация | ❌ |
| 1 | `Чем занимаетесь?` | preset | Услуги по одной строке (заголовок + суть) | ❌ |
| 2 | `Сколько стоит?` | preset | Вилки цен по всем услугам | ❌ |
| 3 | `Локальный инференс — что это?` | preset | Определение и что это даёт | ❌ |
| 4 | `Проанализируйте мой сайт` | analysis | Запрашивает URL → бесплатный мини-аудит | ✅ |
| 5 | `Когда свободны?` | availability | Живой статус занятости (тот же, что бейдж) | ❌ |
| 6 | `Оставить заявку` | lead_request | Что сделать, чтобы оставить заявку | ❌ |

Hero-терминал на главной показывает подмножество `HERO_PRESET_INDICES = [0, 1, 2]`.

### Правила для preset-кнопок
1. **Каждая кнопка = конкретный вопрос** — никаких абстрактных текстов
2. **Ответ — по факту вопроса, без воды.** Никаких вводных «Отлично!», пересказа
   всего профиля и приглашений оставить заявку в тексте ответа. Кнопки
   следующего шага рисует UI (`suggested_next`), а не сам ответ.
3. **Ориентир по длине — экран телефона (~900 символов).** Всё, что длиннее,
   возвращает нас к простыням, из-за которых ответы переписывали. Полные
   описания услуг живут на `/services`, а не в чате.
4. **После ответа — всегда следующая кнопка** — не давать посетителю думать «что дальше?»
5. **Кнопка "Оставить заявку" должна быть видна в любом диалоге** — floating button или bottom bar

### Защита от рассинхрона
Тест `every_preset_button_routes_to_a_known_kind` проходит по всему `PRESETS` и
требует, чтобы у каждой кнопки был прямой ответ (для `kind = "preset"` — не
`Free`). Добавили кнопку, но забыли обработку — тест падает, а не молчаливое
«вопрос ушёл в модель».

### Альтернативные варианты (A/B тестирование, не реализовывать одновременно):
```
Вариант A (по услугам):
  [SEO-аудит]  [Продвижение]  [Техническое SEO]  [Контент]  [Контакты]

Вариант B (по проблеме):
  [Нет трафика]  [Сайт медленный]  [Не продаёт]  [Хочу больше заявок]

Вариант C (выбранный — по шагам клиента):
  Кто вы? → Чем занимаетесь? → Сколько стоит? → Локальный инференс →
  Анализ сайта → Когда свободны? → Заявка
```

---

## Механика работы чата

### Flow: Пользователь → Вопрос → Ответ

```
1. Пользователь нажимает preset-кнопку или вводит свой вопрос
              │
              ▼
2. Frontend отправляет POST /api/chat { message, question_type }
              │
              ▼
3. Backend: проверка запроса и двухуровневый rate limiting
     ├─> Длина message > chat_message_max_chars → 400 (валидация)
     ├─> Антифлуд: > max_requests_per_minute на IP → 429
     └─> Квота LLM: > llm_requests_per_hour на IP → 429
         Лимиты берутся из секции [limits] / [ratelimit] config.toml,
         те же числа уходят в UI (maxlength + подсказка) через src/limits.rs.
              │
              ▼
4. ChatService.process_query(message, question_type)
              │
              ├── preset (preset-кнопка) → lookup готового ответа в OpenViking
              ├── analysis (URL анализ) → вызов LLM с промптом для анализа
              ├── lead_request → redirect на форму лида + сохранить intent
              └── free (свободный ввод) → semantic search + LLM
              │
              ▼
5. Redis cache check: exists? return cached answer
              │
              ▼
6. OpenViking semantic_search(query) → получает context из URI
              │
              ▼
7. Build prompt: system_prompt + viking_context + user_question
              │
              ▼
8. LLM provider call (DeepSeek V4 / OpenAI-compatible API)
              │
              ▼
9. Post-process response: add CTA at the end
              │
              ▼
10. Save to SQLite + cache in Redis
              │
              ▼
11. Return JSON response to frontend
```

### Flow: Анализ сайта (preset #4)

```
Пользователь нажимает "🔍 Проанализируйте мой сайт"
    → Chatbot просит ввести URL
    → Пользователь вводит URL (например: example.com)
    → Chatbot выполняет бесплатный анализ через LLM:
       ├── Попробует fetch контент сайта (если публичный)
       ├── Или использует известные данные если сайт найден
       └── Даёт 3-5 рекомендаций по SEO на основе публичных данных
    → Предлагает полный аудит за X рублей
    → Предлагает оставить заявку
```

### Flow: Конверсия в лид (после любого диалога)

```
Chatbot отвечает на вопрос
    → В конце ответа автоматически добавляется CTA:
       "Хотите обсудить ваш проект? [Оставить заявку]"
    → Пользователь может:
       1. Нажать CTA → форма лида
       2. Продолжить диалог (задать ещё вопрос)
       3. Закрыть чат и перейти к форме контактов
```

---

## Типы ответов и их источники

### Источник A: Готовый ответ из контента сайта (preset questions)
Для частых вопросов (кто вы, услуги, цены) — готовые ответы из `SiteContent`
(профиль, услуги, цены из `src/memory/content.rs`). Это быстрее, точнее и не
требует LLM. Формулировки живут в `src/services/chat.rs` (`answer_*`), а не в
prompt-шаблонах: это фиксированный текст, а не генерация.

```rust
// В ChatService, по типу вопроса (см. resolve_kind)
QuestionKind::Pricing => (answer_pricing(), "openviking_direct"),
QuestionKind::Availability => (
    format_availability(&SiteSettings::availability()),  // живой статус
    "openviking_direct",
),
```

Правило формулировок: **ответ по факту вопроса, без воды.** Никаких вводных
«Отлично!», пересказа всего профиля и «оставьте заявку» в тексте — кнопки
следующего шага рисует UI. Ориентир по длине — экран телефона.

### Источник B: Semantic search + LLM (free text)
Для нестандартных вопросов — ищем релевантный контекст в OpenViking, подставляем в промпт, получаем ответ от LLM.

### Источник C: Direct API response (status, availability)
Для запросов статуса — прямой JSON response из `/api/status.json`, без LLM.

Живой статус занятости живёт в `src/settings.rs` (`SiteSettings`), а не в
контенте: владелец меняет его из Telegram-бота настроек
(см. `docs/settings-bot.md`). `SiteContent.availability` из OpenViking — только
значение по умолчанию, пока в БД нет сохранённой записи.

`StatusBadge` (шапка и главная), `/api/status` и ответ чата на вопрос о
занятости читают **один и тот же** `SiteSettings::availability()`, поэтому
бейдж на сайте и слова ассистента не могут разойтись.

Формулировки при этом берутся не из литералов, а из общего словаря
`services/status.rs::presentation` — из него же собираются подпись бейджа и
текст Telegram-бота. Добавляя новую поверхность статуса, используйте
`status_line` / `tooltip` / `facts`, а не свои строки: тест
`all_surfaces_share_one_vocabulary` поймает расхождение.

Вопрос о занятости сохраняется в историю чатов с `question_type = 'availability'`
(раньше CHECK-ограничение такого значения не знало, вставка падала, а
`save_chat` глотает ошибку — вопрос терялся молча; исправлено миграцией в
`src/storage/mod.rs`).

### Таблица источников по типам вопросов

| Вопрос | Источник | Требует LLM? | Скорость |
|--------|----------|-------------|----------|
| Кто вы? | `SiteContent` (профиль) | ❌ Нет | < 5ms |
| Чем занимаетесь? | `SiteContent` (услуги, первое предложение описания) | ❌ Нет | < 5ms |
| Сколько стоит? | `SiteContent` (вилки цен услуг) | ❌ Нет | < 5ms |
| Локальный инференс | встроенный текст | ❌ Нет | < 5ms |
| Когда свободны? | `SiteSettings` (живой статус из SQLite) | ❌ Нет | < 30ms |
| Оставить заявку | встроенный текст | ❌ Нет | < 5ms |
| Анализ сайта | LLM (prompt: analyze_url) | ✅ Да | ~2-5s |
| «Как увеличить трафик?» | LLM + context | ✅ Да | ~1-3s |
| «Сделайте аудит моего…» | LLM (prompt: analyze_url) | ✅ Да | ~2-5s |

---

## Промпт для AI-ассистента

### Системный промпт (хранится в OpenViking: `viking://user/inteli-dev/prompts/chatbot-system.md`)

```markdown
# Системный промпт AI-чата

Ты — AI-ассистент Ивана Петрова, специалиста по продвижению сайтов с 10-летним опытом.
Помогаешь посетителям сайта понять, чем вы можете помочь, и подталкиваете к оставлению заявки.

## О специалисте:
{context_from_viking_profile}

## Об услугах:
{context_from_viking_services}

## Правила общения:
1. Отвечай на русском языке (если пользователь пишет на русском)
2. Если вопрос не по теме сайта — вежливо перенаправь к основной теме
3. Не обещай конкретных результатов (CTR, позиции), только ориентиры и опыт
4. Упоминай конкретные цифры: 10 лет опыта, 200+ проектов, +340% средняя конверсия

## Структура ответа на вопрос о ценах:
- Сначала дай ориентировочный диапазон
- Уточни, что точная стоимость зависит от проекта
- Предложи обсудить детали или рассчитать стоимость

## Каждый ответ должен заканчиваться CTA (Call To Action):
"Хотите узнать точную стоимость для вашего проекта? [Оставить заявку]"
или
"Ещё вопросы? Я на связи в Telegram: @username"
```

### Промпт для анализа сайта

```markdown
# Промпт для анализа сайта пользователя

Проанализируйте сайт {url} с точки зрения SEO-оптимизации.
Ваша задача — дать 3-5 конкретных рекомендаций по улучшению позиций в поиске.

Правила:
1. Начинайте с самой критичной проблемы
2. Каждая рекомендация должна быть конкретным действием (не "улучшите скорость", а "сжимайте изображения в WebP и добавьте lazy loading")
3. Укажите, насколько серьёзна каждая проблема (High / Medium / Low)
4. В конце предложите полный аудит за 15 000-35 000 ₽
5. Не критикуйте резко — конструктивный тон

Если сайт недоступен или не удалось получить данные:
"К сожалению, я не смог получить доступ к {url}. Это может быть признаком технической проблемы. Рекомендую проверить robots.txt и sitemap.xml."
```

### Промпт для квалификации лида

```markdown
# Промпт для квалификации лида

На основе сообщения пользователя оцените его:
- Тип проекта (интернет-магазин / лендинг / корпоративный сайт / блог)
- Бюджет (низкий < 30к / средний 30-100к / высокий > 100к)
- Срочность (срочно сейчас / в течение месяца / без ограничений)
- Готовность к сотрудничеству (холодный / тёплый / горячий лид)

Верните JSON:
{
    "project_type": "ecommerce",
    "budget_range": "medium",
    "urgency": "high",
    "lead_quality": 8,  // 1-10
    "recommended_action": "schedule_call"
}
```

---

## Обработка ошибок и крайних случаев

### Сценарий: LLM API недоступен
```rust
// Fallback: если LLM не отвечает — показываем готовый ответ + контакт
match llm_provider.chat(prompt).await {
    Ok(response) => response,
    Err(ApiError::Timeout) | Err(ApiError::ServiceUnavailable) => {
        // Использовать cached answer или fallback-ответ из OpenViking
        let fallback = viking_client.read_uri("viking://user/inteli-dev/prompts/fallback-chat.md").await?;
        
        ChatResponse {
            answer: format!("Временно ответ на ваш вопрос может занять больше времени. Вот что могу сказать:\n\n{}", fallback),
            source: "llm_fallback",
            suggested_next: vec![CTA::ContactViaTelegram, CTA::LeaveLead],
        }
    }
}
```

### Сценарий: OpenViking недоступен
```rust
// Fallback: использовать embedded profile data при недоступности OpenViking
match viking_client.semantic_search(query).await {
    Ok(results) => results,
    Err(ApiError::ConnectionRefused) => {
        // Использовать in-memory fallback profile (загружается при старте)
        let cached = get_cached_profile();  // static OnceLock
        
        ChatResponse {
            answer: build_answer_from_cached(cached, query)?,
            source: "cached_fallback",
            suggested_next: vec![CTA::LeaveLead],
        }
    }
}
```

### Сценарий: Rate limiting — два независимых уровня

Лимиты разделены по смыслу: технический антифлуд защищает сервер, часовая
квота защищает бюджет на токены.

```rust
// Уровень 1 — антифлуд (все типы запросов, включая preset): 10 в минуту на IP.
// Fixed window: RateLimiter в src/cache.rs.
if !state.rate_limiter.check(&ip_hash) {
    return Err(AppError::RateLimitExceeded { retry_after: 60 }.into());
}

// Уровень 2 — квота на AI-ответы: 3 в час на IP (скользящее окно).
// Списывается ТОЛЬКО за запросы, которые реально тратят токены:
//   • requires_llm(req) — free и analysis;
//   • preset / availability / lead_request идут из контента бесплатно;
//   • ответ из кэша модель не вызывает, поэтому квоту не расходует.
if requires_llm(&req) && !state.chat.is_cached(&req) && !state.hourly_limiter.check(&ip_hash) {
    let retry_after = state.hourly_limiter.retry_after(&ip_hash);
    return Err(AppError::RateLimitExceeded { retry_after }.into());
}
```

Скользящее окно (`SlidingWindowLimiter`) выбрано вместо fixed window, чтобы
нельзя было удвоить квоту на стыке часов: 3 запроса в 12:59 + 3 в 13:00.

Ответ 429 отдаётся с человеческим текстом и полем `retry_after`:

```json
{ "error": "Лимит AI-ответов исчерпан. Попробуйте снова через час — или напишите мне в Telegram.",
  "retry_after": 2413 }
```

### Сценарий: Лимиты ввода (единый источник правды)

Числа живут в `[limits]` config.toml и публикуются в `src/limits.rs`
(`Limits::set_global` при старте). И SSR-разметка (`maxlength`, подсказка), и
серверная валидация читают `Limits::get()` — поэтому подсказка не может
разойтись с фактическим лимитом.

| Поле | Лимит по умолчанию | Где проверяется |
|------|--------------------|-----------------|
| `chat.message` | 500 символов | `ChatRequest::validate` + `maxlength` |
| `chat.url` (analysis) | 300 символов | `ChatRequest::validate` |
| `chat.session_id` | 64 символа | `ChatRequest::validate` |
| `lead.name` | 80 символов | `LeadSubmission::validate` + `maxlength` |
| `lead.message` | 1000 символов | `LeadSubmission::validate` + `maxlength` |
| `lead.phone` | 20 символов | `LeadSubmission::validate` + `maxlength` |
| `lead.email` | 254 символа | `is_valid_email` |

Длину считаем в **символах**, а не байтах: кириллица в UTF-8 занимает 2 байта,
иначе русский текст упирался бы в лимит вдвое раньше.

### Сценарий: Экономия токенов

Промпт уходит в модель на каждом free-запросе, поэтому вход ограничен:

1. **Кэш с нормализованным ключом** — `cache_key()` приводит сообщение к
   нижнему регистру и схлопывает пробелы, поэтому «Сколько стоит?» и
   «сколько  стоит?» — один платный запрос, а не два.
2. **Системный промпт сжат** без потери фактов (~1720 → ~1130 символов).
3. **Бюджет RAG-контекста**: `context_top_k` фрагментов ×
   `context_snippet_chars`, суммарно не больше `context_max_chars`,
   плюс отсев по `context_min_score`. URI источника не включаем — модель на
   него не ссылается, а это ~50 символов на фрагмент.
   Страховка: если после порога не осталось ничего, берётся исходный топ,
   иначе поиск молча выключился бы целиком.
4. **Потолок выходных токенов** — `max_tokens = 600` (промпт требует
   «1-2 предложения + максимум 3 пункта»), защищает от простыни.
5. **Preset-ответы не идут в LLM** — прямые ответы из контента.
6. **Ответы-кнопки короткие** — по факту вопроса, ориентир ~900 символов:
   клиент читает с телефона, а простыня уводит от ответа. Полные описания услуг
   живут на `/services`, а не в чате. За длиной следит тест
   `direct_answers_stay_short_and_have_no_cta_tail`.

---

## UX/UI чата

### Дизайн-принципы чата
1. **Chat должен быть встроен в страницу** (floating widget) или на отдельной странице `/chat`
2. **Preset-кнопки — всегда сверху** перед полем ввода
3. **Ответы AI — не как переписка, а как FAQ**: читабельные блоки, не пузыри
4. **CTA после каждого ответа** — кнопка/ссылка «Оставить заявку»

### Минимальный UI чата (Leptos component)
```rust
#[component]
fn ChatBot() -> impl IntoView {
    let mut messages = use_signal::<Vec<ChatMessage>>(|| vec![]);
    
    view! {
        <div class="chat-container">
            <!-- Preset buttons area -->
            <div class="preset-buttons">
                <PresetButton index=0 text="👤 Кто вы?" />
                <PresetButton index=1 text="💼 Чем можете помочь?" />
                <PresetButton index=2 text="💰 Сколько стоит?" />
                <PresetButton index=3 text="🔍 Проанализируйте сайт" />
                <PresetButton index=4 text="🟢 Когда свободны?" />
            </div>
            
            <!-- Chat messages area -->
            <div class="messages-area">
                {messages.read().iter().map(|msg| view! {
                    if msg.is_user {
                        <UserMessage text={msg.text.clone()} />
                    } else {
                        <BotMessage 
                            text={msg.text.clone()}
                            cta={msg.cta.clone()}
                        />
                    }
                })}
            </div>
            
            <!-- Input area -->
            <div class="input-area">
                <input placeholder="Ваш вопрос..." on:keyup=handle_keyup />
                <button class="btn-primary" on:click=send_message>Отправить</button>
            </div>
            
            <!-- Always-visible CTA (bottom) -->
            <a href="/contact" class="floating-cta">
                📩 Оставить заявку
            </a>
        </div>
    }
}

// CSS для чата — минималистичный, без «пузырей»
.chat-container {
    max-width: 700px;
    margin: 0 auto;
    padding: 2rem;
    font-family: inherit;
}

.preset-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 1.5rem;
}

.preset-buttons button {
    padding: 0.5rem 1rem;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    background: white;
    cursor: pointer;
    font-size: 0.9rem;
}

.preset-buttons button:hover {
    background: #f5f5f5;
    border-color: #1a73e8;
}

.messages-area {
    min-height: 200px;
    margin-bottom: 1rem;
}

.user-message {
    text-align: right;
    padding: 0.75rem 1rem;
    background: #f0f0f0;
    border-radius: 8px 8px 0 8px;
    margin: 0.5rem 0;
    max-width: 70%;
    margin-left: auto;
}

.bot-message {
    padding: 0.75rem 1rem;
    border-radius: 8px 8px 8px 0;
    margin: 0.5rem 0;
    max-width: 90%;
}

.cta-button {
    display: inline-block;
    padding: 0.75rem 1.5rem;
    background: #1a73e8;
    color: white;
    text-decoration: none;
    border-radius: 8px;
    margin-top: 0.5rem;
    font-weight: 500;
}
```

---

## Аналитика чата

### Что собираем из чата
| Метрика | Как считаем | Зачем |
|---------|------------|-------|
| Количество запросов за день | `COUNT(*)` из SQLite chats по дате | Понимать нагрузку |
| Top preset-вопросов | `GROUP BY question_type` | Оптимизировать контент страницы |
| Conversion rate (чат → лид) | `leads.count() / chats.count()` где chat lead-request | Оценка эффективности AI-чата |
| Avg response time | `AVG(response_time_ms)` из SQLite | Мониторить производительность LLM API |
| Frequent free-text questions | `ORDER BY length(message) DESC` или NLP clustering | Улучшать preset-кнопки |

### Dashboard для админа (см. [admin-panel.md](./admin-panel.md))
- Графики по дням/неделям
- Таблица топ-вопросов
- Прогресс бар конверсий

---

*См. также: [ai-fabric.md](./ai-fabric.md), [admin-panel.md](./admin-panel.md)*
