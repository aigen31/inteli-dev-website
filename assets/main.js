// inteli.dev.ru — ванильный JS: интерактивность без фреймворков (P2: лёгкость).
// Обработчики: чат (POST /api/chat), форма заявки (POST /api/lead), fade-in.

(function () {
  'use strict';

  // ---------- Fade-in секций при скролле (IntersectionObserver) ----------
  function initReveal() {
    var targets = document.querySelectorAll('.section, .hero');
    if (!('IntersectionObserver' in window)) {
      targets.forEach(function (t) { t.classList.add('is-visible'); });
      return;
    }
    var io = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        if (entry.isIntersecting) {
          entry.target.classList.add('is-visible');
          io.unobserve(entry.target);
        }
      });
    }, { threshold: 0.1 });
    targets.forEach(function (t) { io.observe(t); });
  }

  // ---------- Чат ----------
  function initChat() {
    var app = document.getElementById('chat-app');
    if (!app) return;

    var messages = document.getElementById('chat-messages');
    var input = document.getElementById('chat-input');
    var sendBtn = document.getElementById('chat-send');

    function addMessage(text, who) {
      var div = document.createElement('div');
      div.className = 'chat-message ' + (who === 'user' ? 'user-message' : 'bot-message');
      div.textContent = text;
      messages.appendChild(div);
      messages.scrollTop = messages.scrollHeight;
      return div;
    }

    function addSuggestions(suggested) {
      if (!Array.isArray(suggested) || !suggested.length) return;
      var row = document.createElement('div');
      row.className = 'chat-suggestions';
      suggested.forEach(function (s) {
        var b = document.createElement('button');
        b.type = 'button';
        b.className = 'preset-button';
        b.textContent = s.text;
        b.addEventListener('click', function () {
          if (s.type && s.type.indexOf('link:') === 0) {
            window.location.href = s.type.slice(5);
          } else {
            send({ message: s.text, question_type: 'lead_request' });
          }
        });
        row.appendChild(b);
      });
      messages.appendChild(row);
      messages.scrollTop = messages.scrollHeight;
    }

    async function send(payload) {
      try {
        var res = await fetch('/api/chat', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(payload)
        });
        var data = await res.json().catch(function () { return {}; });
        if (!res.ok) {
          addMessage(data.error || 'Ошибка. Попробуйте позже.', 'bot');
          return;
        }
        addMessage(data.answer, 'bot');
        addSuggestions(data.suggested_next);
      } catch (err) {
        addMessage('Сетевая ошибка. Попробуйте ещё раз.', 'bot');
      }
    }

    function sendFree() {
      var text = input.value.trim();
      if (!text) return;
      addMessage(text, 'user');
      input.value = '';
      send({ message: text, question_type: 'free' });
    }

    sendBtn.addEventListener('click', sendFree);
    input.addEventListener('keydown', function (e) {
      if (e.key === 'Enter') sendFree();
    });

    app.querySelectorAll('.preset-button[data-kind]').forEach(function (btn) {
      btn.addEventListener('click', function () {
        var kind = btn.getAttribute('data-kind');
        var index = btn.getAttribute('data-index');
        addMessage(btn.textContent, 'user');

        if (kind === 'analysis') {
          var url = window.prompt('Введите ссылку на ваш сайт (например, example.com):');
          if (!url || !url.trim()) return;
          send({ message: btn.textContent, question_type: 'analysis', url: url.trim() });
        } else {
          var payload = { message: btn.textContent, question_type: kind };
          if (index) payload.preset_index = parseInt(index, 10);
          send(payload);
        }
      });
    });
  }

  // ---------- Форма заявки ----------
  function initLeadForm() {
    var form = document.getElementById('lead-form');
    if (!form) return;
    var status = document.getElementById('lead-form-status');

    form.addEventListener('submit', async function (e) {
      e.preventDefault();
      var data = {
        name: form.name.value.trim(),
        message: form.message.value.trim(),
        source: 'form'
      };
      if (form.email && form.email.value.trim()) data.email = form.email.value.trim();
      if (form.phone && form.phone.value.trim()) data.phone = form.phone.value.trim();

      if (!data.name || !data.message) {
        status.textContent = 'Заполните имя и сообщение.';
        status.className = 'form-status error';
        return;
      }

      try {
        var res = await fetch('/api/lead', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(data)
        });
        var json = await res.json().catch(function () { return {}; });
        if (res.ok) {
          status.textContent = json.message || 'Заявка отправлена!';
          status.className = 'form-status success';
          form.reset();
        } else {
          status.textContent = json.error || 'Ошибка отправки.';
          status.className = 'form-status error';
        }
      } catch (err) {
        status.textContent = 'Сетевая ошибка. Попробуйте ещё раз.';
        status.className = 'form-status error';
      }
    });
  }

  // ---------- Админ-панель ----------
  function escapeHtml(s) {
    return String(s == null ? '' : s)
      .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;').replace(/'/g, '&#39;');
  }

  function initAdmin() {
    var app = document.getElementById('admin-app');
    if (!app) return;

    var login = document.getElementById('admin-login');
    var dashboard = document.getElementById('admin-dashboard');
    var tokenInput = document.getElementById('admin-token');
    var loginBtn = document.getElementById('admin-login-btn');
    var loginStatus = document.getElementById('admin-login-status');

    var STORAGE_KEY = 'admin_token';

    function token() {
      return localStorage.getItem(STORAGE_KEY) || '';
    }

    function authHeaders() {
      return { 'Authorization': 'Bearer ' + token(), 'Content-Type': 'application/json' };
    }

    async function api(path, options) {
      var res = await fetch(path, Object.assign({ headers: authHeaders() }, options || {}));
      var body = await res.json().catch(function () { return {}; });
      return { ok: res.ok, status: res.status, body: body };
    }

    function showLogin(message) {
      login.hidden = false;
      dashboard.hidden = true;
      if (message) { loginStatus.textContent = message; loginStatus.className = 'form-status error'; }
    }

    function showDashboard() {
      login.hidden = true;
      dashboard.hidden = false;
    }

    async function loadDashboard() {
      var stats = await api('/api/admin/stats');
      if (stats.status === 401) { showLogin('Неверный или пустой токен.'); return; }

      var [leads, chats] = await Promise.all([
        api('/api/admin/leads?limit=100'),
        api('/api/admin/chats?limit=100')
      ]);

      renderKpi(stats.body);
      renderLeads(leads.body);
      renderChats(chats.body);
      showDashboard();
    }

    function renderKpi(s) {
      var el = document.getElementById('admin-kpi');
      var avg = s.avg_response_time_ms == null ? '—' : Math.round(s.avg_response_time_ms) + 'ms';
      var cards = [
        ['📩 Заявки сегодня', s.leads_today != null ? s.leads_today : '—'],
        ['💬 Чатов сегодня', s.chats_today != null ? s.chats_today : '—'],
        ['⚡ Ср. время ответа', avg]
      ];
      el.innerHTML = cards.map(function (c) {
        return '<div class="kpi-card"><div class="kpi-value">' + escapeHtml(c[1]) +
               '</div><div class="kpi-label">' + escapeHtml(c[0]) + '</div></div>';
      }).join('');
    }

    function renderLeads(leads) {
      var table = document.getElementById('admin-leads-table');
      if (!Array.isArray(leads) || !leads.length) {
        table.innerHTML = '<tbody><tr><td colspan="6">Заявок пока нет.</td></tr></tbody>';
        return;
      }
      var statuses = ['new', 'processing', 'contacted', 'converted', 'dismissed'];
      var rows = leads.map(function (l) {
        var contact = [l.email, l.phone].filter(Boolean).join(' / ') || '—';
        var select = '<select class="lead-status" data-id="' + l.id + '">' + statuses.map(function (st) {
          return '<option value="' + st + '"' + (st === l.status ? ' selected' : '') + '>' + st + '</option>';
        }).join('') + '</select>';
        return '<tr><td data-label="Дата">' + escapeHtml((l.created_at || '').slice(0, 16).replace('T', ' ')) +
          '</td><td data-label="Имя">' + escapeHtml(l.name) +
          '</td><td data-label="Контакт">' + escapeHtml(contact) +
          '</td><td data-label="Сообщение">' + escapeHtml((l.message || '').slice(0, 120)) +
          '</td><td data-label="Источник">' + escapeHtml(l.source) +
          '</td><td data-label="Статус">' + select + '</td></tr>';
      }).join('');
      table.innerHTML = '<thead><tr><th>Дата</th><th>Имя</th><th>Контакт</th><th>Сообщение</th><th>Источник</th><th>Статус</th></tr></thead><tbody>' + rows + '</tbody>';

      table.querySelectorAll('.lead-status').forEach(function (sel) {
        sel.addEventListener('change', async function () {
          var id = sel.getAttribute('data-id');
          var res = await api('/api/admin/leads/' + id, {
            method: 'PATCH',
            body: JSON.stringify({ status: sel.value })
          });
          if (!res.ok) { alert('Не удалось обновить статус: ' + (res.body.error || res.status)); }
        });
      });
    }

    function renderChats(chats) {
      var table = document.getElementById('admin-chats-table');
      if (!Array.isArray(chats) || !chats.length) {
        table.innerHTML = '<tbody><tr><td colspan="5">Чатов пока нет.</td></tr></tbody>';
        return;
      }
      var rows = chats.map(function (c) {
        var rt = c.response_time_ms == null ? '—' : c.response_time_ms + 'ms';
        return '<tr><td data-label="Дата">' + escapeHtml((c.created_at || '').slice(0, 16).replace('T', ' ')) +
          '</td><td data-label="Вопрос">' + escapeHtml((c.user_message || '').slice(0, 100)) +
          '</td><td data-label="Ответ">' + escapeHtml((c.bot_response || '').slice(0, 100)) +
          '</td><td data-label="Тип">' + escapeHtml(c.question_type || 'free') +
          '</td><td data-label="Время">' + escapeHtml(rt) + '</td></tr>';
      }).join('');
      table.innerHTML = '<thead><tr><th>Дата</th><th>Вопрос</th><th>Ответ</th><th>Тип</th><th>Время отв.</th></tr></thead><tbody>' + rows + '</tbody>';
    }

    loginBtn.addEventListener('click', async function () {
      var value = tokenInput.value.trim();
      if (!value) { loginStatus.textContent = 'Введите токен.'; loginStatus.className = 'form-status error'; return; }
      localStorage.setItem(STORAGE_KEY, value);
      await loadDashboard();
    });
    tokenInput.addEventListener('keydown', function (e) {
      if (e.key === 'Enter') loginBtn.click();
    });

    // Автовход, если токен уже сохранён.
    if (token()) { loadDashboard(); }
  }

  // ---------- Интерактивные карточки: tilt + radial spotlight ----------
  function initHeroCanvas() {
    var canvas = document.getElementById('hero-canvas');
    if (!canvas) return;
    var ctx = canvas.getContext('2d');
    if (!ctx) return;
    var hero = canvas.parentElement;
    var reduceMotion = !!(window.matchMedia &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches);

    var W = 0, H = 0, cols = 0, rows = 0;
    var grid = [];
    var mouse = { x: -1e5, y: -1e5, active: false };
    var rafId = 0, running = false;
    var CURSOR_R = 190;   // радиус влияния курсора
    var PUSH = 30;        // сила отталкивания точек
    var IDLE_AMP = 3;     // лёгкое «дыхание» в покое
    var FADE_START = 620; // Y-координата, с которой начинается затухание сетки (фиксированная)

    function build() {
      // Force canvas to match the CSS-stretched dimensions (inset bottom: -120px)
      var hero = canvas.parentElement;
      var heroRect = hero.getBoundingClientRect();
      
      // Wait for layout to settle, then read actual canvas bounding rect
      requestAnimationFrame(function() {
        var canvasRect = canvas.getBoundingClientRect();
        W = canvasRect.width;
        H = canvasRect.height;
        if (W < 40 || H < 40) return;

        var dpr = Math.min(window.devicePixelRatio || 1, 2);
        canvas.width = Math.round(W * dpr);
        canvas.height = Math.round(H * dpr);
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

        // Плотная сетка: базовый шаг, расширяемый при ресайзе canvas.
        var base = W < 640 ? 130 : 110;   // крупные ячейки
        // Растягиваем сетку по всей высоте canvas (без зоны сохранения снизу).
        cols = Math.max(2, Math.round(W / base));
        rows = Math.max(2, Math.round(H / base));
        var spX = cols > 1 ? W / (cols - 1) : W;
        var spY = rows > 1 ? H / (rows - 1) : H;

        grid = [];
        for (var r = 0; r < rows; r++) {
          var row = [];
          for (var c = 0; c < cols; c++) {
            row.push({ hx: c * spX, hy: r * spY, cx: c * spX, cy: r * spY, ph: Math.random() * 6.283 });
          }
          grid.push(row);
        }
        layoutReady = true;
        if (reduceMotion) draw();
      });
    }

    function draw() {
      if (W < 40 || H < 40) return;
      ctx.clearRect(0, 0, W, H);

      // Зона затухания — фиксированная Y=620. С этой высоты начинается прозрачность.
      var fadeStart = FADE_START; // 620px
      var fadeRange = H - fadeStart; // ~241px при canvas.height=861

      // Линии сетки — с постепенным затуханием к низу.
      ctx.lineWidth = 1;
      for (var r = 0; r < rows; r++) {
        var row = grid[r];
        for (var c = 0; c < cols; c++) {
          var d = row[c];

          // Горизонтальная линия к соседу справа.
          if (c + 1 < cols) {
            var nx = row[c + 1].cx, ny = row[c + 1].cy;
            var avgY = (d.cy + ny) / 2;
            var lineAlpha = avgY < fadeStart ? 0.045 : Math.max(0, 0.045 * (1 - (avgY - fadeStart) / fadeRange));
            ctx.strokeStyle = 'rgba(255,255,255,' + lineAlpha.toFixed(3) + ')';
            ctx.beginPath();
            ctx.moveTo(d.cx, d.cy);
            ctx.lineTo(nx, ny);
            ctx.stroke();
          }

          // Вертикальная линия к соседу снизу.
          if (r + 1 < rows) {
            var sx = grid[r + 1][c].cx, sy = grid[r + 1][c].cy;
            var avgYV = (d.cy + sy) / 2;
            var lineAlphaV = avgYV < fadeStart ? 0.045 : Math.max(0, 0.045 * (1 - (avgYV - fadeStart) / fadeRange));
            ctx.strokeStyle = 'rgba(255,255,255,' + lineAlphaV.toFixed(3) + ')';
            ctx.beginPath();
            ctx.moveTo(d.cx, d.cy);
            ctx.lineTo(sx, sy);
            ctx.stroke();
          }
        }
      }

      // Точки сетки — с постепенным затуханием к низу.
      for (r = 0; r < rows; r++) {
        row = grid[r];
        for (c = 0; c < cols; c++) {
          d = row[c];
          var dotAlpha = d.cy < fadeStart ? 0.32 : Math.max(0, 0.32 * (1 - (d.cy - fadeStart) / fadeRange));
          ctx.fillStyle = 'rgba(214,210,255,' + dotAlpha.toFixed(3) + ')';
          ctx.beginPath();
          ctx.moveTo(d.cx + 2.2, d.cy);
          ctx.arc(d.cx, d.cy, 2.2, 0, 6.283);
          ctx.fill();
        }
      }

      // Финальное затемнение — чёрный градиент поверх зоны fade для полного растворения в фоне.
      if (fadeRange > 0) {
        var grad = ctx.createLinearGradient(0, fadeStart, 0, H);
        grad.addColorStop(0, 'rgba(10,10,20,0)');
        grad.addColorStop(0.6, 'rgba(10,10,20,0.4)');
        grad.addColorStop(1, 'rgba(10,10,20,0.95)');
        ctx.globalCompositeOperation = 'source-over';
        ctx.fillStyle = grad;
        ctx.fillRect(0, fadeStart, W, fadeRange);
      }
    }

    // Вычисляем целевые координаты и плавно двигаем точки.
    function update() {
      var t = performance.now() / 1000;
      for (var r = 0; r < rows; r++) {
        var row = grid[r];
        for (var c = 0; c < cols; c++) {
          var d = row[c];
          var idle = reduceMotion ? 0 : Math.sin(t * 1.1 + d.ph) * IDLE_AMP;
          var tx = d.hx;
          var ty = d.hy + idle;

          if (mouse.active && !reduceMotion) {
            var vx = d.hx - mouse.x;
            var vy = d.hy - mouse.y;
            var dist = Math.sqrt(vx * vx + vy * vy);
            if (dist < CURSOR_R && dist > 0.01) {
              var k = 1 - dist / CURSOR_R;
              var push = PUSH * k * k;
              tx += (vx / dist) * push;
              ty += (vy / dist) * push;
            }
          }
          d.cx += (tx - d.cx) * 0.1;
          d.cy += (ty - d.cy) * 0.1;
        }
      }
    }

    function frame() {
      if (!running) return;
      rafId = requestAnimationFrame(frame);
      update();
      draw();
    }

    function start() {
      if (running || reduceMotion) return;
      running = true;
      cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(frame);
    }
    function stop() {
      running = false;
      cancelAnimationFrame(rafId);
    }

    function onMove(e) {
      var rect = canvas.getBoundingClientRect();
      mouse.x = e.clientX - rect.left;
      mouse.y = e.clientY - rect.top;
      mouse.active = true;
    }

    window.addEventListener('pointermove', onMove, { passive: true });
    hero.addEventListener('pointerleave', function () { mouse.active = false; });
    window.addEventListener('blur', function () { mouse.active = false; });

    // Запускаем/останавливаем анимацию при попадании hero в вьюпорт.
    if ('IntersectionObserver' in window) {
      new IntersectionObserver(function (entries) {
        entries.forEach(function (en) {
          if (en.isIntersecting) { start(); } else { stop(); }
        });
      }, { threshold: 0.05 }).observe(hero);
    }

    var layoutReady = false; // flag что build() завершил асинхронно

    function onResize() {
      build();
      // Ждём завершения requestAnimationFrame перед запуском анимации
      setTimeout(function() {
        if (reduceMotion) { draw(); } else if (layoutReady) { start(); }
      }, 50);
    }

    if ('ResizeObserver' in window) {
      new ResizeObserver(onResize).observe(hero);
    } else {
      window.addEventListener('resize', onResize);
    }

    build();
    // После начального build() ждём RAF и стартуем
    setTimeout(function() {
      layoutReady = true;
      if (!reduceMotion) { start(); }
    }, 100);
  }

  // ---------- Hero: интерактивная AI-терминальная сессия (CLI) ----------
  function initHeroTerminal() {
    var terminal = document.getElementById('hero-terminal');
    if (!terminal) return;

    var output = document.getElementById('hero-terminal-output');
    var input = document.getElementById('hero-terminal-input');
    var inputRow = document.getElementById('hero-terminal-input-row');
    var reduceMotion = !!(window.matchMedia &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches);

    // Состояние терминала
    var isTyping = false;       // идёт анимация печати ответа
    var isBusy = false;         // запрос к API ещё не завершён

    // ---------- Утилиты ----------
    function scrollToBottom() {
      output.scrollTop = output.scrollHeight;
    }

    function appendLine(className, text) {
      var line = document.createElement('div');
      line.className = 'terminal-line ' + (className || '');
      line.textContent = text;
      output.appendChild(line);
      scrollToBottom();
      return line;
    }

    // ---------- Анимация посимвольной печати ----------
    function typeText(element, text, callback) {
      if (!element) return; // null element — просто пропускаем (для приветствия используем appendLine)
      if (reduceMotion) {
        element.textContent = text;
        if (callback) callback();
        return;
      }
      isTyping = true;
      var i = 0;
      var cursorEl = document.createElement('span');
      cursorEl.className = 'terminal-cursor';
      cursorEl.innerHTML = '█';
      element.textContent = '';
      element.appendChild(cursorEl);

      function typeChar() {
        if (i < text.length) {
          // Вставляем символ перед курсором
          var span = document.createElement('span');
          span.style.opacity = '0.85';
          span.textContent = text[i];
          element.insertBefore(span, cursorEl);
          i++;
          scrollToBottom();
          setTimeout(typeChar, 12 + Math.random() * 8); // случайная скорость для реалистичности
        } else {
          // Убираем мигающий курсор после завершения
          setTimeout(function() {
            if (cursorEl.parentNode) cursorEl.remove();
            isTyping = false;
            enableInput();
            if (callback) callback();
          }, 400);
        }
      }
      typeChar();
    }

    // ---------- Анимация загрузочного индикатора ("думает...") ----------
    function showLoading() {
      var line = document.createElement('div');
      line.className = 'terminal-line terminal-loading';
      line.innerHTML = '<span class="code-fn">[думает]</span><span class="loading-dots">...</span>';
      output.appendChild(line);
      scrollToBottom();

      // Анимация точек
      var dotsEl = line.querySelector('.loading-dots');
      var dotCount = 0;
      var dotInterval = setInterval(function() {
        dotCount = (dotCount + 1) % 4;
        dotsEl.textContent = '.'.repeat(dotCount);
      }, 500);

      return function hideLoading() {
        clearInterval(dotInterval);
        line.remove();
      };
    }

    // ---------- Управление состоянием ввода ----------
    function disableInput() {
      isBusy = true;
      input.disabled = true;
      input.placeholder = 'ждём ответ...';
      inputRow.classList.add('is-busy');
    }

    function enableInput() {
      isBusy = false;
      input.disabled = false;
      input.placeholder = 'Задайте вопрос...';
      inputRow.classList.remove('is-busy');
      input.focus();
    }

    // ---------- Приветственное сообщение (typewriter) ----------
    var welcomeLine = document.createElement('div');
    welcomeLine.className = 'terminal-line terminal-welcome';
    output.appendChild(welcomeLine);
    typeText(welcomeLine, '> inteli-dev CLI v1.0 — спрашивайте обо мне, услугах, ценах или оставьте заявку.', function() {
      input.focus();
    });

    // ---------- Отправка сообщения в API ----------
    async function sendMessage(message, questionType) {
      if (isTyping) return; // только проверка typing — isBusy будет установлен сразу

      disableInput();
      var loading = showLoading();

      try {
        var res = await fetch('/api/chat', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ message: message, question_type: questionType })
        });

        if (!res.ok) throw new Error('HTTP ' + res.status);

        var data = await res.json();
        loading(); // убрать лоадер

        appendLine('terminal-user', '$ > ' + message);

        // Напечатать ответ от бота с анимацией
        var botLine = document.createElement('div');
        botLine.className = 'terminal-line terminal-bot-response';
        output.appendChild(botLine);

        typeText(botLine, data.answer, function() {
          scrollToBottom();
          enableInput();
        });

      } catch (err) {
        loading(); // убрать лоадер в случае ошибки
        appendLine('terminal-error', '⚠ Ошибка: ' + err.message);
        enableInput();
      }
    }

    // ---------- Обработчики событий ----------
    function handleSend() {
      var text = input.value.trim();
      if (!text) return;
      sendMessage(text, 'free');
      input.value = '';
    }

    input.addEventListener('keydown', function(e) {
      if (e.key === 'Enter') handleSend();
    });

    // Preset-кнопки
    terminal.querySelectorAll('.terminal-preset-btn').forEach(function(btn) {
      btn.addEventListener('click', function() {
        var kind = btn.getAttribute('data-kind');
        var index = btn.getAttribute('data-index');
        var message = btn.textContent.replace(/^["']|["']$/g, ''); // убрать кавычки из шаблона

        if (kind === 'analysis') {
          var url = prompt('Введите ссылку на ваш сайт (например, example.com):');
          if (!url || !url.trim()) return;
          sendMessage(message + ' (' + url.trim() + ')', 'analysis');
        } else {
          var payload = { message: message, question_type: kind };
          if (index) payload.preset_index = parseInt(index, 10);
          sendMessage(message, kind);
        }
      });
    });
  }

  // ---------- Кнопки: radial spotlight под курсором (без box-shadow) ----------
  function initButtonSpotlight() {
    var buttons = document.querySelectorAll('.btn');
    if (!buttons.length) return;

    buttons.forEach(function(btn) {
      function onMove(e) {
        var rect = btn.getBoundingClientRect();
        var x = e.clientX - rect.left;
        var y = e.clientY - rect.top;
        // CSS-переменные для radial spotlight.
        btn.style.setProperty('--mx', x + 'px');
        btn.style.setProperty('--my', y + 'px');
      }

      function onLeave() {
        btn.style.setProperty('--mx', '50%');
        btn.style.setProperty('--my', '50%');
      }

      btn.addEventListener('pointermove', onMove, { passive: true });
      btn.addEventListener('pointerleave', onLeave);
    });
  }

  // ---------- Hero: интерактивная геометрическая сетка (реагирует на курсор) ----------
  function initCardEffects() {
    var cards = document.querySelectorAll('.card');
    if (!cards.length) return;

    var MAX_TILT = 6; // градусов максимального наклона
    var ROTATE_SPEED = 0.15; // плавность возврата (0-1, меньше = медленнее)

    cards.forEach(function(card) {
      var currentRotateX = 0;
      var currentRotateY = 0;
      var targetRotateX = 0;
      var targetRotateY = 0;
      var rafId = 0;
      var reduceMotion = !!(window.matchMedia &&
        window.matchMedia('(prefers-reduced-motion: reduce)').matches);

      function onMove(e) {
        var rect = card.getBoundingClientRect();
        var x = e.clientX - rect.left;
        var y = e.clientY - rect.top;

        // CSS-переменные для radial spotlight.
        card.style.setProperty('--mx', x + 'px');
        card.style.setProperty('--my', y + 'px');

        if (!reduceMotion) {
          // Вычисляем tilt: нормализуем позицию к [-1, 1], умножаем на MAX_TILT.
          targetRotateX = ((y / rect.height - 0.5) * -2) * MAX_TILT;
          targetRotateY = ((x / rect.width - 0.5) * 2) * MAX_TILT;
        }

        // Запускаем анимацию если не запущена.
        if (!rafId && !reduceMotion) { rafId = animate(); }
      }

      function onLeave() {
        targetRotateX = 0;
        targetRotateY = 0;
        card.style.setProperty('--mx', '50%');
        card.style.setProperty('--my', '50%');
      }

      function animate() {
        currentRotateX += (targetRotateX - currentRotateX) * ROTATE_SPEED;
        currentRotateY += (targetRotateY - currentRotateY) * ROTATE_SPEED;

        // Условие остановки: близко к нулю.
        if (Math.abs(currentRotateX) < 0.01 && Math.abs(currentRotateY) < 0.01 &&
            targetRotateX === 0 && targetRotateY === 0) {
          currentRotateX = 0;
          currentRotateY = 0;
          card.style.transform = '';
          rafId = 0;
          return;
        }

        card.style.transform = 'perspective(800px) rotateX(' + currentRotateX.toFixed(2) +
          'deg) rotateY(' + currentRotateY.toFixed(2) + 'deg)';
        rafId = requestAnimationFrame(animate);
      }

      // Убираем raf на mouseleave чтобы цикл остановился.
      card.addEventListener('pointermove', onMove, { passive: true });
      card.addEventListener('pointerleave', onLeave);
    });
  }

  initReveal();
  initHeroTerminal();
  initHeroCanvas();
  initCardEffects();
  initButtonSpotlight();
  initChat();
  initLeadForm();
  initAdmin();
})();
