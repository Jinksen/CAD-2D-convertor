# CAD2Maxwell documentation

Start with the [project README](https://github.com/Jinksen/CAD-2D-convertor/blob/main/README.md) for download, quick start and shortcuts.

## User guide

| Page | Covers |
|------|--------|
| [First trial](first-trial.md) | Walkthrough from STEP to DXF, troubleshooting |
| [Settings](settings.md) | Every setting, its default and where it is stored |
| [Maxwell import checklist](maxwell-import-checklist.md) | Verifying the DXF in Maxwell 2D |
| [Demo model](demo/demo-motor.step) | Synthetic motor used in screenshots and tests |

## Reference

| Page | Covers |
|------|--------|
| [Number format](number-format.md) | ISO 80000-1 display notation |
| [DXF export](dxf-export.md) | DXF entities, names, manifest |
| [Projects and formats](projects-and-formats.md) | `.c2mproj` file, supported and planned formats |
| [UX](ux.md) | Workspace behaviour |

## Developer guide

| Page | Covers |
|------|--------|
| [Architecture](architecture.md) | Rune UI, Rust adapter, Python service boundaries |
| [Geometry](geometry.md) | Import, sectioning and validation, and why |
| [Rune migration](rune-migration.md) | Pinned Rune revision, gates, known gaps |
| [Release](release.md) | Release checklist |
| [Contributing](https://github.com/Jinksen/CAD-2D-convertor/blob/main/CONTRIBUTING.md) | Workflow, one spec = one branch = one PR |
| [Research](research/format-spikes.md) | Background notes (FreeCAD 3D/2D workflow) |

## Plans and specs

| Spec | Status |
|------|--------|
| [UI clarity, settings, formats and documentation](superpowers/specs/2026-11-24-ui-settings-formats.md) | Proposal, issues #48–#78 |
| [v0.1 completion](superpowers/specs/2026-11-01-v0.1-completion.md) | Done |
| [UX performance](superpowers/specs/2026-11-01-ux-performance.md) | Done |

Release notes are in [release notes](release-notes/v0.3.0.md) and the [changelog](https://github.com/Jinksen/CAD-2D-convertor/blob/main/CHANGELOG.md).
Documentation site: https://jinksen.github.io/CAD-2D-convertor/ (built from this folder with MkDocs Material, see [#78](https://github.com/Jinksen/CAD-2D-convertor/issues/78)).
Preview locally with `pip install -r docs/requirements.txt && mkdocs serve`.

## Writing docs

- English, short sentences, tables for reference data.
- Explain why for geometry decisions, not only what (spec §54).
- A user-visible change updates its page in the same PR. A UI change also refreshes the
  screenshots in `images/`.
