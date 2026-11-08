//! Typed wire contracts of the local geometry service (mirrors the TS `contracts.ts` validators).
//! Unknown fields are ignored, as in the TS validators; everything the app reads is typed.

use base64::Engine;
use serde::{Deserialize, Serialize};

pub type Vec3 = [f64; 3];
pub type Point2 = [f64; 2];

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Health {
    pub status: String,
    pub service: String,
    pub api_version: String,
}

impl Health {
    pub fn validate(&self) -> Result<(), String> {
        (self.status == "ok")
            .then_some(())
            .ok_or_else(|| "health status is not ok".into())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct BoundsMm {
    pub min_xyz: Vec3,
    pub max_xyz: Vec3,
}

impl BoundsMm {
    fn validate(&self) -> bool {
        self.min_xyz
            .iter()
            .zip(&self.max_xyz)
            .all(|(low, high)| low <= high)
    }
    pub fn center(&self) -> Vec3 {
        std::array::from_fn(|i| (self.min_xyz[i] + self.max_xyz[i]) / 2.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NameProvenance {
    Occurrence,
    Product,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorProvenance {
    Instance,
    Shape,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ImportComponent {
    pub id: String,
    pub source_id: Option<String>,
    pub source_name: Option<String>,
    pub display_name: String,
    pub export_name: String,
    pub hierarchy_path: Vec<String>,
    pub source_color: Option<[u8; 3]>,
    pub name_provenance: Option<NameProvenance>,
    pub color_provenance: Option<ColorProvenance>,
    pub body_ordinal: u32,
    pub bounds_mm: BoundsMm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ImportDiagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub component_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ImportSummary {
    pub import_id: String,
    pub path: String,
    pub sha256: String,
    pub source_unit: Option<String>,
    pub to_mm_scale: Option<f64>,
    pub bounds_mm: BoundsMm,
    pub component_count: usize,
    pub components: Vec<ImportComponent>,
    pub diagnostics: Vec<ImportDiagnostic>,
}

impl ImportSummary {
    pub fn validate(&self) -> Result<(), String> {
        if self.component_count != self.components.len() {
            return Err("component_count does not match components".into());
        }
        if !self.bounds_mm.validate() || self.components.iter().any(|c| !c.bounds_mm.validate()) {
            return Err("bounds are inverted".into());
        }
        Ok(())
    }
}

/// Section plane as sent and as echoed back: principal planes carry an offset along their axis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SectionPlane {
    XY {
        offset_mm: f64,
    },
    XZ {
        offset_mm: f64,
    },
    YZ {
        offset_mm: f64,
    },
    #[serde(rename = "custom")]
    Custom {
        origin_xyz: Vec3,
        normal_xyz: Vec3,
        x_dir_xyz: Vec3,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SectionCurve {
    Line {
        start: Point2,
        end: Point2,
    },
    Circle {
        center: Point2,
        radius: f64,
        x_axis: Point2,
        y_axis: Point2,
        start_parameter: f64,
        end_parameter: f64,
    },
    Ellipse {
        center: Point2,
        major_radius: f64,
        minor_radius: f64,
        x_axis: Point2,
        y_axis: Point2,
        start_parameter: f64,
        end_parameter: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WireRole {
    Outer,
    Hole,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SectionWire {
    pub closed: bool,
    pub role: Option<WireRole>,
    pub curves: Vec<SectionCurve>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SectionComponent {
    pub component_id: String,
    pub wires: Vec<SectionWire>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SectionDiagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub component_id: Option<String>,
    #[serde(default)]
    pub related_component_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SectionResponse {
    pub section_id: String,
    pub import_id: String,
    pub plane: SectionPlane,
    pub components: Vec<SectionComponent>,
    pub diagnostics: Vec<SectionDiagnostic>,
}

impl SectionResponse {
    pub fn validate(&self) -> Result<(), String> {
        let curves = self
            .components
            .iter()
            .flat_map(|c| &c.wires)
            .flat_map(|w| &w.curves);
        for curve in curves {
            let ok = match curve {
                SectionCurve::Line { .. } => true,
                SectionCurve::Circle { radius, .. } => *radius > 0.0,
                SectionCurve::Ellipse {
                    major_radius,
                    minor_radius,
                    ..
                } => *major_radius > 0.0 && *minor_radius > 0.0,
            };
            if !ok {
                return Err("curve radius must be positive".into());
            }
        }
        if self
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Info)
        {
            return Err("section diagnostics are warning or error".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[allow(dead_code)] // used by the 3D preview and section features in phase 3
pub struct ComponentPlane {
    pub component_id: String,
    pub normal_xyz: Vec3,
    pub offset_mm: f64,
    pub planar_area_mm2: f64,
}

/// Preview mesh as sent: buffers are base64 little-endian typed arrays.
#[derive(Debug, Clone, Deserialize)]
pub struct PreviewMeshWire {
    pub import_id: String,
    pub components: Vec<PreviewComponentWire>,
    pub deflection_mm: f64,
    pub coarsened: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PreviewComponentWire {
    pub component_id: String,
    pub color: Option<[u8; 3]>,
    pub positions: String,
    pub normals: String,
    pub indices: String,
    pub edge_indices: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreviewComponentMesh {
    pub component_id: String,
    pub color: Option<[u8; 3]>,
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
    pub edge_indices: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreviewMesh {
    pub import_id: String,
    pub components: Vec<PreviewComponentMesh>,
    pub deflection_mm: f64,
    pub coarsened: bool,
}

fn words(base64: &str) -> Result<Vec<[u8; 4]>, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64)
        .map_err(|e| e.to_string())?;
    if bytes.len() % 4 != 0 {
        return Err("Preview buffer is not 32-bit aligned.".into());
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|w| [w[0], w[1], w[2], w[3]])
        .collect())
}

fn floats(base64: &str) -> Result<Vec<f32>, String> {
    Ok(words(base64)?.into_iter().map(f32::from_le_bytes).collect())
}

fn ints(base64: &str) -> Result<Vec<u32>, String> {
    Ok(words(base64)?.into_iter().map(u32::from_le_bytes).collect())
}

impl PreviewMeshWire {
    /// Decodes and validates the buffers like the TS `decodePreviewMesh`.
    pub fn decode(self) -> Result<PreviewMesh, String> {
        let components = self
            .components
            .into_iter()
            .map(|wire| {
                let positions = floats(&wire.positions)?;
                let normals = floats(&wire.normals)?;
                let indices = ints(&wire.indices)?;
                let edge_indices = ints(&wire.edge_indices)?;
                let nodes = positions.len() / 3;
                if positions.len() % 3 != 0
                    || normals.len() != positions.len()
                    || indices.len() % 3 != 0
                    || edge_indices.len() % 2 != 0
                {
                    return Err("Preview buffers have inconsistent lengths.".to_string());
                }
                if !positions.iter().chain(&normals).all(|v| v.is_finite()) {
                    return Err("Preview buffers contain non-finite values.".into());
                }
                if indices
                    .iter()
                    .chain(&edge_indices)
                    .any(|&i| i as usize >= nodes)
                {
                    return Err("Preview indices are out of range.".into());
                }
                Ok(PreviewComponentMesh {
                    component_id: wire.component_id,
                    color: wire.color,
                    positions,
                    normals,
                    indices,
                    edge_indices,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(PreviewMesh {
            import_id: self.import_id,
            components,
            deflection_mm: self.deflection_mm,
            coarsened: self.coarsened,
        })
    }
}
