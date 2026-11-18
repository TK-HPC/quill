import configparser
import os
import sys
from pathlib import Path


def load_language():
    configured_home = os.environ.get("XDG_CONFIG_HOME", "")
    config_home = Path(configured_home)
    if config_home.is_absolute() == False:
        config_home = Path.home() / ".config"
    path = config_home / "quill/preferences.ini"
    preferences = configparser.ConfigParser(interpolation=None)
    try:
        with path.open(encoding="utf-8") as source:
            preferences.read_file(source)
        language = preferences.get("Transcription", "language", fallback="en").strip()
        if language not in ("en", "auto"):
            raise ValueError("Transcription language must be en or auto")
        return language
    except FileNotFoundError:
        return "en"
    except (OSError, UnicodeError, configparser.Error, ValueError) as error:
        print(f"Quill: could not read language setting: {error}. Using English.",
              file=sys.stderr, flush=True)
        return "en"
