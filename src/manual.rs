use core::panic;

use crate::{Event, State};

#[derive(Clone)]
enum EventOrSnapshot<S: State<OrderKey>, OrderKey: Ord = usize>
where
    OrderKey: Clone,
{
    Event(S::Event),
    Snapshot(S),
}

impl<S: State<OrderKey>, OrderKey: Ord + Clone> EventOrSnapshot<S, OrderKey> {
    pub fn is_snapshot(&self) -> bool {
        if let EventOrSnapshot::Snapshot(_) = self {
            return true;
        }
        false
    }
    pub fn as_snapshot(&self) -> &S {
        if let EventOrSnapshot::Snapshot(s) = self {
            return s;
        }
        panic!("Should never happen!");
    }
    pub fn as_event(&self) -> &S::Event {
        if let EventOrSnapshot::Event(e) = self {
            return e;
        }
        panic!("Should never happen!");
    }
}

pub struct ManualLagBuffer<S: State<OrderKey>, const SIZE: usize, OrderKey: Ord + Clone = usize> {
    buffer: Vec<EventOrSnapshot<S, OrderKey>>,
}

impl<S: State<OrderKey>, const SIZE: usize, OrderKey: Ord + Clone>
    ManualLagBuffer<S, SIZE, OrderKey>
{
    pub fn new(initial_state: S) -> Self {
        Self {
            buffer: vec![EventOrSnapshot::Snapshot(initial_state)],
        }
    }

    pub fn update(&mut self, event: S::Event) {
        let in_order = match self.buffer.last() {
            Some(EventOrSnapshot::Event(last_event)) => {
                last_event.get_order_key() <= event.get_order_key()
            }
            _ => true,
        };
        if in_order {
            self.buffer.push(EventOrSnapshot::Event(event));
        } else {
        }
    }

    /// Returns a reference to the current state.
    ///
    /// # Returns
    ///
    /// A reference to the current state after applying all events.
    pub fn state(&self) -> S {
        let pos = self
            .buffer
            .iter()
            .rev()
            .position(|i| i.is_snapshot())
            .unwrap();
        let mut state = self.buffer[self.buffer.len() - pos].as_snapshot().clone();
        for e in &self.buffer[self.buffer.len() - (pos - 1)..] {
            state.apply(e.as_event());
        }
        state
    }
}
