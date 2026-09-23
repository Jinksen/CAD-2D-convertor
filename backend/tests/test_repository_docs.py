from pathlib import Path


def test_readme_contains_required_developer_sections() -> None:
    repository_root = Path(__file__).resolve().parents[2]
    readme = (repository_root / "README.md").read_text(encoding="utf-8").lower()

    required_headings = (
        "## status",
        "## supported files",
        "## prerequisites",
        "## setup",
        "## development",
        "## test",
        "## package",
        "## known limitations",
        "## roadmap",
    )
    for heading in required_headings:
        assert heading in readme, f"README is missing required heading: {heading}"
