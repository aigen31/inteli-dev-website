# RULES: Админ-панель — правила проектирования и реализации

## 📋 Содержание
- [Назначение админ-панели](#назначение-админ-панели)
- [Авторизация и безопасность](#авторизация-и-безопасность)
- [Dashboard (аналитика)](#dashboard-аналитика)
- [История чатов](#история-чатов)
- [Управление лидом/заявками](#управление-лидомзаявками)
- [Управление контентом через OpenViking](#управление-контентом-через-openviking)
- [Настройки и конфигурация](#настройки-и-конфигурация)

---

## Назначение админ-панели

### Зачем она нужна
Админ-панель — это **единственный интерфейс управления** сайтом. Автор (владелец сайта) использует её для:
1. Просмотра статистики: сколько лидов, конверсий, активности
2. Чтения истории чатов с посетителями
3. Управления контентом сайта (без редактирования кода)
4. Настройки уведомлений и интеграций

### Дизайн-принцип
> **"Админка — это не сайт. Она должна быть функциональной, а не красивой."**
> 
> Меньше анимаций, больше данных на экране. Таблицы > графики для детального просмотра.

---

## Авторизация и безопасность

### Механизм авторизации
```
Admin login → токен (not password) → cookie/JWT
```

### Параметры безопасности
| Параметр | Значение |
|---|---|
| Способ входа | Токен (длинная случайная строка, не пароль) |
| Время жизни сессии | 24 часа |
| Хранение токена | Redis (для мгновенной инвалидации при компрометации) |
| Transport | HTTPS only (http://localhost для dev) |

### Токен авторизации — как генерировать и хранить
```bash
# Генерация токена (запускать один раз при первом деплое)
$ openssl rand -base64 32
# Результат: "aBcDeFgHiJkLmNoPqRsTuVwXyZ1234567890=="

# Сохранить в config.toml или env var:
# ADMIN_TOKEN = "aBcDeFgHiJkLmNoPqRsTuVwXyZ1234567890=="
```

### Middleware авторизации
```rust
// api/admin_auth.rs
pub struct AdminAuthMiddleware;

impl AdminAuthMiddleware {
    pub fn check(req: &Request) -> Result<(), AuthError> {
        let token = req.cookies().get("admin_session")
            .map(|c| c.value())
            .ok_or(AuthError::NoToken)?;
        
        // Check Redis session store
        redis_client.get(&format!("sess:admin:{}", sha256(token.as_bytes())).chars().take(16).collect::<String>())
            .await?
            .ok_or(AuthError::SessionExpired)?;
        
        Ok(())
    }
}

// Protection for all admin routes
pub fn protected_route(handler: HandlerFn) -> MiddlewareFn {
    move |req, next| {
        AdminAuthMiddleware::check(&req).map_err(|e| {
            Response::builder()
                .status(401)
                .body(format!("Authorization required. Error: {:?}", e))
        })?;
        next(req)
    }
}
```

### Никогда не делать
- ❌ Хранить пароль в коде или Git
- ❌ Отправлять токен по HTTP (без HTTPS)
- ❌ Логировать полный токен в access log
- ❌ Использовать одинаковый токен на prod и dev без разделения

---

## Dashboard (аналитика)

### Что отображается на главной странице админки

#### Карточки KPI (верхний ряд)
| Карточка | Метрика | Расчёт | Обновление |
|----------|---------|--------|-----------|
| 📩 Заявки сегодня | `COUNT(leads WHERE date = today)` | SQL query | Реальное время |
| 📊 Конверсия за 7 дней | `(leads / unique_visitors) * 100%` | SQL + Redis visitor count | Каждые 5 минут |
| 💬 Чатов сегодня | `COUNT(chats WHERE date = today)` | SQL query | Реальное время |
| ⚡ Среднее время ответа | `AVG(response_time_ms)` | SQL aggregation | Последние 24ч |

#### Графики (средний ряд)
```
1. Лиды за последние 7/30 дней (line chart)
   X: день, Y: количество лидов
   
2. Активность чатов за последние 7/30 дней (bar chart)  
   X: час дня, Y: количество запросов → видно пиковые часы

3. Источник лидов (pie/donut chart)
   form / chat / telegram / vk — процентное соотношение
```

#### Таблица последних действий (нижний ряд)
| Дата | Тип | Описание | Статус |
|------|-----|----------|--------|
| 07.09 14:23 | lead | Заявка от "Иван И." через форму | ✅ Отправлено в TG+VK |
| 07.09 13:45 | chat | Вопрос: "Сколько стоит SEO?" → Ответ + CTA | ✓ Ответлен |
| 07.09 12:10 | lead | Заявка через Telegram бота | 🔄 В обработке |

### Пример реализации Leptos admin dashboard component
```rust
#[component]
fn AdminDashboard() -> impl IntoView {
    let stats = get_dashboard_stats().await;
    
    view! {
        <div class="admin-dashboard">
            <!-- KPI Cards -->
            <div class="kpi-cards">
                <KpiCard title="Заявки сегодня" value={stats.leads_today.to_string()} trend={stats.leads_trend} />
                <KpiCard title="Конверсия" value={`${stats.conversion_rate}%`} trend={stats.conversion_trend} />
                <KpiCard title="Чатов сегодня" value={stats.chats_today.to_string()} trend={None} />
                <KpiCard title="Ср. время ответа" value={format!("{}ms", stats.avg_response_time)} trend={None} />
            </div>
            
            <!-- Charts -->
            <div class="charts-row">
                <ChartCard title="Лиды за 7 дней" chart_type="line" data={stats.leads_weekly} />
                <ChartCard title="Активность чатов" chart_type="bar" data={stats.chat_activity_by_hour} />
                <ChartCard title="Источники лидов" chart_type="donut" data={stats.lead_sources} />
            </div>
            
            <!-- Recent Activity -->
            <div class="recent-activity">
                <h3>Последняя активность</h3>
                <ActivityTable entries={stats.recent_activity} />
            </div>
        </div>
    }
}

// CSS для админки — функциональный, без излишеств
.admin-dashboard {
    padding: 1.5rem;
    font-family: 'Inter', system-ui, sans-serif;
    max-width: 1200px;
    margin: 0 auto;
}

.kpi-cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
    margin-bottom: 2rem;
}

.kpi-card {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 1rem 1.5rem;
}

.kpi-card .value {
    font-size: 2rem;
    font-weight: 700;
    color: #1a1a2e;
}

.charts-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(350px, 1fr));
    gap: 1rem;
    margin-bottom: 2rem;
}
```

---

## История чатов

### Что показывать в таблице чатов
| Колонка | Описание | Длина ограничения |
|---------|----------|------------------|
| Дата и время | `2026-09-07 14:23` | — |
| Вопрос (обрезка) | Первые 100 символов вопроса | 100 chars |
| Тип вопроса | preset / free / analysis / lead_request | badge |
| Ответ (обрезка) | Первые 100 символов ответа AI | 100 chars |
| Конверсия в лид | ✅ если после этого чата была заявка / ❌ если нет | icon |
| Время ответа | `{ms}ms` | number |

### Фильтры и поиск
```
Фильтры:
├── По дате: [от 01.09] — [до 07.09] [Применить]
├── По типу вопроса: [Все ▼] preset / free / analysis / lead_request
└── По наличию лида: [Все ▼] С лидом / Без лида

Поиск: [введите текст для поиска...        ] [🔍]
→ ищет по message И response (полнотекстовый поиск в SQLite)
```

### Детальный просмотр чата
При клике на строку таблицы — разворачивается полный диалог:
```
╔══════════════════════════════════════════╗
║ Чат #1234 · 07.09.2026 14:23            ║
╠══════════════════════════════════════════╣
║                                          ║
║ Вы: Сколько стоит SEO-продвижение?       ║
║                                          ║
║ AI: SEO-продвижение начинается от        ║
║     50 000 ₽ в месяц. Стоимость зависит  ║
║     от сложности проекта...              ║
║                                          ║
║ AI: Хотите узнать точную стоимость для   ║
║     вашего проекта? [Оставить заявку]    ║
║                                          ║
╠══════════════════════════════════════════╣
║ Тип: preset  |  Ответ: 2.3s             ║
║ Конверсия в лид: ✅ (заявка через 5 мин) ║
╚══════════════════════════════════════════╝
```

### Экспорт чатов
- **CSV экспорт**: все колонки таблицы → скачиваемый файл
- Фильтры применяются перед экспортом
- Формат: `chats_export_2026-09-07.csv`

---

## Управление лидом/заявками

### Таблица лидов
| Колонка | Описание | Действие |
|---------|----------|----------|
| Дата | Когда пришла заявка | — |
| Имя | {name} | Клик → копировать |
| Email | {email} (если есть) | Клик → отправить email |
| Телефон | {phone} (если есть) | Клик → позвонить / скопировать |
| Сообщение | Первые 150 символов | Клик → полный текст |
| Источник | form / chat / telegram / vk | badge цветным |
| Статус | new / processing / contacted / converted | dropdown: сменить статус |

### Статусы лида и их значение
| Статус | Значение | Цвет | Когда менять |
|--------|----------|------|-------------|
| `new` | Новая заявка, не обработана | 🔵 Синий | Автоматически при создании |
| `processing` | В процессе обработки (ответили в TG/VK) | 🟡 Жёлтый | При первом контакте |
| `contacted` | Связались с клиентом | 🟣 Фиолетовый | После звонка/письма |
| `converted` | Проект заключён / оплачен | 🟢 Зелёный | При оплате или подписании |
| `dismissed` | Отказали / не актуально | ⚪ Серый | Когда клиент отказался |

### Действия по лидe (inline в таблице)
```
╔══════════════════════════════════════════╗
║ Заявка #1234                             ║
╠══════════════════════════════════════════╣
║ Иван Иванов · ivan@example.com          ║
║ +7 (999) 123-45-67                      ║
║                                          ║
║ "Нужно продвинуть сайт интернет-магазина │
║  электроники, бюджет ~80к в месяц"      ║
║                                          ║
╠══════════════════════════════════════════╣
║ [📞 Позвонить] [✉️ Написать на email]   ║
║ [💬 Telegram @ivanov]                   ║
║ [⚙️ Изменить статус ▼]                  ║
╚══════════════════════════════════════════╝
```

---

## Управление контентом через OpenViking

### Принцип работы
Админ **не редактирует код** сайта. Админ обновляет данные через админку → данные записываются в OpenViking (viking:// URIs) → сайт автоматически подтягивает обновления из OpenViking.

### Что можно редактировать
| Раздел | URI в OpenViking | Поля для редактирования |
|--------|------------------|------------------------|
| Профиль | `viking://user/inteli-dev/profile/main.md` | Имя, должность, опыт, контакты |
| Навыки | `viking://user/inteli-dev/profile/skills.md` | Список навыков, уровни экспертизы |
| Услуги | `viking://user/inteli-dev/services/*.md` | Названия, описания, цены |
| Проекты | `viking://user/inteli-dev/projects/*.md` | Кейсы "До/После" с метриками |
| Статус занятости | `viking://user/inteli-dev/policies/availability.md` | Свободен / занят / полная загрузка |

### UI управления контентом
```
Админка → Настройки → Управление контентом

┌───────────────────────────────────────────────┐
│  Выберите раздел для редактирования:           │
│                                              │
│  📝 Профиль          [Изменить]               │
│  💼 Услуги           [Изменить]               │
│  🔧 Проекты          [Добавить кейс]          │
│  🟢 Статус занятости [Изменить статус]        │
└───────────────────────────────────────────────┘

При выборе "Профиль" → форма с полями:
┌───────────────────────────────────────────────┐
│ Имя:          [Иван Петров              ]      │
│ Должность:    [Специалист по SEO        ]      │
│ Опыт лет:     [10                      ]      │
│ Telegram:     [@username               ]      │
│ Email:        [email@example.com       ]      │
│ VK:           [vk.com/username         ]      │
│ Телефон:      [+7 (999) 123-45-67    ]      │
│                                              │
│              [💾 Сохранить в OpenViking]     │
└───────────────────────────────────────────────┘

При сохранении: 
 → markdown собирается из полей формы
 → записывается через viking_client.write_uri()
 → кэш Redis инвалидируется автоматически
```

---

## Настройки и конфигурация

### Раздел "Настройки" в админке

| Категория | Параметр | Описание | Значение по умолчанию |
|-----------|---------|----------|---------------------|
| Telegram | Bot Token | Токен Telegram бота для уведомлений | — (обязательно) |
| Telegram | Admin Chat ID | ID чата, куда приходят уведомления | — (обязательно) |
| VK | OAuth Token | Токен доступа к VK API для отправки сообщений | — (обязательно) |
| VK | Admin User ID | ID пользователя VK для уведомлений | — (обязательно) |
| LLM | Provider | Провайдер LLM: `deepseek` / `openai` / `custom` | `deepseek` |
| LLM | Base URL | Base URL API | `https://api.deepseek.com/v1` |
| LLM | Model | Модель LLM | `deepseek-chat` |
| Cache | TTL ответа чата | Время кэширования ответов | 3600 (сек) |
| Status | Занятость по умолчанию | Начальный статус при старте | `available` |

### Как менять настройки
```
1. Админ вводит новый токен/параметр в форму настроек
2. Сервер сохраняет их в config.toml (encrypted at rest) ИЛИ в env vars
3. Для активных соединений — обновляется runtime-конфигурация
4. При перезагрузке сервера — настройки читаются из config.toml / env
```

### Безопасное хранение токенов
```toml
# config.toml — НЕ коммитить в Git!
[telegram]
token = "123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11"
admin_chat_id = 123456789

[vk]
oauth_token = "a1b2c3d4e5f6..."
admin_user_id = 123456789

[llm]
provider = "deepseek"
base_url = "https://api.deepseek.com/v1"
model = "deepseek-chat"
# API key из env: LLM_API_KEY (не хранить в конфиге!)

[admin]
token_env = "ADMIN_TOKEN"  # имя env var для токена админа
```

### .gitignore
```
config.toml
*.env
*.sqlite
*.db
.DS_Store
target/
node_modules/
```

---

*См. также: [architecture.md](./architecture.md), [memory-storage.md](./memory-storage.md)*
