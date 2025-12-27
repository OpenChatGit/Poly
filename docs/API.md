# Poly API Documentation

Complete reference for all Poly JavaScript APIs.

## Table of Contents

- [Window Control](#window-control)
- [Multi-Window](#multi-window)
- [System Tray](#system-tray)
- [Clipboard](#clipboard)
- [Notifications](#notifications)
- [Deep Links](#deep-links)
- [Dialogs](#dialogs)
- [File System](#file-system)
- [Auto-Updater](#auto-updater)
- [AI/LLM Integration](#aillm-integration)
- [IPC (Backend Functions)](#ipc-backend-functions)

---

## Window Control

Control the current window. Only available in native mode (`poly run --native`).

> **Note:** Poly does NOT inject a titlebar automatically. When using `decorations = false` (frameless mode), you must build your own titlebar using the `polyWindow` API.

### `polyWindow.minimize()`

Minimize the window.

```javascript
polyWindow.minimize();
```

### `polyWindow.maximize()`

Toggle maximize/restore.

```javascript
polyWindow.maximize();
```

### `polyWindow.close()`

Close the window.

```javascript
polyWindow.close();
```

### `polyWindow.drag()`

Start window drag. Call this on mousedown for custom titlebar.

```javascript
polyWindow.drag();
```

### `polyWindow.isFrameless()`

Check if the window is running in frameless mode (no native titlebar).

**Returns:** `Promise<boolean>`

```javascript
const frameless = await polyWindow.isFrameless();
if (frameless) {
  // Show custom titlebar
}
```

### Custom Titlebar Example

```html
<div class="titlebar" onmousedown="polyWindow.drag()">
  <span>My App</span>
  <button onclick="polyWindow.minimize()">─</button>
  <button onclick="polyWindow.maximize()">□</button>
  <button onclick="polyWindow.close()">✕</button>
</div>
```

---

## Multi-Window

Create and manage multiple windows.

### `poly.windows.create(options)`

Create a new window.

**Parameters:**
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `title` | string | "Poly Window" | Window title |
| `width` | number | 800 | Window width |
| `height` | number | 600 | Window height |
| `url` | string | - | URL to load |
| `html` | string | - | HTML content |
| `resizable` | boolean | true | Allow resize |
| `decorations` | boolean | true | Show native titlebar |

**Returns:** `{ id: number }` - Window handle

```javascript
// Create window with URL
const win = await poly.windows.create({
  title: 'Settings',
  width: 600,
  height: 400,
  url: 'http://localhost:3000/settings.html'
});

// Create frameless window with custom titlebar
const win2 = await poly.windows.create({
  title: 'About',
  width: 400,
  height: 300,
  decorations: false,
  html: `
    <!DOCTYPE html>
    <html>
    <head>
      <style>
        body { margin: 0; background: #1a1a1f; color: #fff; font-family: system-ui; }
        .titlebar { 
          height: 32px; background: #0f0f1a; display: flex;
          justify-content: space-between; align-items: center; padding: 0 12px;
        }
        .titlebar-btn { 
          background: none; border: none; color: #888; 
          width: 28px; height: 24px; cursor: pointer; 
        }
        .titlebar-btn:hover { background: rgba(255,255,255,0.1); color: #fff; }
        .titlebar-btn.close:hover { background: #e81123; }
        .content { padding: 20px; }
      </style>
    </head>
    <body>
      <div class="titlebar" onmousedown="polyWindow.drag()">
        <span>About</span>
        <div>
          <button class="titlebar-btn" onclick="polyWindow.minimize()">─</button>
          <button class="titlebar-btn" onclick="polyWindow.maximize()">□</button>
          <button class="titlebar-btn close" onclick="polyWindow.close()">✕</button>
        </div>
      </div>
      <div class="content">
        <h1>My App v1.0.0</h1>
        <p>Built with Poly</p>
      </div>
    </body>
    </html>
  `
});
```

### `poly.windows.close(id)`

Close a window by ID.

```javascript
await poly.windows.close(win.id);
```

### `poly.windows.closeAll()`

Close all created windows.

```javascript
await poly.windows.closeAll();
```

### `poly.windows.list()`

Get array of all window IDs.

```javascript
const ids = await poly.windows.list();
// [1, 2, 3]
```

### `poly.windows.count()`

Get number of open windows.

```javascript
const count = await poly.windows.count();
```

---

## System Tray

Display an icon in the system tray with a context menu. Configure in `poly.toml`.

### Configuration

```toml
[tray]
enabled = true
tooltip = "My App"
minimize_to_tray = false  # Minimize button hides to tray
close_to_tray = true      # Close button hides to tray instead of exiting

[[tray.menu]]
id = "show"
label = "Show Window"

[[tray.menu]]
id = "separator"

[[tray.menu]]
id = "settings"
label = "Settings"

[[tray.menu]]
id = "quit"
label = "Exit"
```

### Special Menu IDs

| ID | Behavior |
|----|----------|
| `show` | Shows and focuses the window |
| `quit` or `exit` | Exits the application |
| `separator` | Adds a separator line |

Any other ID will trigger a `polytray` event that you can handle in JavaScript.

### `poly.tray.onMenuClick(callback)`

Listen for custom tray menu clicks.

```javascript
poly.tray.onMenuClick((id) => {
  switch (id) {
    case 'settings':
      openSettings();
      break;
    case 'about':
      showAbout();
      break;
  }
});
```

### `poly.tray.isEnabled()`

Check if system tray is enabled.

**Returns:** `Promise<boolean>`

```javascript
if (await poly.tray.isEnabled()) {
  console.log('Tray is active');
}
```

### Tray Behavior

- When `close_to_tray = true`: Clicking the window close button hides the window to tray instead of exiting
- When `minimize_to_tray = true`: Clicking minimize hides to tray instead of minimizing
- Clicking the tray icon shows and focuses the window
- The tray icon uses the app icon from `assets/icon.png` if available

---

## Clipboard

Read and write system clipboard.

### `poly.clipboard.read()`

Read text from clipboard.

```javascript
const text = await poly.clipboard.read();
console.log(text);
```

### `poly.clipboard.write(text)`

Write text to clipboard.

```javascript
await poly.clipboard.write('Hello World');
```

### `poly.clipboard.clear()`

Clear clipboard contents.

```javascript
await poly.clipboard.clear();
```

---

## Notifications

Display native OS notifications.

### `poly.notification.show(title, body, icon?)`

Show a notification.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `title` | string | Notification title |
| `body` | string | Notification message |
| `icon` | string | (Optional) Path to icon |

```javascript
await poly.notification.show('Download Complete', 'Your file has been downloaded');

// With icon
await poly.notification.show('New Message', 'You have a new message', './assets/icon.png');
```

### `poly.notification.showWithTimeout(title, body, timeout)`

Show a notification that auto-dismisses.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `title` | string | Notification title |
| `body` | string | Notification message |
| `timeout` | number | Auto-dismiss time in milliseconds |

```javascript
// Show notification for 5 seconds
await poly.notification.showWithTimeout('Saved', 'Your changes have been saved', 5000);
```

---

## Deep Links

Register and handle custom URL protocols (e.g., `myapp://action`).

### `poly.deeplink.register(protocol, appName)`

Register a custom URL protocol in the system registry.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `protocol` | string | Protocol name (e.g., 'myapp' for myapp://) |
| `appName` | string | Display name for the protocol |

```javascript
// Register myapp:// protocol
await poly.deeplink.register('myapp', 'My Application');

// Now myapp://anything will open your app
```

### `poly.deeplink.unregister(protocol)`

Remove a custom URL protocol from the system registry.

```javascript
await poly.deeplink.unregister('myapp');
```

### `poly.deeplink.isRegistered(protocol)`

Check if a protocol is registered.

**Returns:** `boolean`

```javascript
const registered = await poly.deeplink.isRegistered('myapp');
if (!registered) {
  await poly.deeplink.register('myapp', 'My App');
}
```

### `poly.deeplink.get()`

Get the deep link URL that launched the app.

**Returns:** `string | null` - The full URL or null if not launched via deep link

```javascript
const link = await poly.deeplink.get();
if (link) {
  // Parse and handle the deep link
  // e.g., myapp://open/document/123
  const url = new URL(link);
  console.log(url.pathname); // /open/document/123
}
```

### `poly.deeplink.has()`

Check if the app was launched via a deep link.

**Returns:** `boolean`

```javascript
if (await poly.deeplink.has()) {
  const link = await poly.deeplink.get();
  handleDeepLink(link);
}
```

### Deep Link Example

```javascript
// On app startup
async function init() {
  // Register protocol if not already
  if (!await poly.deeplink.isRegistered('myapp')) {
    await poly.deeplink.register('myapp', 'My App');
  }
  
  // Handle deep link if launched with one
  if (await poly.deeplink.has()) {
    const link = await poly.deeplink.get();
    // myapp://action/param1/param2
    const url = new URL(link);
    
    switch (url.pathname) {
      case '/open':
        openDocument(url.searchParams.get('id'));
        break;
      case '/settings':
        showSettings();
        break;
    }
  }
}
```

---

## Dialogs

Native file dialogs and custom in-app dialogs.

### `poly.dialog.open(options?)`

Open file picker dialog.

**Parameters:**
| Option | Type | Description |
|--------|------|-------------|
| `title` | string | Dialog title |
| `filters` | array | File type filters |

**Returns:** `string | null` - Selected file path or null if cancelled

```javascript
const file = await poly.dialog.open({
  title: 'Select Image',
  filters: [['Images', ['png', 'jpg', 'gif']]]
});

if (file) {
  console.log('Selected:', file);
}
```

### `poly.dialog.openMultiple(options?)`

Open file picker for multiple files.

**Returns:** `string[]` - Array of selected file paths

```javascript
const files = await poly.dialog.openMultiple({
  title: 'Select Files'
});
```

### `poly.dialog.save(options?)`

Open save file dialog.

**Parameters:**
| Option | Type | Description |
|--------|------|-------------|
| `title` | string | Dialog title |
| `defaultName` | string | Default filename |
| `filters` | array | File type filters |

**Returns:** `string | null` - Selected save path or null

```javascript
const path = await poly.dialog.save({
  title: 'Save Document',
  defaultName: 'document.txt'
});
```

### `poly.dialog.folder(options?)`

Open folder picker dialog.

**Returns:** `string | null` - Selected folder path or null

```javascript
const folder = await poly.dialog.folder({
  title: 'Select Output Folder'
});
```

### `poly.dialog.message(title, message, level?)`

Show a message dialog.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `title` | string | Dialog title |
| `message` | string | Message content |
| `level` | string | 'info', 'warning', or 'error' |

```javascript
await poly.dialog.message('Success', 'File saved successfully!', 'info');
await poly.dialog.message('Warning', 'This action cannot be undone', 'warning');
await poly.dialog.message('Error', 'Failed to save file', 'error');
```

### `poly.dialog.confirm(title, message)`

Show a confirmation dialog.

**Returns:** `boolean` - true if confirmed, false if cancelled

```javascript
const confirmed = await poly.dialog.confirm(
  'Delete File?',
  'Are you sure you want to delete this file?'
);

if (confirmed) {
  // Delete the file
}
```

### `poly.dialog.custom(options)`

Show a fully custom dialog.

**Parameters:**
| Option | Type | Description |
|--------|------|-------------|
| `type` | string | 'info', 'warning', 'error', 'confirm' |
| `title` | string | Dialog title |
| `message` | string | Message content |
| `buttons` | array | Button definitions |

**Button definition:**
| Property | Type | Description |
|----------|------|-------------|
| `text` | string | Button label |
| `value` | any | Return value when clicked |
| `primary` | boolean | Primary button styling |

**Returns:** Button value or null if dismissed

```javascript
const result = await poly.dialog.custom({
  type: 'warning',
  title: 'Unsaved Changes',
  message: 'Do you want to save before closing?',
  buttons: [
    { text: 'Discard', value: 'discard' },
    { text: 'Cancel', value: 'cancel' },
    { text: 'Save', value: 'save', primary: true }
  ]
});

switch (result) {
  case 'save': await saveFile(); break;
  case 'discard': closeWithoutSaving(); break;
  case 'cancel': /* do nothing */ break;
}
```

---

## File System

Read and write files.

### `poly.fs.read(path)`

Read file contents as string.

```javascript
const content = await poly.fs.read('config.json');
const config = JSON.parse(content);
```

### `poly.fs.write(path, content)`

Write string content to file.

```javascript
await poly.fs.write('config.json', JSON.stringify(config, null, 2));
```

### `poly.fs.exists(path)`

Check if file or directory exists.

**Returns:** `boolean`

```javascript
if (await poly.fs.exists('config.json')) {
  // Load config
}
```

### `poly.fs.readDir(path)`

List directory contents.

**Returns:** `string[]` - Array of filenames

```javascript
const files = await poly.fs.readDir('./documents');
```

---

## Auto-Updater

Check for and install updates from GitHub Releases.

### `poly.updater.checkGithub(repo, currentVersion)`

Check GitHub for updates.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `repo` | string | GitHub repo (e.g., 'user/repo') |
| `currentVersion` | string | Current app version |

**Returns:**
```javascript
{
  update_available: boolean,
  latest_version: string,
  current_version: string,
  download_url: string | null,
  release_notes: string | null
}
```

```javascript
const info = await poly.updater.checkGithub('myuser/myapp', '1.0.0');

if (info.update_available) {
  console.log('New version:', info.latest_version);
}
```

### `poly.updater.download(url)`

Download update file.

**Returns:** `string` - Path to downloaded file

```javascript
const path = await poly.updater.download(info.download_url);
```

### `poly.updater.install(path)`

Install downloaded update.

```javascript
await poly.updater.install(path);
```

### `poly.updater.checkAndPrompt(options)`

Convenience method: check, prompt user, download, and install.

```javascript
await poly.updater.checkAndPrompt({
  repo: 'myuser/myapp',
  currentVersion: '1.0.0'
});
```

---

## AI/LLM Integration

Built-in support for AI chat APIs.

### `poly.ai.ollama(model, messages)`

Chat with local Ollama.

```javascript
const response = await poly.ai.ollama('llama3', [
  { role: 'user', content: 'Hello!' }
]);

console.log(response.content);
```

### `poly.ai.openai(model, messages, apiKey)`

Chat with OpenAI.

```javascript
const response = await poly.ai.openai('gpt-4', [
  { role: 'system', content: 'You are helpful.' },
  { role: 'user', content: 'Hello!' }
], 'sk-your-api-key');
```

### `poly.ai.anthropic(model, messages, apiKey, options?)`

Chat with Anthropic Claude.

```javascript
const response = await poly.ai.anthropic(
  'claude-3-5-sonnet-20241022',
  [{ role: 'user', content: 'Explain quantum computing' }],
  'your-api-key',
  { enableThinking: true, thinkingBudget: 10000 }
);

console.log(response.thinking); // Reasoning process
console.log(response.content);  // Final answer
```

### `poly.ai.custom(baseUrl, model, messages)`

Chat with OpenAI-compatible APIs (LM Studio, LocalAI, etc).

```javascript
const response = await poly.ai.custom(
  'http://localhost:1234/v1',
  'local-model',
  [{ role: 'user', content: 'Hello!' }]
);
```

### `poly.ai.checkOllama()`

Check if Ollama is running.

**Returns:** `boolean`

```javascript
if (await poly.ai.checkOllama()) {
  // Ollama is available
}
```

### `poly.ai.listModels()`

List available Ollama models.

**Returns:** `string[]`

```javascript
const models = await poly.ai.listModels();
// ['llama3', 'codellama', 'mistral']
```

---

## IPC (Backend Functions)

Call functions defined in `main.poly`.

### `poly.invoke(functionName, args?)`

Call a backend function.

```javascript
// In main.poly:
// fn greet(name) { return "Hello, " + name }

const result = await poly.invoke('greet', { name: 'World' });
// "Hello, World"
```

---

## Custom Titlebar Example

Complete example of a custom frameless titlebar. Set `decorations = false` in `poly.toml` to use this.

```html
<!DOCTYPE html>
<html>
<head>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body { background: #1a1a1f; color: #fff; font-family: system-ui; }
    
    .titlebar {
      height: 32px;
      background: #0f0f1a;
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 0 12px;
      user-select: none;
    }
    
    .titlebar-title {
      font-size: 12px;
      color: #888;
    }
    
    .titlebar-buttons {
      display: flex;
      gap: 4px;
    }
    
    .titlebar-btn {
      width: 28px;
      height: 24px;
      border: none;
      background: transparent;
      color: #888;
      cursor: pointer;
      font-size: 12px;
      border-radius: 4px;
    }
    
    .titlebar-btn:hover {
      background: rgba(255,255,255,0.1);
      color: #fff;
    }
    
    .titlebar-btn.close:hover {
      background: #e81123;
    }
    
    .content {
      padding: 20px;
    }
  </style>
</head>
<body>
  <div class="titlebar" onmousedown="polyWindow.drag()">
    <div class="titlebar-title">My App</div>
    <div class="titlebar-buttons">
      <button class="titlebar-btn" onclick="polyWindow.minimize()">─</button>
      <button class="titlebar-btn" onclick="polyWindow.maximize()">□</button>
      <button class="titlebar-btn close" onclick="polyWindow.close()">✕</button>
    </div>
  </div>
  
  <div class="content">
    <h1>Welcome</h1>
    <p>Your app content here</p>
  </div>
</body>
</html>
```
