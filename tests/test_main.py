import socket
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from wisprflow import main


class ShortcutTests(unittest.TestCase):
    def test_second_launch_sends_toggle_to_running_app(self):
        with tempfile.TemporaryDirectory() as directory:
            address = str(Path(directory) / "app.sock")
            with (
                patch.object(main, "SOCKET_PATH", address),
                socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM) as server,
            ):
                server.bind(address)
                server.settimeout(1)
                self.assertTrue(main.toggle_running_app())
                self.assertEqual(server.recv(32), b"toggle")

    def test_missing_or_stale_socket_allows_app_start(self):
        with tempfile.TemporaryDirectory() as directory:
            address = str(Path(directory) / "app.sock")
            with patch.object(main, "SOCKET_PATH", address):
                self.assertFalse(main.toggle_running_app())
                with socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM) as server:
                    server.bind(address)
                self.assertFalse(main.toggle_running_app())

    def test_launcher_works_outside_project_directory(self):
        launcher = Path(sys.executable).parent / "wisprflow"
        with tempfile.TemporaryDirectory() as directory:
            address = str(Path(directory) / "app.sock")
            with socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM) as server:
                server.bind(address)
                server.settimeout(1)
                subprocess.run(
                    [sys.executable, "-c", (
                        "import runpy, sys; from wisprflow import main; "
                        "main.SOCKET_PATH = sys.argv[1]; "
                        "runpy.run_path(sys.argv[2], run_name='__main__')"
                    ), address, str(launcher)],
                    cwd=directory, check=True, timeout=10,
                    capture_output=True, text=True,
                )
                self.assertEqual(server.recv(32), b"toggle")

    @patch.object(main, "Capsule")
    def test_first_launch_removes_stale_socket_and_starts_recording(self, capsule):
        with tempfile.TemporaryDirectory() as directory:
            address = Path(directory) / "app.sock"
            with socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM) as server:
                server.bind(str(address))
            with patch.object(main, "SOCKET_PATH", str(address)):
                main.main()
            self.assertFalse(address.exists())
        capsule.assert_called_once_with()
        capsule.return_value.toggle.assert_called_once_with()
        capsule.return_value.run.assert_called_once_with()


if __name__ == "__main__":
    unittest.main()
