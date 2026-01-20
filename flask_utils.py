import threading
from flask import Flask, send_from_directory
import os

from alarm import info

SITE_DIR = r"C:\Bots\commonData\ordering\tripethall"

app = Flask(
    __name__,
    static_folder=SITE_DIR,
    template_folder=SITE_DIR,
)

@app.route("/")
def index():
    return send_from_directory(SITE_DIR, "index.html")

@app.route("/<path:path>")
def static_proxy(path):
    return send_from_directory(SITE_DIR, path)

def run_site():
    info.put("Поднимаю triplethall.ru")
    app.run(host="0.0.0.0", port=5000, debug=False, use_reloader=False)

def start_site_in_thread():
    t = threading.Thread(target=run_site, daemon=True)
    t.start()


