# State Management & Routing in Poly

Poly includes built-in functions for client-side routing and state management. These generate JavaScript code that runs in the browser.

## Routing

### Basic Router

```poly
# Define your page content
let home_page = """
<h1>Home</h1>
<p>Welcome to my app!</p>
<a href="#/about">Go to About</a>
"""

let about_page = """
<h1>About</h1>
<p>This is the about page.</p>
<a href="#/">Back to Home</a>
"""

# Create router with routes dict
let routes = {
    "/": home_page,
    "/about": about_page
}

let router_js = router(routes, "<h1>404 - Page Not Found</h1>")
```

### How It Works

1. Routes use hash-based navigation (`#/path`)
2. The router listens for `hashchange` events
3. Content is rendered into `<div id="app"></div>`

### Full Example

```poly
let home = "<h1>Home</h1><a href='#/about'>About</a>"
let about = "<h1>About</h1><a href='#/'>Home</a>"

let routes = {"/": home, "/about": about}
let router_js = router(routes)

let body = """
<nav>
    <a href="#/">Home</a>
    <a href="#/about">About</a>
</nav>
<div id="app"></div>
"""

let page = html("My SPA", body, "", router_js)
write_file("web/index.html", page)
```

### Navigation

```javascript
// In your JavaScript:
navigate('/about')  // Programmatic navigation

// In HTML:
<a href="#/about">About</a>  // Link navigation
```

---

## State Management

### Creating a Store

```poly
# Define initial state
let initial_state = {
    "count": 0,
    "name": "Poly"
}

# Define actions (JavaScript code that modifies state)
let actions = {
    "increment": "this.state.count++",
    "decrement": "this.state.count--",
    "setName": "this.state.name = payload"
}

let store_js = store("Counter", initial_state, actions)
```

### Generated Store API

```javascript
// Get current state
counterStore.getState()  // { count: 0, name: "Poly" }

// Call actions
counterStore.increment()
counterStore.decrement()
counterStore.setName("New Name")

// Subscribe to changes
const unsubscribe = counterStore.subscribe((state) => {
    console.log('State changed:', state)
    // Update your UI here
})

// Unsubscribe when done
unsubscribe()
```

### Full Counter Example

```poly
let initial = {"count": 0}
let actions = {
    "increment": "this.state.count++",
    "decrement": "this.state.count--",
    "reset": "this.state.count = 0"
}

let store_js = store("App", initial, actions)

let app_js = store_js + """
// Update UI when state changes
appStore.subscribe((state) => {
    document.getElementById('count').textContent = state.count;
});

// Initial render
document.getElementById('count').textContent = appStore.getState().count;
"""

let body = """
<div style="text-align: center; padding: 2rem;">
    <h1>Counter: <span id="count">0</span></h1>
    <button onclick="appStore.decrement()">-</button>
    <button onclick="appStore.reset()">Reset</button>
    <button onclick="appStore.increment()">+</button>
</div>
"""

let css = """
button { 
    padding: 10px 20px; 
    margin: 5px; 
    font-size: 18px; 
    cursor: pointer;
}
"""

let page = html("Counter App", body, css, app_js)
write_file("web/index.html", page)
```

---

## Components

### Creating Reusable Components

```poly
# component(name, template, props_list)
let card_js = component("Card", """
<div class="card">
    <h2>${title}</h2>
    <p>${content}</p>
</div>
""", ["title", "content"])
```

### Using Components

```javascript
// In your JavaScript:
document.body.innerHTML = Card("Hello", "This is a card component")
```

### Full Example with Components

```poly
let button_js = component("Button", """
<button class="btn" onclick="${onclick}">${text}</button>
""", ["text", "onclick"])

let card_js = component("Card", """
<div class="card">
    <h3>${title}</h3>
    <p>${body}</p>
</div>
""", ["title", "body"])

let components = button_js + "\n" + card_js

let app_js = components + """
document.getElementById('app').innerHTML = 
    Card('Welcome', 'This is my app') +
    Button('Click Me', 'alert("Hello!")');
"""

let body = "<div id='app'></div>"

let css = """
.card { padding: 20px; border: 1px solid #333; margin: 10px; border-radius: 8px; }
.btn { padding: 10px 20px; background: #5dc1d2; border: none; border-radius: 4px; cursor: pointer; }
"""

let page = html("Components Demo", body, css, app_js)
write_file("web/index.html", page)
```

---

## Combining Router + Store

```poly
# State
let initial = {"user": "Guest", "loggedIn": false}
let actions = {
    "login": "this.state.user = payload; this.state.loggedIn = true",
    "logout": "this.state.user = 'Guest'; this.state.loggedIn = false"
}
let store_js = store("Auth", initial, actions)

# Pages
let home = "<h1>Home</h1><p>Welcome, <span id='user'></span>!</p>"
let login = """
<h1>Login</h1>
<button onclick="authStore.login('John'); navigate('/')">Login as John</button>
"""

# Router
let routes = {"/": home, "/login": login}
let router_js = router(routes)

# Combine and add UI updates
let app_js = store_js + router_js + """
authStore.subscribe((state) => {
    const el = document.getElementById('user');
    if (el) el.textContent = state.user;
});
"""

let body = """
<nav>
    <a href="#/">Home</a>
    <a href="#/login">Login</a>
    <button onclick="authStore.logout()">Logout</button>
</nav>
<div id="app"></div>
"""

let page = html("Auth Demo", body, "", app_js)
write_file("web/index.html", page)
```

---

## API Reference

| Function | Parameters | Returns |
|----------|------------|---------|
| `router(routes, not_found?)` | `routes`: dict of path→html, `not_found`: 404 html | JavaScript router code |
| `store(name, initial, actions?)` | `name`: store name, `initial`: state dict, `actions`: dict of name→js | JavaScript store code |
| `component(name, template, props?)` | `name`: function name, `template`: html with `${prop}`, `props`: list | JavaScript function |

## Tips

1. **Always include `<div id="app"></div>`** for the router to render into
2. **Use hash links** (`href="#/path"`) for navigation
3. **Subscribe to store changes** to update your UI
4. **Combine generated JS** by concatenating strings: `store_js + router_js`
