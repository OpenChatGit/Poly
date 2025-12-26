# 🎨 Poly UI

High-performance cross-platform UI framework built with Rust. A Flutter/Tauri alternative with native performance.

## Features

- **GPU-accelerated rendering** via wgpu (WebGPU)
- **Flexbox layout** via Taffy (same engine as Tauri)
- **Reactive state** management
- **Cross-platform**: Desktop (Windows, macOS, Linux) + Web (WASM)
- **Poly language** integration for scripting
- **Hot reload** support

## Quick Start

```rust
use poly_ui::prelude::*;

fn main() {
    App::new("My App")
        .size(800, 600)
        .root(
            Column::new()
                .padding(20.0)
                .child(Text::new("Hello, Poly UI!").size(24.0))
                .child(Button::new("Click me").on_click(|| println!("Clicked!")))
        )
        .run();
}
```

## Widgets

### Layout
- `Column` - Vertical flex container
- `Row` - Horizontal flex container
- `Stack` - Overlapping children
- `Container` - Styled box
- `Spacer` - Flexible space
- `Gap` - Fixed space

### Display
- `Text` - Text display
- `Image` - Image display
- `Icon` - Vector icons

### Input
- `Button` - Clickable button
- `TextInput` - Text field
- `Checkbox` - Toggle checkbox
- `Toggle` - Switch
- `Slider` - Range input

### Scrolling
- `ScrollView` - Scrollable container
- `ListView` - Virtualized list

## Styling

```rust
Container::new()
    .width(200.0)
    .height(100.0)
    .padding(16.0)
    .background(Color::rgb(30, 30, 30))
    .border_radius(8.0)
    .child(Text::new("Styled!"))
```

## State Management

```rust
let count = State::new(0);
let count_clone = count.clone();

Button::new("Increment")
    .on_click(move || count_clone.update(|c| *c += 1))
```

## Build

```bash
# Desktop
cargo run --example counter

# Web (WASM)
cargo build --target wasm32-unknown-unknown --features web
```

## Architecture

```
poly-ui/
├── src/
│   ├── core/       # Widget trait, state, events
│   ├── widgets/    # Built-in widgets
│   ├── style/      # Styling system
│   ├── layout/     # Flexbox layout (Taffy)
│   ├── render/     # GPU rendering (wgpu)
│   ├── runtime/    # Poly language integration
│   └── app/        # Application entry
└── examples/
```

## Roadmap

- [x] Core widget system
- [x] Styling API
- [x] State management
- [ ] wgpu renderer
- [ ] Text rendering (cosmic-text)
- [ ] Hot reload
- [ ] Poly DSL for UI
- [ ] Animations
- [ ] Accessibility
