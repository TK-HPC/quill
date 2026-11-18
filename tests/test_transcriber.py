import os
import sqlite3
import subprocess
import tempfile
import unittest
import wave
from contextlib import closing
from pathlib import Path
from unittest.mock import patch

from wisprflow import transcriber


class TranscriberTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.output = str(Path(self.directory.name) / "recording.wav")
        self.original_output = transcriber.OUTPUT_PATH
        output_patch = patch.object(transcriber, "OUTPUT_PATH", self.output)
        output_patch.start()
        self.addCleanup(output_patch.stop)
        environment_patch = patch.dict(os.environ, {
            "XDG_DATA_HOME": self.directory.name,
            "XDG_CONFIG_HOME": self.directory.name,
        })
        environment_patch.start()
        self.addCleanup(environment_patch.stop)

    @patch.object(transcriber.subprocess, "run")
    def test_writes_pcm_and_returns_transcript(self, run):
        run.return_value = subprocess.CompletedProcess([], 0, "timing: 1s\ntext: hello world  \n")
        frames = [b"\x00\x00", b"\x01\x00"]

        self.assertEqual(transcriber.Transcriber._transcribe(frames), "hello world")

        with wave.open(self.output) as recording:
            self.assertEqual(recording.getnchannels(), 1)
            self.assertEqual(recording.getsampwidth(), 2)
            self.assertEqual(recording.getframerate(), 16000)
            self.assertEqual(recording.readframes(2), b"".join(frames))
        run.assert_called_once()
        command = run.call_args.args[0]
        self.assertEqual(command[0], transcriber.CLI_PATH)
        self.assertEqual(command[command.index("-m") + 1], transcriber.MODEL_PATH)
        self.assertEqual(command[command.index("--vocabulary-file") + 1], transcriber.KEYWORDS_PATH)
        self.assertIn(self.output, command)
        self.assertEqual(command[command.index("--language") + 1], "en")
        self.assertEqual(command[-2:], ["--backend", "auto"])
        self.assertEqual(run.call_args.kwargs, {"capture_output": True, "text": True, "check": True})

    @patch.object(transcriber.subprocess, "run")
    def test_language_changes_are_read_for_each_transcription(self, run):
        run.return_value = subprocess.CompletedProcess([], 0, "text: hello\n")
        transcriber.Transcriber._transcribe([])
        self.assertIn("--language", run.call_args.args[0])
        preferences = Path(self.directory.name) / "quill/preferences.ini"
        preferences.parent.mkdir()
        preferences.write_text("[Transcription]\nlanguage=auto\n")
        transcriber.Transcriber._transcribe([])
        self.assertNotIn("--language", run.call_args.args[0])
        preferences.write_text("[Transcription]\nlanguage=en\n")
        transcriber.Transcriber._transcribe([])
        command = run.call_args.args[0]
        self.assertEqual(command[command.index("--language") + 1], "en")

    @patch.object(transcriber.subprocess, "run")
    def test_missing_transcript_is_an_error(self, run):
        run.return_value = subprocess.CompletedProcess([], 0, "timing: 1s\n")
        with self.assertRaisesRegex(RuntimeError, "no transcript"):
            transcriber.Transcriber._transcribe([])
        run.assert_called_once()

    def test_worker_reports_errors_to_capsule(self):
        recorder = transcriber.Transcriber()
        error = RuntimeError("transcription failed")
        with (
            patch.object(recorder, "_record", return_value=[]),
            patch.object(recorder, "_transcribe", side_effect=error),
            patch.object(transcriber, "record_dictation") as save_history,
        ):
            recorder._run()
        self.assertIs(recorder.error, error)
        self.assertIsNone(recorder.transcript)
        save_history.assert_not_called()

    def test_history_failure_still_publishes_transcript(self):
        recorder = transcriber.Transcriber()
        with (
            patch.object(recorder, "_record", return_value=[bytes(32000)]),
            patch.object(recorder, "_transcribe", return_value="hello world"),
            patch.object(transcriber, "record_dictation", side_effect=OSError("disk full")),
            patch("sys.stderr") as stderr,
        ):
            recorder._run()
        self.assertEqual(recorder.transcript, "hello world")
        self.assertIsNone(recorder.error)
        self.assertIn("disk full", "".join(call.args[0] for call in stderr.write.call_args_list))

    def test_worker_saves_actual_pcm_duration_before_publishing(self):
        recorder = transcriber.Transcriber()

        def check_history(text, duration_seconds):
            self.assertIsNone(recorder.transcript)
            self.assertEqual(text, "hello world")
            self.assertEqual(duration_seconds, 1.5)

        with (
            patch.object(recorder, "_record", return_value=[bytes(32000), bytes(16000)]),
            patch.object(recorder, "_transcribe", return_value="hello world"),
            patch.object(transcriber, "record_dictation", side_effect=check_history) as save_history,
        ):
            recorder._run()
        save_history.assert_called_once()
        self.assertEqual(recorder.transcript, "hello world")
        self.assertIsNone(recorder.error)

    @patch.object(transcriber.subprocess, "run")
    def test_cpu_fallback_records_only_one_history_row(self, run):
        recorder = transcriber.Transcriber()
        run.side_effect = [
            subprocess.CalledProcessError(1, [], stderr="CUDA out of memory"),
            subprocess.CompletedProcess([], 0, "text: CPU transcript\n"),
        ]
        with (
            patch.object(recorder, "_record", return_value=[bytes(32000)]),
            patch("sys.stderr"),
        ):
            recorder._run()
        self.assertEqual(recorder.transcript, "CPU transcript")
        self.assertIsNone(recorder.error)
        database_path = Path(self.directory.name) / "quill/history.sqlite3"
        with closing(sqlite3.connect(database_path)) as connection:
            self.assertEqual(
                connection.execute("SELECT text, word_count, duration_seconds FROM dictations").fetchall(),
                [("CPU transcript", 2, 1.0)],
            )

    @patch.object(transcriber.subprocess, "run")
    def test_gpu_failure_retries_same_recording_on_cpu(self, run):
        for code, message in ((1, "CUDA out of memory"), (-6, "CUDA error"), (1, "No CUDA devices")):
            with self.subTest(message=message):
                run.reset_mock()
                run.side_effect = [
                    subprocess.CalledProcessError(code, [], stderr=message),
                    subprocess.CompletedProcess([], 0, "text: CPU transcript\n"),
                ]
                with patch("sys.stderr") as stderr:
                    self.assertEqual(transcriber.Transcriber._transcribe([]), "CPU transcript")
                self.assertEqual(run.call_count, 2)
                automatic, cpu = [call.args[0] for call in run.call_args_list]
                self.assertEqual(cpu, [*automatic[:-1], "cpu"])
                self.assertEqual(cpu[cpu.index("--language") + 1], "en")
                self.assertIn(message, "".join(call.args[0] for call in stderr.write.call_args_list))

    @patch.object(transcriber.subprocess, "run")
    def test_cpu_failure_is_reported_without_further_retries(self, run):
        gpu_error = subprocess.CalledProcessError(1, [], stderr="CUDA out of memory")
        cpu_error = subprocess.CalledProcessError(1, [], stderr="CPU allocation failed")
        run.side_effect = [gpu_error, cpu_error]
        with (
            patch("sys.stderr") as stderr,
            self.assertRaises(subprocess.CalledProcessError) as raised,
        ):
            transcriber.Transcriber._transcribe([])
        self.assertIs(raised.exception, cpu_error)
        self.assertEqual(run.call_count, 2)
        messages = "".join(call.args[0] for call in stderr.write.call_args_list)
        self.assertIn(gpu_error.stderr, messages)
        self.assertIn(cpu_error.stderr, messages)

    @patch.object(transcriber.subprocess, "run", side_effect=FileNotFoundError("missing CLI"))
    def test_missing_cli_is_not_retried(self, run):
        with self.assertRaises(FileNotFoundError):
            transcriber.Transcriber._transcribe([])
        run.assert_called_once()

    def test_assets_stay_in_project_root(self):
        root = Path(__file__).resolve().parents[1]
        self.assertEqual(Path(transcriber.ROOT_DIR), root)
        for path in (self.original_output, transcriber.MODEL_PATH,
                     transcriber.KEYWORDS_PATH, transcriber.CLI_PATH):
            self.assertTrue(Path(path).is_absolute())
            self.assertTrue(Path(path).is_relative_to(root))


if __name__ == "__main__":
    unittest.main()
