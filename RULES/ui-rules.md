# RULES: UI — Правила проектирования интерфейса

## 📋 Содержание
- [Философия дизайна](#философия-дизайна)
- [Дизайн-токены](#дизайн-токены)
- [Типографика](#типографика)
- [Цветовая система](#цветовая-система)
- [Сетка и пространство](#сетка-и-пространство)
- [Компоненты](#компоненты)
- [Страницы проекта](#страницы-проекта)
- [Адаптивный дизайн](#адаптивный-дизайн)
- [Микроанимации](#микроанимации)
- [Доступность (a11y)](#доступность-a11y)

---

## Философия дизайна

### Главный принцип
> **"Минимум кнопок и текста, максимум полезности и смысла.**
> С пары кнопок я мог получить лид."

Это означает:
- Каждый пиксель несёт пользу — если не несёт, убираем
- Навигация — максимум 4 пункта, не 8
- CTA (призывы к действию) — максимум 2 primary кнопки на экране
- Белый пространство важнее контента — воздух = восприятие премиальности
- Нет баннеров, нет попапов, нет счётчиков «онлайн», нет «скидка до конца дня»

### Три типа элементов
| Тип | Правило | Примеры |
|-----|---------|---------|
| **Primary** (CTA) | Максимум 2 на экране. Всегда ведут к цели: заявка или чат | «Задать вопрос», «Оставить заявку» |
| **Secondary** (навигация) | Топ 4 пункта меню, без выделения цветом | Услуги, Проекты, Блог, Контакты |
| **Tertiary** (информация) | Текст, иконки, бейджи — не ведут на страницу | Бейдж статуса (кнопка-раскрытие подсказки), метрики, соцсети |

### Запрещённые паттерны для этого проекта
- ❌ Поп-апы при входе/выходе с сайта
- ❌ Счётчики «Сейчас на сайте: 5» (обман пользователей)
- ❌ Таймеры обратного отсчёта («Скидка заканчивается через...»)
- ❌ Копирайт «Нам доверяют 1000+ клиентов» без конкретики
- ❌ Автовоспроизведение видео/звука
- ❌ Фиксированный баннер с призывом внизу (кроме CTA в чате)
- ❌ Карусели — они не работают на портфолио

---

## Дизайн-токены

### Цвета
```css
/* CSS Custom Properties — дизайн-токены */
:root {
  /* === Primary Brand === */
  --color-primary:       #0d6efd;   /* Основной синий — CTA кнопки, ссылки */
  --color-primary-hover: #0b5ed7;   /* Hover для primary */
  --color-primary-light: #e7f1ff;   /* Фон подсветки primary элементов */

  /* === Secondary / Neutral === */
  --color-secondary:     #6c757d;   /* Второстепенный текст, неактивные элементы */
  --color-muted:         #8b95a5;   /* Подсказки, placeholder, вспомогательный текст */
  --color-accent:        #22c55e;   /* Успех, положительная статистика */
  --color-warning:       #f59e0b;   /* Предупреждения */
  --color-danger:        #ef4444;   /* Ошибки, критичные проблемы */

  /* === Status Indicators (для /status) === */
  --status-available:    #22c55e;   /* 🟢 Свободен */
  --status-busy:         #f59e0b;   /* 🟡 Занят */
  --status-full:         #ef4444;   /* 🔴 Полностью загружен */

  /* === Backgrounds === */
  --color-bg-page:       #ffffff;   /* Фон основной страницы */
  --color-bg-section:    #f8fafc;   /* Фон чередующихся секций */
  --color-bg-elevated:   #ffffff;   /* Фон карточек, модалок (тени) */

  /* === Surfaces === */
  --color-surface:       #f1f5f9;   /* Фон для второстепенных блоков */
  --color-surface-hover: #e2e8f0;   /* Hover состояние поверхности */

  /* === Text === */
  --color-text-primary:    #0f172a; /* Основной текст (заголовки) */
  --color-text-secondary:  #475569; /* Вторичный текст (параграфы) */
  --color-text-tertiary:   #94a3b8; /* Третичный текст (подсказки) */
  --color-text-inverse:    #ffffff; /* Текст на тёмном фоне */

  /* === Borders === */
  --color-border:        #e2e8f0;   /* Базовая граница */
  --color-border-focus:  #0d6efd;   /* Граница при фокусе */

  /* === Shadows === */
  --shadow-sm:   0 1px 2px rgba(0,0,0,0.05);
  --shadow-md:   0 4px 6px -1px rgba(0,0,0,0.07), 0 2px 4px -2px rgba(0,0,0,0.05);
  --shadow-lg:   0 10px 15px -3px rgba(0,0,0,0.08), 0 4px 6px -4px rgba(0,0,0,0.04);
  --shadow-xl:   0 20px 25px -5px rgba(0,0,0,0.1),  0 8px 10px -6px rgba(0,0,0,0.06);

  /* === Radii === */
  --radius-sm:   4px;   /* Элементы форм, чипы */
  --radius-md:   8px;   /* Карточки, кнопки */
  --radius-lg:   12px;  /* Большие контейнеры */
  --radius-xl:   16px;  /* Hero секции */
  --radius-full: 9999px; /* Avatars, бейджи с текстом */

  /* === Transitions === */
  --transition-fast:   150ms cubic-bezier(0.4, 0, 0.2, 1);
  --transition-base:   200ms cubic-bezier(0.4, 0, 0.2, 1);
  --transition-slow:   300ms cubic-bezier(0.4, 0, 0.2, 1);

  /* === Fonts (переопределяются ниже) === */
  --font-heading: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
  --font-body:    'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
  --font-mono:    'JetBrains Mono', ui-monospace, monospace;

  /* === Z-indexes === */
  --z-base:      0;
  --z-nav:       100;
  --z-chat:      200;
  --z-modal:     300;
  --z-toast:     400;
}
```

---

## Типографика

### Иерархия заголовков
| Тег | Размер (px) | Weight | Line height | Используется для |
|-----|-------------|--------|-------------|------------------|
| H1  | 48px (3rem) | 700    | 1.2         | Hero: имя специалиста |
| H2  | 36px (2.25rem) | 700 | 1.3         | Заголовки секций («Услуги», «Проекты») |
| H3  | 24px (1.5rem) | 600   | 1.4         | Карточки услуг, заголовки кейсов |
| H4  | 20px (1.25rem) | 600  | 1.4         | Подзаголовки в текстах |
| Body | 16px (1rem)  | 400    | 1.6         | Основной текст, параграфы |
| Small | 14px (0.875rem) | 400 | 1.5      | Подсказки, даты, метки |
| Caption | 12px (0.75rem) | 500 | 1.4     | Юридический текст, alt-текст |

**Уровень карточки зависит от страницы, размер — нет.** На главной карточки
стоят внутри секции с `<h2>`, поэтому их заголовок — `<h3>`. На `/services` и
`/projects` карточки лежат прямо в секции страницы, и там правильный уровень —
`<h2>`: иначе получается пропуск (`<h1>` → `<h3>`), который краулер отметил как
ошибку доступности на двух страницах. Размер задаёт класс (`.card-title`,
`.post-card-title` с явным `font-weight: 600`), поэтому заголовки выглядят
одинаково при любом уровне. Уровень передаётся пропом (`ServiceCard`,
`ProjectCard`, `article_card`).

### Mobile breakpoints для типографики
```css
/* Пересчёт шрифтов на мобильных */
@media (max-width: 768px) {
  :root {
    --fs-h1:   32px;  /* 48 → 32 */
    --fs-h2:   28px;  /* 36 → 28 */
    --fs-h3:   20px;  /* 24 → 20 */
    --fs-body: 16px;  /* остаётся 16 */
  }
}

@media (max-width: 480px) {
  :root {
    --fs-h1:   28px;  /* 32 → 28 */
    --fs-h2:   24px;  /* 28 → 24 */
    --fs-h3:   18px;  /* 20 → 18 */
  }
}
```

### Правила набора текста
- **Межстрочное расстояние в параграфах**: минимум 1.6 — читается легко
- **Максимальная длина строки**: 65-75 символов (не растягивать текст на всю ширину)
- **Заголовки секций**: не более одной строки, если влезает — ок; если нет — перенос без «разрыва мысли»
- **Цифры и метрики**: моноширинный шрифт или font-variant: tabular-nums для выравнивания

---

## Цветовая система

### Использование цветов по ролям
| Элемент | Default | Hover | Active | Disabled |
|---------|---------|-------|--------|----------|
| Primary кнопка | `--color-primary` (#0d6efd) | `--color-primary-hover` (#0b5ed7) | darken 8% | #c4d4f0 + opacity 0.5 |
| Secondary кнопка | transparent + border | `--color-bg-section` | `--color-surface-hover` | inherit + opacity 0.4 |
| Ссылки | `--color-primary` | underline | underline + darken | — |
| Input border | `--color-border` | `--color-border-focus` | — | `--color-surface` |

### Статусы занятости (для /status)

Состояние описывается **парой**: постоянное подлежащее + короткое состояние.
Подлежащее не меняется от состояния к состоянию — иначе бейдж читается как три
разных текста, а «ограниченная доступность» без подлежащего непонятна
посетителю, который только зашёл.

| Статус | Бейдж | Цвет точки |
|---|---|---|
| `available` | Приём заявок: открыт | `--color-status-available`, пульсирует |
| `busy` | Приём заявок: ограничен | `--color-status-busy` |
| `full` | Приём заявок: закрыт | `--color-status-full`, без пульсации |

Правила:

- **Цвет — дублирующий сигнал, не единственный.** Слово состояния обязано
  читаться без наведения: на телефоне hover недоступен, скринридер цвет не
  видит. Поэтому в разметке есть `.status-state`, а полный текст дублируется в
  `aria-label`.
- Подсказка с подробностями — **дополнение** к надписи, а не её замена.
  Раскрывается наведением (CSS `@media (hover: hover)`), фокусом с клавиатуры
  (`:focus-visible`) и тапом (`initStatusBadge` в `assets/main.js`).
  Внутри: расшифровка одной фразой, «В работе N проектов», «Ближайший слот».
- Подлежащее и формулировки берутся из `services/status.rs::presentation` —
  единственного словаря для бейджа, чата, `/api/status` и Telegram-бота.
- Классы: `.status-badge.status-{status}`, внутри `.status-dot`,
  `.status-subject`, `.status-state`, `.status-popover`.
- Разметка и стили — `src/ui/shared.rs::StatusBadge` и `assets/style.css`.
  В правилах они не дублируются: копия CSS здесь уже один раз разошлась с
  кодом (были `.status-badge.available`, в коде — `.status-available`).

### Доступность — проверка контраста
| Комбинация | Контраст | Проходит WCAG AA? |
|------------|----------|-------------------|
| `#0f172a` на `#ffffff` | 15.4:1 | ✅ Да (≥4.5) |
| `#475569` на `#ffffff` | 7.8:1  | ✅ Да (≥4.5) |
| `#0d6efd` на `#ffffff` | 4.6:1  | ✅ Да (≥4.5) — barely! |
| `#8b95a5` на `#ffffff` | 3.2:1  | ❌ Нет (< 3.0 для small text) |
| `#e7f1ff` на `#ffffff` | 1.4:1  | ❌ Только как фон, не текст |

**Правило**: Never use `--color-text-tertiary` (#94a3b8) for body text on white background. Use it only for decorative elements or increase weight to at least 500.

---

## Сетка и пространство

### Breakpoints
```css
/* Mobile-first breakpoints */
$breakpoints: (
  mobile:    0px,   /* ≤768px — мобильные */
  tablet:    768px, /* ≥769px — планшеты */
  desktop:   1024px,/* ≥1025px — десктоп */
  wide:      1280px,/* ≥1281px — большие экраны */
);
```

### Container max-widths
| Breakpoint | Max width | Padding horizontal |
|-----------|-----------|-------------------|
| Mobile (<768px) | 100% (full bleed для hero), 90vw контентный | 1.5rem |
| Tablet (≥768px) | 720px контент, 100% секции фона | 2rem |
| Desktop (≥1024px) | 960px контент, 100% секции фона | 2.5rem |
| Wide (≥1280px) | 1200px контент, 100% секции фона | 3rem |

### Вертикальный ритм (spacing scale)
```css
/* Модульная шкала: 4px base unit */
--space-xs:   0.25rem; /* 4px   — микроотступы */
--space-sm:   0.5rem;  /* 8px   — между элементами в карточке */
--space-md:   1rem;    /* 16px  — стандартный отступ */
--space-lg:   1.5rem;  /* 24px  — между секциями мелкими */
--space-xl:   2.5rem;  /* 40px  — между крупными блоками */
--space-2xl:  4rem;    /* 64px  — между секциями страницы */
--space-3xl:  6rem;    /* 96px  — hero padding vertical */

/* Применяется через utility классы или напрямую */
.section-padding {
  padding-top: var(--space-3xl);
  padding-bottom: var(--space-3xl);
}

.card-padding {
  padding: var(--space-xl);
}
```

---

## Компоненты

### 1. Кнопки (Buttons)

#### Primary Button
```css
.btn-primary {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.875rem 2rem;
  background: var(--color-primary);
  color: var(--color-text-inverse);
  font-family: var(--font-body);
  font-size: 1rem;
  font-weight: 600;
  line-height: 1;
  text-decoration: none;
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--transition-base), transform var(--transition-fast), box-shadow var(--transition-base);
  box-shadow: var(--shadow-sm);
}

.btn-primary:hover {
  background: var(--color-primary-hover);
  transform: translateY(-1px);
  box-shadow: var(--shadow-md);
}

.btn-primary:active {
  transform: translateY(0);
  box-shadow: none;
}

.btn-primary:focus-visible {
  outline: 3px solid color-mix(in srgb, var(--color-primary) 40%, transparent);
  outline-offset: 2px;
}

/* Размеры */
.btn-primary.sm { padding: 0.5rem 1rem; font-size: 0.875rem; }
.btn-primary.lg { padding: 1rem 2.5rem; font-size: 1.125rem; }
```

#### Secondary Button
```css
.btn-secondary {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.875rem 2rem;
  background: transparent;
  color: var(--color-primary);
  font-family: var(--font-body);
  font-size: 1rem;
  font-weight: 600;
  line-height: 1;
  text-decoration: none;
  border: 2px solid var(--color-primary);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-base);
}

.btn-secondary:hover {
  background: color-mix(in srgb, var(--color-primary) 8%, transparent);
  transform: translateY(-1px);
}
```

#### Button pair (CTA на главной)
```css
/* На мобильном — вертикально, один под другим */
@media (max-width: 768px) {
  .cta-buttons {
    flex-direction: column;
    gap: var(--space-sm);
  }
}

/* На десктопе — горизонтально, рядом */
@media (min-width: 769px) {
  .cta-buttons {
    display: flex;
    gap: var(--space-md);
  }
  
  .btn-primary, .btn-secondary {
    min-width: 220px; /* чтобы кнопки не «схлопывались» */
  }
}
```

### 2. Навигация (Navbar)
```css
.navbar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: var(--z-nav);
  background: color-mix(in srgb, var(--color-bg-page) 92%, transparent);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border-bottom: 1px solid var(--color-border);
  transition: box-shadow var(--transition-base);
}

/* Тень появляется при скролле вниз */
.navbar.scrolled {
  box-shadow: var(--shadow-sm);
}

.navbar-inner {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 var(--space-md);
  height: 64px; /* mobile: имя слева + бургер справа */
  display: flex;
  align-items: center;
  justify-content: space-between;
}

/* Desktop navbar: три ячейки — имя │ меню по центру │ статус.
   Боковые `1fr` равны, поэтому ссылки стоят ровно посередине «пилюли». */
@media (min-width: 768px) {
  .navbar-inner {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
  }
  .navbar-menu { display: contents; }
}

.navbar__logo {
  font-weight: 700;
  font-size: 1.125rem;
  color: var(--color-text-primary);
  text-decoration: none;
}

.navbar__links {
  display: flex;
  gap: var(--space-md);
  list-style: none;
}

.navbar__link {
  font-size: 0.9375rem;
  color: var(--color-text-secondary);
  text-decoration: none;
  padding: 0.25rem 0;
  border-bottom: 2px solid transparent;
  transition: color var(--transition-fast), border-color var(--transition-fast);
}

.navbar__link:hover,
.navbar__link.active {
  color: var(--color-text-primary);
  border-bottom-color: var(--color-primary);
}
```

#### Состав шапки (слева направо)

| Элемент | Класс | Условие показа |
|---|---|---|
| Имя владельца | `.navbar-brand` | всегда |
| Ссылки разделов | `.navbar-links` | Главная, Услуги, Проекты, Блог, Чат, Контакты |
| Статус занятости | `.status-badge` | всегда |

**Внешних профилей в шапке нет.** GitHub и Habr переехали в подвал (см. §7):
в шапке две круглые кнопки спорили за место с навигацией, а на телефоне
раздували выпадающую панель. Статус занятости остаётся крайним справа — он
главный сигнал шапки.

**Меню стоит по центру «пилюли».** На десктопе (≥768px) `.navbar-inner` — сетка
`1fr auto 1fr`: боковые колонки равны, поэтому блок ссылок стоит посередине
независимо от ширины имени и бейджа. Обёртка `.navbar-menu` при этом не создаёт
бокс (`display: contents`), и её дети — ссылки и бейдж — становятся прямыми
ячейками сетки. На мобильном обёртка снова коробка: выпадающая панель.

Почему не `margin-left: auto` и не абсолютное центрирование: первое прижимает
меню вправо, второе ломается от длинной подписи статуса — её владелец задаёт из
админки, и при `position: absolute` бейдж налез бы на ссылки. Сетка раздвигает
колонки и не даёт элементам пересечься; на узких ширинах (768–800px) контент
занимает «пилюлю» почти целиком, поэтому ссылки смещаются от центра на 1–15px,
а от 900px и выше совпадение точное.

Правила для внешних профилей (`.social-link`, подвал — см. §7):
- **Брендовые знаки, а не интерфейсные иконки.** GitHub рисуется силуэтом
  (`BrandIcon`, заливка `currentColor`), в отличие от интерфейсных Lucide
  (`LucideIcon`, обводка 1px). Наборы не смешиваются.
- **Источник GitHub — официальные Octicons**, регенерируются
  `scripts/update-icons.sh`. Руками путь не правится: в Lucide брендовые иконки
  удалили.
- Ссылки внешние: `target="_blank"` + `rel="me noopener noreferrer"`.
  `rel="me"` подтверждает, что профиль принадлежит владельцу сайта.
- **GitHub не зависит от снимка статистики.** Адрес публикуется при старте из
  конфига (`services/github.rs::set_profile`), поэтому кнопка не исчезает, когда
  GitHub API недоступен, лимит исчерпан или блок «Открытый код» выключен через
  `enabled`. Прячется только пустым `username`.

/* Mobile hamburger (только на мобильных) */
.navbar__toggle {
  display: flex; /* visible on mobile */
  background: none;
  border: none;
  padding: 0.5rem;
  cursor: pointer;
}

@media (min-width: 1024px) {
  .navbar__toggle { display: none; }
}
```

### 3. Карточки (Cards)
```css
.card {
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: var(--space-xl);
  transition: box-shadow var(--transition-base), transform var(--transition-base);
}

.card:hover {
  box-shadow: var(--shadow-md);
  transform: translateY(-2px);
}

/* Для карточек услуг — иконка сверху */
.card__icon {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-primary-light);
  color: var(--color-primary);
  border-radius: var(--radius-md);
  margin-bottom: var(--space-md);
  font-size: 1.5rem;
}

.card__title {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--color-text-primary);
  margin-bottom: var(--space-sm);
}

.card__description {
  font-size: 0.9375rem;
  color: var(--color-text-secondary);
  line-height: 1.5;
  margin-bottom: var(--space-md);
}

/* Grid для карточек услуг — 3 колонки на десктопе */
.services-grid {
  display: grid;
  gap: var(--space-lg);
  grid-template-columns: 1fr; /* mobile first */
}

@media (min-width: 768px) {
  .services-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (min-width: 1024px) {
  .services-grid {
    grid-template-columns: repeat(3, 1fr);
  }
}
```

### 4. Статус-бейдж (Availability Badge)

Полная спецификация — в разделе «Статусы занятости (для /status)» выше.
Кратко: постоянное подлежащее + короткое состояние, цветная точка как
дублирующий сигнал, подсказка с фактами по наведению, фокусу и тапу.

### 5. Формы (Forms)
```css
.form-group {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  margin-bottom: var(--space-md);
}

.form-label {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--color-text-secondary);
}

.form-input,
.form-textarea {
  padding: 0.75rem 1rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  font-family: var(--font-body);
  font-size: 1rem;
  color: var(--color-text-primary);
  background: var(--color-bg-page);
  transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
}

.form-input:focus,
.form-textarea:focus {
  outline: none;
  border-color: var(--color-border-focus);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 15%, transparent);
}

.form-input::placeholder,
.form-textarea::placeholder {
  color: var(--color-text-tertiary);
}

/* Validation states */
.form-input.error {
  border-color: var(--color-danger);
}

.form-input.success {
  border-color: var(--color-accent);
}

.error-message {
  font-size: 0.8125rem;
  color: var(--color-danger);
  margin-top: 0.25rem;
}

/* Textarea с минимальной высотой */
.form-textarea {
  min-height: 120px;
  resize: vertical;
}

/* Поля в строку на десктопе, друг под другом на мобильном */
.form-row {
  display: grid;
  gap: var(--space-md);
  grid-template-columns: 1fr; /* mobile first */
}

@media (min-width: 640px) {
  .form-row.two-col {
    grid-template-columns: repeat(2, 1fr);
  }
}
```

### 6. Чат-виджет (Chat Widget)
```css
/* Chat container — встроенный в страницу */
.chat-container {
  max-width: 720px;
  margin: var(--space-2xl) auto 0;
  padding: var(--space-xl);
}

/* Preset кнопки — горизонтально на десктопе, wrap на мобильных */
.preset-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-sm);
  margin-bottom: var(--space-lg);
}

.preset-button {
  padding: 0.625rem 1.25rem;
  background: var(--color-bg-page);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-family: var(--font-body);
  font-size: 0.875rem;
  color: var(--color-text-primary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.preset-button:hover {
  border-color: var(--color-primary);
  background: var(--color-primary-light);
  color: var(--color-primary);
}

.preset-button:active {
  transform: scale(0.98);
}

/* Область сообщений */
.messages-area {
  min-height: 300px;
  max-height: 50vh;
  overflow-y: auto;
  padding-bottom: var(--space-md);
}

.message {
  margin-bottom: var(--space-md);
  padding: 1rem 1.25rem;
  border-radius: var(--radius-lg);
  max-width: 85%; /* для user — 85% */
  line-height: 1.6;
}

.message.user {
  background: var(--color-surface);
  color: var(--color-text-primary);
  margin-left: auto;
  border-bottom-right-radius: var(--radius-sm);
}

.message.bot {
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border);
  box-shadow: var(--shadow-sm);
  margin-right: auto;
  border-bottom-left-radius: var(--radius-sm);
}

/* CTA внутри ответа бота */
.message .cta-button {
  display: inline-block;
  margin-top: var(--space-sm);
  padding: 0.5rem 1.25rem;
  background: var(--color-primary);
  color: white;
  text-decoration: none;
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  font-weight: 500;
  transition: background var(--transition-fast);
}

.message .cta-button:hover {
  background: var(--color-primary-hover);
}

/* Input area */
.chat-input-area {
  display: flex;
  gap: var(--space-sm);
  padding-top: var(--space-md);
  border-top: 1px solid var(--color-border);
}

.chat-input {
  flex: 1;
  padding: 0.75rem 1rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-family: var(--font-body);
  font-size: 1rem;
}

.chat-input:focus {
  outline: none;
  border-color: var(--color-border-focus);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 15%, transparent);
}
```

#### Плавающий чат на главной (`.chat-fab`)

Терминал в hero — единственный вход в диалог на главной, и при прокрутке он
уезжает вверх. Кнопка в правом нижнем углу возвращает вход и раскрывает
**компактную версию того же чата** — обычными пузырями сообщений
(`.chat-message`), не терминалом: второй терминал на странице спорил бы с hero.

Правила:

- **Кнопка привязана к прокрутке, а не к таймеру.** Появляется, когда секция
  `.hero` полностью ушла выше экрана (`getBoundingClientRect().bottom <= 0`), а
  не когда показался какой-то процент страницы: до этого диалог доступен в
  самом терминале.
- **«Отображается постоянно» — это память браузера.** Если человек уже открывал
  чат, ключ `inteli:chat-opened` в `localStorage` показывает кнопку сразу, без
  прокрутки, и состояние переживает обновление страницы. `sessionStorage` для
  этого не годится: визит — не вкладка. Недоступное хранилище (приватный режим)
  не ломает чат — кнопка просто ведёт себя как при первом визите.
- **Панель лежит в разметке и закрыта атрибутом `hidden`.** Собранная из JS
  панель не существовала бы вовсе, если скрипт не доехал; открытая для всех
  сразу — мешала бы читать страницу. Открывает и закрывает её JS
  (`initFloatingChat`), `hidden` в разметке — состояние по умолчанию.
- **Открытую панель прокрутка не прячет**: человек открыл её сам. Кнопка
  скрывается только пока панель закрыта.
- **Три способа закрыть**: своя кнопка «×» в шапке панели, `Escape` и повторный
  клик по кнопке. Открытая панель прячет кнопку (`.chat-fab.is-open`): две
  кнопки рядом с одним действием — лишний выбор.
- **Прокручивается только лента сообщений.** Поле ввода и шапка остаются на
  виду — как строка ввода в терминале.
- **Кнопки-подсказки — то же подмножество, что в терминале**
  (`services::chat::hero_presets`), и уходят после первого вопроса: в компактной
  панели дорога каждая строка.
- Транспорт один на все три входа: `postChat` в `assets/main.js` (страница
  /chat, терминал, панель). Своя обёртка вокруг `fetch` у каждого означала бы,
  что «сеть упала» и «сервер отказал» обрабатываются по-разному.
- Панель **не модальное окно**: `z-index: 45`, ниже шапки и ниже подсказки
  статуса. На телефоне ширина — `min(360px, 100vw - 2 * --space-md)`.
- **Есть на каждой странице, кроме `/chat`**: там диалог занимает весь экран, и
  вторая кнопка «Спросить» дублировала бы поле ввода под ней. Разметку рендерит
  `ui::layout::Layout`, а страницу-исключение он узнаёт из `RequestUrl` — сама
  страница чата про виджет ничего не знает и знать не должна.
- **Привязка к прокрутке — только на главной.** Терминал в hero есть ровно там,
  поэтому на остальных страницах (и на 404, где секция `.hero` тоже есть) кнопка
  видна сразу: `initFloatingChat` ищет привязку по `#hero-terminal`, а не по
  `.hero`.

---

### 7. Подвал (Footer)

Два столбца: слева — подпись автора, контакты и внешние профили, справа —
«работает на Beget». На мобильном столбцы складываются в колонку.

```css
.footer { border-top: 1px solid var(--color-border); padding: var(--space-xl) var(--space-lg); }
.footer-inner {
  max-width: var(--container-max);
  display: flex; align-items: center; justify-content: space-between;
  flex-wrap: wrap; gap: var(--space-lg);
}
.footer-col { display: flex; flex-direction: column; gap: var(--space-sm); }
.footer-social { display: flex; align-items: center; gap: var(--space-sm); margin-top: var(--space-xs); }
```

Правила:

- **Внешние профили живут здесь, а не в шапке.** GitHub и Habr — две витрины
  автора (код и статьи), одной строкой `.footer-social` под контактами. В шапке
  они спорили за место с навигацией, а на телефоне раздували выпадающую панель;
  статус занятости при этом остаётся крайним справа.
- **Круглая кнопка `.social-link`**: 36×36, `border-radius: 999px`, та же рамка и
  фон, что у остальных вторичных элементов. Подпись — в `aria-label` и `title`,
  знак внутри — декоративный.
- **GitHub — инлайн-`<svg>`** (`BrandIcon`, заливка `currentColor`), поэтому
  наведение красит его вместе с рамкой.
- **Habr — отдельный файл** `/assets/habr.svg` с версией в адресе и годовым
  кэшем: официальная фигура весит ~16 КБ, и в разметке каждой страницы она
  удваивала бы вес документа. В `<img>` `currentColor` не работает, поэтому
  заливка в файле белая, а вес гасит `opacity` (0.62 → 1 на
  `:hover`/`:focus-visible`) — кнопки должны читаться одинаково. Источник фигуры
  — Simple Icons (CC0), файл не правится вручную.
- Адрес профиля Habr — константа в коде (`ui::layout::HABR_URL`), а не поле
  контента: у Habr нет API, из которого сайт мог бы перечитать логин. GitHub
  берётся из `services::github::profile()` и не зависит от снимка статистики.
- Обе ссылки внешние: `target="_blank"` + `rel="me noopener noreferrer"`.

---

## Страницы проекта

### Главная страница (/) — Hero секция

**Layout:**
```
┌─────────────────────────────────────┐
│              Navbar                  │
├─────────────────────────────────────┤
│                                      │
│         [Имя Фамилия]               │     ← H1, 48px/700
│   Специалист по продвижению сайтов    │     ← subtitle, 20px/500
│          · 10 лет опыта              │     ← accent text
│                                      │
│      🟢 [Свободен для новых проектов] │     ← status badge
│                                      │
│   [💬 Задать вопрос]                 │     ← Primary CTA
│   [📩 Оставить заявку]               │     ← Secondary CTA
│                                      │
│         (3 карточки услуг)           │     ← preview below CTA
│                                      │
├─────────────────────────────────────┤
│              Footer                  │
└─────────────────────────────────────┘
```

**Правила Hero секции:**
- Vertical padding: 6rem top/bottom (desktop), 4rem (mobile)
- **Сверху — воздух `--hero-top-gap`** (48px на десктопе, 32px на телефоне)
  поверх отступа от липкой шапки: между шапкой и первым экраном (терминал и CTA)
  должен быть видимый зазор, иначе они читаются как продолжение шапки. Формула —
  `padding: calc(var(--nav-offset) + var(--hero-top-gap)) …`, сама переменная
  живёт в токенах: зазор подбирается глазами и меняется в одном месте
- Content центрирован по горизонтали и вертикали
- Имя — H1, максимум 30 символов на строку (2 строки max)
- subtitle + опыт — в одну строку через CSS Flex
- Две CTA кнопки: primary + secondary, одинакового размера
- Под CTA — горизонтальный разделитель (`<hr>`) → затем 3 карточки услуг
- Каждая карточка услуги: иконка (emoji или SVG) + название + 1 предложение описания
- Окно терминала не ограничено по высоте (`max-height` у `.terminal-window`
  нет). Прокручивается ровно одна область — лента сообщений (`.terminal-body`:
  180px, на телефоне 100–140px), а строка ввода, подсказка о лимитах и кнопки
  обязаны быть видны целиком: это единственный вход в чат на главной. Пока окно
  стояло на `max-height`, эти три ряда в сумме в него не помещались и нижний ряд
  срезало `overflow: hidden` — на 1024–1199px кнопки переносятся на вторую
  строку, и окно растёт вместе с ними. Поэтому высота окна — следствие
  содержимого, а не заданное число
- Подвал терминала (`.terminal-footer`: строка ввода, подсказка о лимитах,
  кнопки) лежит на своей подложке — на 4–5% белого светлее ленты сообщений и с
  более заметной кромкой (`--color-terminal-footer`, `--color-terminal-ledge`).
  Это не украшение, а второй план окна: без подложки окно — плоская тёмная
  плашка, и маскот висит в ней картинкой. Контраст держится маленьким, чтобы
  подвал не читался отдельной панелью. Черта принадлежит границе «лента —
  подвал», поэтому `border-top` живёт на `.terminal-footer`, а не на строке ввода
- Маскот (ведьма Кики) сидит внутри AI-терминала на верхней кромке подвала
  (`border-top` у `.terminal-footer`) — это граница между лентой сообщений и
  полем «Задайте вопрос...». Поэтому разметку маскота рендерит `HeroTerminal`, и
  стоит она внутри строки ввода: лента сообщений растёт и прокручивается, а
  кромка остаётся на месте — фигура не уезжает вместе с текстом. Опора кадра —
  80% его высоты (`transform: translateY(-80%)`): там у `base` силуэт падает с
  126px до 46px, выше идёт платье, ниже — только ноги, и они свешиваются на
  подвал. Для остальных кадров поза другая, и опору поправляет `--seat-nudge` на
  самом кадре (`base`/`dissatisfied` +1.5%, `smirk` +1%, `thinking` −4%,
  `read` −6% — замер по строке, где корпус перестаёт быть широким). Без поправки
  у кадров с поднятыми коленями кромка идёт по бёдрам, и фигура читается висящей
- Тень маскота — тёмная и направленная: свет сверху и чуть спереди, поэтому тень
  уходит вниз-вправо. Два слоя `drop-shadow`: контактный (2×3px, размытие 3px,
  0.6) сажает фигуру на кромку, отброшенный (10×16px, размытие 18px, 0.45) даёт
  длинную мягкую тень на подвале и ленте. Цвет — почти чёрный с фиолетовым
  подтоном, не серый. Светлый ореол в цвет primary здесь запрещён: ровное
  свечение вокруг силуэта читается как левитация, а не как опора. Тень скрыта под
  самой фигурой (drop-shadow рисуется позади картинки), обрезана `overflow:
  hidden` окна по правому краю и не поднимается выше шляпы — смещение только
  вниз, поэтому текст в ленте над фигурой остаётся чистым. Множитель
  `--mascot-k` держит пропорции тени при уменьшении кадра
- Кадры `assets/mascot/*.png` — настоящий пиксель-арт 144×192, а не картинка
  «под пиксели»; в CSS они показываются размером, кратным холсту (пропорция 3:4
  через `--mascot-w`: 144px на широких экранах, 120px на 1024–1199px, 96px на
  планшете, 120px на телефоне), потому что дробный масштаб у пиксель-арта
  разъезжается. Клики сквозь маскот проходят (`pointer-events: none`), для
  скринридера он декоративный (`aria-hidden` на контейнере, `alt=""` на
  кадрах). Место — только hero главной: второй экземпляр на сайте не нужен
- Маскот виден на всех размерах экрана. Раньше он прятался ниже 768px — тогда
  он висел над верхней гранью окна, и ему не хватало 97px до шапки. Опора
  переехала внутрь окна, поэтому причина исчезла: на телефоне кадр снова
  крупный (120px), а не скрытый
- Эмоции маскота — реакция на действия в терминале, а не украшение: `base` в
  покое, `read` пока запрос в полёте, `thinking` пока печатается ответ,
  `smirk` когда ответ допечатан, `dissatisfied` когда запрос не удался. Все
  пять кадров лежат в разметке сразу, переключается только `data-emotion` на
  контейнере (см. `initMascot` в main.js): смена эмоции не ждёт сети и не
  мигает пустым местом. Реакции на событие (`smirk`, `dissatisfied`) держатся
  несколько секунд и возвращаются в `base` — иначе ведьма осталась бы
  довольной до следующего вопроса
- Маскот делается из картинок-источников один раз, вне репозитория, скриптом
  `scripts/make-mascot.mjs [каталог] [ширина] [высота]` (по умолчанию
  144×192): обрезка по альфе, уменьшение усреднением по площади (RGB
  усредняется premultiplied, иначе по краям остаётся тёмный ореол), альфа
  бинарная, выравнивание по низу и центру общего холста. Холст и масштаб
  считаются по самому высокому и самому широкому кадру: если считать их по
  каждому кадру отдельно, при смене эмоции персонаж менял бы размер. Размер
  холста продублирован в `api::assets::{MASCOT_WIDTH, MASCOT_HEIGHT}` — это
  единственное место, откуда его берут разметка и тест. Исходники — рабочий
  материал художника, в репозиторий они не попадают; сам скрипт остаётся,
  иначе следующий маскот придётся вырезать заново на глаз
- Знак `</>` над именем убран. Он занимал верх композиции, но ничего не
  сообщал: терминал слева уже говорит, что перед посетителем разработчик.
  Вместе с разметкой убрано и правило `.hero-eyebrow` — CSS без элемента
  возвращается на место «само», при первой же правке соседних стилей
- Навыки (`profile.skills`, `.tech-pill`) — витрина услуг, а не список
  технологий. Шесть пунктов: «Приватные AI-системы», «Локальный инференс
  (Qwen 3.6/3.8)», «MCP-серверы и AI Skills», «Разработка под ключ»,
  «Docker & DevOps», «Генерация изображений». Названия инструментов
  («Голосовые ассистенты», «ComfyUI Mass Production») отсюда убраны: они
  читались как стек, а не как то, что можно заказать. Список живёт в
  `memory::content::fallback_content`, а не в разметке
- Текст у финального CTA — рамка, а не срок: «Оставьте заявку — отвечу при
  наличии свободного окна». Обещания ответить за 24 часа на сайте нет нигде: это
  обязательство перед каждым, кто написал, и оно превращает «посмотрю, если тема
  моя» в «отвечу всем и вовремя». Рамка говорит о расписании автора, а не о
  ценности того, кто написал: формулировки вроде «если смогу быть полезен»
  отклонены — условие в них направлено на автора, и клиент читает это как «может,
  и не смогу». Текст один на все поверхности: главная, `/contact`, подвал статьи,
  ответ чата о заявке, промпты модели и описание страницы для поисковиков

**Content на главной:**
```html
<h1>Иван Петров</h1>
<p class="subtitle">Специалист по продвижению сайтов · 10 лет опыта</p>
<StatusBadge status="available" />
<div class="cta-buttons">
  <a href="/chat" class="btn-primary btn-lg">💬 Задать вопрос</a>
  <a href="#contact-form" class="btn-secondary btn-lg">📩 Оставить заявку</a>
</div>
<hr />
```

### Главная страница (/) — порядок секций

Актуальный состав главной (сверху вниз). Блок «Открытый код» стоит **между
«Результатами» и финальным CTA**: числа с GitHub — внешне проверяемый
аргумент, поэтому он идёт последним перед призывом, уже после собственных кейсов.

| # | Секция | Класс | Обязательна |
|---|--------|-------|-------------|
| 1 | Hero (терминал + имя + статус + навыки) | `.hero` | да |
| 2 | Услуги (превью 3 карточек) | `.section-services` | да |
| 3 | Результаты (превью 2 проектов) | `.section-results` | да |
| 4 | Открытый код (статистика GitHub) | `.section-github` | **нет** |
| 5 | Финальный CTA | `.section-cta` | да |

Секция «Открытый код» рендерится **только при наличии снимка статистики**
(`GitHubStats::get()`): если GitHub недоступен, логин выключен в конфиге или
запрос не удался, в разметке не остаётся даже заголовка — пустая рамка с нулями
хуже отсутствия блока. Состав плиток: контрибуции за год, публичные
репозитории, число языков, лет на GitHub. Звёзды и подписчики показываются
только при `show_stars = true` в `[github]` — маленькие числа на странице продаж
работают против нас. Подробности — `docs/github-stats.md`.

**Layout:**
```
┌──────────── Открытый код ───────────────┐
│  Открытый код        github.com/u/ →    │  ← H2 + section-link
│  ┌────────────────────────────────────┐ │
│  │ (аватар) Имя                       │ │  ← ссылка на профиль
│  │          @login                    │ │
│  ├────────────────────────────────────┤ │
│  │ [125]      [48]     [6]     [10]   │ │  ← плитки «число + подпись»
│  │ вкладов    репози-  языков  лет    │ │
│  ├────────────────────────────────────┤ │
│  │ ▓▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░░░░░░░░ │ │  ← полоса языков (Linguist)
│  │ ● JavaScript 9 реп.  ● PHP 8 реп.  │ │  ← легенда
│  └────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

### Страница услуг (/services)

**Layout:**
```
┌──────────── Service Section ────────────┐
│                                          │
│  Услуги                                  │     ← H2, по левому краю
│  Комплексный подход к SEO-продвижению    │     ← subtitle, серый текст
│                                          │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ │
│  │ 🔍       │ │ 📈       │ │ ⚙️       │ │
│  │ SEO-аудит │ │ Продвиж. │ │ Тех. SEO │ │     ← Cards grid
│  │ ...      │ │ Яндекс/  │ │ ...      │ │
│  │ от 15к ₽ │ │ Google   │ │ ...      │ │
│  └──────────┘ └──────────┘ └──────────┘ │
│                                          │
└──────────────────────────────────────────┘
```

### Страница проектов (/projects)

**Layout:**
```
┌─────────── Projects Section ─────────────┐
│                                           │
│  Проекты                                  │     ← H2
│                                           │
│  ┌───────────────────────────────────┐    │
│  │ [Изображение/скриншот]             │    │
│  │                                   │    │
│  │ Интернет-магазин электроники       │    │ ← H3
│  │                                   │    │
│  │ Было: 120 визитов/день            │    │ ← Метрика "до"
│  │ Стало: 890 визитов/день           │    │ ← Метрика "после"
│  │ Рост +642% за 4 месяца            │    │ ← Итог
│  │                                   │    │
│  │ [Посмотреть кейс →]               │    │     ← Text link (не кнопка)
│  └───────────────────────────────────┘    │
│                                           │
│  ... ещё кейсы по одному под другим       │    │
└───────────────────────────────────────────┘
```

### Страница AI-чата (/chat)

**Layout:**
```
┌──────────── Chat Section ───────────────┐
│                                          │
│  Задать вопрос                           │     ← H2 (по левому краю)
│  AI-ассистент поможет вам быстро         │     ← subtitle
│                                           │
│  ┌────────────────────────────────────┐  │
│  │ Preset-кнопки (6 шт, wrap grid)    │  │
│  │ [👤 Кто вы?] [💼 Услуги]           │  │
│  │ [💰 Цены]   [🔍 Анализ сайта]      │  │
│  │ [🟢 Статус]   [📩 Заявка]          │  │
│  ├────────────────────────────────────┤  │
│  │                                    │  │
│  │ [Поле ввода текста...]             │  │ ← chat input
│  │                        [Отправить →] │  │
│  └────────────────────────────────────┘  │
│                                           │
│  Или свяжитесь напрямую:                  │     ← Footer чата
│  Telegram @username  |  Email: ...       │
└───────────────────────────────────────────┘
```

### Страница контактов (/contact)

**Layout:**
```
┌───────── Contact Section ────────────────┐
│                                           │
│  Оставить заявку                          │     ← H2
│                                           │
│  ┌────────────────────────────────────┐   │
│  │                                    │   │
│  │ Имя * [_______________]            │   │
│  │ Email        [_______________]     │   │
│  │ Телефон      [_______________]     │   │
│  │ Сообщение* [_________________]     │   │ ← textarea, min-height: 120px
│  │          [_____________________]     │   │
│  │          [_____________________]     │   │
│  │                                    │   │
│  │       [📩 Отправить заявку]        │   │ ← Primary CTA, full-width mobile
│  │                                    │   │
│  └────────────────────────────────────┘   │
│                                           │
│  Или напишите напрямую:                    │     ← контакты под формой
│  Telegram @username    VK vk.com/user    │
│                                           │
└───────────────────────────────────────────┘
```

---

## Адаптивный дизайн

### Mobile-first подход

| Элемент | <768px (Mobile) | ≥768px (Tablet) | ≥1024px (Desktop) |
|---------|-----------------|-----------------|--------------------|
| Navbar | Hamburger menu, лого слева | Full links visible, меню по центру | Ссылки по центру, статус справа |
| Hero H1 | 32px, центрирован | 40px, центрирован | 48px, центрирован |
| CTA кнопки | Вертикально (column), full-width | Горизонтально, min-width | Горизонтально, auto |
| Карточки услуг | 1 колонка | 2 колонки | 3 колонки |
| Кейсы проектов | 1 под другим | 1 под другим | 2 рядом в grid |
| Форма контактов | Одноколоночная (все поля) | email+phone в строку | Стандартная форма |
| Chat preset кнопки | Wrap, max 2 в ряд | Wrap, max 3-4 в ряд | Wrap, max 5-6 в ряд |

### Breakpoint-specific rules

```css
/* === Mobile (< 768px) === */
@media (max-width: 767px) {
  /* Hero: центрирование текста */
  .hero-content { text-align: center; }
  
  /* CTA кнопки — full width, один под другим */
  .cta-buttons { flex-direction: column; }
  .cta-buttons button { width: 100%; }
  
  /* Убираем лишние отступы секций */
  section { padding: var(--space-xl) 0; }
}

/* === Tablet (≥768px and <1024px) === */
@media (min-width: 768px) and (max-width: 1023px) {
  /* Сетка услуг — 2 колонки */
  .services-grid { grid-template-columns: repeat(2, 1fr); }
  
  /* Кейсы проектов — можно показать preview рядом с текстом */
}

/* === Desktop (≥1024px) === */
@media (min-width: 1024px) {
  /* Сетка услуг — 3 колонки */
  .services-grid { grid-template-columns: repeat(3, 1fr); }
  
  /* Hero content: max-width для читаемости */
  .hero-content { max-width: 800px; }
}

/* === Wide (≥1280px) === */
@media (min-width: 1280px) {
  /* Максимальная ширина контента */
  .container { max-width: 1200px; }
}
```

---

## Микроанимации

### Правила микроанимаций
1. **Время ≤ 300ms** — дольше раздражает, чем помогает
2. **Только для интерактивных элементов** — hover, click, focus
3. **Никаких анимаций при загрузке страницы** (кроме fade-in секций)
4. **Отключаем для prefers-reduced-motion**

### Hover эффекты
```css
/* Кнопки: подъём + тень */
.btn-primary:hover {
  transform: translateY(-1px);
  box-shadow: var(--shadow-md);
}

/* Карточки: лёгкий подъём */
.card:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-md);
}

/* Ссылки: underline-эффект */
.navbar__link::after {
  content: '';
  display: block;
  width: 0;
  height: 2px;
  background: var(--color-primary);
  transition: width var(--transition-base);
}

.navbar__link:hover::after {
  width: 100%;
}
```

### Scroll-based анимации (fade-in секций)
```css
/* Секции появляются при скролле */
.fade-in-section {
  opacity: 0;
  transform: translateY(20px);
  transition: opacity var(--transition-slow), transform var(--transition-slow);
}

.fade-in-section.visible {
  opacity: 1;
  transform: translateY(0);
}

/* Intersection Observer включается в JS */
const observer = new IntersectionObserver(
  (entries) => entries.forEach(entry => {
    if (entry.isIntersecting) {
      entry.target.classList.add('visible');
      observer.unobserve(entry.target);
    }
  }),
  { threshold: 0 }
);

document.querySelectorAll('.fade-in-section').forEach(el => observer.observe(el));
```

**Порог — по первому пикселю, а не по доле пересечения.** Здесь скрытие в CSS
означает, что при неудаче JS контента нет вообще, поэтому `threshold` обязан
быть достижим при любой высоте блока. Доля пересечения ограничена сверху
отношением высоты экрана к высоте секции: у статьи на 10 000 px при экране
720 px максимум ≈7%, и `threshold: 0.1` не наступит никогда — блок навсегда
останется с `opacity: 0`. Ровно так пропала статья «35B-модель на RTX 5080»
(сентябрь 2026). Анимация — это украшение, поэтому у наблюдателя есть
`try/catch` с показом всех секций, а для выключенного JS — `<noscript>` в
`shell()`. Действующая реализация — `initReveal()` в `assets/main.js`.

### Focus-visible для доступности
```css
/* Показываем фокус только при навигации клавиатурой */
button:focus, a:focus {
  outline: none;
}

button:focus-visible, a:focus-visible {
  outline: 3px solid var(--color-primary);
  outline-offset: 2px;
  border-radius: var(--radius-sm);
}
```

---

## Доступность (a11y)

### Обязательные правила WCAG 2.1 AA

| Требование | Как реализуем |
|-----------|--------------|
| Контраст текста ≥ 4.5:1 | Все primary текст: `#0f172a` на `#fff` = 15.4:1 ✅ |
| Навигация с клавиатуры | tabindex не нужен — порядок в HTML, :focus-visible |
| Alt-текст для изображений | Все картинки портфолио имеют alt; обложка статьи — её заголовок |
| ARIA-лейблы для кнопок без текста | Кнопки с emoji имеют aria-label |
| Доступное имя у ссылки-иконки | Ссылка на обложку статьи ведёт туда же, куда заголовок, но имени из текста у неё нет — имя задаёт `aria-label` |
| Форма: label + input связаны | Каждая форма имеет `<label for="...">` |
| Поле без видимой подписи | Доступное имя задаёт `aria-label` (поле чата, терминал на главной): краулер считает поле без имени ошибкой |
| Уровни заголовков без пропусков | Уровень карточки зависит от страницы — см. «Иерархия заголовков» |
| Цвет ≠ единственный индикатор | Статусы: цвет + иконка + текст (не только зелёный) |

### Семантический HTML
```html
<!-- ✅ Правильно -->
<header role="banner">...</header>
<nav aria-label="Основная навигация">...</nav>
<main id="content" role="main">...</main>
<footer role="contentinfo">...</footer>

<section aria-labelledby="services-heading">
  <h2 id="services-heading">Услуги</h2>
  ...
</section>

<!-- ❌ Никогда -->
<div class="nav"> (используйте <nav>)
<span class="button"> (используйте <button> или <a>)
```

### ARIA для чат-виджета
```html
<div role="region" aria-label="AI-чат с ассистентом">
  <!-- Preset buttons -->
  <div role="group" aria-label="Частые вопросы">
    <button aria-label="Узнать о специалисте">👤 Кто вы?</button>
    ...
  </div>
  
  <!-- Messages area (live region для динамического контента) -->
  <div 
    role="log" 
    aria-live="polite" 
    aria-relevant="additions"
    class="messages-area"
  >
    <!-- Сообщения добавляются динамически, screen reader прочитает новые -->
  </div>
  
  <!-- Input -->
  <form aria-label="Отправить сообщение в чат">
    <input type="text" aria-label="Ваш вопрос" placeholder="Введите ваш вопрос..." />
    <button type="submit" aria-label="Отправить сообщение">Отправить</button>
  </form>
</div>
```

### Skip link (переход к контенту)
```html
<!-- Первый элемент в body, видим только при Tab -->
<a href="#content" class="skip-link">Перейти к основному содержимому</a>

.skip-link {
  position: absolute;
  top: -100%;
  left: 50%;
  transform: translateX(-50%);
  padding: 0.75rem 1.5rem;
  background: var(--color-primary);
  color: white;
  border-radius: var(--radius-sm);
  z-index: var(--z-toast);
  transition: top var(--transition-fast);
}

.skip-link:focus {
  top: var(--space-sm);
}
```

### Keyboard navigation
| Клавиша | Действие |
|---------|----------|
| Tab / Shift+Tab | Навигация по интерактивным элементам |
| Enter / Space | Активация кнопки/ссылки |
| Escape | Закрыть модальное окно (если будет) |
| Arrow Up/Down | Навигация в чате между preset-кнопками (опционально) |

---

*См. также: [architecture.md](./architecture.md), [chatbot.md](./chatbot.md)*
