#!/usr/bin/env python3
"""Статический сервер для визуальной проверки hero-секции без базы.

Отдаёт заранее снятый HTML главной вместе с ассетами и отвечает на POST
/api/chat: с задержкой 1.2 с (чтобы поймать состояние «read») и ошибкой 500,
если в вопросе есть слово fail. Нужен только для ручной проверки маскота:
эмоции переключаются по жизненному циклу запроса, а он без заглушки не
воспроизводится.

Запуск: python3 .fool-work/mascot/serve.py [порт] [каталог]
"""

import json
import sys
import time
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path(sys.argv[2] if len(sys.argv) > 2 else ".fool-work/mascot/site")
ANSWER = (
    "Делаю технический аудит, собираю семантику и правлю то, что мешает "
    "индексации. Обычно это две недели до первых сдвигов."
)


class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(ROOT), **kwargs)

    def do_POST(self):  # noqa: N802 — имя задано базовым классом
        length = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(length).decode("utf-8", "replace")
        try:
            message = json.loads(body or "{}").get("message", "")
        except json.JSONDecodeError:
            message = ""
        time.sleep(1.2)  # окно, в котором маскот обязан быть в «read»
        if "fail" in message.lower():
            payload = json.dumps({"error": "заглушка: так задумано"}).encode()
            self.send_response(500)
        else:
            payload = json.dumps({"answer": ANSWER}).encode()
            self.send_response(200)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, *args):  # тише в консоли
        pass


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8899
    print(f"http://127.0.0.1:{port}/  (root: {ROOT.resolve()})", flush=True)
    ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
