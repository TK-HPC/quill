import math
import os
import re
import sqlite3
import time
from pathlib import Path


def record_dictation(text, duration_seconds):
    if text.strip() == "":
        return
    if math.isfinite(duration_seconds) == False or duration_seconds < 0:
        raise ValueError("Dictation duration must be finite and nonnegative")

    configured_data_home = os.environ.get("XDG_DATA_HOME", "")
    if configured_data_home == "":
        data_home = Path.home() / ".local/share"
    else:
        data_home = Path(configured_data_home)
    if data_home.is_absolute() == False:
        data_home = Path.home() / ".local/share"
    directory = data_home / "quill"
    directory.mkdir(mode=0o700, parents=True, exist_ok=True)
    directory.chmod(0o700)
    path = directory / "history.sqlite3"
    descriptor = os.open(path, os.O_CREAT | os.O_WRONLY, 0o600)
    os.close(descriptor)
    path.chmod(0o600)

    connection = sqlite3.connect(path, timeout=0.25)
    try:
        with connection:
            connection.execute("BEGIN IMMEDIATE")
            version = connection.execute("PRAGMA user_version").fetchone()[0]
            if version > 1:
                raise RuntimeError(f"Unsupported history schema version: {version}")
            connection.execute("""
                CREATE TABLE IF NOT EXISTS dictations (
                    id INTEGER PRIMARY KEY,
                    created_at INTEGER NOT NULL,
                    text TEXT NOT NULL,
                    word_count INTEGER NOT NULL CHECK (word_count >= 0),
                    duration_seconds REAL NOT NULL CHECK (duration_seconds >= 0)
                )
            """)
            connection.execute("PRAGMA user_version = 1")
            word_count = len(re.findall(r"[^\W_]+(?:['’][^\W_]+)*", text))
            connection.execute(
                "INSERT INTO dictations (created_at, text, word_count, duration_seconds) "
                "VALUES (?, ?, ?, ?)",
                (int(time.time()), text, word_count, duration_seconds),
            )
    finally:
        connection.close()
