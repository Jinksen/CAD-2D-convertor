from pathlib import Path

import ezdxf
import pytest
from step_fixtures import named_colored_box

from cad2maxwell_backend.cli import main


def test_cli_writes_draft_dxf_for_absolute_local_step(tmp_path: Path) -> None:
    source = named_colored_box(tmp_path / "coil.step")
    output = tmp_path / "coil-section.dxf"

    status = main(["--input", str(source), "--plane", "XY", "--offset-mm", "15",
                   "--output", str(output)])

    assert status == 0
    assert output.is_file()
    assert output.with_suffix(".json").is_file()
    assert len(ezdxf.readfile(output).modelspace().query("LINE")) == 4


def test_cli_reports_missing_input_without_traceback(
    tmp_path: Path, capsys: pytest.CaptureFixture[str],
) -> None:
    status = main(["--input", str(tmp_path / "missing.step"), "--plane", "XY",
                   "--offset-mm", "0", "--output", str(tmp_path / "out.dxf")])

    assert status == 2
    assert "Traceback" not in capsys.readouterr().err
    assert not (tmp_path / "out.dxf").exists()
