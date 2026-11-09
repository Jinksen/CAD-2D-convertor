from pathlib import Path

import pytest

REPOSITORY = Path(__file__).resolve().parents[2]


@pytest.hookimpl(tryfirst=True)
def pytest_configure(config: pytest.Config) -> None:
    # Keep test files in the repo's ignored .test-tmp/ from any working directory.
    # An explicit --basetemp (scripts/test.ps1) wins.
    if config.option.basetemp is None:
        basetemp = REPOSITORY / ".test-tmp" / "pytest"
        basetemp.parent.mkdir(exist_ok=True)
        config.option.basetemp = str(basetemp)
