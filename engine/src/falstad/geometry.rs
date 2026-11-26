//! Geometry → connectivity by exact endpoint-coordinate matching.

use crate::circuit_doc::CircuitElement;
use std::collections::HashMap;

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
pub(crate) fn assign_nodes(elements: Vec<CircuitElement>) -> Vec<CircuitElement> {
    let mut node_ids: HashMap<(i64, i64), String> = HashMap::new();
    let mut next_index = 1usize;
    let mut result = Vec::new();
    for (index, mut element) in elements.into_iter().enumerate() {
        let geometry = element.geometry.as_ref().unwrap().clone();
        let nodes = match element.kind.as_str() {
            "ground" => vec!["gnd".to_string()],
            _ => vec![
                node_for((geometry.x1, geometry.y1), &mut node_ids, &mut next_index),
                node_for((geometry.x2, geometry.y2), &mut node_ids, &mut next_index),
            ],
        };
        element.nodes = nodes;
        element.id = format!("e{}", index + 1);
        result.push(element);
    }
    result
}
