//! Collision detection for dynaturtles.
//!
//! This starts v0.2's collision foundation with a spatial-hash broad phase and
//! circle-vs-circle narrow phase. Edge contacts are included because Atari LOGO
//! style demons often trigger on screen boundaries.

use std::collections::{HashMap, HashSet};

use crate::dynaturtle::{TurtleId, TurtleStore};
use crate::turtle::Point;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionConfig {
    pub cell_size: f64,
    pub turtle_radius: f64,
    pub bounds: Option<Bounds>,
}

impl Default for CollisionConfig {
    fn default() -> Self {
        Self {
            cell_size: 32.0,
            turtle_radius: 8.0,
            bounds: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Bounds {
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CollisionPair {
    pub a: TurtleId,
    pub b: TurtleId,
}

impl CollisionPair {
    pub fn new(a: TurtleId, b: TurtleId) -> Self {
        if a.index() <= b.index() {
            Self { a, b }
        } else {
            Self { a: b, b: a }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Bottom,
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeContact {
    pub turtle: TurtleId,
    pub edge: Edge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollisionReport {
    pub turtle_pairs: Vec<CollisionPair>,
    pub edge_contacts: Vec<EdgeContact>,
}

#[derive(Debug, Default, Clone)]
pub struct SpatialHash {
    cells: HashMap<(i64, i64), Vec<TurtleId>>,
}

impl SpatialHash {
    pub fn rebuild(store: &TurtleStore, cell_size: f64) -> Self {
        assert!(cell_size > 0.0, "cell_size must be positive");
        let mut cells: HashMap<(i64, i64), Vec<TurtleId>> = HashMap::new();
        for (index, position) in store.positions().iter().enumerate() {
            cells
                .entry(cell_for(*position, cell_size))
                .or_default()
                .push(TurtleId::new(index));
        }
        Self { cells }
    }

    pub fn candidate_pairs(&self) -> Vec<CollisionPair> {
        let mut pairs = HashSet::new();
        for (&cell, turtles) in &self.cells {
            for neighbor in neighboring_cells(cell) {
                let Some(other_turtles) = self.cells.get(&neighbor) else {
                    continue;
                };
                for &a in turtles {
                    for &b in other_turtles {
                        if a != b {
                            pairs.insert(CollisionPair::new(a, b));
                        }
                    }
                }
            }
        }
        let mut pairs: Vec<CollisionPair> = pairs.into_iter().collect();
        pairs.sort_by_key(|pair| (pair.a.index(), pair.b.index()));
        pairs
    }
}

pub fn detect_collisions(store: &TurtleStore, config: CollisionConfig) -> CollisionReport {
    let spatial_hash = SpatialHash::rebuild(store, config.cell_size);
    let max_distance = config.turtle_radius * 2.0;
    let max_distance_squared = max_distance * max_distance;
    let turtle_pairs = spatial_hash
        .candidate_pairs()
        .into_iter()
        .filter(|pair| {
            let a = store.positions()[pair.a.index()];
            let b = store.positions()[pair.b.index()];
            distance_squared(a, b) <= max_distance_squared
        })
        .collect();

    let edge_contacts = config
        .bounds
        .map(|bounds| detect_edge_contacts(store, bounds, config.turtle_radius))
        .unwrap_or_default();

    CollisionReport {
        turtle_pairs,
        edge_contacts,
    }
}

pub fn touching(store: &TurtleStore, a: TurtleId, b: TurtleId, radius: f64) -> bool {
    let Some(a_pos) = store.positions().get(a.index()).copied() else {
        return false;
    };
    let Some(b_pos) = store.positions().get(b.index()).copied() else {
        return false;
    };
    let max_distance = radius * 2.0;
    distance_squared(a_pos, b_pos) <= max_distance * max_distance
}

fn detect_edge_contacts(store: &TurtleStore, bounds: Bounds, radius: f64) -> Vec<EdgeContact> {
    let mut contacts = Vec::new();
    for (index, position) in store.positions().iter().copied().enumerate() {
        let turtle = TurtleId::new(index);
        if position.x - radius <= bounds.min_x {
            contacts.push(EdgeContact {
                turtle,
                edge: Edge::Left,
            });
        }
        if position.x + radius >= bounds.max_x {
            contacts.push(EdgeContact {
                turtle,
                edge: Edge::Right,
            });
        }
        if position.y - radius <= bounds.min_y {
            contacts.push(EdgeContact {
                turtle,
                edge: Edge::Bottom,
            });
        }
        if position.y + radius >= bounds.max_y {
            contacts.push(EdgeContact {
                turtle,
                edge: Edge::Top,
            });
        }
    }
    contacts
}

fn cell_for(point: Point, cell_size: f64) -> (i64, i64) {
    (
        (point.x / cell_size).floor() as i64,
        (point.y / cell_size).floor() as i64,
    )
}

fn neighboring_cells((x, y): (i64, i64)) -> impl Iterator<Item = (i64, i64)> {
    (-1..=1).flat_map(move |dx| (-1..=1).map(move |dy| (x + dx, y + dy)))
}

fn distance_squared(a: Point, b: Point) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with_positions(positions: &[Point]) -> TurtleStore {
        let mut store = TurtleStore::new();
        for (index, position) in positions.iter().copied().enumerate() {
            store.set_position(TurtleId::new(index), position);
        }
        store
    }

    #[test]
    fn spatial_hash_finds_neighbor_candidates() {
        let store = store_with_positions(&[Point::new(0.0, 0.0), Point::new(5.0, 0.0)]);
        let hash = SpatialHash::rebuild(&store, 32.0);
        assert_eq!(
            hash.candidate_pairs(),
            vec![CollisionPair::new(TurtleId::new(0), TurtleId::new(1))]
        );
    }

    #[test]
    fn narrow_phase_filters_distant_candidates() {
        let store = store_with_positions(&[
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(100.0, 100.0),
        ]);
        let report = detect_collisions(
            &store,
            CollisionConfig {
                cell_size: 128.0,
                turtle_radius: 8.0,
                bounds: None,
            },
        );
        assert_eq!(
            report.turtle_pairs,
            vec![CollisionPair::new(TurtleId::new(0), TurtleId::new(1))]
        );
    }

    #[test]
    fn touching_checks_specific_pair() {
        let store = store_with_positions(&[Point::new(0.0, 0.0), Point::new(16.0, 0.0)]);
        assert!(touching(&store, TurtleId::new(0), TurtleId::new(1), 8.0));
        assert!(!touching(&store, TurtleId::new(0), TurtleId::new(1), 7.0));
    }

    #[test]
    fn detects_edge_contacts() {
        let store = store_with_positions(&[Point::new(-95.0, 95.0), Point::new(0.0, 0.0)]);
        let report = detect_collisions(
            &store,
            CollisionConfig {
                cell_size: 32.0,
                turtle_radius: 8.0,
                bounds: Some(Bounds::new(-100.0, -100.0, 100.0, 100.0)),
            },
        );
        assert_eq!(
            report.edge_contacts,
            vec![
                EdgeContact {
                    turtle: TurtleId::new(0),
                    edge: Edge::Left
                },
                EdgeContact {
                    turtle: TurtleId::new(0),
                    edge: Edge::Top
                },
            ]
        );
    }
}
