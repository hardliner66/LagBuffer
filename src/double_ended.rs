use crate::{Event, State};

pub struct CircularBuffer<T, const SIZE: usize> {
    buffer: [Option<T>; SIZE],
    capacity: usize,
    start: usize,
    end: usize,
    full: bool,
}

impl<T, const SIZE: usize> CircularBuffer<T, SIZE> {
    // Create a new circular buffer with a given capacity
    pub fn new() -> Self {
        assert!(SIZE > 0, "Capacity must be greater than 0");
        CircularBuffer {
            buffer: [const { None }; SIZE],
            capacity: SIZE,
            start: 0,
            end: 0,
            full: false,
        }
    }

    // Push an element into the circular buffer
    // Returns an Option containing the dropped element, if any
    pub fn push(&mut self, item: T) -> Option<T> {
        let mut dropped = None;

        if self.full {
            // If the buffer is full, the element at `start` will be replaced
            dropped = self.buffer[self.start].take();
            self.start = (self.start + 1) % self.capacity;
        }

        self.buffer[self.end] = Some(item);
        self.end = (self.end + 1) % self.capacity;

        // Check if the buffer is full
        if self.end == self.start {
            self.full = true;
        }

        dropped
    }

    // Get the current size of the buffer
    #[cfg(test)]
    pub fn size(&self) -> usize {
        if self.full {
            self.capacity
        } else if self.end >= self.start {
            self.end - self.start
        } else {
            self.capacity - self.start + self.end
        }
    }

    // Get the capacity of the buffer
    #[cfg(test)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    // Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        !self.full && self.start == self.end
    }
    //
    // Check if the buffer is full
    #[cfg(test)]
    pub fn is_full(&self) -> bool {
        self.full
    }

    // Pop an element from the front of the buffer
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let item = self.buffer[self.start].take();
        self.start = (self.start + 1) % self.capacity;
        self.full = false;

        item
    }

    // // Peek at the next element to be popped, without removing it
    // pub fn peek(&self) -> Option<&T> {
    //     if self.is_empty() {
    //         None
    //     } else {
    //         // Safely access the element at the start index
    //         self.buffer[self.start].as_ref()
    //     }
    // }

    // Peek at the next element to be popped, without removing it
    pub fn peek_end(&self) -> Option<&T> {
        if self.is_empty() {
            None
        } else {
            // Calculate the previous index (the last inserted element)
            let end_index = if self.end == 0 {
                self.capacity - 1
            } else {
                self.end - 1
            };

            // Safely access the element at the calculated index
            self.buffer[end_index].as_ref()
        }
    }
}

pub struct DoubleEndedLagBuffer<S: State<OrderKey>, const SIZE: usize, OrderKey: Ord = usize> {
    buffer: CircularBuffer<S::Event, SIZE>,
    head: S,
    tail: S,
}

impl<S: State<OrderKey>, const SIZE: usize, OrderKey: Ord> DoubleEndedLagBuffer<S, SIZE, OrderKey> {
    pub fn new(initial_state: S) -> Self {
        Self {
            buffer: CircularBuffer::new(),
            head: initial_state.clone(),
            tail: initial_state,
        }
    }

    pub fn update(&mut self, event: S::Event) {
        let in_order = match self.buffer.peek_end() {
            Some(last_event) => last_event.get_order_key() <= event.get_order_key(),
            None => true,
        };
        if in_order {
            self.head.apply(&event);
            if let Some(ev) = self.buffer.push(event) {
                self.tail.apply(&ev);
            }
        } else {
            let mut ev = Some(event);
            let mut cb = CircularBuffer::<S::Event, SIZE>::new();
            self.head = self.tail.clone();
            while let Some(event) = self.buffer.pop() {
                if let Some(e) = &ev {
                    if event.get_order_key() > e.get_order_key() {
                        self.head.apply(&e);
                        if let Some(e) = ev.take() {
                            cb.push(e);
                        }
                    }
                    self.head.apply(&event);
                    cb.push(event);
                }
            }
        }
    }

    /// Returns a reference to the current state.
    ///
    /// # Returns
    ///
    /// A reference to the current state after applying all events.
    pub fn state_ref(&self) -> &S {
        &self.head
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Example State and Event implementation for testing.

    #[derive(Clone, PartialEq)]
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
        let mut buffer = DoubleEndedLagBuffer::<MyState, 4>::new(MyState::new());

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
        assert_eq!(buffer.state_ref().data, vec![10, 20, 30, 40, 50]);
    }

    #[test]
    fn test_event_application_out_of_order() {
        let mut buffer = DoubleEndedLagBuffer::<MyState, 4>::new(MyState::new());

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
        assert_eq!(buffer.state_ref().data, vec![10, 20, 30]);
    }

    #[test]
    fn test_replace_action() {
        let mut buffer = DoubleEndedLagBuffer::<MyState, 4>::new(MyState::new());

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
        assert_eq!(buffer.state_ref().data, vec![10, 99, 30]);
    }

    #[test]
    fn test_push_to_empty_buffer() {
        let mut buffer = CircularBuffer::<usize, 3>::new();

        assert_eq!(buffer.push(1), None);
        assert_eq!(buffer.push(2), None);
        assert_eq!(buffer.push(3), None);

        assert_eq!(buffer.is_full(), true);
        assert_eq!(buffer.is_empty(), false);
    }

    #[test]
    fn test_push_when_full() {
        let mut buffer = CircularBuffer::<usize, 3>::new();

        buffer.push(1);
        buffer.push(2);
        buffer.push(3);

        // Buffer is full, pushing another element should drop the oldest (1)
        assert_eq!(buffer.push(4), Some(1));
        assert_eq!(buffer.push(5), Some(2));
        assert_eq!(buffer.push(6), Some(3));

        assert_eq!(buffer.is_full(), true);
    }

    #[test]
    fn test_pop_from_buffer() {
        let mut buffer = CircularBuffer::<usize, 3>::new();

        buffer.push(1);
        buffer.push(2);
        buffer.push(3);

        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), None); // Buffer is empty now

        assert_eq!(buffer.is_empty(), true);
    }

    #[test]
    fn test_push_and_pop_interleaved() {
        let mut buffer = CircularBuffer::<usize, 3>::new();

        assert_eq!(buffer.push(1), None);
        assert_eq!(buffer.push(2), None);

        // Pop one element
        assert_eq!(buffer.pop(), Some(1));

        // Push another element
        assert_eq!(buffer.push(3), None);

        // Pop remaining elements
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));

        // Now the buffer is empty
        assert_eq!(buffer.pop(), None);
        assert_eq!(buffer.is_empty(), true);
    }

    #[test]
    fn test_buffer_wraparound() {
        let mut buffer = CircularBuffer::<usize, 3>::new();

        // Fill the buffer
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);

        // Buffer is full, this will start overwriting
        assert_eq!(buffer.push(4), Some(1));
        assert_eq!(buffer.push(5), Some(2));

        // Pop remaining elements
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), Some(4));
        assert_eq!(buffer.pop(), Some(5));

        assert_eq!(buffer.is_empty(), true);
    }

    #[test]
    fn test_size_and_capacity() {
        let mut buffer = CircularBuffer::<usize, 3>::new();

        assert_eq!(buffer.size(), 0);
        assert_eq!(buffer.capacity(), 3);

        buffer.push(1);
        assert_eq!(buffer.size(), 1);

        buffer.push(2);
        assert_eq!(buffer.size(), 2);

        buffer.push(3);
        assert_eq!(buffer.size(), 3);
        assert_eq!(buffer.is_full(), true);

        // Buffer is full, now overwriting
        buffer.push(4);
        assert_eq!(buffer.size(), 3);
    }
}
