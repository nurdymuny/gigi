"""Command-construction test; mocked compiler output is never release evidence."""
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
import monodromy_mutations


class BuildIsolationGate(unittest.TestCase):
    def test_every_mutation_command_uses_a_separate_target(self):
        commands = []

        def fake_run(cmd, *, stdout, **kwargs):
            commands.append(cmd)
            failing = len(commands) > 2
            stdout.write("test planted ... FAILED\ntest result: FAILED\n" if failing
                         else "test result: ok. 1 passed; 0 failed\n")
            return SimpleNamespace(returncode=101 if failing else 0)

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            engine_target = root / "engine-target"
            with patch.object(monodromy_mutations.subprocess, "run", side_effect=fake_run):
                monodromy_mutations.run(root / "receipts", engine_target)
            self.assertEqual(len(commands), 8)
            for cmd in commands:
                actual = Path(cmd[cmd.index("--target-dir") + 1])
                self.assertEqual(actual, engine_target / "monodromy-mutants")
                self.assertNotEqual(actual, engine_target)


if __name__ == "__main__":
    unittest.main()
