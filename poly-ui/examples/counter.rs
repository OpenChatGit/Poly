//! Counter example - demonstrates state management

use poly_ui::prelude::*;
use poly_ui::core::State;

fn main() {
    let count = State::new(0i32);
    let count_inc = count.clone();
    let count_dec = count.clone();
    
    let app = App::new("Counter")
        .size(300, 200)
        .root(
            Column::new()
                .padding(24.0)
                .gap(16.0)
                .child(
                    Text::new(format!("Count: {}", count.get()))
                        .size(32.0)
                        .bold()
                        .center()
                )
                .child(
                    Row::new()
                        .gap(12.0)
                        .child(
                            Button::new("-")
                                .on_click(move || count_dec.update(|c| *c -= 1))
                        )
                        .child(
                            Button::new("+")
                                .on_click(move || count_inc.update(|c| *c += 1))
                        )
                )
        );
    
    app.run();
}
