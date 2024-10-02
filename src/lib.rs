mod double_buffered;
pub use double_buffered::DoubleBufferedLagBuffer;

mod double_ended;
pub use double_ended::DoubleEndedLagBuffer;

mod manual;
pub use manual::ManualLagBuffer;

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

pub trait BaseLagBuffer<S: State<O>, O: Ord = usize> {
    fn update(&mut self, event: S::Event);
}

pub trait LagBufferState<S: State<O>, O: Ord = usize>: BaseLagBuffer<S, O> {
    fn state(&self) -> S;
}
pub trait LagBufferStateRef<S: State<O>, O: Ord = usize>: BaseLagBuffer<S, O> {
    fn state_ref(&self) -> &S;
}

impl<S: State<OrderKey>, const SIZE: usize, OrderKey: Ord> BaseLagBuffer<S, OrderKey>
    for DoubleBufferedLagBuffer<S, SIZE, OrderKey>
{
    fn update(&mut self, event: S::Event) {
        (self as &mut DoubleBufferedLagBuffer<S, SIZE, OrderKey>).update(event);
    }
}

impl<S: State<OrderKey>, const SIZE: usize, OrderKey: Ord> LagBufferState<S, OrderKey>
    for DoubleBufferedLagBuffer<S, SIZE, OrderKey>
{
    fn state(&self) -> S {
        (self as &DoubleBufferedLagBuffer<S, SIZE, OrderKey>)
            .state_ref()
            .clone()
    }
}

impl<S: State<OrderKey>, const SIZE: usize, OrderKey: Ord> LagBufferStateRef<S, OrderKey>
    for DoubleBufferedLagBuffer<S, SIZE, OrderKey>
{
    fn state_ref(&self) -> &S {
        (self as &DoubleBufferedLagBuffer<S, SIZE, OrderKey>).state_ref()
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
            }
        }
    }

    #[derive(Clone, Debug)]
    enum Action {
        Insert,
    }

    #[derive(Clone, Debug)]
    struct MyEvent {
        id: usize,
        value: i32,
        action: Action,
    }

    impl Event<usize> for MyEvent {
        fn get_order_key(&self) -> usize {
            self.id
        }
    }

    #[test]
    fn test_trait() {
        let mut buffer: Box<dyn LagBufferStateRef<MyState>> =
            Box::new(DoubleBufferedLagBuffer::<MyState, 4>::new(MyState::new()));

        // Apply 4 insert events in order.
        buffer.update(MyEvent {
            id: 1,
            value: 10,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 2,
            value: 20,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 3,
            value: 30,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 4,
            value: 40,
            action: Action::Insert,
        });
        buffer.update(MyEvent {
            id: 5,
            value: 50,
            action: Action::Insert,
        });

        // Verify that the current state is as expected (order matters here).
        assert_eq!(buffer.state_ref().data, vec![10, 20, 30, 40, 50]);
    }
}
