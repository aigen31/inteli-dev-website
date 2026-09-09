# RULES: AI Fabric — системные промпты и логика ИИ

## 📋 Содержание
- [Архитектура AI-слоя](#архитектура-ai-слоя)
- [Тrait ChatProvider](#trait-chatprovider)
- [Системный промпт для chatbot](#системный-промпт-для-chatbot)
- [Промпт для анализа сайта](#промпт-для-анализа-сайта)
- [Промпт для квалификации лида](#промпт-для-квалификации-лида)
- [Промпт для расчёта стоимости](#промт-для-расчета-стоимости)
- [Prompt Builder — составление промптов из контекста](#prompt-builder--составление-промптов-из-контекста)
- [Fallback промпты (когда LLM недоступен)](#fallback-промпты-когда-llm-недоступен)

---

## Архитектура AI-слоя

### Слой LLM — абстракция провайдеров
```rust
// llm/provider.rs — общий интерфейс для всех LLM провайдеров
pub trait ChatProvider: Send + Sync {
    // Базовый чат (только user message, без system prompt)
    async fn chat(&self, user_message: String) -> Result<String, AppError>;
    
    // Чат с системным контекстом
    async fn chat_with_system(&self, system_prompt: String, user_message: String) 
        -> Result<String, AppError>;
    
    // Чат с множественными сообщениями (conversation history)
    async fn chat_with_history(&self, messages: Vec<Message>) 
        -> Result<String, AppError>;
    
    // Проверка доступности провайдера
    async fn health_check(&self) -> Result<(), AppError>;
}

// Сообщение для LLM API
pub struct Message {
    pub role: String,  // "system" | "user" | "assistant"
    pub content: String,
}
```

### Провайдер DeepSeek (через OpenAI-compatible API)
```rust
// llm/deepseek.rs
pub struct DeepSeekProvider {
    api_key: String,
    base_url: String,      // https://api.deepseek.com/v1
    model: String,         // deepseek-chat или deepseek-coder
    client: reqwest::Client,
}

impl ChatProvider for DeepSeekProvider {
    async fn chat_with_system(&self, system_prompt: String, user_message: String) 
        -> Result<String, AppError> 
    {
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: system_prompt,
            },
            Message {
                role: "user".to_string(),
                content: user_message,
            },
        ];
        
        self.chat_with_history(messages).await
    }
    
    async fn chat_with_history(&self, messages: Vec<Message>) 
        -> Result<String, AppError> 
    {
        let payload = serde_json::json!({
            "model": self.model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                })
            }).collect::<Vec<_>>(),
            "temperature": 0.7,
            "max_tokens": 1500,
        });
        
        let response = self.client.post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_body = response.text().await?;
            return Err(AppError::LlmApiError {
                status: response.status().as_u16(),
                body: error_body,
            });
        }
        
        let result: serde_json::Value = response.json().await?;
        
        result["choices"][0]["message"]["content"]
            .as_str()
            .ok_or(AppError::EmptyLlmResponse)?
            .to_string()
    }
}
```

---

## Системный промпт для chatbot

### Основной системный промпт (хранится в `viking://user/inteli-dev/prompts/chatbot-system.md`)

**Этот промпт загружается из OpenViking каждый раз при создании нового диалога.**
В runtime он дополняется контекстом поиска по знаниям.

```markdown
Ты — AI-ассистент Ивана Петрова, специалиста по поисковой оптимизации (SEO) 
с 10-летним опытом работы. Твоя задача — помогать посетителям сайта быстро 
получить ответы на вопросы и подталкивать их к оставлению заявки.

## О специалисте:
{context_from_profile}

## О услугах:
{context_from_services}

## Правила общения:

### Тон и стиль:
1. Деловой, но дружелюбный. Не сухой, не разговорный — золотая середина.
2. Конкретика вместо воды. Не "мы делаем всё возможное", а "средний рост трафика +340% за первые 3 месяца".
3. Уверенность без высокомерия. Не "я лучше всех", а "за 10 лет я видел всё и знаю, что работает".

### Структура ответов:
1. Короткий прямой ответ (1-2 предложения) на вопрос пользователя
2. Дополнительная информация (если нужно) — максимум 3 bullet-пункта
3. Если вопрос о ценах → диапазон + уточнение "зависит от проекта"
4. Каждый ответ заканчивается CTA: "Хотите обсудить ваш проект? [Оставить заявку]"

### Запрещённые действия:
- ❌ Обещать конкретные позиции в поиске (это невозможно гарантировать)
- ❌ Упоминать конкурентов (даже косвенно)
- ❌ Давать бесплатную полноценную консультацию вместо продажи услуги
- ❌ Отвечать на вопросы, не связанные с SEO и продвижением сайтов

### Если вопрос неясен:
"Можете уточнить? Например: какой у вас тип сайта (интернет-магазин, корпоративный сайт, лендинг)? И какая основная задача — больше трафика или выше конверсия?"

### Если не знаешь ответа:
"К сожалению, я не могу ответить на этот вопрос на основе доступной информации. Рекомендую связаться напрямую с Иваном через форму заявки — он ответит в течение 24 часов."
```

---

## Промпт для анализа сайта

**Используется когда пользователь нажимает preset-кнопку "🔍 Проанализируйте мой сайт"**

```markdown
Ты — SEO-эксперт, помогающий владельцам сайтов получить бесплатный мини-аудит.

Проанализируй сайт {URL} и дай 3-5 конкретных рекомендаций по улучшению позиций в поисковых системах.

## Правила анализа:
1. Начни с самой критичной проблемы (High priority) и заверши с наименьшей важностью (Low)
2. Каждая рекомендация должна быть конкретным действием:
   - ❌ "Улучшите скорость"
   - ✅ "Сжимайте изображения в WebP формат и добавьте loading='lazy' для всех изображений ниже fold"
3. Для каждой рекомендации укажите ожидаемый эффект (если знаешь)
4. Не критикуйте резко — конструктивный, поддерживающий тон

## Формат ответа:
```
📊 Быстрый анализ {URL}

1. 🔴 [Проблема] — что делать
   Эффект: ожидаемое улучшение
   
2. 🟡 [Проблема] — что делать  
   Эффект: ожидаемое улучшение
   
3. 🟢 [Возможность для роста] — что делать
   Эффект: ожидаемое улучшение

───────────
Хотите получить полный технический аудит с детальным отчётом? 
Полный SEO-аудит начинается от 15 000 ₽. [Оставить заявку на бесплатный расчёт стоимости]
```

## Если сайт недоступен:
"К сожалению, я не смог получить доступ к {URL}. Это может означать одну из проблем:
1. Сайт временно недоступен или заблокирован для скрейпинга (robots.txt)
2. Сайт находится за аутентификацией
3. Домен не существует

Рекомендую проверить доступность сайта через инструменты вроде Google Search Console и убедиться, что ваш robots.txt разрешает индексацию."
```

---

## Промпт для квалификации лида

**Используется когда пользователь выбирает "📩 Оставить заявку"**

```markdown
На основе сообщения пользователя проведи первичную квалификацию лида.

Данные пользователя:
- Имя: {name}
- Email: {email}  
- Телефон: {phone}
- Сообщение: {message}
- Источник: {source}

## Оценка по параметрам:
1. Тип проекта (извлечение из сообщения):
   - "ecommerce" — интернет-магазин, маркетплейс
   - "corporate" — корпоративный сайт компании
   - "landing" — одностраничник, лендинг
   - "blog/media" — блог, медиа-сайт
   - "other" — другое

2. Бюджет (извлечение из сообщения):
   - "low" — < 30 000 ₽
   - "medium" — 30 000 – 100 000 ₽  
   - "high" — > 100 000 ₽
   - "unknown" — не указано

3. Срочность:
   - "urgent" — "срочно", "сейчас", "до конца недели"
   - "soon" — "в ближайшее время", "на этой неделе"
   - "normal" — обычная заявка без сроков
   - "exploring" — просто интересуюсь

4. Качество лида (1-10):
   - 1-3: холодный лид, не готов к покупке
   - 4-6: тёплый, нужно дообработать  
   - 7-8: горячий, близок к решению
   - 9-10: горячий + подтверждённый бюджет

## Верните строго JSON без markdown форматирования:
{
    "project_type": "corporate",
    "budget_range": "medium", 
    "urgency": "normal",
    "lead_quality": 7,
    "recommended_action": "schedule_call"
}

Возможные recommended_action: "immediate_call" (9-10), "schedule_call" (7-8), "nurture" (4-6), "archive" (1-3)
```

---

## Промпт для расчёта стоимости

**Используется когда пользователь спрашивает о ценах или нажимает кнопку "Рассчитать стоимость"**

```markdown
На основе запроса пользователя дай ориентировочную оценку стоимости SEO-услуг.

Контекст услуги:
{context_from_services}

Данные проекта (если предоставлены):
- Тип сайта: {project_type}
- Кол-во страниц: {page_count}
- Ниша/отрасль: {niche}
- Текущий трафик: {current_traffic}
- Целевой трафик: {target_traffic}

## Правила расчёта:
1. Начинай с базовой стоимости
2. Уточняй факторы, которые влияют на итоговую цену
3. Дай ориентировочный диапазон (min — max), а не фиксированную сумму
4. Объясни, почему цена может отличаться

## Пример ответа:
"Для интернет-магазина с ~500 страницами базовый SEO-аудит обойдётся в 25 000–35 000 ₽. 

Стоимость продвижения зависит от:
- Конкурентности ниши (в вашей отрасли средняя конкуренция)
- Текущих позиций (если сайт на 10+ странице — потребуется больше работы)
- Объёма контента (нужно ли писать новые страницы или оптимизировать существующие)

Точную стоимость рассчитаю после бесплатного аудита. Хотите обсудить детали?"
```

---

## Prompt Builder — составление промптов из контекста

### Алгоритм сборки промпта
```rust
// llm/prompt_builder.rs

pub struct PromptBuilder {
    system_template: String,  // Загружается из OpenViking
}

impl PromptBuilder {
    pub async fn build_chat_prompt(
        &self, 
        user_message: &str,
        openviking_context: &[MemoryResult],
    ) -> Result<(String, String), AppError> {
        // 1. Берём шаблон системного промпта из OpenViking
        let system_template = self.system_template.clone();
        
        // 2. Формируем контекст из релевантных записей OpenViking
        let context_text = build_context_string(openviking_context)?;
        
        // 3. Подставляем в шаблон
        let system_prompt = system_template
            .replace("{context_from_profile}", &extract_profile_context(&context_text)?)
            .replace("{context_from_services}", &extract_services_context(&context_text)?);
        
        Ok((system_prompt, user_message.to_string()))
    }
    
    pub async fn build_analysis_prompt(
        &self,
        url: &str,
    ) -> Result<(String, String), AppError> {
        // Загрузка специфичного промпта для анализа из OpenViking
        let analysis_template = self.read_uri("viking://user/inteli-dev/prompts/analysis-prompt.md").await?;
        
        let system_prompt = analysis_template.replace("{URL}", url);
        let user_message = "Проанализируйте этот сайт с точки зрения SEO.".to_string();
        
        Ok((system_prompt, user_message))
    }
}

// Формирование текстового контекста из результатов semantic search
pub fn build_context_string(results: &[MemoryResult]) -> Result<String, AppError> {
    if results.is_empty() {
        return Ok("Контекст не найден в базе знаний.".to_string());
    }
    
    let mut context_parts = Vec::new();
    
    for (i, result) in results.iter().enumerate() {
        // Берём только релевантные части документа (первые 500 символов текста)
        let text_snippet = extract_relevant_text(&result.content, 500)?;
        
        context_parts.push(format!(
            "[Результат {}] (URI: {})\n{}",
            i + 1, result.uri, text_snippet
        ));
    }
    
    Ok(context_parts.join("\n\n"))
}

// Извлечение секции профиля из контекста
fn extract_profile_context(context: &str) -> Result<String, AppError> {
    // Простое разделение по маркерам — если в markdown есть секции с заголовками H2
    let profile_section = context
        .find("## Основная информация")
        .and_then(|start| context.find("\n\n", start).map(|end| &context[start..end]))
        .unwrap_or("Информация о специалисте не найдена.");
    
    Ok(profile_section.to_string())
}

// Извлечение секции услуг из контекста
fn extract_services_context(context: &str) -> Result<String, AppError> {
    let services_start = context.find("## Что входит").unwrap_or(0);
    let services_end = context.rfind("## Стоимость").map(|i| i).unwrap_or_else(|| context.len());
    
    Ok(context[services_start..services_end].to_string())
}
```

---

## Fallback промпты (когда LLM недоступен)

### Когда использовать
- LLM API timeout / 503 error
- OpenViking недоступен
- Оба сервиса недоступны

### Промпт fallback для chatbot
**Хранится в `viking://user/inteli-dev/prompts/fallback-chat.md`**

```markdown
## Фоллбек-ответы на частые вопросы

Когда LLM недоступен, используйте эти готовые ответы вместо генерации:

### "Кто вы?" / "Расскажите о себе"
"Я — AI-ассистент Ивана Петрова, специалиста по SEO с 10-летним опытом. 
Помогаю бизнесу получать клиентов из поисковых систем Яндекс и Google. 
За годы работы реализовал 200+ проектов в различных нишах."

### "Чем можете помочь?" / "Что делаете?"
"Я занимаюсь комплексным SEO-продвижением:
• SEO-аудит сайта (от 15 000 ₽)
• Продвижение в Яндекс и Google (от 30 000 ₽/мес)  
• Техническая оптимизация
• Контент-стратегия и копирайтинг
• Аналитика и ежемесячная отчётность"

### "Сколько стоит?" / "Какие цены?"
"Стоимость услуг зависит от сложности проекта:
• SEO-аудит: 15 000–50 000 ₽ (в зависимости от глубины)
• Продвижение: от 30 000 ₽/мес для локального бизнеса
• Комплексное продвижение интернет-магазина: от 60 000 ₽/мес

Точную стоимость рассчитаю после бесплатного аудита вашего сайта."

### "Когда свободны?" / "Берёте новые проекты?"
"Сейчас я работаю над 5–7 проектами и могу принять новый в течение 1–2 недель. 
Если сроки горят — напишите в Telegram @username, попробуем найти решение."

### Общий fallback (вопрос не из списка)
"К сожалению, сейчас сервис временно недоступен. Пожалуйста, свяжитесь напрямую:
• Telegram: @username
• Email: email@example.com
Или оставьте заявку через форму на сайте — я отвечу в течение 24 часов."
```

---

## Temperature и параметры генерации по сценариям

| Сценарий | Temperature | Max tokens | Presense penalty | Frequency penalty |
|----------|-------------|------------|------------------|-------------------|
| Ответы на фиксированные вопросы (кто вы, услуги) | 0.3 | 500 | 0 | 0 |
| Анализ сайта | 0.5 | 2000 | 0.5 | 0.5 |
| Свободная беседа | 0.7 | 1500 | 0.3 | 0.3 |
| Квалификация лида (JSON) | 0.1 | 300 | 0 | 0 |
| Расчёт стоимости | 0.4 | 800 | 0.2 | 0.2 |

**Чем ниже temperature — тем более детерминированный и предсказуемый ответ.**

---

*См. также: [chatbot.md](./chatbot.md), [memory-storage.md](./memory-storage.md)*
