import os
import sqlite3
import tempfile
import unittest
from contextlib import closing
from pathlib import Path
from unittest.mock import patch

from wisprflow.history import record_dictation


class HistoryTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        environment_patch = patch.dict(os.environ, {"XDG_DATA_HOME": self.directory.name})
        environment_patch.start()
        self.addCleanup(environment_patch.stop)
        self.database_path = Path(self.directory.name) / "quill/history.sqlite3"

    def test_persists_text_counts_duration_and_timestamp_across_connections(self):
        with patch("wisprflow.history.time.time", return_value=1234567890):
            record_dictation("I can't — don’t ... stop!", 2.5)
            record_dictation("'); DROP TABLE dictations; --", 0)
        with closing(sqlite3.connect(self.database_path)) as connection:
            self.assertEqual(connection.execute("PRAGMA user_version").fetchone(), (1,))
            rows = connection.execute(
                "SELECT created_at, text, word_count, duration_seconds FROM dictations ORDER BY id"
            ).fetchall()
        self.assertEqual(rows[0], (1234567890, "I can't — don’t ... stop!", 4, 2.5))
        self.assertEqual(rows[1][1], "'); DROP TABLE dictations; --")
        self.assertEqual(rows[1][2:], (3, 0.0))
        self.assertEqual(self.database_path.stat().st_mode & 0o777, 0o600)
        self.assertEqual(self.database_path.parent.stat().st_mode & 0o777, 0o700)

    def test_blank_text_does_not_create_database(self):
        record_dictation(" \n\t", 1)
        self.assertFalse(self.database_path.exists())

    def test_punctuation_only_counts_zero_words(self):
        record_dictation("... — !!! ___", 1)
        with closing(sqlite3.connect(self.database_path)) as connection:
            self.assertEqual(connection.execute("SELECT word_count FROM dictations").fetchone(), (0,))

    def test_rejects_invalid_duration_before_writing(self):
        for duration in (-1, float("nan"), float("inf")):
            with self.subTest(duration=duration), self.assertRaises(ValueError):
                record_dictation("hello", duration)
        self.assertFalse(self.database_path.exists())

    def test_newer_schema_is_left_unchanged(self):
        self.database_path.parent.mkdir()
        with closing(sqlite3.connect(self.database_path)) as connection:
            connection.execute("PRAGMA user_version = 2")
            connection.execute("CREATE TABLE future (value TEXT)")
            connection.commit()
        with self.assertRaisesRegex(RuntimeError, "version: 2"):
            record_dictation("hello", 1)
        with closing(sqlite3.connect(self.database_path)) as connection:
            self.assertEqual(connection.execute("PRAGMA user_version").fetchone(), (2,))
            self.assertEqual(connection.execute("SELECT name FROM sqlite_master").fetchall(), [("future",)])

    def test_busy_database_raises_without_partial_record(self):
        record_dictation("first", 1)
        with closing(sqlite3.connect(self.database_path)) as connection:
            connection.execute("BEGIN IMMEDIATE")
            with self.assertRaisesRegex(sqlite3.OperationalError, "locked"):
                record_dictation("second", 1)
        with closing(sqlite3.connect(self.database_path)) as connection:
            self.assertEqual(connection.execute("SELECT text FROM dictations").fetchall(), [("first",)])

    def test_unset_or_relative_xdg_uses_home(self):
        for data_home in ("", "relative"):
            with self.subTest(data_home=data_home):
                with (
                    patch.dict(os.environ, {"XDG_DATA_HOME": data_home}),
                    patch("wisprflow.history.Path.home", return_value=Path(self.directory.name)),
                ):
                    record_dictation("home", 1)
                self.assertTrue((Path(self.directory.name) / ".local/share/quill/history.sqlite3").exists())


if __name__ == "__main__":
    unittest.main()
