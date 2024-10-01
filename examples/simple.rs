use lagbuffer::{Event, LagBuffer, State};

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

fn main() {
    // Example usage in a real program
    let mut buffer = LagBuffer::<MyState, 4>::new(MyState::new());

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
        id: 4,
        value: 30,
        target: 0,
        action: Action::Insert,
    });
    buffer.update(MyEvent {
        id: 3,
        value: 100,
        target: 20,
        action: Action::Replace,
    });

    assert_eq!(buffer.state().data, vec![10, 100, 30]);
    println!("Current state data: {:?}", buffer.state().data);
}
