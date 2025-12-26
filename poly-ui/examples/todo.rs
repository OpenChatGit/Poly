//! Todo app example

use poly_ui::prelude::*;
use poly_ui::core::State;

#[allow(dead_code)]
#[derive(Clone)]
struct TodoItem {
    id: u32,
    text: String,
    done: bool,
}

fn main() {
    let _todos: State<Vec<TodoItem>> = State::new(vec![
        TodoItem { id: 1, text: "Learn Poly".into(), done: false },
        TodoItem { id: 2, text: "Build UI".into(), done: true },
        TodoItem { id: 3, text: "Ship it!".into(), done: false },
    ]);
    
    let app = App::new("Todo App")
        .size(400, 500)
        .root(
            Column::new()
                .padding(16.0)
                .gap(12.0)
                .child(Text::new("My Todos").size(24.0).bold())
                .child(
                    TextInput::new()
                        .placeholder("Add a new todo...")
                )
                .child(Button::new("Add").primary())
        );
    
    app.run();
}
