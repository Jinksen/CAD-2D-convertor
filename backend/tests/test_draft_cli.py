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


def _job(tmp_path: Path, source: Path, **extra: object) -> Path:
    import json

    out = tmp_path / "out"
    out.mkdir(exist_ok=True)
    job = {"input": str(source), "output_dir": str(out), "sections": [
        {"name": "mid", "plane": "XY", "offset_mm": 15},
        {"name": "end", "plane": "XY", "offset_mm": 25}], **extra}
    path = tmp_path / "job.json"
    path.write_text(json.dumps(job))
    return path


def test_job_writes_one_export_per_section_and_refuses_overwrite(
    tmp_path: Path, capsys: pytest.CaptureFixture[str],
) -> None:
    job = _job(tmp_path, named_colored_box(tmp_path / "coil.step"))

    assert main(["--job", str(job)]) == 0
    assert (tmp_path / "out" / "mid.dxf").is_file() and (tmp_path / "out" / "end.dxf").is_file()
    assert main(["--job", str(job)]) == 2
    assert "already exist" in capsys.readouterr().err


def test_job_schema_errors_are_reported_without_writing(
    tmp_path: Path, capsys: pytest.CaptureFixture[str],
) -> None:
    job = _job(tmp_path, named_colored_box(tmp_path / "coil.step"), bogus=1)

    assert main(["--job", str(job)]) == 2
    assert "Batch export failed" in capsys.readouterr().err
    assert not list((tmp_path / "out").iterdir())


def test_job_mixed_with_single_run_args_is_rejected(tmp_path: Path) -> None:
    with pytest.raises(SystemExit):
        main(["--job", "x.json", "--plane", "XY"])
