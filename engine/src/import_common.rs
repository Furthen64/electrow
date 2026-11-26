//! Shared import pipeline: post-geometry computation, coordinate merging
//! and node-id assignment. Used by the Falstad text and reference-XML
//! importers so both produce identical documents for equivalent files.

use crate::circuit_doc::{CircuitElement, Geometry};
use std::collections::HashMap;

const MAX_COORD: i64 = 1_000_000;

#[derive(Clone, Debug)]
pub(crate) struct PendingRecord {
    pub line: usize,
    pub type_token: String,
    pub element: CircuitElement,
}

#[derive(Clone, Debug)]
pub(crate) struct GeometryFailure {
    pub line: usize,
    pub type_token: String,
    pub reason: String,
}

fn coords_in_range(geometry: &Geometry) -> bool {
    geometry.x1.saturating_abs() <= MAX_COORD
        && geometry.y1.saturating_abs() <= MAX_COORD
        && geometry.x2.saturating_abs() <= MAX_COORD
        && geometry.y2.saturating_abs() <= MAX_COORD
}

fn sign(x: i64) -> i64 {
    if x < 0 {
        -1
    } else if x == 0 {
        0
    } else {
        1
    }
}

// interpPoint2: two points at fraction `f` along a→b, offset perpendicular by ±g.
// Java: gx = b.y-a.y; gy = a.x-b.x; g /= hypot(gx, gy);
//       c.x = floor(a.x*(1-f)+b.x*f + g*gx + .48); c.y = floor(...+g*gy+.48)
fn interp_point2(a: (i64, i64), b: (i64, i64), f: f64, g: f64) -> [(i64, i64); 2] {
    let (gx, gy) = (b.1 - a.1, a.0 - b.0);
    let g_mag = ((gx as f64) * (gx as f64) + (gy as f64) * (gy as f64)).sqrt();
    let g = g / g_mag;
    let ax = a.0 as f64 * (1.0 - f) + b.0 as f64 * f;
    let ay = a.1 as f64 * (1.0 - f) + b.1 as f64 * f;
    let c = (
        (ax + g * gx as f64 + 0.48).floor() as i64,
        (ay + g * gy as f64 + 0.48).floor() as i64,
    );
    let d = (
        (ax - g * gx as f64 + 0.48).floor() as i64,
        (ay - g * gy as f64 + 0.48).floor() as i64,
    );
    [c, d]
}

/// BJT posts: base, collector, emitter. Ports TransistorElm::setPoints (FLAG_FLIP = 1).
fn bjt_posts(x1: i64, y1: i64, x2: i64, y2: i64, flags: u32, pnp: i64) -> Vec<(i64, i64)> {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let dn = ((dx as f64) * (dx as f64) + (dy as f64) * (dy as f64)).sqrt();
    if dn < 1e-6 {
        return Vec::new();
    }
    let mut dsign = if dy == 0 { sign(dx) } else { sign(dy) };
    if flags & 1 != 0 {
        dsign = -dsign;
    }
    let hs2 = 16 * dsign * pnp;
    let [collector, emitter] = interp_point2((x1, y1), (x2, y2), 1.0, hs2 as f64);
    vec![(x1, y1), collector, emitter]
}

/// MOSFET posts: gate, source, drain. Ports MosfetElm::setPoints (FLAG_FLIP = 8).
fn mosfet_posts(x1: i64, y1: i64, x2: i64, y2: i64, flags: u32) -> Vec<(i64, i64)> {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let dn = ((dx as f64) * (dx as f64) + (dy as f64) * (dy as f64)).sqrt();
    if dn < 1e-6 {
        return Vec::new();
    }
    let dsign = if dy == 0 { sign(dx) } else { sign(dy) };
    let mut hs2 = 16 * dsign;
    if flags & 8 != 0 {
        hs2 = -hs2;
    }
    // interpPoint2(point1, point2, src[0], drn[0], 1, -hs2)
    let [source, drain] = interp_point2((x1, y1), (x2, y2), 1.0, -(hs2) as f64);
    vec![(x1, y1), source, drain]
}

fn post_points(element: &CircuitElement) -> Result<Vec<(i64, i64)>, String> {
    let geometry = element
        .geometry
        .as_ref()
        .ok_or_else(|| "missing geometry".to_string())?;
    if !coords_in_range(geometry) {
        return Err(format!("coordinate outside ±{MAX_COORD}"));
    }
    match element.kind.as_str() {
        "bjt" => {
            let pnp = if element.params.get("pnp").copied().unwrap_or(0.0) == 1.0 {
                -1
            } else {
                1
            };
            let posts =
                bjt_posts(geometry.x1, geometry.y1, geometry.x2, geometry.y2, geometry.flags, pnp);
            if posts.is_empty() {
                return Err("zero-length BJT".to_string());
            }
            Ok(posts)
        }
        "mosfet" => {
            let posts = mosfet_posts(
                geometry.x1,
                geometry.y1,
                geometry.x2,
                geometry.y2,
                geometry.flags,
            );
            if posts.is_empty() {
                return Err("zero-length MOSFET".to_string());
            }
            Ok(posts)
        }
        "ground" => Ok(vec![(geometry.x1, geometry.y1)]),
        _ => {
            if geometry.x1 == geometry.x2 && geometry.y1 == geometry.y2 {
                return Err(format!("zero-length {}", element.kind));
            }
            Ok(vec![(geometry.x1, geometry.y1), (geometry.x2, geometry.y2)])
        }
    }
}

fn node_for(
    key: (i64, i64),
    node_ids: &mut HashMap<(i64, i64), String>,
    next_index: &mut usize,
) -> String {
    if let Some(id) = node_ids.get(&key) {
        return id.clone();
    }
    let id = format!("n{}", *next_index);
    *next_index += 1;
    node_ids.insert(key, id.clone());
    id
}

/// Convert element geometry into explicit terminal connectivity by exact
/// endpoint-coordinate matching (ground elements share one reference node).
/// Elements whose geometry is degenerate fail as `GeometryFailure` and must
/// be reported as unsupported records, never silently merged.
pub(crate) fn assign_nodes(pending: Vec<PendingRecord>) -> (Vec<CircuitElement>, Vec<GeometryFailure>) {
    let mut node_ids: HashMap<(i64, i64), String> = HashMap::new();
    let mut next_index = 1usize;
    let mut elements = Vec::new();
    let mut failures = Vec::new();

    // Register ground posts before any other element so endpoints drawn on a
    // ground symbol resolve to the shared reference node regardless of line
    // order. Elements with missing geometry are reported in the main loop.
    for record in &pending {
        if record.element.kind == "ground" {
            if let Ok(posts) = post_points(&record.element) {
                for post in posts {
                    node_ids.insert(post, "gnd".to_string());
                }
            }
        }
    }
    for (index, record) in pending.into_iter().enumerate() {
        let mut element = record.element;
        let posts = match post_points(&element) {
            Ok(posts) => posts,
            Err(reason) => {
                failures.push(GeometryFailure {
                    line: record.line,
                    type_token: record.type_token,
                    reason,
                });
                continue;
            }
        };
        let nodes = match element.kind.as_str() {
            "ground" => vec!["gnd".to_string()],
            _ => posts
                .iter()
                .map(|p| node_for(*p, &mut node_ids, &mut next_index))
                .collect(),
        };
        element.nodes = nodes;
        element.id = format!("e{}", index + 1);
        elements.push(element);
    }
    (elements, failures)
}
