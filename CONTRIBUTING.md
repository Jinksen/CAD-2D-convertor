# Contributing to CAD2Maxwell

Thanks for helping improve CAD2Maxwell. The project is in an early, draft DXF stage; check the README's status and limitations before proposing changes.

## Before changing code

- Read [CAD2Maxwell_PROJECT_SPEC.md](CAD2Maxwell_PROJECT_SPEC.md) and [AGENTS.md](AGENTS.md) for the architecture and working conventions.
- Open an issue for substantial changes so the intended behavior can be discussed first.
- Keep geometry decisions in the Python/OpenCascade backend. Keep the Rust bridge thin and use typed DTOs between processes.

## Development

Follow the [README setup instructions](README.md#setup) on Windows. Run `./scripts/test.ps1` before opening a pull request. Include focused regression tests for geometry defects and update the relevant docs when behavior changes.

Use small pull requests with a clear description of the problem, the change, and the checks run. State any CAD model or Maxwell versions used for manual verification.

## CAD data and security

Do not commit proprietary CAD files, user models, exports, credentials, or local environment settings. Synthetic test fixtures may be committed intentionally after reviewing their content and provenance; the Git ignore rules require explicitly adding CAD files. Follow [SECURITY.md](SECURITY.md) for vulnerabilities rather than posting exploit details in an issue.
