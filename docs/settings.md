# Settings

CAD2Maxwell has two kinds of settings:

| Kind | Stored in | Travels with the project |
|------|-----------|--------------------------|
| Application preferences | `%APPDATA%\com.cad2maxwell.desktop\prefs.json` | No |
| Project settings | the `.c2mproj` file | Yes |

## Application settings

Open **Settings** in the ribbon or press **Ctrl+,**. The dialog is modal: Escape or the close
button dismisses it. Every change applies at once and is remembered on this machine. **Reset**
at the top of a page restores that category only. Geometry and DXF export always stay in
millimetres; the units settings change only how numbers are shown and typed.

### General

| Setting | Values | Default |
|---------|--------|---------|
| Theme | System, Light, Dark | System |
| Recent projects | 5, 10, 15, 20 | 10 |
| Ask before discarding unsaved changes | on, off | on |

### Units and numbers

| Setting | Values | Default |
|---------|--------|---------|
| Display unit | mm, cm, m, in | mm |
| Length decimals | 0-9 | 3 |
| Direction decimals (plane normals) | 0-9 | 4 |
| Decimal marker | Point (`12.5`), Comma (`12,5`) | Point |
| Digit grouping | on, off | on |
| Trailing zeros | on, off | on |

Notation rules are in [number-format.md](number-format.md).

### Viewport

| Setting | Values | Default |
|---------|--------|---------|
| 2D fill opacity | 15 %, 28 %, 50 % | 28 % |
| 2D line width | 0.75, 1, 1.5 px | 1 px |
| Show grid when a section opens | on, off | off |
| 3D background | Theme, Dark, Light | Theme |
| Preview quality | Draft, Normal, Fine | Normal |
| Mouse preset | CAD2Maxwell, Inventor, ANSYS | CAD2Maxwell |

Preview quality scales the resolution of the software renderer (0.5x, 1x, 1.5x); the mesh
itself comes from the service. Mouse presets for the 3D view:

| Preset | Orbit | Pan | Pick |
|--------|-------|-----|------|
| CAD2Maxwell | left drag | middle or right drag | left click |
| Inventor | Shift + middle drag | middle drag | left click |
| ANSYS | middle drag | Shift + middle or right drag | left click |

The wheel zooms in every preset.

### Export

| Setting | Values | Default |
|---------|--------|---------|
| Default folder | an existing folder | system default |
| Layers | Component, Group, Hybrid | Component |
| Colours | Original, Group, Material, Monochrome | Original |
| Formats | DXF (always), CSV, SVG, FEMM, Gmsh, PyAEDT | DXF only |
| Write one ZIP bundle | on, off | on |

See [dxf-export.md](dxf-export.md) for what each layer and colour mode writes.

### Service

Shows the service status and log folder and opens the folder in the file manager. The log
level and **Restart service** are disabled with a reason: the service has no runtime log
setting, and the window starts it once and keeps its session token.

Settings are stored in `%APPDATA%\com.cad2maxwell.desktop\prefs.json` under `theme`,
`number_format` and `settings`. If `prefs.json` cannot be read, it is renamed to
`prefs.json.bad` and defaults are used. Deleting the file resets every setting.

## Project settings

Saved with **Save** (Ctrl+S) in the `.c2mproj`: STEP path and SHA-256, section plane and offset,
number format, body visibility, user groups and collapsed folders. Geometry is never stored.
See [projects-and-formats.md](projects-and-formats.md).

## Logs

The geometry service writes `%LOCALAPPDATA%\com.cad2maxwell.desktop\logs\backend.log`.
