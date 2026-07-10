//! Dynaturtle event demons (`WHEN ... [...]`).
//!
//! Demons are registered conditions plus instruction-list bodies. Collision
//! detection produces events; matching demons enqueue work. The queue is drained
//! with a fuel budget so a runaway interaction cannot monopolize a simulation
//! tick.

use std::collections::VecDeque;

use crate::collision::{CollisionPair, Edge, EdgeContact};
use crate::dynaturtle::TurtleId;
use crate::value::List;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DemonCondition {
    Touching(TurtleId, TurtleId),
    Edge(Option<TurtleId>),
    OverColor(u32),
}

impl DemonCondition {
    fn matches(&self, event: &DemonEvent) -> bool {
        match (self, event) {
            (DemonCondition::Touching(a, b), DemonEvent::Touching(pair)) => {
                CollisionPair::new(*a, *b) == *pair
            }
            (DemonCondition::Edge(None), DemonEvent::Edge(_)) => true,
            (DemonCondition::Edge(Some(turtle)), DemonEvent::Edge(contact)) => {
                *turtle == contact.turtle
            }
            (
                DemonCondition::OverColor(color),
                DemonEvent::OverColor {
                    color: event_color, ..
                },
            ) => color == event_color,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DemonEvent {
    Touching(CollisionPair),
    Edge(EdgeContact),
    OverColor { turtle: TurtleId, color: u32 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Demon {
    id: DemonId,
    condition: DemonCondition,
    body: List,
}

impl Demon {
    pub fn id(&self) -> DemonId {
        self.id
    }

    pub fn condition(&self) -> &DemonCondition {
        &self.condition
    }

    pub fn body(&self) -> &List {
        &self.body
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemonId(u64);

impl DemonId {
    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DemonWorkItem {
    pub demon_id: DemonId,
    pub event: DemonEvent,
    pub body: List,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrainResult {
    pub drained: Vec<DemonWorkItem>,
    pub remaining: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DemonScheduler {
    demons: Vec<Demon>,
    queue: VecDeque<DemonWorkItem>,
    next_id: u64,
}

impl DemonScheduler {
    pub fn new() -> Self {
        Self {
            demons: Vec::new(),
            queue: VecDeque::new(),
            next_id: 1,
        }
    }

    pub fn register(&mut self, condition: DemonCondition, body: List) -> DemonId {
        let id = DemonId(self.next_id);
        self.next_id += 1;
        self.demons.push(Demon {
            id,
            condition,
            body,
        });
        id
    }

    pub fn forget(&mut self, id: DemonId) -> bool {
        let old_len = self.demons.len();
        self.demons.retain(|demon| demon.id != id);
        old_len != self.demons.len()
    }

    pub fn demons(&self) -> &[Demon] {
        &self.demons
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn push_event(&mut self, event: DemonEvent) {
        for demon in &self.demons {
            if demon.condition.matches(&event) {
                self.queue.push_back(DemonWorkItem {
                    demon_id: demon.id,
                    event: event.clone(),
                    body: demon.body.clone(),
                });
            }
        }
    }

    pub fn push_collision_report(
        &mut self,
        pairs: impl IntoIterator<Item = CollisionPair>,
        edge_contacts: impl IntoIterator<Item = EdgeContact>,
    ) {
        for pair in pairs {
            self.push_event(DemonEvent::Touching(pair));
        }
        for contact in edge_contacts {
            self.push_event(DemonEvent::Edge(contact));
        }
    }

    pub fn drain_with_fuel(&mut self, fuel: usize) -> DrainResult {
        let mut drained = Vec::new();
        for _ in 0..fuel {
            let Some(item) = self.queue.pop_front() else {
                break;
            };
            drained.push(item);
        }
        DrainResult {
            drained,
            remaining: self.queue.len(),
        }
    }
}

impl Default for DemonScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
fn _edge_is_used(edge: Edge) -> Edge {
    // Keeps Edge imported as part of the public demon vocabulary even before
    // matching on specific edges is implemented.
    edge
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::{detect_collisions, Bounds, CollisionConfig};
    use crate::dynaturtle::TurtleStore;
    use crate::turtle::Point;
    use crate::value::Value;

    fn body_word(word: &str) -> List {
        let mut interner = crate::value::Interner::new();
        List::from_values([Value::word(&mut interner, word)])
    }

    #[test]
    fn touching_event_enqueues_matching_demon() {
        let mut scheduler = DemonScheduler::new();
        let id = scheduler.register(
            DemonCondition::Touching(TurtleId::new(0), TurtleId::new(1)),
            body_word("boom"),
        );
        scheduler.push_event(DemonEvent::Touching(CollisionPair::new(
            TurtleId::new(1),
            TurtleId::new(0),
        )));
        let drained = scheduler.drain_with_fuel(10);
        assert_eq!(drained.drained.len(), 1);
        assert_eq!(drained.drained[0].demon_id, id);
        assert_eq!(drained.remaining, 0);
    }

    #[test]
    fn edge_condition_can_match_any_or_specific_turtle() {
        let mut scheduler = DemonScheduler::new();
        scheduler.register(DemonCondition::Edge(None), body_word("any"));
        scheduler.register(
            DemonCondition::Edge(Some(TurtleId::new(2))),
            body_word("two"),
        );
        scheduler.push_event(DemonEvent::Edge(EdgeContact {
            turtle: TurtleId::new(2),
            edge: Edge::Top,
        }));
        assert_eq!(scheduler.drain_with_fuel(10).drained.len(), 2);
    }

    #[test]
    fn drain_respects_fuel_budget() {
        let mut scheduler = DemonScheduler::new();
        scheduler.register(DemonCondition::Edge(None), body_word("edge"));
        for _ in 0..5 {
            scheduler.push_event(DemonEvent::Edge(EdgeContact {
                turtle: TurtleId::new(0),
                edge: Edge::Left,
            }));
        }
        let drained = scheduler.drain_with_fuel(2);
        assert_eq!(drained.drained.len(), 2);
        assert_eq!(drained.remaining, 3);
        assert_eq!(scheduler.queue_len(), 3);
    }

    #[test]
    fn can_enqueue_from_collision_report() {
        let mut store = TurtleStore::new();
        store.set_position(TurtleId::new(0), Point::new(0.0, 0.0));
        store.set_position(TurtleId::new(1), Point::new(5.0, 0.0));
        let report = detect_collisions(
            &store,
            CollisionConfig {
                cell_size: 32.0,
                turtle_radius: 8.0,
                bounds: Some(Bounds::new(-100.0, -100.0, 100.0, 100.0)),
            },
        );
        let mut scheduler = DemonScheduler::new();
        scheduler.register(
            DemonCondition::Touching(TurtleId::new(0), TurtleId::new(1)),
            body_word("touch"),
        );
        scheduler.push_collision_report(report.turtle_pairs, report.edge_contacts);
        assert_eq!(scheduler.drain_with_fuel(10).drained.len(), 1);
    }

    #[test]
    fn forget_removes_demon() {
        let mut scheduler = DemonScheduler::new();
        let id = scheduler.register(DemonCondition::OverColor(3), body_word("color"));
        assert!(scheduler.forget(id));
        scheduler.push_event(DemonEvent::OverColor {
            turtle: TurtleId::new(0),
            color: 3,
        });
        assert_eq!(scheduler.drain_with_fuel(10).drained.len(), 0);
    }
}
