# STEP Import Backend Design

## Goal

Add the first vertical slice of CAD2Maxwell Milestone 1: import a local STEP/STP file in the Python backend, enumerate exact B-Rep bodies, preserve available source metadata, compute body and model bounds, and expose a typed API response. This slice does not generate preview meshes or modify the frontend.

Success means a caller can provide an absolute local STEP path and receive a deterministic, traceable summary whose components retain their exact OpenCascade shapes inside the backend session for later sectioning and mesh generation.

## Scope

Included:

- local `.step` and `.stp` path validation;
- STEP import through OpenCascade XCAF;
- assembly traversal and solid enumeration;
- source names, colors, hierarchy paths, and STEP references when available;
- per-body and aggregate axis-aligned bounding boxes;
- source-unit reporting and conversion metadata;
- SHA-256 source fingerprinting;
- deterministic component identifiers and ASCII-safe export names;
- typed request, response, and diagnostic DTOs;
- backend session ownership of exact shapes;
- focused API, importer, naming, and service tests;
- geometry and architecture documentation updates.

Excluded:

- mesh generation and serialization;
- React model-tree or 3D viewport integration;
- section-plane geometry;
- geometry repair or validation beyond import integrity;
- project-file persistence;
- accepting file contents through multipart upload.

## Architecture

The implementation keeps OpenCascade types behind a geometry boundary:

```text
POST /api/v1/imports/step
        |
        v
ImportService
  - validates the local path
  - fingerprints the source file
  - invokes the importer
  - creates a backend session
  - maps internal records to DTOs
        |
        v
XCAF STEP importer
  - reads document metadata
  - traverses occurrences and references
  - enumerates exact solids
  - computes units and bounds
        |
        +--> in-memory exact-shape session store
        |
        +--> serializable Pydantic response
```

Suggested modules:

- `geometry/step_importer.py`: XCAF/OCP integration and exact-shape extraction;
- `geometry/bounds.py`: focused OpenCascade bounding-box conversion;
- `domain/import_result.py`: internal typed records that may reference backend-only shapes;
- `models/imports.py`: serializable Pydantic API contracts;
- `services/import_service.py`: path validation, hashing, naming, session creation, and DTO mapping;
- `services/import_sessions.py`: process-local ownership of exact imported shapes;
- `naming.py`: deterministic transliteration and duplicate resolution;
- `api.py`: thin route wiring and HTTP error mapping.

The service and DTO layers must not expose or depend on OpenCascade implementation details. React and Rust remain unchanged in this slice.

## Import Strategy

Use `STEPCAFControl_Reader` with an XCAF document. Enable name, color, layer, and material modes before transfer. Traverse free shapes and assembly components through the XCAF shape tool, retaining occurrence paths rather than flattening the document immediately.

For each occurrence, recursively inspect referred shapes and their placements. Enumerate solid subshapes as importable bodies. Apply the occurrence location when computing bounds and when retaining the session-owned shape, so later consumers receive geometry in assembly coordinates.

Metadata precedence is explicit:

1. occurrence metadata, when available;
2. referred/product shape metadata;
3. `null` when the source did not provide a value.

CAD2Maxwell may generate `display_name` and `export_name`, but must not present either as source-provided metadata. Color lookup follows instance color first, then shape color, recording the source used when practical. Hierarchy is represented as an ordered list of source labels/names. Missing hierarchy entries remain unnamed instead of being invented.

No significant geometry repair is performed during import. Reader warnings and failures become diagnostics.

## Units and Coordinates

The response reports the source unit when OpenCascade can determine it and records the scale applied to the CAD2Maxwell millimetre convention. Exact session shapes and all reported bounds use millimetres at the application boundary. Unknown or contradictory source units produce an explicit diagnostic; they must not be silently assumed to be millimetres.

Bounding boxes are axis-aligned and represented as minimum and maximum XYZ coordinates in millimetres. Void or invalid boxes are rejected for the affected body with an import diagnostic rather than serialized as non-finite values.

## Identity and Naming

The service computes the file SHA-256 before import. A component ID is derived deterministically from:

- the source SHA-256;
- the normalized occurrence/hierarchy path;
- the solid ordinal within the terminal source shape.

This makes repeated imports of unchanged content stable without relying on absolute file location or random UUIDs. Duplicate hierarchy paths are disambiguated by deterministic source-label and solid ordinals.

Each component exposes distinct fields:

- `source_name`: nullable and only populated from STEP/XCAF metadata;
- `display_name`: a readable generated fallback when the source name is absent;
- `export_name`: deterministic ASCII-safe transliteration using `[A-Za-z0-9_-]`, with `_02`, `_03`, and later suffixes for duplicates.

## API Contract

`POST /api/v1/imports/step` accepts JSON:

```json
{
  "path": "C:/Projects/Motor/assembly.step"
}
```

The path must be absolute, exist, resolve to a regular readable file, and have a case-insensitive `.step` or `.stp` suffix. The service does not execute or interpret embedded scripts. Path validation happens before invoking OpenCascade.

The successful response contains:

- a process-local `import_id` used to address exact shapes later;
- source path, SHA-256, detected units, and millimetre scale;
- aggregate bounding box;
- component count and ordered component list;
- import diagnostics.

Each component contains its stable ID, nullable source metadata, display/export names, hierarchy path, nullable RGB color, body ordinal, bounding box, and optional source entity/label reference. No OpenCascade object, pointer, or opaque serialized payload crosses the API boundary.

The initial session store is process-local and intentionally non-persistent. A backend restart invalidates `import_id`; project persistence will rebuild geometry from the source file and verify its hash in a later milestone.

## Errors and Diagnostics

HTTP status mapping:

- `422`: invalid or non-absolute path, unsupported suffix, or non-file path;
- `404`: source file does not exist;
- `403`: source file cannot be read;
- `400`: OpenCascade rejects the STEP content or no importable solid exists;
- `500`: unexpected internal import failure, with no sensitive traceback in the response.

Expected import warnings are returned as typed diagnostics with a stable code, severity, message, and optional component ID. Examples include missing source units, missing metadata, skipped non-solid shapes, and invalid body bounds. Missing names or colors alone do not fail an otherwise valid import.

The service must avoid leaking arbitrary local file contents or stack traces through errors. Logs may include the resolved path for local engineering diagnosis but must not include CAD data.

## Dependency and Packaging

Add the maintained OCP Python binding compatible with Python 3.12. Pin it to a bounded compatible version range and update `uv.lock`. OpenCascade remains a backend-only dependency. Packaging compatibility with the final bundled runtime is tracked as a later Milestone 1 concern; this slice verifies normal uv-managed Windows development and test execution.

## Testing

Follow test-driven development.

Unit tests cover:

- absolute-path and suffix validation;
- deterministic IDs across repeated imports;
- transliteration and duplicate export-name resolution;
- DTO validation and finite bounds;
- HTTP error mapping;
- session lookup and invalidation behavior.

Integration tests generate small STEP fixtures during the test run using OCP rather than committing opaque binary fixtures. Fixtures include:

- one named colored box;
- two independently placed solids;
- duplicate and non-ASCII names;
- a STEP file without optional metadata;
- malformed STEP input.

Assertions cover body count, transformed bounding boxes, available metadata, millimetre conversion, stable ordering, and stable identity. The locally supplied engineering STEP file may be used for a manual regression check only when readable and appropriate; production behavior is never hard-coded to its expected body count.

Focused backend tests run first, followed by Ruff, mypy, the complete backend suite, and the repository-wide quality gate. Documentation is updated to distinguish implemented import behavior from still-planned mesh and UI features.

## Limitations of This Slice

- Exact shapes exist only for the lifetime of the backend process.
- There is no eviction policy beyond explicit replacement/removal and process shutdown; bounded session lifecycle work accompanies later UI integration if concurrent imports are introduced.
- Preview triangulation is deliberately deferred.
- Complex STEP presentation styles and uncommon metadata encodings may produce diagnostics and require future regression fixtures.
- The final packaged-runtime compatibility of OCP is not established by this backend-only slice.
