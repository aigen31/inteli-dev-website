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

  initReveal();
  initChat();
  initLeadForm();
  initAdmin();
})();
