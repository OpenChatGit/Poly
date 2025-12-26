# Web Rendering in Poly

## Overview

Poly uses a **static HTML generation** approach. Your Poly code runs and outputs HTML files, which are then served by the dev server or displayed in a native WebView.

```
┌──────────────┐      ┌──────────────┐      ┌──────────────┐
│  main.poly   │  ──► │  Poly Engine │  ──► │  index.html  │
│  (your code) │      │  (executes)  │      │  (output)    │
└──────────────┘      └──────────────┘      └──────────────┘
                                                   │
                      ┌────────────────────────────┴────────────────────────────┐
                      │                                                         │
                      ▼                                                         ▼
               ┌──────────────┐                                          ┌──────────────┐
               │  Dev Server  │                                          │   WebView    │
               │  (browser)   │                                          │   (native)   │
               └──────────────┘                                          └──────────────┘
```

## The `html()` Function

```poly
let page = html(title, body, css, js)
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `title` | string | Page title in `<title>` tag |
| `body` | string | HTML content inside `<body>` |
| `css` | string | CSS styles inside `<style>` |
| `js` | string | JavaScript inside `<script>` |

**Returns:** Complete HTML document as string.

## Basic Example

```poly
let css = """
body { background: #000; color: #fff; }
"""

let body = "<h1>Hello World</h1>"

let page = html("My App", body, css, "")
write_file("web/index.html", page)
```

**Output (`web/index.html`):**
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>My App</title>
    <style>
body { background: #000; color: #fff; }
    </style>
</head>
<body>
<h1>Hello World</h1>
</body>
</html>
```

## Dynamic Content

Use f-strings and loops to generate HTML dynamically:

```poly
let users = ["Alice", "Bob", "Charlie"]
let items = ""

for user in users:
    items = items + f"<li>{user}</li>"

let body = f"<ul>{items}</ul>"
let page = html("Users", body, "", "")
write_file("web/index.html", page)
```

## Hot Reload

When running `poly dev`:

1. File watcher monitors `.poly` files
2. On change → Poly re-executes your code
3. New HTML is generated
4. Browser auto-refreshes

```bash
poly dev .
# Server at http://localhost:3000
# Edit main.poly → browser updates automatically
```

## Native Mode

Same HTML, but rendered in a native window:

```bash
poly run . --native
```

Uses WebView2 (Windows) / WebKit (macOS/Linux) - same as Tauri.
