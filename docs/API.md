# Poly API Documentation

Complete reference for all Poly JavaScript APIs.

**Version:** 0.3.2

> **Note:** This documentation is comprehensive and covers all available APIs in detail. Take your time - it's completely normal if you don't understand everything at first. Start with the basics and explore more advanced features as you need them.

---

## Table of Contents

- [Overview](#overview)
- [Window Control](#window-control)
- [Multi-Window](#multi-window)
- [Shell API](#shell-api)
- [App API](#app-api)
- [OS API](#os-api)
- [Clipboard](#clipboard)
- [Notifications](#notifications)
- [Dialogs](#dialogs)
- [File System](#file-system)
- [Deep Links](#deep-links)
- [System Tray](#system-tray)
- [Auto-Updater](#auto-updater)
- [AI/LLM Integration](#aillm-integration)
- [IPC (Backend Functions)](#ipc-backend-functions)
- [Configuration (poly.toml)](#configuration-polytoml)
- [Building Apps](#building-apps)
- [Code Signing](#code-signing)

---

## Overview

Poly provides all APIs through the global `poly` object. All API methods are asynchronous and return Promises.

```javascript
// Example: Combining multiple APIs
const platform = await poly.os.platform();
const home = await poly.os.homedir();
await poly.notification.show('System Info', `${platform} - ${home}`);
```

### Availability

| API | Dev Server | Native Mode |
|-----|------------|-------------|
| `poly.dialog.*` | ✅ | ✅ |
| `poly.fs.*` | ✅ | ✅ |
| `poly.clipboard.*` | ✅ | ✅ |
| `poly.notification.*` | ✅ | ✅ |
| `poly.shell.*` | ✅ | ✅ |
| `poly.app.*` | ✅ | ✅ |
| `poly.os.*` | ✅ | ✅ |
| `poly.windows.*` | ✅ | ✅ |
| `poly.deeplink.*` | ✅ | ✅ |
| `poly.updater.*` | ✅ | ✅ |
| `poly.tray.*` | ❌ | ✅ |
| `poly.window.*` (setTitle, center, etc.) | ❌ | ✅ |
| `polyWindow.*` (minimize, maximize, etc.) | ❌ | ✅ |

---

## Window Control

Control the current window. Only available in native mode (`poly run --native`).

> **Important:** Poly does NOT inject a titlebar automatically. When using `decorations = false` (frameless), you must build your own titlebar using the `polyWindow` API.

### `polyWindow.minimize()`

Minimizes the window.

```javascript
polyWindow.minimize();
```

### `polyWindow.maximize()`

Toggles between maximized and normal state.

```javascript
polyWindow.maximize();
```

### `polyWindow.close()`

Closes the window.

```javascript
polyWindow.close();
```

### `polyWindow.drag()`

Starts window dragging. Call this on `mousedown` for a custom titlebar.

```javascript
// In your custom titlebar
<div class="titlebar" onmousedown="polyWindow.drag()">
  <span>My App</span>
</div>
```

### `polyWindow.isFrameless()`

Checks if the window is running in frameless mode.

**Returns:** `Promise<boolean>`

```javascript
const frameless = await polyWindow.isFrameless();
if (frameless) {
  // Show custom titlebar
  document.getElementById('custom-titlebar').style.display = 'flex';
}
```

### Extended Window API

These APIs require Native Mode and are not available in the Dev Server.

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `poly.window.setTitle(title)` | `string` | `Promise<void>` | Sets the window title |
| `poly.window.getTitle()` | - | `Promise<string>` | Gets the window title |
| `poly.window.center()` | - | `Promise<void>` | Centers the window |
| `poly.window.setSize(w, h)` | `number, number` | `Promise<void>` | Sets window size |
| `poly.window.getSize()` | - | `Promise<{width, height}>` | Gets window size |
| `poly.window.setPosition(x, y)` | `number, number` | `Promise<void>` | Sets window position |
| `poly.window.getPosition()` | - | `Promise<{x, y}>` | Gets window position |
| `poly.window.setMinSize(w, h)` | `number, number` | `Promise<void>` | Sets minimum size |
| `poly.window.setMaxSize(w, h)` | `number, number` | `Promise<void>` | Sets maximum size |
| `poly.window.setAlwaysOnTop(v)` | `boolean` | `Promise<void>` | Window always on top |
| `poly.window.setFullscreen(v)` | `boolean` | `Promise<void>` | Fullscreen mode on/off |
| `poly.window.isFullscreen()` | - | `Promise<boolean>` | Checks fullscreen mode |
| `poly.window.isMaximized()` | - | `Promise<boolean>` | Checks if maximized |
| `poly.window.isMinimized()` | - | `Promise<boolean>` | Checks if minimized |

```javascript
// Example: Configure window
await poly.window.setTitle('My App - Document.txt');
await poly.window.setSize(1200, 800);
await poly.window.center();
await poly.window.setMinSize(800, 600);
```

### Custom Titlebar Example

Complete example of a custom titlebar for frameless windows:

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
      -webkit-app-region: drag;
    }
    
    .titlebar-title { font-size: 12px; color: #888; }
    .titlebar-buttons { display: flex; gap: 4px; -webkit-app-region: no-drag; }
    
    .titlebar-btn {
      width: 28px; height: 24px;
      border: none; background: transparent;
      color: #888; cursor: pointer;
      font-size: 12px; border-radius: 4px;
    }
    .titlebar-btn:hover { background: rgba(255,255,255,0.1); color: #fff; }
    .titlebar-btn.close:hover { background: #e81123; }
    
    .content { padding: 20px; }
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

---

## Multi-Window

Create and manage multiple windows.

### `poly.windows.create(options)`

Creates a new window.

**Parameters:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `title` | string | "Poly Window" | Window title |
| `width` | number | 800 | Window width in pixels |
| `height` | number | 600 | Window height in pixels |
| `url` | string | - | URL to load |
| `html` | string | - | HTML content directly |
| `resizable` | boolean | true | Allow resizing |
| `decorations` | boolean | true | Show native titlebar |

**Returns:** `Promise<{ id: number }>` - Window handle

```javascript
// Create window with URL
const win = await poly.windows.create({
  title: 'Settings',
  width: 600,
  height: 400,
  url: 'http://localhost:3000/settings.html'
});

// Frameless window with HTML content
const aboutWin = await poly.windows.create({
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
        .content { padding: 20px; text-align: center; }
      </style>
    </head>
    <body>
      <div class="titlebar" onmousedown="polyWindow.drag()">
        <span>About</span>
        <button onclick="polyWindow.close()" style="background:none;border:none;color:#888;cursor:pointer">✕</button>
      </div>
      <div class="content">
        <h2>My App</h2>
        <p>Version 1.0.0</p>
        <p>Built with Poly</p>
      </div>
    </body>
    </html>
  `
});
```

### `poly.windows.close(id)`

Closes a window by its ID.

```javascript
await poly.windows.close(win.id);
```

### `poly.windows.closeAll()`

Closes all created windows.

```javascript
await poly.windows.closeAll();
```

### `poly.windows.list()`

Returns an array of all window IDs.

**Returns:** `Promise<number[]>`

```javascript
const ids = await poly.windows.list();
console.log('Open windows:', ids); // [1, 2, 3]
```

### `poly.windows.count()`

Returns the number of open windows.

**Returns:** `Promise<number>`

```javascript
const count = await poly.windows.count();
console.log(`${count} windows open`);
```

### Extended Window Control

Control individual windows after creation.

#### `poly.windows.minimize(id)`

Minimizes a window.

```javascript
await poly.windows.minimize(win.id);
```

#### `poly.windows.maximize(id)`

Toggles maximize state.

```javascript
await poly.windows.maximize(win.id);
```

#### `poly.windows.restore(id)`

Restores a window from minimized/maximized state.

```javascript
await poly.windows.restore(win.id);
```

#### `poly.windows.show(id)` / `poly.windows.hide(id)`

Shows or hides a window.

```javascript
await poly.windows.hide(win.id);
// Later...
await poly.windows.show(win.id);
```

#### `poly.windows.focus(id)`

Brings a window to front and focuses it.

```javascript
await poly.windows.focus(win.id);
```

#### `poly.windows.setTitle(id, title)`

Changes the window title.

```javascript
await poly.windows.setTitle(win.id, 'New Title');
```

#### `poly.windows.setSize(id, width, height)`

Resizes a window.

```javascript
await poly.windows.setSize(win.id, 800, 600);
```

#### `poly.windows.setPosition(id, x, y)`

Moves a window to specific coordinates.

```javascript
await poly.windows.setPosition(win.id, 100, 100);
```

#### `poly.windows.setAlwaysOnTop(id, value)`

Sets whether window stays on top.

```javascript
await poly.windows.setAlwaysOnTop(win.id, true);
```

#### `poly.windows.setFullscreen(id, value)`

Toggles fullscreen mode.

```javascript
await poly.windows.setFullscreen(win.id, true);
```

#### `poly.windows.navigate(id, url)`

Navigates the window's WebView to a URL.

```javascript
await poly.windows.navigate(win.id, 'https://example.com');
```

#### `poly.windows.eval(id, script)`

Executes JavaScript in the window's WebView.

```javascript
await poly.windows.eval(win.id, 'document.body.style.background = "red"');
```

#### `poly.windows.getState(id)`

Gets the current state of a window.

**Returns:**
```javascript
{
  id: number,
  title: string,
  width: number,
  height: number,
  x: number,
  y: number,
  is_visible: boolean,
  is_minimized: boolean,
  is_maximized: boolean,
  is_fullscreen: boolean,
  is_focused: boolean
}
```

```javascript
const state = await poly.windows.getState(win.id);
console.log(`Window at (${state.x}, ${state.y}), size ${state.width}x${state.height}`);
```

### Complete Window Options

All options for `poly.windows.create()`:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `title` | string | "Poly Window" | Window title |
| `width` | number | 800 | Window width |
| `height` | number | 600 | Window height |
| `min_width` | number | - | Minimum width |
| `min_height` | number | - | Minimum height |
| `max_width` | number | - | Maximum width |
| `max_height` | number | - | Maximum height |
| `url` | string | - | URL to load |
| `html` | string | - | HTML content |
| `resizable` | boolean | true | Allow resizing |
| `decorations` | boolean | false | Show native titlebar |
| `always_on_top` | boolean | false | Stay on top |
| `transparent` | boolean | false | Transparent background |
| `fullscreen` | boolean | false | Start fullscreen |
| `maximized` | boolean | false | Start maximized |
| `visible` | boolean | true | Initially visible |
| `focused` | boolean | true | Initially focused |
| `icon_path` | string | - | Window icon (PNG) |
| `background_color` | array | [26,26,26,255] | RGBA background |
| `x` | number | center | Initial X position |
| `y` | number | center | Initial Y position |
| `devtools` | boolean | false | Enable DevTools |

```javascript
// Full example with all options
const win = await poly.windows.create({
  title: 'Advanced Window',
  width: 1200,
  height: 800,
  min_width: 400,
  min_height: 300,
  url: 'http://localhost:3000/app.html',
  resizable: true,
  decorations: false,
  always_on_top: false,
  transparent: false,
  icon_path: 'assets/icon.png',
  background_color: [26, 26, 31, 255],
  x: 100,
  y: 100,
  devtools: true
});
```

### Window Manipulation Guide

After creating a window, you can fully control and modify it using the window ID.

#### Changing Window Properties

```javascript
// Create a window
const win = await poly.windows.create({
  title: 'My Window',
  width: 600,
  height: 400,
  html: '<h1>Hello</h1>'
});

// Change title dynamically
await poly.windows.setTitle(win.id, 'New Title - Updated!');

// Resize the window
await poly.windows.setSize(win.id, 800, 600);

// Move the window
await poly.windows.setPosition(win.id, 100, 100);

// Make it always on top
await poly.windows.setAlwaysOnTop(win.id, true);
```

#### Modifying Window Content

You can change what's displayed in a window after creation:

```javascript
// Navigate to a URL
await poly.windows.navigate(win.id, 'https://example.com');

// Or execute JavaScript to modify the DOM
await poly.windows.eval(win.id, `
  document.body.innerHTML = '<h1>Content Updated!</h1>';
  document.body.style.background = '#1a1a2e';
  document.body.style.color = '#fff';
`);
```

#### Dynamic UI Updates via eval()

The `eval()` method is powerful for updating window content dynamically:

```javascript
// Update a status display
async function updateStatus(windowId, status, color) {
  await poly.windows.eval(windowId, `
    document.getElementById('status').textContent = '${status}';
    document.getElementById('status').style.color = '${color}';
  `);
}

// Show a notification in the window
async function showNotification(windowId, message) {
  await poly.windows.eval(windowId, `
    const notification = document.createElement('div');
    notification.textContent = '${message}';
    notification.style.cssText = 'position:fixed;top:20px;right:20px;background:#4ade80;color:#000;padding:12px 20px;border-radius:8px;';
    document.body.appendChild(notification);
    setTimeout(() => notification.remove(), 3000);
  `);
}

// Usage
await updateStatus(win.id, 'Connected', '#4ade80');
await showNotification(win.id, 'Settings saved!');
```

#### Window State Management

```javascript
// Get current state
const state = await poly.windows.getState(win.id);
console.log(`Position: (${state.x}, ${state.y})`);
console.log(`Size: ${state.width}x${state.height}`);
console.log(`Maximized: ${state.is_maximized}`);

// Control visibility
await poly.windows.hide(win.id);    // Hide window
await poly.windows.show(win.id);    // Show and focus
await poly.windows.focus(win.id);   // Bring to front

// Window state
await poly.windows.minimize(win.id);
await poly.windows.maximize(win.id);
await poly.windows.restore(win.id);
```

#### Complete Example: Settings Window

```javascript
// Create a settings window
let settingsWindow = null;

async function openSettings() {
  // If already open, just focus it
  if (settingsWindow) {
    try {
      await poly.windows.focus(settingsWindow);
      return;
    } catch (e) {
      // Window was closed, create new one
      settingsWindow = null;
    }
  }
  
  const win = await poly.windows.create({
    title: 'Settings',
    width: 500,
    height: 400,
    decorations: false,
    html: `
      <!DOCTYPE html>
      <html>
      <head>
        <style>
          * { margin: 0; padding: 0; box-sizing: border-box; }
          body { background: #1a1a2e; color: #fff; font-family: system-ui; }
          .titlebar {
            height: 36px; background: #16213e;
            display: flex; justify-content: space-between;
            align-items: center; padding: 0 12px;
          }
          .titlebar button {
            background: none; border: none;
            color: #888; cursor: pointer; padding: 4px 8px;
          }
          .titlebar button:hover { color: #fff; }
          .content { padding: 24px; }
          h2 { margin-bottom: 20px; }
          .setting {
            display: flex; justify-content: space-between;
            align-items: center; padding: 12px 0;
            border-bottom: 1px solid #2a2a4e;
          }
          .toggle {
            width: 48px; height: 24px;
            background: #333; border-radius: 12px;
            cursor: pointer; position: relative;
          }
          .toggle.active { background: #4ade80; }
          .toggle::after {
            content: ''; position: absolute;
            width: 20px; height: 20px;
            background: #fff; border-radius: 50%;
            top: 2px; left: 2px; transition: 0.2s;
          }
          .toggle.active::after { left: 26px; }
        </style>
      </head>
      <body>
        <div class="titlebar" onmousedown="polyWindow.drag()">
          <span>⚙️ Settings</span>
          <button onclick="polyWindow.close()">✕</button>
        </div>
        <div class="content">
          <h2>Preferences</h2>
          <div class="setting">
            <span>Dark Mode</span>
            <div class="toggle active" onclick="this.classList.toggle('active')"></div>
          </div>
          <div class="setting">
            <span>Notifications</span>
            <div class="toggle" onclick="this.classList.toggle('active')"></div>
          </div>
          <div class="setting">
            <span>Auto-Update</span>
            <div class="toggle active" onclick="this.classList.toggle('active')"></div>
          </div>
        </div>
      </body>
      </html>
    `
  });
  
  settingsWindow = win.id;
}

// Update settings window title when changes are made
async function markSettingsChanged() {
  if (settingsWindow) {
    await poly.windows.setTitle(settingsWindow, 'Settings *');
  }
}
```

---

## Shell API

Open URLs, files, and folders with the system default application.

### `poly.shell.open(url)`

Opens a URL in the default browser.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `url` | string | The URL to open |

**Returns:** `Promise<boolean>`

```javascript
// Open website
await poly.shell.open('https://github.com/OpenChatGit/Poly');

// Open email
await poly.shell.open('mailto:support@example.com');

// Tel link
await poly.shell.open('tel:+1234567890');
```

### `poly.shell.openPath(path)`

Opens a file path or folder with the default application.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `path` | string | Path to file or folder |

**Returns:** `Promise<boolean>`

```javascript
// Open folder in Explorer/Finder
const home = await poly.os.homedir();
await poly.shell.openPath(home);

// Open file with default application
await poly.shell.openPath('C:\\Users\\User\\Documents\\report.pdf');

// Open downloads folder
const downloads = await poly.app.getPath('downloads');
await poly.shell.openPath(downloads);
```

### `poly.shell.openWith(path, app)`

Opens a file with a specific application.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `path` | string | Path to file |
| `app` | string | Path to application or application name |

**Returns:** `Promise<boolean>`

```javascript
// Open file with specific editor
await poly.shell.openWith('document.txt', 'notepad.exe');

// Open image with specific program
await poly.shell.openWith('image.png', 'C:\\Program Files\\GIMP\\gimp.exe');
```

---

## App API

Information about the application and system paths.

### `poly.app.getVersion()`

Returns the Poly version.

**Returns:** `Promise<string>`

```javascript
const version = await poly.app.getVersion();
console.log('Poly Version:', version); // "0.3.0"
```

### `poly.app.getName()`

Returns the application name (from the executable name).

**Returns:** `Promise<string>`

```javascript
const name = await poly.app.getName();
console.log('App Name:', name); // "demo-app" or "poly"
```

### `poly.app.getPath(name)`

Returns system paths.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | Path name (see table) |

**Available Paths:**

| Name | Windows | macOS | Linux |
|------|---------|-------|-------|
| `exe` | Path to .exe | Path to binary | Path to binary |
| `home` | `C:\Users\User` | `/Users/user` | `/home/user` |
| `data` / `appData` | `%APPDATA%` | `~/Library/Application Support` | `~/.local/share` |
| `config` | `%APPDATA%` | `~/Library/Preferences` | `~/.config` |
| `cache` | `%LOCALAPPDATA%\Temp` | `~/Library/Caches` | `~/.cache` |
| `temp` | `%TEMP%` | `/tmp` | `/tmp` |
| `desktop` | `%USERPROFILE%\Desktop` | `~/Desktop` | `~/Desktop` |
| `documents` | `%USERPROFILE%\Documents` | `~/Documents` | `~/Documents` |
| `downloads` | `%USERPROFILE%\Downloads` | `~/Downloads` | `~/Downloads` |

**Returns:** `Promise<string | null>`

```javascript
// Get all important paths
const paths = {
  home: await poly.app.getPath('home'),
  data: await poly.app.getPath('data'),
  config: await poly.app.getPath('config'),
  cache: await poly.app.getPath('cache'),
  desktop: await poly.app.getPath('desktop'),
  documents: await poly.app.getPath('documents'),
  downloads: await poly.app.getPath('downloads'),
  temp: await poly.app.getPath('temp')
};

console.log(paths);
```

### `poly.app.exit(code?)`

Exits the application.

**Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `code` | number | 0 | Exit code (0 = success) |

```javascript
// Normal exit
await poly.app.exit();

// Exit with error code
await poly.app.exit(1);
```

### `poly.app.relaunch()`

Restarts the application.

```javascript
// Restart after update
await poly.app.relaunch();
```

---

## OS API

Information about the operating system.

### `poly.os.platform()`

Returns the operating system.

**Returns:** `Promise<string>` - `"windows"`, `"macos"`, `"linux"`, or `"unknown"`

```javascript
const platform = await poly.os.platform();

switch (platform) {
  case 'windows':
    console.log('Windows-specific logic');
    break;
  case 'macos':
    console.log('macOS-specific logic');
    break;
  case 'linux':
    console.log('Linux-specific logic');
    break;
}
```

### `poly.os.arch()`

Returns the CPU architecture.

**Returns:** `Promise<string>` - `"x64"`, `"arm64"`, `"x86"`, or `"unknown"`

```javascript
const arch = await poly.os.arch();
console.log('Architecture:', arch); // "x64" or "arm64"
```

### `poly.os.version()`

Returns the OS version.

**Returns:** `Promise<string>`

```javascript
const version = await poly.os.version();
console.log('OS Version:', version);
```

### `poly.os.hostname()`

Returns the computer name.

**Returns:** `Promise<string>`

```javascript
const hostname = await poly.os.hostname();
console.log('Computer:', hostname); // "DESKTOP-ABC123"
```

### `poly.os.homedir()`

Returns the home directory.

**Returns:** `Promise<string>`

```javascript
const home = await poly.os.homedir();
console.log('Home:', home); // "C:\\Users\\User" or "/home/user"
```

### `poly.os.tempdir()`

Returns the temporary directory.

**Returns:** `Promise<string>`

```javascript
const temp = await poly.os.tempdir();
console.log('Temp:', temp);
```

### Complete Example

```javascript
async function getSystemInfo() {
  const info = {
    platform: await poly.os.platform(),
    arch: await poly.os.arch(),
    hostname: await poly.os.hostname(),
    homedir: await poly.os.homedir(),
    tempdir: await poly.os.tempdir()
  };
  
  console.log('System Information:');
  console.log(JSON.stringify(info, null, 2));
  
  return info;
}
```

---

## Clipboard

Read and write the system clipboard.

### `poly.clipboard.read()`

Reads text from the clipboard.

**Returns:** `Promise<string>`

```javascript
const text = await poly.clipboard.read();
console.log('Clipboard:', text);
```

### `poly.clipboard.write(text)`

Writes text to the clipboard.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `text` | string | The text to copy |

**Returns:** `Promise<boolean>`

```javascript
await poly.clipboard.write('Hello World');
console.log('Text copied!');
```

### `poly.clipboard.clear()`

Clears the clipboard.

**Returns:** `Promise<boolean>`

```javascript
await poly.clipboard.clear();
```

### Practical Example

```javascript
// Copy button implementation
async function copyToClipboard(text) {
  try {
    await poly.clipboard.write(text);
    await poly.notification.show('Copied!', 'Text has been copied to clipboard.');
  } catch (e) {
    await poly.dialog.message('Error', 'Copy failed: ' + e.message, 'error');
  }
}

// Paste button implementation
async function pasteFromClipboard() {
  const text = await poly.clipboard.read();
  document.getElementById('input').value = text;
}
```

---

## Notifications

Display native operating system notifications.

### `poly.notification.show(title, body, icon?)`

Shows a notification.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `title` | string | Notification title |
| `body` | string | Message text |
| `icon` | string | (Optional) Path to icon |

**Returns:** `Promise<boolean>`

```javascript
// Simple notification
await poly.notification.show('Download complete', 'Your file has been downloaded');

// With icon
await poly.notification.show(
  'New message',
  'You have received a new message',
  './assets/icon.png'
);
```

### `poly.notification.showWithTimeout(title, body, timeout)`

Shows a notification that automatically dismisses.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `title` | string | Notification title |
| `body` | string | Message text |
| `timeout` | number | Auto-dismiss time in milliseconds |

**Returns:** `Promise<boolean>`

```javascript
// Show notification for 5 seconds
await poly.notification.showWithTimeout('Saved', 'Changes have been saved', 5000);

// Short confirmation (3 seconds)
await poly.notification.showWithTimeout('✓', 'Action successful', 3000);
```

---

## Dialogs

Native file dialogs and custom in-app dialogs.

### Native File Dialogs

#### `poly.dialog.open(options?)`

Opens a file picker dialog.

**Parameters:**
| Option | Type | Description |
|--------|------|-------------|
| `title` | string | Dialog title |
| `filters` | array | File type filters |

**Returns:** `Promise<string | null>` - Selected file path or null if cancelled

```javascript
// Simple file dialog
const file = await poly.dialog.open();

// With title and filters
const image = await poly.dialog.open({
  title: 'Select Image',
  filters: [['Images', ['png', 'jpg', 'gif']]]
});

if (image) {
  console.log('Selected:', image);
}
```

#### `poly.dialog.openMultiple(options?)`

Opens a dialog for multiple files.

**Returns:** `Promise<string[]>` - Array of selected file paths

```javascript
const files = await poly.dialog.openMultiple({
  title: 'Select Files'
});

console.log(`${files.length} files selected`);
```

#### `poly.dialog.save(options?)`

Opens a save dialog.

**Parameters:**
| Option | Type | Description |
|--------|------|-------------|
| `title` | string | Dialog title |
| `defaultName` | string | Suggested filename |
| `filters` | array | File type filters |

**Returns:** `Promise<string | null>` - Selected save path or null

```javascript
const path = await poly.dialog.save({
  title: 'Save Document',
  defaultName: 'document.txt'
});

if (path) {
  await poly.fs.write(path, documentContent);
}
```

#### `poly.dialog.folder(options?)`

Opens a folder picker dialog.

**Returns:** `Promise<string | null>` - Selected folder path or null

```javascript
const folder = await poly.dialog.folder({
  title: 'Select Output Folder'
});

if (folder) {
  console.log('Folder:', folder);
}
```

### In-App Dialogs

Poly also provides beautiful in-app dialogs that match your app's style.

#### `poly.dialog.message(title, message, level?)`

Shows a message dialog.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `title` | string | Dialog title |
| `message` | string | Message text |
| `level` | string | `'info'`, `'warning'`, or `'error'` |

**Returns:** `Promise<boolean>`

```javascript
// Info dialog
await poly.dialog.message('Success', 'File has been saved!', 'info');

// Warning
await poly.dialog.message('Warning', 'This action cannot be undone', 'warning');

// Error
await poly.dialog.message('Error', 'Could not load file', 'error');
```

#### `poly.dialog.confirm(title, message)`

Shows a confirmation dialog.

**Returns:** `Promise<boolean>` - true if confirmed, false if cancelled

```javascript
const confirmed = await poly.dialog.confirm(
  'Delete file?',
  'Do you really want to delete this file?'
);

if (confirmed) {
  // Delete file
}
```

#### `poly.dialog.custom(options)`

Shows a fully customizable dialog.

**Parameters:**
| Option | Type | Description |
|--------|------|-------------|
| `type` | string | `'info'`, `'warning'`, `'error'`, `'confirm'` |
| `title` | string | Dialog title |
| `message` | string | Message text |
| `buttons` | array | Button definitions |

**Button Definition:**
| Property | Type | Description |
|----------|------|-------------|
| `text` | string | Button text |
| `value` | any | Return value when clicked |
| `primary` | boolean | Primary button style |

**Returns:** `Promise<any>` - Button value or null if closed

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
  case 'save':
    await saveFile();
    closeWindow();
    break;
  case 'discard':
    closeWindow();
    break;
  case 'cancel':
    // Do nothing
    break;
}
```

---

## File System

Read and write files.

### `poly.fs.read(path)`

Reads file contents as a string.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `path` | string | Path to file |

**Returns:** `Promise<string>`

```javascript
// Read text file
const content = await poly.fs.read('config.json');
const config = JSON.parse(content);

// Relative paths
const readme = await poly.fs.read('./README.md');

// Absolute paths
const home = await poly.os.homedir();
const notes = await poly.fs.read(`${home}/notes.txt`);
```

### `poly.fs.write(path, content)`

Writes string content to a file.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `path` | string | Path to file |
| `content` | string | Content to write |

**Returns:** `Promise<boolean>`

```javascript
// Save JSON
const config = { theme: 'dark', language: 'en' };
await poly.fs.write('config.json', JSON.stringify(config, null, 2));

// Save text
await poly.fs.write('notes.txt', 'My notes...');
```

### `poly.fs.exists(path)`

Checks if a file or directory exists.

**Returns:** `Promise<boolean>`

```javascript
if (await poly.fs.exists('config.json')) {
  const config = JSON.parse(await poly.fs.read('config.json'));
} else {
  // Create default config
  await poly.fs.write('config.json', '{}');
}
```

### `poly.fs.readDir(path)`

Lists directory contents.

**Returns:** `Promise<Array<{name, path, isDir}>>`

```javascript
const entries = await poly.fs.readDir('./documents');

for (const entry of entries) {
  console.log(`${entry.isDir ? '📁' : '📄'} ${entry.name}`);
}

// Output:
// 📁 Projects
// 📄 notes.txt
// 📄 todo.md
```

### Practical Example: Save Settings

```javascript
const SETTINGS_FILE = 'app-settings.json';

// Load settings
async function loadSettings() {
  if (await poly.fs.exists(SETTINGS_FILE)) {
    const content = await poly.fs.read(SETTINGS_FILE);
    return JSON.parse(content);
  }
  return { theme: 'light', fontSize: 14 }; // Defaults
}

// Save settings
async function saveSettings(settings) {
  await poly.fs.write(SETTINGS_FILE, JSON.stringify(settings, null, 2));
}

// Usage
const settings = await loadSettings();
settings.theme = 'dark';
await saveSettings(settings);
```

---

## Deep Links

Register and handle custom URL protocols (e.g., `myapp://action`).

### `poly.deeplink.register(protocol, appName)`

Registers a custom URL protocol in the system registry.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `protocol` | string | Protocol name (e.g., 'myapp' for myapp://) |
| `appName` | string | Display name for the protocol |

**Returns:** `Promise<boolean>`

```javascript
// Register myapp:// protocol
await poly.deeplink.register('myapp', 'My Application');

// Now myapp://anything will open your app
```

### `poly.deeplink.unregister(protocol)`

Removes a URL protocol from the system registry.

**Returns:** `Promise<boolean>`

```javascript
await poly.deeplink.unregister('myapp');
```

### `poly.deeplink.isRegistered(protocol)`

Checks if a protocol is registered.

**Returns:** `Promise<boolean>`

```javascript
const registered = await poly.deeplink.isRegistered('myapp');
if (!registered) {
  await poly.deeplink.register('myapp', 'My App');
}
```

### `poly.deeplink.get()`

Returns the deep link URL that launched the app.

**Returns:** `Promise<string | null>` - The full URL or null

```javascript
const link = await poly.deeplink.get();
if (link) {
  // e.g., myapp://open/document/123
  const url = new URL(link);
  console.log(url.pathname); // /open/document/123
  console.log(url.searchParams.get('id')); // Query parameters
}
```

### `poly.deeplink.has()`

Checks if the app was launched via a deep link.

**Returns:** `Promise<boolean>`

```javascript
if (await poly.deeplink.has()) {
  const link = await poly.deeplink.get();
  handleDeepLink(link);
}
```

### Complete Example

```javascript
// On app startup
async function initDeepLinks() {
  // Register protocol if needed
  if (!await poly.deeplink.isRegistered('myapp')) {
    await poly.deeplink.register('myapp', 'My App');
    console.log('Protocol myapp:// registered');
  }
  
  // Handle deep link if present
  if (await poly.deeplink.has()) {
    const link = await poly.deeplink.get();
    handleDeepLink(link);
  }
}

function handleDeepLink(link) {
  // myapp://action/param1/param2?key=value
  const url = new URL(link);
  
  switch (url.pathname) {
    case '/open':
      const docId = url.searchParams.get('id');
      openDocument(docId);
      break;
      
    case '/settings':
      showSettings();
      break;
      
    case '/share':
      const data = url.searchParams.get('data');
      handleShare(data);
      break;
      
    default:
      console.log('Unknown deep link:', link);
  }
}

// Example URLs:
// myapp://open?id=123
// myapp://settings
// myapp://share?data=hello
```

---

## System Tray

Display an icon in the system tray with a context menu. Configuration is done in `poly.toml`.

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
id = "separator"

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

All other IDs trigger a `polytray` event that you can handle in JavaScript.

### `poly.tray.onMenuClick(callback)`

Listens for clicks on custom tray menu items.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `callback` | function | Callback with the menu ID |

```javascript
poly.tray.onMenuClick((id) => {
  switch (id) {
    case 'settings':
      openSettingsWindow();
      break;
    case 'about':
      showAboutDialog();
      break;
    case 'check-updates':
      checkForUpdates();
      break;
  }
});
```

### `poly.tray.isEnabled()`

Checks if the system tray is enabled.

**Returns:** `Promise<boolean>`

```javascript
if (await poly.tray.isEnabled()) {
  console.log('Tray is active');
}
```

### Tray Behavior

- **`close_to_tray = true`**: Clicking the close button hides the window to tray instead of exiting the app
- **`minimize_to_tray = true`**: Clicking minimize hides to tray instead of minimizing
- **Tray icon click**: Shows and focuses the window
- **Icon**: Uses `assets/icon.png` if available

### Complete Example

```toml
# poly.toml
[package]
name = "My App"
version = "1.0.0"

[window]
width = 1024
height = 768

[tray]
enabled = true
tooltip = "My App - Running in background"
close_to_tray = true
minimize_to_tray = false

[[tray.menu]]
id = "show"
label = "Open Window"

[[tray.menu]]
id = "separator"

[[tray.menu]]
id = "pause"
label = "Pause"

[[tray.menu]]
id = "settings"
label = "Settings..."

[[tray.menu]]
id = "separator"

[[tray.menu]]
id = "quit"
label = "Exit"
```

```javascript
// JavaScript
let isPaused = false;

poly.tray.onMenuClick((id) => {
  switch (id) {
    case 'pause':
      isPaused = !isPaused;
      updateTrayMenu();
      break;
    case 'settings':
      poly.windows.create({
        title: 'Settings',
        width: 500,
        height: 400,
        url: 'settings.html'
      });
      break;
  }
});
```

---

## Auto-Updater

Check for updates and install them from GitHub Releases.

### `poly.updater.checkGithub(repo, currentVersion)`

Checks GitHub for new releases.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `repo` | string | GitHub repository (e.g., 'user/repo') |
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
const info = await poly.updater.checkGithub('OpenChatGit/Poly', '0.2.6');

if (info.update_available) {
  console.log('New version available:', info.latest_version);
  console.log('Release Notes:', info.release_notes);
}
```

### `poly.updater.checkUrl(url, currentVersion)`

Checks a custom URL for updates.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `url` | string | URL to update info JSON |
| `currentVersion` | string | Current app version |

```javascript
// Custom update server
const info = await poly.updater.checkUrl(
  'https://myapp.com/updates/latest.json',
  '1.0.0'
);
```

### `poly.updater.download(url)`

Downloads an update file.

**Returns:** `Promise<string>` - Path to downloaded file

```javascript
const path = await poly.updater.download(info.download_url);
console.log('Downloaded to:', path);
```

### `poly.updater.install(path)`

Installs a downloaded update.

```javascript
await poly.updater.install(path);
// App will restart
```

### `poly.updater.checkAndPrompt(options)`

Convenience method: Checks, prompts user, downloads, and installs.

**Parameters:**
| Option | Type | Description |
|--------|------|-------------|
| `repo` | string | GitHub repository |
| `currentVersion` | string | Current version |

```javascript
// Simplest usage
await poly.updater.checkAndPrompt({
  repo: 'myuser/myapp',
  currentVersion: '1.0.0'
});
```

### Complete Update Example

```javascript
async function checkForUpdates() {
  const currentVersion = await poly.app.getVersion();
  
  try {
    const info = await poly.updater.checkGithub('myuser/myapp', currentVersion);
    
    if (info.update_available) {
      // Ask user
      const shouldUpdate = await poly.dialog.confirm(
        'Update Available',
        `Version ${info.latest_version} is available.\n\n` +
        `Current version: ${currentVersion}\n\n` +
        `${info.release_notes || ''}\n\n` +
        `Download and install now?`
      );
      
      if (shouldUpdate) {
        await poly.notification.show('Update', 'Download starting...');
        
        const downloadPath = await poly.updater.download(info.download_url);
        
        await poly.notification.show('Update', 'Installing...');
        await poly.updater.install(downloadPath);
        // App restarts automatically
      }
    } else {
      await poly.dialog.message(
        'No Update',
        `You are already on the latest version (${currentVersion}).`,
        'info'
      );
    }
  } catch (e) {
    await poly.dialog.message(
      'Update Error',
      'Could not check for updates: ' + e.message,
      'error'
    );
  }
}
```

---

## AI/LLM Integration

Built-in support for AI chat APIs with Ollama, OpenAI, Anthropic, and custom providers.

### `poly.ai.ollama(model, messages, options?)`

Chat with local Ollama. Supports thinking/reasoning for compatible models.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `model` | string | Model name (e.g., 'llama3', 'qwen3', 'deepseek-r1') |
| `messages` | array | Chat messages |
| `options` | object | Optional settings |

**Options:**
| Option | Type | Description |
|--------|------|-------------|
| `temperature` | number | Temperature (0.0 - 2.0, default: 0.7) |
| `maxTokens` | number | Max tokens to generate |
| `think` | boolean | Enable thinking mode (for qwen3, deepseek-r1, etc.) |

```javascript
// Basic usage
const response = await poly.ai.ollama('llama3', [
  { role: 'user', content: 'Explain recursion in one sentence.' }
]);
console.log(response.content);

// With thinking enabled (for reasoning models)
const response = await poly.ai.ollama('qwen3', [
  { role: 'user', content: 'How many r are in strawberry?' }
], { think: true });

console.log(response.thinking); // Reasoning process
console.log(response.content);  // Final answer
```

### `poly.ai.openai(model, messages, apiKey, options?)`

Chat with OpenAI.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `model` | string | Model name (e.g., 'gpt-4', 'gpt-4o', 'gpt-3.5-turbo') |
| `messages` | array | Chat messages |
| `apiKey` | string | OpenAI API key |
| `options` | object | Optional settings |

**Options:**
| Option | Type | Description |
|--------|------|-------------|
| `temperature` | number | Temperature (0.0 - 2.0, default: 0.7) |
| `maxTokens` | number | Max tokens to generate |

```javascript
const response = await poly.ai.openai('gpt-4', [
  { role: 'system', content: 'You are a helpful assistant.' },
  { role: 'user', content: 'What is the capital of Germany?' }
], 'sk-your-api-key');

console.log(response.content);
```

### `poly.ai.anthropic(model, messages, apiKey, options?)`

Chat with Anthropic Claude. Supports extended thinking.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `model` | string | Model name (e.g., 'claude-3-5-sonnet-20241022') |
| `messages` | array | Chat messages |
| `apiKey` | string | Anthropic API key |
| `options` | object | Optional settings |

**Options:**
| Option | Type | Description |
|--------|------|-------------|
| `temperature` | number | Temperature (0.0 - 2.0, default: 0.7) |
| `maxTokens` | number | Max tokens to generate |
| `enableThinking` | boolean | Enable extended thinking |
| `thinkingBudget` | number | Token budget for thinking (default: 10000) |

```javascript
const response = await poly.ai.anthropic(
  'claude-3-5-sonnet-20241022',
  [{ role: 'user', content: 'Explain quantum computing' }],
  'your-api-key',
  { enableThinking: true, thinkingBudget: 10000 }
);

console.log(response.thinking); // Thinking process
console.log(response.content);  // Final answer
```

### `poly.ai.custom(baseUrl, model, messages, options?)`

Chat with OpenAI-compatible APIs (LM Studio, LocalAI, vLLM, etc).

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `baseUrl` | string | API base URL |
| `model` | string | Model name |
| `messages` | array | Chat messages |
| `options` | object | Optional settings |

```javascript
const response = await poly.ai.custom(
  'http://localhost:1234/v1',
  'local-model',
  [{ role: 'user', content: 'Hello!' }],
  { temperature: 0.5 }
);
```

### `poly.ai.chat(options)`

Generic chat function with full control over provider and settings.

**Options:**
| Option | Type | Description |
|--------|------|-------------|
| `provider` | string | 'ollama', 'openai', 'anthropic', or 'custom' |
| `model` | string | Model name |
| `messages` | array | Chat messages |
| `baseUrl` | string | Custom API URL (optional) |
| `apiKey` | string | API key (for OpenAI/Anthropic) |
| `temperature` | number | Temperature |
| `maxTokens` | number | Max tokens |
| `enableThinking` | boolean | Enable thinking |
| `thinkingBudget` | number | Thinking token budget |

```javascript
const response = await poly.ai.chat({
  provider: 'ollama',
  model: 'qwen3',
  messages: [{ role: 'user', content: 'Hello!' }],
  temperature: 0.7,
  enableThinking: true
});
```

### `poly.ai.checkOllama()`

Checks if Ollama is running locally.

**Returns:** `Promise<boolean>`

```javascript
if (await poly.ai.checkOllama()) {
  console.log('Ollama is available');
}
```

### `poly.ai.listModels()`

Lists available Ollama models.

**Returns:** `Promise<string[]>`

```javascript
const models = await poly.ai.listModels();
console.log('Available models:', models);
// ['llama3', 'qwen3', 'deepseek-r1', 'codellama']
```

### Response Format

All AI methods return a response object:

```javascript
{
  content: "The answer...",      // Main response text
  thinking: "Let me think...",   // Reasoning (if enabled, null otherwise)
  model: "qwen3",                // Model used
  usage: {                       // Token usage (if available)
    prompt_tokens: 10,
    completion_tokens: 50,
    total_tokens: 60
  }
}
```

---

## IPC (Backend Functions)

Call functions defined in `main.poly`.

### `poly.invoke(functionName, args?)`

Calls a backend function.

**Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `functionName` | string | Function name |
| `args` | object | Arguments as object |

```poly
// In main.poly:
fn greet(name) {
  return "Hello, " + name + "!"
}

fn calculate(a, b) {
  return a + b
}

fn getUser(id) {
  // Simulate database query
  return {
    "id": id,
    "name": "John Doe",
    "email": "john@example.com"
  }
}
```

```javascript
// In JavaScript:
const greeting = await poly.invoke('greet', { name: 'World' });
console.log(greeting); // "Hello, World!"

const sum = await poly.invoke('calculate', { a: 5, b: 3 });
console.log(sum); // 8

const user = await poly.invoke('getUser', { id: 123 });
console.log(user.name); // "John Doe"
```

### Stateful Interpreter

The Poly interpreter maintains its state between calls:

```poly
// main.poly
let counter = 0

fn increment() {
  counter = counter + 1
  return counter
}

fn getCounter() {
  return counter
}
```

```javascript
await poly.invoke('increment'); // 1
await poly.invoke('increment'); // 2
await poly.invoke('increment'); // 3
const count = await poly.invoke('getCounter'); // 3
```

---

## Configuration (poly.toml)

Complete reference for all configuration options. Poly reads all settings from `poly.toml` - no hardcoded values.

### Basic Structure

```toml
[package]
name = "My App"
version = "1.0.0"
description = "An awesome app"
author = "Your Name"

[web]
dir = "web"

[window]
title = "My App"
width = 1024
height = 768
resizable = true
background_color = "#1a1a1a"
transparent = false
decorations = true
always_on_top = false
fullscreen = false
# min_width = 400
# min_height = 300
# max_width = 1920
# max_height = 1080
# default_popup_width = 800
# default_popup_height = 600

[dev]
port = 3000
devtools = false
reload_interval = 2000
# inject_alpine = true
# inject_lucide = true

[network]
timeout = 30
# user_agent = "Mozilla/5.0 ..."
max_body_size = 50000000

[app]
notification_timeout = 5000

[tray]
enabled = false
tooltip = "My App"
# icon_path = "assets/icon.png"
icon_size = 32
minimize_to_tray = false
close_to_tray = false

[browser]
# ui_height = 80
# width = 1200
# height = 800

[build]
icon_size = 64
# icon_path = "assets/icon.png"

[signing.windows]
certificate = "path/to/cert.pfx"
timestamp_url = "http://timestamp.digicert.com"

[signing.macos]
identity = "Developer ID Application: Name (TEAMID)"
team_id = "YOURTEAMID"

[dependencies]
alpinejs = "3.14.3"
lodash = "4.17.21"
```

### [package] Section

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `name` | string | "Poly App" | App name (used as window title if not set) |
| `version` | string | "0.1.0" | App version |
| `description` | string | - | Description |
| `author` | string | - | Author |

### [web] Section

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `dir` | string | "web" | Directory containing web assets |

### [window] Section

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `title` | string | package.name | Window title |
| `width` | number | 1024 | Window width in pixels |
| `height` | number | 768 | Window height in pixels |
| `resizable` | boolean | true | Allow window resizing |
| `background_color` | string | "#1a1a1a" | Background color (hex) |
| `transparent` | boolean | false | Transparent window background |
| `decorations` | boolean | true | Show native titlebar |
| `always_on_top` | boolean | false | Window always on top |
| `fullscreen` | boolean | false | Start in fullscreen |
| `icon_path` | string | - | Path to window icon (PNG) |
| `min_width` | number | - | Minimum window width |
| `min_height` | number | - | Minimum window height |
| `max_width` | number | - | Maximum window width |
| `max_height` | number | - | Maximum window height |
| `default_popup_width` | number | 800 | Default width for popup windows |
| `default_popup_height` | number | 600 | Default height for popup windows |

### [dev] Section

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `port` | number | 3000 | Port for dev server |
| `devtools` | boolean | false | Enable DevTools in native mode |
| `reload_interval` | number | 2000 | Hot reload polling interval (ms) |
| `inject_alpine` | boolean | false | Auto-inject Alpine.js |
| `inject_lucide` | boolean | false | Auto-inject Lucide Icons |

**Note:** By default, Poly does NOT inject any libraries. You control your own dependencies. If you want Alpine.js or Lucide Icons auto-injected into your HTML, enable them here:

```toml
[dev]
inject_alpine = true
inject_lucide = true
```

### [network] Section

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `timeout` | number | 30 | HTTP request timeout in seconds |
| `user_agent` | string | Chrome UA | Custom User-Agent string |
| `max_body_size` | number | 50000000 | Max response body size in bytes |

### [app] Section

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `notification_timeout` | number | 5000 | Default notification timeout (ms) |

### [tray] Section

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | boolean | false | Enable system tray |
| `tooltip` | string | package.name | Tooltip on hover |
| `icon_path` | string | - | Path to tray icon |
| `icon_size` | number | 32 | Tray icon size in pixels |
| `minimize_to_tray` | boolean | false | Minimize to tray |
| `close_to_tray` | boolean | false | Close to tray |

### [[tray.menu]] Entries

| Option | Type | Description |
|--------|------|-------------|
| `id` | string | Unique ID (or 'show', 'quit', 'separator') |
| `label` | string | Displayed text |

### [browser] Section

Browser mode creates a dual-WebView window (UI + Content).

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `ui_height` | number | 80 | Height of UI WebView in pixels |
| `width` | number | 1200 | Browser window width |
| `height` | number | 800 | Browser window height |

### [build] Section

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `icon_size` | number | 64 | Icon size for window icon |
| `icon_path` | string | - | Path to app icon |

### [signing.windows] Section

| Option | Type | Description |
|--------|------|-------------|
| `certificate` | string | Path to .pfx file |
| `timestamp_url` | string | Timestamp server URL |

### [signing.macos] Section

| Option | Type | Description |
|--------|------|-------------|
| `identity` | string | Signing identity |
| `entitlements` | string | Path to entitlements.plist |
| `team_id` | string | Team ID for notarization |

### [dependencies] Section

NPM packages with versions:

```toml
[dependencies]
alpinejs = "3.14.3"
lodash = "4.17.21"
chart.js = "4.4.1"
```

---

## Building Apps

Build your Poly app into a standalone executable.

### Basic Commands

```bash
# Development build
poly build demo-app

# Release build (optimized, no console window)
poly build --release demo-app

# For specific platform
poly build --release --target windows demo-app
poly build --release --target macos demo-app
poly build --release --target linux demo-app

# Build and sign
poly build --release --sign demo-app

# Create installer
poly build --release --installer demo-app

# Generate GitHub Actions workflow
poly build --ci demo-app
```

### Target Platforms

| Target | Aliases | Description |
|--------|---------|-------------|
| `current` | - | Build for current OS (default) |
| `windows` | `win`, `win64` | Windows x64 |
| `macos` | `mac`, `darwin`, `osx` | macOS x64 |
| `linux` | - | Linux x64 |

> **Note:** Cross-compilation (e.g., building Windows on macOS) requires building on the target platform or using GitHub Actions.

### Build Output

```
demo-app/
└── dist/
    └── windows/
        ├── demo-app.exe    # Standalone executable
        └── bundle/         # Bundled web assets
            ├── web/
            ├── assets/
            └── poly.toml
```

### No Console Window

Built apps automatically run as GUI applications without a console window. This is achieved by:
- Building with the `gui` feature which sets the Windows subsystem to "windows"
- The executable detects the `bundle/` folder and runs as a native app

### Build Flags

| Flag | Description |
|------|-------------|
| `--release` | Optimized build, no console window |
| `--target <platform>` | Target platform (windows/macos/linux/current) |
| `--sign` | Sign executable (requires certificate) |
| `--installer` | Create installer/package |
| `--ci` | Generate GitHub Actions workflow |

### Cross-Platform Builds with GitHub Actions

```bash
poly build --ci demo-app
```

This generates `.github/workflows/build.yml` that automatically builds for Windows, macOS, and Linux.

---

## Code Signing

Sign your executables to avoid security warnings on Windows and macOS.

### Usage

```bash
poly build --release --sign demo-app
```

### Configuration in poly.toml

```toml
[signing.windows]
certificate = "path/to/certificate.pfx"
timestamp_url = "http://timestamp.digicert.com"

[signing.macos]
identity = "Developer ID Application: Your Name (TEAMID)"
entitlements = "entitlements.plist"
team_id = "YOURTEAMID"
```

### Environment Variables

For CI/CD, use environment variables instead of storing secrets in poly.toml:

| Variable | Description |
|----------|-------------|
| `POLY_WINDOWS_CERTIFICATE` | Path to .pfx certificate file |
| `POLY_WINDOWS_CERTIFICATE_PASSWORD` | Certificate password |
| `POLY_WINDOWS_TIMESTAMP_URL` | Timestamp server URL |
| `POLY_MACOS_IDENTITY` | Signing identity |
| `POLY_MACOS_ENTITLEMENTS` | Path to entitlements.plist |
| `POLY_MACOS_APPLE_ID` | Apple ID for notarization |
| `POLY_MACOS_APP_PASSWORD` | App-specific password |
| `POLY_MACOS_TEAM_ID` | Team ID for notarization |

### Windows Requirements

- Windows SDK installed (for signtool.exe)
- Code signing certificate (.pfx file)

### macOS Requirements

- Xcode Command Line Tools
- Developer ID certificate in Keychain
- For notarization: Apple Developer account

---

## Single Instance

Prevent multiple instances of your app from running simultaneously.

### Configuration

```toml
[window]
single_instance = true
```

### Behavior

When a second instance is launched:
- It detects the existing instance via a lock file
- Exits immediately with a message
- The first instance continues running

This is useful for apps that should only have one window, like system utilities or tray apps.

---

## Dev Server Configuration

Configure the development server in `poly.toml`:

```toml
[dev]
port = 3000  # Custom port (default: auto-find free port)
```

If `port = 0` or not specified, Poly will automatically find a free port.

---

## Project Structure

```
my-app/
├── poly.toml        # Configuration
├── poly.lock        # Package lockfile
├── packages/        # NPM packages
├── src/
│   └── main.poly    # Backend logic (optional)
├── web/
│   ├── index.html   # Your app
│   ├── app.js       # JavaScript
│   └── styles.css   # Styles
└── assets/
    └── icon.png     # App icon (for tray and taskbar)
```

---

## Package Manager

```bash
# Add package
poly add alpinejs

# Specific version
poly add lodash -v 4.17.21

# Remove package
poly remove lodash

# Install packages from lockfile
poly install

# Verify hashes
poly install --verify
```

Packages are stored in `packages/` and can be loaded via `/packages/` in the browser:

```html
<script src="/packages/alpinejs/dist/cdn.min.js"></script>
```

---

## Browser Mode

Poly provides a built-in browser mode for creating browser-like applications with a custom UI and separate content WebView. This is the recommended way to build browsers with Poly.

### Quick Start

There are two ways to enable browser mode:

**1. Using poly.toml (recommended):**

Add a `[browser]` section to your `poly.toml`:

```toml
[package]
name = "My Browser"
version = "1.0.0"

[window]
width = 1280
height = 800
decorations = false

[browser]
ui_height = 80
```

Then run:
```bash
poly run --native path/to/app
```

**2. Using --browser flag:**

```bash
poly run --native --browser path/to/app
poly run --native --browser --ui-height 100 path/to/app
```

**3. Using poly browser command (standalone):**

```bash
poly browser https://google.com --ui-html path/to/ui.html
```

### Configuration Options

**poly.toml [browser] section:**

| Option | Default | Description |
|--------|---------|-------------|
| `ui_height` | 80 | Height of UI area in pixels |

**Command line flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--browser` | false | Enable browser mode |
| `--ui-height` | 80 | Height of UI area in pixels |

### Architecture

Browser mode creates a frameless window with two WebViews:

1. **Content WebView** - Displays web pages (created first, bottom layer)
2. **UI WebView** - Your custom browser UI (created second, top layer)

```
┌─────────────────────────────────────┐
│  UI WebView (titlebar + toolbar)    │  ← ui_height (80px)
├─────────────────────────────────────┤
│                                     │
│         Content WebView             │  ← Remaining height
│         (web pages)                 │
│                                     │
└─────────────────────────────────────┘
```

> **Note:** On Windows/WebView2, the creation order matters for z-order. Browser mode handles this automatically by creating the content WebView first, then the UI WebView on top.

### UI HTML Requirements

Your `web/index.html` serves as the UI. Use `window.ipc.postMessage()` to communicate with the native layer:

**IPC Commands:**

| Command | Description |
|---------|-------------|
| `navigate:URL` | Navigate content WebView to URL |
| `minimize` | Minimize window |
| `maximize` | Toggle maximize |
| `close` | Close window |
| `drag` | Start window drag (call on mousedown) |

### Event Callbacks

The native layer calls these functions on your UI when events occur:

| Function | Parameters | Description |
|----------|------------|-------------|
| `window.onNavStart(url)` | URL string | Navigation started |
| `window.onLoadEnd(url)` | URL string | Page finished loading |
| `window.onTitleChange(title)` | Title string | Page title changed |

### Complete UI Example

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    html, body { height: 100%; overflow: hidden; }
    body { 
      font-family: 'Segoe UI', system-ui, sans-serif; 
      background: #1a1a1f; 
      color: #fff;
      display: flex;
      flex-direction: column;
    }
    
    /* Titlebar - draggable */
    .titlebar {
      height: 32px;
      background: #0f0f12;
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0 12px;
      -webkit-app-region: drag;
    }
    
    .titlebar-title { font-size: 12px; color: #888; }
    
    /* Window controls - not draggable */
    .window-controls {
      display: flex;
      -webkit-app-region: no-drag;
    }
    
    .win-btn {
      width: 46px;
      height: 32px;
      border: none;
      background: transparent;
      color: #888;
      cursor: pointer;
    }
    .win-btn:hover { background: #333; color: #fff; }
    .win-btn.close:hover { background: #e81123; }
    
    /* Toolbar */
    .toolbar {
      height: 48px;
      background: #18181b;
      display: flex;
      align-items: center;
      padding: 0 8px;
      gap: 4px;
    }
    
    .nav-btn {
      width: 32px;
      height: 32px;
      border: none;
      background: transparent;
      color: #888;
      cursor: pointer;
      border-radius: 6px;
    }
    .nav-btn:hover { background: #333; color: #fff; }
    
    .url-input {
      flex: 1;
      height: 32px;
      background: #27272a;
      border: 1px solid #3f3f46;
      border-radius: 8px;
      padding: 0 12px;
      color: #fff;
      font-size: 13px;
    }
    .url-input:focus { border-color: #22d3ee; outline: none; }
    
    /* Loading bar */
    .loading { 
      height: 2px; 
      background: #22d3ee; 
      width: 0; 
      transition: width 0.3s; 
    }
    .loading.active { width: 100%; }
  </style>
</head>
<body>
  <div class="titlebar">
    <span class="titlebar-title" id="title">Poly Browser</span>
    <div class="window-controls">
      <button class="win-btn" onclick="window.ipc.postMessage('minimize')">─</button>
      <button class="win-btn" onclick="window.ipc.postMessage('maximize')">□</button>
      <button class="win-btn close" onclick="window.ipc.postMessage('close')">✕</button>
    </div>
  </div>
  
  <div class="toolbar">
    <button class="nav-btn" onclick="goBack()">←</button>
    <button class="nav-btn" onclick="goForward()">→</button>
    <button class="nav-btn" onclick="reload()">↻</button>
    <input type="text" class="url-input" id="url" 
           placeholder="Enter URL or search" 
           onkeydown="if(event.key==='Enter')navigate()">
    <button class="nav-btn" onclick="goHome()">🏠</button>
  </div>
  
  <div class="loading" id="loading"></div>

  <script>
    const urlInput = document.getElementById('url');
    const loading = document.getElementById('loading');
    const title = document.getElementById('title');
    let currentUrl = '';
    
    // Event callbacks from native
    window.onNavStart = function(url) {
      currentUrl = url;
      urlInput.value = url;
      loading.classList.add('active');
    };
    
    window.onLoadEnd = function(url) {
      loading.classList.remove('active');
    };
    
    window.onTitleChange = function(newTitle) {
      title.textContent = newTitle || 'Poly Browser';
    };
    
    // Navigation functions
    function navigate() {
      let url = urlInput.value.trim();
      if (!url) return;
      
      // Add https:// if missing
      if (!url.startsWith('http://') && !url.startsWith('https://')) {
        // Check if it's a search query
        if (!url.includes('.')) {
          url = 'https://google.com/search?q=' + encodeURIComponent(url);
        } else {
          url = 'https://' + url;
        }
      }
      
      urlInput.value = url;
      window.ipc.postMessage('navigate:' + url);
    }
    
    function goBack() {
      window.ipc.postMessage('navigate:javascript:history.back()');
    }
    
    function goForward() {
      window.ipc.postMessage('navigate:javascript:history.forward()');
    }
    
    function reload() {
      if (currentUrl) {
        window.ipc.postMessage('navigate:' + currentUrl);
      }
    }
    
    function goHome() {
      urlInput.value = 'https://google.com';
      navigate();
    }
  </script>
</body>
</html>
```

### Example: Minimal Browser

Create a file `browser-ui.html` with the UI above, then run:

```bash
poly browser https://google.com --ui-html browser-ui.html
```

### Tips

1. **Z-Order on Windows**: Content WebView is created first (bottom), UI WebView second (top)
2. **Dragging**: Use `-webkit-app-region: drag` on titlebar, `no-drag` on buttons
3. **Search**: Check if input contains `.` to distinguish URLs from search queries
4. **Loading State**: Use `onNavStart` to show loading, `onLoadEnd` to hide
5. **Title Updates**: `onTitleChange` fires when page title changes

---

## Browser Tab API

For building multi-tab browsers, Poly provides a dedicated Tab API that manages separate WebView instances for each tab. This is the recommended approach for browser applications.

### Configuration

Add a `[browser]` section to your `poly.toml`:

```toml
[browser]
ui_height = 80
start_url = "https://google.com"
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `ui_height` | number | 80 | Height of UI area in pixels |
| `start_url` | string | "https://google.com" | Initial URL for the first tab |

### IPC Commands

Send commands from your UI to the native layer using `window.ipc.postMessage()`:

| Command | Format | Description |
|---------|--------|-------------|
| Create Tab | `createTab:URL` | Creates a new tab with the given URL |
| Close Tab | `closeTab:ID` | Closes the tab with the given ID |
| Switch Tab | `switchTab:ID` | Switches to the tab with the given ID |
| Navigate | `navigate:ID:URL` | Navigates a specific tab to a URL |
| Go Back | `goBack:ID` | Goes back in history for a tab |
| Go Forward | `goForward:ID` | Goes forward in history for a tab |

```javascript
// Create a new tab
window.ipc.postMessage('createTab:https://google.com');

// Close a tab
window.ipc.postMessage('closeTab:' + tabId);

// Switch to a tab
window.ipc.postMessage('switchTab:' + tabId);

// Navigate a specific tab
window.ipc.postMessage('navigate:' + tabId + ':https://example.com');

// Navigate active tab (use 0 or omit tab ID)
window.ipc.postMessage('navigate:https://example.com');

// Go back/forward
window.ipc.postMessage('goBack:' + tabId);
window.ipc.postMessage('goForward:' + tabId);
```

### Event Callbacks

The native layer calls these functions on your UI when tab events occur:

| Function | Parameters | Description |
|----------|------------|-------------|
| `window.onTabCreated(tabId)` | Tab ID (number) | A new tab was created |
| `window.onTabActivated(tabId)` | Tab ID (number) | A tab became active |
| `window.onTabClosed(tabId)` | Tab ID (number) | A tab was closed |
| `window.onTabNavStart(tabId, url)` | Tab ID, URL | Navigation started in a tab |
| `window.onTabTitleChange(tabId, title)` | Tab ID, Title | Page title changed in a tab |
| `window.onTabLoadEnd(tabId)` | Tab ID (number) | Page finished loading in a tab |

```javascript
// Handle tab creation
window.onTabCreated = function(tabId) {
  console.log('Tab created:', tabId);
  // Add tab to your UI
};

// Handle tab activation
window.onTabActivated = function(tabId) {
  console.log('Tab activated:', tabId);
  // Update UI to show active tab
};

// Handle tab close
window.onTabClosed = function(tabId) {
  console.log('Tab closed:', tabId);
  // Remove tab from your UI
};

// Handle navigation start
window.onTabNavStart = function(tabId, url) {
  console.log('Tab', tabId, 'navigating to:', url);
  // Update URL bar, show loading indicator
};

// Handle title change (usually means page loaded)
window.onTabTitleChange = function(tabId, title) {
  console.log('Tab', tabId, 'title:', title);
  // Update tab title in UI, hide loading indicator
};

// Handle load complete
window.onTabLoadEnd = function(tabId) {
  console.log('Tab', tabId, 'loaded');
  // Hide loading indicator
};
```

### Complete Tab Browser Example

```javascript
// Tab state management
const tabs = new Map(); // tabId -> { url, title, loading }
let activeTabId = null;

// Create a new tab
function createTab(url = 'https://google.com') {
  window.ipc.postMessage('createTab:' + url);
}

// Close a tab
function closeTab(tabId) {
  if (tabs.size > 1) {
    window.ipc.postMessage('closeTab:' + tabId);
  }
}

// Switch to a tab
function switchTab(tabId) {
  if (tabId !== activeTabId) {
    window.ipc.postMessage('switchTab:' + tabId);
  }
}

// Navigate the active tab
function navigate(url) {
  if (activeTabId) {
    window.ipc.postMessage('navigate:' + activeTabId + ':' + url);
  }
}

// Event handlers
window.onTabCreated = function(tabId) {
  tabs.set(tabId, { url: 'about:blank', title: 'New Tab', loading: true });
  renderTabs();
};

window.onTabActivated = function(tabId) {
  activeTabId = tabId;
  const tab = tabs.get(tabId);
  if (tab) {
    document.getElementById('urlInput').value = tab.url;
  }
  renderTabs();
};

window.onTabClosed = function(tabId) {
  tabs.delete(tabId);
  renderTabs();
};

window.onTabNavStart = function(tabId, url) {
  const tab = tabs.get(tabId);
  if (tab) {
    tab.url = url;
    tab.loading = true;
  }
  if (tabId === activeTabId) {
    document.getElementById('urlInput').value = url;
    showLoading();
  }
  renderTabs();
};

window.onTabTitleChange = function(tabId, title) {
  const tab = tabs.get(tabId);
  if (tab) {
    tab.title = title;
    tab.loading = false;
  }
  if (tabId === activeTabId) {
    hideLoading();
  }
  renderTabs();
};

// Render tabs UI
function renderTabs() {
  const container = document.getElementById('tabs');
  container.innerHTML = Array.from(tabs.entries()).map(([id, tab]) => `
    <div class="tab ${id === activeTabId ? 'active' : ''}" onclick="switchTab(${id})">
      <span>${tab.title}</span>
      <button onclick="event.stopPropagation(); closeTab(${id})">×</button>
    </div>
  `).join('');
}

// Keyboard shortcuts
document.addEventListener('keydown', e => {
  if (e.ctrlKey && e.key === 't') {
    e.preventDefault();
    createTab();
  } else if (e.ctrlKey && e.key === 'w') {
    e.preventDefault();
    if (activeTabId) closeTab(activeTabId);
  }
});
```

### Architecture

Each tab is a separate WebView instance with its own:
- Navigation history (back/forward)
- Page state
- JavaScript context

```
┌─────────────────────────────────────┐
│  UI WebView (tabs + toolbar)        │  ← ui_height (80px)
├─────────────────────────────────────┤
│                                     │
│  Tab 1 WebView (visible)            │  ← Active tab
│                                     │
│  Tab 2 WebView (hidden)             │  ← Background tabs
│  Tab 3 WebView (hidden)             │
│                                     │
└─────────────────────────────────────┘
```

> **Note:** On Windows, the UI WebView is always on top. Tab WebViews are shown/hidden based on which tab is active.

---

*Documentation for Poly v0.3.0*


---

## WebView API

The WebView API allows creating and managing multiple WebViews within a window. This is the core API for building browser-like applications with Poly.

> **Note:** On Windows/WebView2, newly created WebViews appear on top of existing ones. Create content WebViews first, then UI WebViews.

### Creating WebViews

#### `poly.webview.create(id, options)`

Creates a new WebView within the current window.

**Parameters:**
| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `id` | string | required | Unique identifier for the WebView |
| `url` | string | "about:blank" | Initial URL to load |
| `html` | string | - | HTML content (alternative to url) |
| `x` | number | 0 | X position in pixels |
| `y` | number | 0 | Y position in pixels |
| `width` | number | 800 | Width in pixels |
| `height` | number | 600 | Height in pixels |
| `visible` | boolean | true | Whether WebView is visible |
| `transparent` | boolean | false | Transparent background |
| `devtools` | boolean | false | Enable DevTools |
| `userAgent` | string | - | Custom user agent string |
| `zoomLevel` | number | 1.0 | Initial zoom level (1.0 = 100%) |
| `autoplay` | boolean | true | Allow media autoplay |

**Returns:** `Promise<{success: boolean, id: string} | {error: string}>`

```javascript
// Create a content WebView
const result = await poly.webview.create('content', {
  url: 'https://example.com',
  x: 0,
  y: 80,
  width: 1200,
  height: 720,
  devtools: true
});
```

#### `poly.webview.destroy(id)`

Removes a WebView.

```javascript
await poly.webview.destroy('content');
```

### Navigation

#### `poly.webview.navigate(id, url)`

Navigates a WebView to a URL.

```javascript
await poly.webview.navigate('content', 'https://google.com');
```

#### `poly.webview.loadHtml(id, html)`

Loads HTML content directly into a WebView.

```javascript
await poly.webview.loadHtml('content', '<h1>Hello World</h1>');
```

#### `poly.webview.goBack(id)`

Navigates back in history.

```javascript
await poly.webview.goBack('content');
```

#### `poly.webview.goForward(id)`

Navigates forward in history.

```javascript
await poly.webview.goForward('content');
```

#### `poly.webview.reload(id)`

Reloads the current page.

```javascript
await poly.webview.reload('content');
```

#### `poly.webview.stop(id)`

Stops loading the current page.

```javascript
await poly.webview.stop('content');
```

### Display & Layout

#### `poly.webview.setBounds(id, bounds)`

Sets the position and size of a WebView.

```javascript
await poly.webview.setBounds('content', {
  x: 0,
  y: 80,
  width: 1200,
  height: 720
});
```

#### `poly.webview.getBounds(id)`

Gets the current bounds of a WebView.

```javascript
const bounds = await poly.webview.getBounds('content');
// {x: 0, y: 80, width: 1200, height: 720}
```

#### `poly.webview.setVisible(id, visible)`

Shows or hides a WebView.

```javascript
await poly.webview.setVisible('content', false);
```

#### `poly.webview.focus(id)`

Focuses a WebView.

```javascript
await poly.webview.focus('content');
```

#### `poly.webview.setZoom(id, level)`

Sets the zoom level (1.0 = 100%).

```javascript
await poly.webview.setZoom('content', 1.5); // 150% zoom
```

#### `poly.webview.setMainBounds(bounds)`

Resizes the main app WebView.

```javascript
await poly.webview.setMainBounds({ x: 0, y: 0, width: 1200, height: 80 });
```

### Content Execution

#### `poly.webview.eval(id, script)`

Executes JavaScript in a WebView.

```javascript
await poly.webview.eval('content', 'document.title');
```

### State & Information

#### `poly.webview.get(id)`

Gets information about a specific WebView.

**Returns:**
```javascript
{
  id: string,
  url: string,
  title: string,
  visible: boolean,
  isLoading: boolean,
  canGoBack: boolean,
  canGoForward: boolean,
  zoomLevel: number,
  bounds: { x, y, width, height }
}
```

```javascript
const info = await poly.webview.get('content');
if (info.canGoBack) {
  // Show back button
}
```

#### `poly.webview.list()`

Lists all WebViews with their state.

```javascript
const webviews = await poly.webview.list();
```

### Events

WebViews emit events for navigation, title changes, loading state, and more. Use `pollEvents()` to retrieve pending events, or use the convenience event listeners.

#### `poly.webview.pollEvents()`

Retrieves all pending events from WebViews.

**Event Types:**
| Type | Data | Description |
|------|------|-------------|
| `navigate` | `{id, url}` | Navigation started |
| `navigateFinish` | `{id, url}` | Navigation completed |
| `titleChange` | `{id, title}` | Page title changed |
| `loadStart` | `{id}` | Page started loading |
| `loadFinish` | `{id}` | Page finished loading |
| `newWindow` | `{id, url, target}` | New window requested (target="_blank") |
| `download` | `{id, url, filename}` | Download requested |
| `close` | `{id}` | WebView was closed |
| `historyChange` | `{id, canGoBack, canGoForward}` | History state changed |
| `permission` | `{id, permission, origin}` | Permission requested |

```javascript
// Poll for events
const { events } = await poly.webview.pollEvents();
for (const event of events) {
  switch (event.type) {
    case 'titleChange':
      document.title = event.title;
      break;
    case 'newWindow':
      // Handle new window request
      await poly.webview.navigate('content', event.url);
      break;
  }
}
```

#### Event Listeners (Convenience API)

```javascript
// Listen for navigation
poly.webview.onNavigate('content', (url) => {
  console.log('Navigated to:', url);
});

// Listen for title changes
poly.webview.onTitleChange('content', (title) => {
  document.title = title;
});

// Listen for loading state
poly.webview.onLoadStart('content', () => {
  showSpinner();
});

poly.webview.onLoadFinish('content', () => {
  hideSpinner();
});

// Listen for new window requests
poly.webview.onNewWindow('content', (url, target) => {
  // Open in same WebView or create new one
  poly.webview.navigate('content', url);
});

// Listen for downloads
poly.webview.onDownload('content', (url, filename) => {
  console.log('Download:', filename);
});

// Listen for history changes
poly.webview.onHistoryChange('content', ({ canGoBack, canGoForward }) => {
  backButton.disabled = !canGoBack;
  forwardButton.disabled = !canGoForward;
});

// Listen to all WebViews (wildcard)
poly.webview.on('titleChange', '*', (id, title) => {
  console.log(`WebView ${id} title: ${title}`);
});
```

### Permissions

#### `poly.webview.respondToPermission(id, permission, granted)`

Responds to a permission request (camera, microphone, geolocation, etc.).

```javascript
poly.webview.on('permission', '*', async (id, { permission, origin }) => {
  const granted = await poly.dialog.confirm(
    'Permission Request',
    `${origin} wants to access your ${permission}. Allow?`
  );
  await poly.webview.respondToPermission(id, permission, granted);
});
```

### Complete Browser Example

```javascript
// Initialize browser UI
async function initBrowser() {
  // Create content WebView (bottom layer)
  await poly.webview.create('content', {
    url: 'https://example.com',
    x: 0, y: 80,
    width: window.innerWidth,
    height: window.innerHeight - 80,
    devtools: true
  });
  
  // Set up event listeners
  poly.webview.onTitleChange('content', (title) => {
    document.getElementById('title').textContent = title;
  });
  
  poly.webview.onNavigate('content', (url) => {
    document.getElementById('url-bar').value = url;
  });
  
  poly.webview.onHistoryChange('content', ({ canGoBack, canGoForward }) => {
    document.getElementById('back-btn').disabled = !canGoBack;
    document.getElementById('forward-btn').disabled = !canGoForward;
  });
  
  poly.webview.onNewWindow('content', (url) => {
    // Open links in same WebView
    poly.webview.navigate('content', url);
  });
  
  poly.webview.onLoadStart('content', () => {
    document.getElementById('loading').style.display = 'block';
  });
  
  poly.webview.onLoadFinish('content', () => {
    document.getElementById('loading').style.display = 'none';
  });
}

// Navigation functions
async function navigate() {
  let url = document.getElementById('url-bar').value;
  if (!url.startsWith('http')) url = 'https://' + url;
  await poly.webview.navigate('content', url);
}

async function goBack() {
  await poly.webview.goBack('content');
}

async function goForward() {
  await poly.webview.goForward('content');
}

async function reload() {
  await poly.webview.reload('content');
}

// Handle window resize
window.addEventListener('resize', async () => {
  await poly.webview.setBounds('content', {
    x: 0, y: 80,
    width: window.innerWidth,
    height: window.innerHeight - 80
  });
});

initBrowser();
```

---

## MultiView API

Create windows with multiple WebViews. Perfect for building browser-like applications, split-pane editors, or any UI that needs multiple independent web content areas.

### Overview

The MultiView API creates a new window with multiple WebViews arranged in a layout. Each WebView can:
- Display different content (URL or HTML)
- Communicate with other views via messages
- Be resized and repositioned dynamically

**Important:** Views are stacked in creation order. The first view in the array is at the bottom, the last is on top. For browser UIs, put the content view first and the UI view last.

### `poly.multiview.create(options)`

Creates a new multi-view window.

**Parameters:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `title` | string | "Poly MultiView" | Window title |
| `width` | number | 1024 | Window width |
| `height` | number | 768 | Window height |
| `decorations` | boolean | false | Show native titlebar |
| `resizable` | boolean | true | Allow resizing |
| `icon` | string | - | Path to window icon |
| `views` | array | [] | Array of view configurations |

**View Configuration:**

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `id` | string | "view" | Unique identifier for the view |
| `url` | string | "about:blank" | URL to load |
| `html` | string | - | HTML content (alternative to url) |
| `x` | number | 0 | X position in window |
| `y` | number | 0 | Y position in window |
| `width` | number | 800 | View width |
| `height` | number | 600 | View height |
| `transparent` | boolean | false | Transparent background |
| `devtools` | boolean | false | Enable DevTools |

**Returns:** `Promise<{id: number}>` - Window ID

```javascript
// Create a browser-like window
const win = await poly.multiview.create({
  title: 'My Browser',
  width: 1200,
  height: 800,
  decorations: false,
  views: [
    // Content view (bottom layer)
    { 
      id: 'content', 
      url: 'https://example.com',
      x: 0, y: 80, 
      width: 1200, height: 720 
    },
    // UI view (top layer)
    { 
      id: 'ui', 
      url: 'http://localhost:3000/browser-ui.html',
      x: 0, y: 0, 
      width: 1200, height: 80,
      transparent: true
    }
  ]
});

console.log('Window created with ID:', win.id);
```

### `poly.multiview.navigate(windowId, viewId, url)`

Navigates a specific view to a URL.

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `windowId` | number | Window ID from create() |
| `viewId` | string | View ID to navigate |
| `url` | string | URL to load |

```javascript
// Navigate content view to a new URL
await poly.multiview.navigate(win.id, 'content', 'https://google.com');
```

### `poly.multiview.postMessage(windowId, viewId, message)`

Sends a message to a view. The view receives it as a `polymessage` event.

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `windowId` | number | Window ID |
| `viewId` | string | Target view ID |
| `message` | any | Message data (will be JSON stringified) |

```javascript
// Send message from backend to UI view
await poly.multiview.postMessage(win.id, 'ui', {
  type: 'urlChanged',
  url: 'https://google.com'
});
```

**Receiving messages in a view:**

```javascript
window.addEventListener('polymessage', (e) => {
  const data = e.detail;
  console.log('Received:', data);
  
  if (data.type === 'urlChanged') {
    document.getElementById('url-bar').value = data.url;
  }
});
```

### `poly.multiview.setBounds(windowId, viewId, bounds)`

Changes the position and size of a view.

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `windowId` | number | Window ID |
| `viewId` | string | View ID |
| `bounds` | object | `{x, y, width, height}` |

```javascript
// Resize content view
await poly.multiview.setBounds(win.id, 'content', {
  x: 0,
  y: 100,
  width: 1200,
  height: 700
});
```

### `poly.multiview.close(windowId)`

Closes a multi-view window.

```javascript
await poly.multiview.close(win.id);
```

### `poly.multiview.list()`

Lists all multi-view windows.

**Returns:** `Promise<{windows: Array<{id, title, views}>}>`

```javascript
const result = await poly.multiview.list();
console.log('Open windows:', result.windows);
```

### `poly.multiview.get(windowId)`

Gets information about a specific window.

**Returns:** `Promise<{id, title, views} | {error: string}>`

```javascript
const info = await poly.multiview.get(win.id);
console.log('Window title:', info.title);
console.log('Views:', info.views);
```

### IPC Between Views

Views can communicate with each other using `window.ipc.postMessage()`:

```javascript
// In UI view - navigate content view
function navigateTo(url) {
  window.ipc.postMessage(`navigate:content:${url}`);
}

// Usage
navigateTo('https://google.com');
```

**IPC Commands:**

| Command | Format | Description |
|---------|--------|-------------|
| Navigate | `navigate:viewId:url` | Navigate a view to URL |
| Minimize | `minimize` | Minimize window |
| Maximize | `maximize` | Toggle maximize |
| Close | `close` | Close window |
| Drag | `drag` | Start window drag |

### Complete Browser Example

**poly.toml:**
```toml
[package]
name = "My Browser"
version = "1.0.0"

[window]
width = 1200
height = 800
decorations = false
```

**main.js (app entry):**
```javascript
// Create browser window on startup
async function init() {
  const win = await poly.multiview.create({
    title: 'My Browser',
    width: 1200,
    height: 800,
    icon: 'assets/icon.png',
    views: [
      // Content area (bottom)
      { 
        id: 'content', 
        url: 'https://example.com',
        x: 0, y: 80, 
        width: 1200, height: 720 
      },
      // Browser UI (top)
      { 
        id: 'ui', 
        url: 'http://localhost:3000/ui.html',
        x: 0, y: 0, 
        width: 1200, height: 80 
      }
    ]
  });
}

init();
```

**ui.html (browser UI):**
```html
<!DOCTYPE html>
<html>
<head>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body { 
      background: #1a1a1f; 
      color: #fff; 
      font-family: system-ui;
      height: 80px;
    }
    .toolbar {
      display: flex;
      align-items: center;
      height: 100%;
      padding: 0 12px;
      gap: 8px;
    }
    .url-bar {
      flex: 1;
      height: 36px;
      background: #2a2a35;
      border: none;
      border-radius: 8px;
      padding: 0 12px;
      color: #fff;
    }
    .btn {
      width: 36px;
      height: 36px;
      background: transparent;
      border: none;
      color: #888;
      cursor: pointer;
      border-radius: 6px;
    }
    .btn:hover { background: rgba(255,255,255,0.1); color: #fff; }
  </style>
</head>
<body>
  <div class="toolbar" onmousedown="polyWindow.drag()">
    <button class="btn" onclick="goBack()">←</button>
    <button class="btn" onclick="goForward()">→</button>
    <button class="btn" onclick="refresh()">↻</button>
    <input type="text" class="url-bar" id="url" 
           placeholder="Enter URL..." 
           onkeydown="if(event.key==='Enter')navigate()">
    <button class="btn" onclick="navigate()">Go</button>
    <div style="width:20px"></div>
    <button class="btn" onclick="polyWindow.minimize()">─</button>
    <button class="btn" onclick="polyWindow.maximize()">□</button>
    <button class="btn" onclick="polyWindow.close()">✕</button>
  </div>

  <script>
    function navigate() {
      let url = document.getElementById('url').value;
      if (!url.startsWith('http')) url = 'https://' + url;
      window.ipc.postMessage('navigate:content:' + url);
    }
    
    function goBack() {
      // Would need history tracking
    }
    
    function goForward() {
      // Would need history tracking
    }
    
    function refresh() {
      const url = document.getElementById('url').value;
      if (url) window.ipc.postMessage('navigate:content:' + url);
    }
  </script>
</body>
</html>
```

---

## Known Issues

### Multi-WebView Z-Order (Windows)

WebViews created via `poly.webview.create()` always appear on top of existing WebViews. There is no way to control the stacking order. This is a limitation of the underlying WebView2 component on Windows.

**Impact:** For browser apps, create content WebView first, then UI WebView.

**Status:** Waiting for upstream fix in wry/WebView2.

### Window Shadow Pulsing (Windows)

On Windows with DWM composition, frameless windows may show pulsing shadows. This is a known WebView2/Windows issue.

**Workaround:** Use `decorations = true` (native titlebar) or accept the visual artifact.
