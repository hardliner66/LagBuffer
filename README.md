# LagBuffer

**LagBuffer** is a Rust crate designed to handle out-of-order events and reconcile state efficiently. It is particularly useful in scenarios such as game development or networked applications, where events may arrive out of sequence due to network latency or other factors.

## Features

- **Event Ordering**: Ensures events are applied in the correct order based on their OrderKey.
- **State Reconstruction**: Efficiently reconstructs state when out-of-order events are received.
- **Buffer Swapping**: Manages memory usage by swapping buffers when they reach capacity.

## Installation

Add the following to your Cargo.toml:

```toml
[dependencies]
lagbuffer = "0.1.0"
```

## Usage

Below is an example demonstrating how to use the `LagBuffer` crate. In this example, we define custom `State` and `Event` implementations to be used with the `LagBuffer`.

### Example: Handling In-Order and Out-of-Order Events

```rust
use lagbuffer::{LagBuffer, State, Event};

#[derive(Clone, Debug)]
struct MyState {
    data: Vec<i32>,
}

impl MyState {
    fn new() -> Self {
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

#[derive(Clone, Debug)]
enum Action {
    Insert,
    Replace,
}

#[derive(Clone, Debug)]
struct MyEvent {
    id: usize,
    value: i32,
    target: i32,
    action: Action,
}

impl Event<usize> for MyEvent {
    fn get_order_key(&self) -> usize {
        self.id
    }
}

fn main() {
    let initial_state = MyState::new();
    const BUFFER_SIZE: usize = 4;
    let mut buffer = LagBuffer::<MyState, BUFFER_SIZE>::new(initial_state);

    // Apply events
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
    }); // Out-of-order event

    // Access the current state
    println!("Current data: {:?}", buffer.state().data);
}
```

### Expected Output:

```
Current data: [10, 20, 30]
```

### Explanation
1) **Define Your State**: Implement the State trait for your custom state (MyState). The apply method defines how events modify the state.
2) **Define Your Events**: Implement the Event trait for your event type (MyEvent). The get_order_key method returns an order key used to determine the event sequence.
3) **Create a LagBuffer**: Instantiate a LagBuffer with your state and specify the buffer size (BUFFER_SIZE).
4) **Update with Events**: Use the update method to process events, whether they arrive in order or out of order.
5) **Access the State**: Use the state method to get a reference to the current state after all events have been applied.


## Notes
- **OrderKey**: The `OrderKey` is used to determine the sequence of events. It must implement the `Ord` trait.
- **Buffer Size**: Choose an appropriate buffer size (`SIZE`) based on your application's requirements. A larger buffer can handle more out-of-order events but uses more memory.

## How It Works

The `LagBuffer` maintains two buffers to store events and reconstructs the state from a base state and the events. When an event is received:

- If it's in order (its `OrderKey` is greater than to the last event's key in the active buffer):
  - The event is applied directly to the current state.
  - It's added to the active buffer.
  - If the active buffer's length exceeds half of `SIZE`, the event is also added to the secondary buffer.

- If it's out of order:
  - The event is inserted into the active buffer at the correct position to maintain order.
  - The current state is reconstructed by cloning the base state of the active buffer and reapplying all events from the active buffer.
  - If the active buffer's length exceeds half of `SIZE` and the secondary buffer is not empty, the event is also inserted into the secondary buffer at the correct position.

- When the active buffer's length exceeds `SIZE`, a buffer swap occurs:
  - The current state is saved as the new base state for the active buffer.
  - The active buffer is cleared.
  - The active and secondary buffers swap roles.
