/// A trait representing an event that has an associated order key of type `OrderKey`.
///
/// Events modify the state, and the order in which they are applied is determined by the `OrderKey`.
///
/// # Type Parameters
/// - `OrderKey`: The type that determines the order of events, which must implement `Ord`.
pub trait Event<OrderKey: Ord> {
    /// Returns the order key of the event.
    fn get_order_key(&self) -> OrderKey;
}

/// A trait representing a state that can be modified by events.
///
/// The state must be clonable and can be updated based on the events it receives. Events
/// must have an associated order key of type `OrderKey` to determine their sequence.
///
/// # Type Parameters
/// - `OrderKey`: The type that determines the order of events, which must implement `Ord`.
pub trait State<OrderKey: Ord>: Clone {
    /// The type of event that modifies the state.
    type Event: Clone + Event<OrderKey>;

    /// Applies an event to the current state, modifying it accordingly.
    ///
    /// # Arguments
    /// - `event`: The event that will be applied to the state.
    fn apply(&mut self, event: &Self::Event);
}

pub trait LagBuffer<S: State<O>, O: Ord = usize> {
    fn update(&mut self, event: S::Event);
    fn state(&self) -> &S;
}

/// A buffer system designed to handle out-of-order events and reconcile the state.
///
/// The `DoubleBufferedLagBuffer` is a generic structure that manages the application of events to a state,
/// ensuring that events are applied in the correct order even if they arrive out of sequence.
/// This is particularly useful in scenarios like networked applications or games, where events
/// may not arrive in the order they were generated due to latency or other network issues.
///
/// The buffer maintains two event buffers and their corresponding base states:
///
/// - **Active Buffer**: The primary buffer where events are stored and applied to the current state.
/// - **Secondary Buffer**: Used to assist in state reconstruction and prepare for buffer swaps.
///
/// The key features of `DoubleBufferedLagBuffer` include:
///
/// - **Event Ordering**: Ensures that events are applied in the correct order based on their `OrderKey`.
/// - **State Reconstruction**: Reconstructs the state efficiently when out-of-order events are received.
/// - **Buffer Swapping**: Manages memory usage by swapping buffers when they reach a certain capacity.
///
/// # Type Parameters
///
/// - `S`: The type of the state, which must implement the [`State`](trait.State.html) trait.
/// - `SIZE`: The maximum number of events each buffer can hold before triggering a swap.
/// - `OrderKey`: The type of the event's order key, which must implement [`Ord`](https://doc.rust-lang.org/std/cmp/trait.Ord.html). Defaults to `usize`.
///
/// # Fields
///
/// - `current_state`: The current state after applying all events from the active buffer.
/// - `active_buffer`: Index indicating which buffer is currently active (0 or 1).
/// - `buffer_bases`: An array holding the base states corresponding to each buffer.
/// - `buffers`: An array of two event buffers (`Vec<S::Event>`) used to store events.
///
/// # Examples
///
/// ```rust
/// use lagbuffer::{DoubleBufferedLagBuffer, State, Event}; // Replace `your_crate` with the actual crate name.
///
/// #[derive(Clone, Debug)]
/// struct MyState {
///     data: Vec<i32>,
/// }
///
/// impl MyState {
///     fn new() -> Self {
///         Self { data: Vec::new() }
///     }
/// }
///
/// impl State<usize> for MyState {
///     type Event = MyEvent;
///
///     fn apply(&mut self, event: &Self::Event) {
///         match event.action {
///             Action::Insert => self.data.push(event.value),
///             Action::Replace => {
///                 if let Some(pos) = self.data.iter().position(|&x| x == event.target) {
///                     self.data[pos] = event.value;
///                 }
///             }
///         }
///     }
/// }
///
/// #[derive(Clone)]
/// enum Action {
///     Insert,
///     Replace,
/// }
///
/// #[derive(Clone)]
/// struct MyEvent {
///     id: usize,
///     value: i32,
///     target: i32, // Used for replacing a specific element
///     action: Action,
/// }
///
/// impl Event<usize> for MyEvent {
///     fn get_order_key(&self) -> usize {
///         self.id
///     }
/// }
///
/// let initial_state = MyState::new();
/// let mut lag_buffer = DoubleBufferedLagBuffer::<MyState, 4>::new(initial_state);
///
/// // Create some events
/// let event1 = MyEvent {
///     id: 1,
///     value: 10,
///     target: 0,
///     action: Action::Insert,
/// };
/// let event3 = MyEvent {
///     id: 3,
///     value: 30,
///     target: 0,
///     action: Action::Insert,
/// };
/// let event2 = MyEvent {
///     id: 2,
///     value: 20,
///     target: 0,
///     action: Action::Insert,
/// }; // Out-of-order event
///
/// // Update the buffer with events, possibly out of order.
/// lag_buffer.update(event1);
/// lag_buffer.update(event3);
/// lag_buffer.update(event2);
///
/// // Access the current state.
/// let state = lag_buffer.state();
/// assert_eq!(state.data, vec![10, 20, 30]); // Should print [10, 20, 30]
/// ```
pub struct DoubleBufferedLagBuffer<S: State<OrderKey>, const SIZE: usize, OrderKey: Ord = usize> {
    pub(crate) current_state: S,
    pub(crate) active_buffer: usize,
    pub(crate) buffer_bases: [S; 2],
    pub(crate) buffers: [Vec<S::Event>; 2],
}

impl<S: State<OrderKey>, const SIZE: usize, OrderKey: Ord>
    DoubleBufferedLagBuffer<S, SIZE, OrderKey>
{
    /// Creates a new `DoubleBufferedLagBuffer` with the given initial state.
    ///
    /// # Arguments
    ///
    /// - `initial_state`: The initial state from which the buffer will start.
    ///
    /// # Returns
    ///
    /// A new `DoubleBufferedLagBuffer` instance initialized with the provided state.
    pub fn new(initial_state: S) -> Self {
        Self {
            buffers: [Vec::with_capacity(SIZE), Vec::with_capacity(SIZE)],
            active_buffer: 0,
            buffer_bases: [initial_state.clone(), initial_state.clone()],
            current_state: initial_state,
        }
    }

    /// Updates the buffer with a new event.
    ///
    /// This method handles the incoming event by determining whether it is in order or out of order
    /// based on its `OrderKey`. It ensures that the events are applied to the state in the correct
    /// order, reconstructing the state if necessary.
    ///
    /// # Behavior
    ///
    /// - **In-Order Event**:
    ///   - The event's `OrderKey` is greater than or equal to the last event's key in the active buffer.
    ///   - The event is applied directly to the `current_state`.
    ///   - The event is added to the active buffer.
    ///   - If the active buffer's length exceeds half of `SIZE`, the event is also added to the secondary buffer.
    ///
    /// - **Out-of-Order Event**:
    ///   - The event's `OrderKey` is less than the last event's key in the active buffer.
    ///   - The event is inserted into the active buffer at the correct position to maintain order.
    ///   - The `current_state` is reconstructed by cloning the base state of the active buffer and
    ///     reapplying all events from the active buffer.
    ///   - If the active buffer's length exceeds half of `SIZE` and the secondary buffer is not empty,
    ///     the event is also inserted into the secondary buffer at the correct position.
    ///
    /// - **Buffer Swap**:
    ///   - Occurs after the event is processed.
    ///   - If the active buffer's length exceeds `SIZE`, a buffer swap is triggered:
    ///     - The `current_state` is saved as the new base state for the active buffer.
    ///     - The active buffer is cleared.
    ///     - The active and secondary buffers swap roles.
    ///
    /// # Arguments
    ///
    /// - `event`: The event to be applied or buffered.
    pub fn update(&mut self, event: S::Event) {
        let active_buffer = self.active_buffer;
        let secondary_buffer = 1 - active_buffer;

        // Determine if the event is in order
        let in_order = match self.buffers[active_buffer].last() {
            Some(last_event) => last_event.get_order_key() <= event.get_order_key(),
            None => true,
        };

        if in_order {
            // In-order event: apply directly and add to active buffer
            self.buffers[active_buffer].push(event.clone());

            // If buffer is more than half full, start populating the secondary buffer
            if self.buffers[active_buffer].len() > (SIZE / 2) {
                if self.buffers[secondary_buffer].is_empty() {
                    self.buffer_bases[secondary_buffer] = self.current_state.clone();
                }
                self.buffers[secondary_buffer].push(event.clone());
            }

            self.current_state.apply(&event);
        } else {
            // Out-of-order event: insert into active buffer and reconstruct state
            let insert_position = self.buffers[active_buffer]
                .binary_search_by_key(&event.get_order_key(), S::Event::get_order_key)
                .unwrap_or_else(|e| e);

            self.buffers[active_buffer].insert(insert_position, event.clone());

            // Reconstruct current state from buffer base and events
            self.current_state = self.buffer_bases[active_buffer].clone();
            for buffered_event in &self.buffers[active_buffer] {
                self.current_state.apply(buffered_event);
            }

            // Update secondary buffer if necessary
            if self.buffers[active_buffer].len() > (SIZE / 2)
                && !self.buffers[secondary_buffer].is_empty()
            {
                let insert_position = self.buffers[secondary_buffer]
                    .binary_search_by_key(&event.get_order_key(), S::Event::get_order_key)
                    .unwrap_or_else(|e| e);

                self.buffers[secondary_buffer].insert(insert_position, event);
            }
        }

        // Check if buffer swap is needed
        if self.buffers[active_buffer].len() > SIZE {
            // Save current state as new buffer base
            self.buffer_bases[active_buffer] = self.current_state.clone();
            // Clear the active buffer
            self.buffers[active_buffer].clear();
            // Swap active and secondary buffers
            self.active_buffer = secondary_buffer;
        }
    }

    /// Returns a reference to the current state.
    ///
    /// # Returns
    ///
    /// A reference to the current state after applying all events.
    pub fn state(&self) -> &S {
        &self.current_state
    }

    #[cfg(test)]
    pub fn get_active_buffer_len(&self) -> usize {
        self.buffers[self.active_buffer].len()
    }

    #[cfg(test)]
    pub fn get_secondary_buffer_len(&self) -> usize {
        let secondary_buffer = if self.active_buffer == 1 { 0 } else { 1 };
        self.buffers[secondary_buffer].len()
    }
}

impl<S: State<OrderKey>, const SIZE: usize, OrderKey: Ord> LagBuffer<S, OrderKey>
    for DoubleBufferedLagBuffer<S, SIZE, OrderKey>
{
    fn update(&mut self, event: S::Event) {
        (self as &mut DoubleBufferedLagBuffer<S, SIZE, OrderKey>).update(event);
    }

    fn state(&self) -> &S {
        (self as &DoubleBufferedLagBuffer<S, SIZE, OrderKey>).state()
    }
}

// Testing section.

#[cfg(test)]
mod tests {
    use super::*;
    // Example State and Event implementation for testing.

    #[derive(Clone, Debug, PartialEq)]
    struct MyState {
        pub data: Vec<i32>,
    }

    impl MyState {
        pub fn new() -> Self {
            Self { data: Vec::new() }
        }
    }

    impl State<usize> for MyState {
        type Event = MyEvent;

        fn apply(&mut self, event: &Self::Event) {
            match event.action {
                Action::Insert => self.data.push(event.value),
                Action::Replace => {
                    if let Some(pos) = self.data.iter().position(|&x| x == event.target) {
                        self.data[pos] = event.value;
                    }
                }
            }
        }
    }

    #[derive(Clone)]
    enum Action {
        Insert,
        Replace,
    }

    #[derive(Clone)]
    struct MyEvent {
        id: usize,
        value: i32,
        target: i32, // Used for replacing a specific element
        action: Action,
    }

    impl Event<usize> for MyEvent {
        fn get_order_key(&self) -> usize {
            self.id
        }
    }

    #[test]
    fn test_event_application_in_order() {
        let mut buffer = DoubleBufferedLagBuffer::<MyState, 4>::new(MyState::new());

        // Apply 4 insert events in order.
        buffer.update(MyEvent {
            id: 1,
            value: 10,
            target: 0,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 2,
            value: 20,
            target: 0,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 3,
            value: 30,
            target: 0,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 4,
            value: 40,
            target: 0,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 5,
            value: 50,
            target: 0,
            action: Action::Insert,
        });

        // Verify that the current state is as expected (order matters here).
        assert_eq!(buffer.state().data, vec![10, 20, 30, 40, 50]);

        // Verify that a buffer swap happened and secondary buffer is cleared.
        assert_eq!(buffer.get_secondary_buffer_len(), 0);
    }

    #[test]
    fn test_same_state() {
        let mut buffer = DoubleBufferedLagBuffer::<MyState, 4>::new(MyState::new());

        // Apply 4 insert events in order.
        buffer.update(MyEvent {
            id: 1,
            value: 10,
            target: 0,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 2,
            value: 20,
            target: 0,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 3,
            value: 30,
            target: 0,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 4,
            value: 40,
            target: 0,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 5,
            value: 50,
            target: 0,
            action: Action::Insert,
        });

        let mut first = dbg!(buffer.buffer_bases[0].clone());
        for event in &buffer.buffers[0] {
            first.apply(event);
        }

        let mut second = dbg!(buffer.buffer_bases[1].clone());
        for event in &buffer.buffers[1] {
            second.apply(event);
        }
        assert_eq!(first, second);
    }

    #[test]
    fn test_event_application_out_of_order() {
        let mut buffer = DoubleBufferedLagBuffer::<MyState, 4>::new(MyState::new());

        // Apply some insert events.
        buffer.update(MyEvent {
            id: 1,
            value: 10,
            target: 0,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 3,
            value: 30,
            target: 0,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 2,
            value: 20,
            target: 0,
            action: Action::Insert,
        }); // Out-of-order event.

        // The state should reflect that the event with id=2 was applied in the correct order.
        assert_eq!(buffer.state().data, vec![10, 20, 30]);
    }

    #[test]
    fn test_buffer_has_half_after_swap() {
        let mut buffer = DoubleBufferedLagBuffer::<MyState, 4>::new(MyState::new());

        // Apply 5 events to trigger buffer swap.
        buffer.update(MyEvent {
            id: 1,
            value: 10,
            target: 0,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 2,
            value: 20,
            target: 0,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 3,
            value: 30,
            target: 0,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 4,
            value: 40,
            target: 0,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 5,
            value: 50,
            target: 0,
            action: Action::Insert,
        });

        // After buffer swap, one more than half of the events should be in the active buffer.
        assert_eq!(buffer.get_active_buffer_len(), 3);
    }

    #[test]
    fn test_replace_action() {
        let mut buffer = DoubleBufferedLagBuffer::<MyState, 4>::new(MyState::new());

        // Apply insert events.
        buffer.update(MyEvent {
            id: 1,
            value: 10,
            target: 0,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 2,
            value: 20,
            target: 0,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 3,
            value: 30,
            target: 0,
            action: Action::Insert,
        });

        // Apply a replace event.
        buffer.update(MyEvent {
            id: 4,
            value: 99,
            target: 20,
            action: Action::Replace,
        });

        // Verify that the replace action was correctly applied.
        assert_eq!(buffer.state().data, vec![10, 99, 30]);
    }
}
