# Poly Language Support for VS Code

Syntax highlighting and language support for the Poly programming language.

## Features

- Syntax highlighting for `.poly` files
- Comment toggling with `#`
- Auto-closing brackets and quotes
- Indentation support for Python-like syntax

## Installation

### From VSIX (local)

1. Build the extension:
   ```bash
   cd poly-vscode
   npm install -g vsce
   vsce package
   ```

2. Install in VS Code:
   - Open VS Code
   - Press `Ctrl+Shift+P`
   - Type "Install from VSIX"
   - Select the generated `.vsix` file

### Manual Installation

Copy the `poly-vscode` folder to:
- Windows: `%USERPROFILE%\.vscode\extensions\poly-lang-0.1.0`
- macOS: `~/.vscode/extensions/poly-lang-0.1.0`
- Linux: `~/.vscode/extensions/poly-lang-0.1.0`

Then restart VS Code.

## Syntax Example

```poly
# This is a comment
let name = "World"
let count = 42

fn greet(who):
    print(f"Hello, {who}!")
    return true

if count > 10:
    greet(name)
else:
    print("Too small")

# Web functions
let page = html("Title", "<h1>Hello</h1>", "body { color: red; }", "")
write_file("index.html", page)
```

## Supported Keywords

- Control flow: `if`, `else`, `elif`, `for`, `while`, `break`, `continue`, `return`, `match`, `case`
- Declarations: `fn`, `let`, `const`, `class`, `import`, `from`, `as`
- Operators: `and`, `or`, `not`, `in`, `is`
- Constants: `true`, `false`, `none`
- Built-in functions: `print`, `len`, `range`, `html`, `router`, `store`, `component`, etc.
