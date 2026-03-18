# Poly Language Extension for VSCode/Kiro

Syntax highlighting and language support for the Poly programming language.

## Features

- **Syntax Highlighting** - Full syntax highlighting for `.poly` files
- **Auto-Closing Pairs** - Automatic closing of brackets, quotes, etc.
- **Comment Support** - Line comments with `#`
- **Indentation Rules** - Smart indentation for control structures
- **Code Folding** - Region-based code folding

## Supported Syntax

### Keywords
- Control flow: `if`, `else`, `elif`, `for`, `while`, `break`, `continue`, `return`, `match`, `case`
- Declarations: `fn`, `def`, `let`, `var`, `const`, `class`, `struct`, `import`, `from`, `as`, `export`
- Operators: `and`, `or`, `not`, `in`, `is`
- Constants: `true`, `false`, `none`, `null`

### Built-in Functions
- I/O: `print`, `input`, `read_file`, `write_file`
- Collections: `len`, `range`, `list`, `dict`, `append`, `push`, `pop`
- String operations: `str`, `join`, `split`, `replace`, `strip`, `upper`, `lower`
- Web: `html`, `router`, `store`, `component`

### String Interpolation
```poly
name = "World"
print(f"Hello, {name}!")
```

### Comments
```poly
# This is a line comment
```

## Installation

### From VSIX (Recommended)
```bash
cd poly-vscode
vsce package
code --install-extension poly-lang-0.1.0.vsix
```

### Manual Installation
1. Copy the `poly-vscode` folder to:
   - Windows: `%USERPROFILE%\.vscode\extensions\`
   - macOS/Linux: `~/.vscode/extensions/`
2. Restart VSCode/Kiro

## Development

### Building the Extension
```bash
npm install -g @vscode/vsce
cd poly-vscode
vsce package
```

### Testing
1. Open the `poly-vscode` folder in VSCode
2. Press F5 to launch Extension Development Host
3. Open a `.poly` file to test syntax highlighting

## License

MIT License - See LICENSE file for details
