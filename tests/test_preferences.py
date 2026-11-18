import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from wisprflow.preferences import load_language


class PreferencesTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        environment = patch.dict(os.environ, {"XDG_CONFIG_HOME": self.directory.name})
        environment.start()
        self.addCleanup(environment.stop)
        self.path = Path(self.directory.name) / "quill/preferences.ini"
        self.path.parent.mkdir()

    def test_missing_or_legacy_preferences_default_to_english(self):
        self.assertEqual(load_language(), "en")
        self.assertFalse(self.path.exists())
        self.path.write_text("[Statistics]\ntyping_words_per_minute=100\n")
        self.assertEqual(load_language(), "en")

    def test_reads_language_from_shared_ini(self):
        for language in ("en", "auto"):
            with self.subTest(language=language):
                self.path.write_text(
                    f"[Statistics]\ntyping_words_per_minute=100\n\n"
                    f"[Transcription]\nlanguage={language}\n"
                )
                self.assertEqual(load_language(), language)

    def test_invalid_or_unreadable_preferences_use_english_and_log(self):
        for contents in (b"not ini", b"\xff", b"[Transcription]\nlanguage=invalid",
                         b"[Transcription]\nlanguage=", b"[Transcription]\nlanguage=%(missing)s"):
            with self.subTest(contents=contents):
                self.path.write_bytes(contents)
                with patch("sys.stderr") as stderr:
                    self.assertEqual(load_language(), "en")
                    self.assertIn("Using English", "".join(
                        call.args[0] for call in stderr.write.call_args_list
                    ))
                self.assertEqual(self.path.read_bytes(), contents)

    def test_unset_or_relative_xdg_uses_home(self):
        home = Path(self.directory.name)
        path = home / ".config/quill/preferences.ini"
        path.parent.mkdir(parents=True)
        path.write_text("[Transcription]\nlanguage=auto\n")
        for configured in ("", "relative"):
            with (
                self.subTest(configured=configured),
                patch.dict(os.environ, {"XDG_CONFIG_HOME": configured}),
                patch("wisprflow.preferences.Path.home", return_value=home),
            ):
                self.assertEqual(load_language(), "auto")


if __name__ == "__main__":
    unittest.main()
