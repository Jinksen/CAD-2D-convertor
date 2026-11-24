# Settings

CAD2Maxwell has two kinds of settings:

| Kind | Stored in | Travels with the project |
|------|-----------|--------------------------|
| Application preferences | `%APPDATA%\com.cad2maxwell.desktop\prefs.json` | No |
| Project settings | the `.c2mproj` file | Yes |

## Application preferences

Open **Preferences** in the Model Builder. Changes apply immediately and are remembered on this
machine. Geometry and DXF export always stay in millimetres; these settings change only how
numbers are shown and typed.

| Setting | Values | Default |
|---------|--------|---------|
| Theme | System, Light, Dark | System |
| Display unit | mm, cm, m, in | mm |
| Length decimals | 0–9 | 3 |
| Direction decimals (plane normals) | 0–9 | 4 |
| Decimal marker | Point (`12.5`), Comma (`12,5`) | Point |
| Digit grouping | on, off | on |
| Trailing zeros | on, off | on |

Notation rules are in [number-format.md](number-format.md).

If `prefs.json` cannot be read, it is renamed to `prefs.json.bad` and defaults are used.
Deleting the file resets every preference.

## Project settings

Saved with **Save** (Ctrl+S) in the `.c2mproj`: STEP path and SHA-256, section plane and offset,
number format, body visibility, user groups and collapsed folders. Geometry is never stored.
See [projects-and-formats.md](projects-and-formats.md).

## Logs

The geometry service writes `%LOCALAPPDATA%\com.cad2maxwell.desktop\logs\backend.log`.

## Planned

A settings dialog (Ctrl+,) with General, Units and numbers, Viewport, Export and Service
categories is tracked in [#61](https://github.com/Jinksen/CAD-2D-convertor/issues/61).
Per-project validation tolerances are [#62](https://github.com/Jinksen/CAD-2D-convertor/issues/62)
and DXF layer and colour modes are [#63](https://github.com/Jinksen/CAD-2D-convertor/issues/63).
