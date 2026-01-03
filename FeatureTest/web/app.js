// Poly 0.3.1 Feature Test App

let createdWindowId = null;

// Check if running in native mode - polyWindow is injected by Poly
function isNative() {
  return typeof polyWindow !== 'undefined' || 
         (typeof window.poly !== 'undefined' && window.poly.window);
}

// Logging
function log(message, type = 'info') {
  const logEl = document.getElementById('log');
  const time = new Date().toLocaleTimeString();
  const entry = document.createElement('div');
  entry.className = `log-entry ${type}`;
  entry.innerHTML = `<span class="time">${time}</span>${message}`;
  logEl.appendChild(entry);
  logEl.scrollTop = logEl.scrollHeight;
  console.log(`[${type}] ${message}`);
}

function clearLog() {
  document.getElementById('log').innerHTML = '';
  log('Log cleared', 'info');
}

function showResult(elementId, content, isError = false) {
  const el = document.getElementById(elementId);
  el.innerHTML = isError 
    ? `<span class="error">${content}</span>`
    : `<span class="success">${content}</span>`;
}

// Test: Configuration
async function testConfig() {
  log('Testing configuration...', 'info');
  try {
    const version = await poly.app.getVersion();
    const name = await poly.app.getName();
    const native = isNative();
    
    showResult('config-results', `Version: ${version}\nApp: ${name}\nMode: ${native ? 'Native' : 'Dev Server'}\npolyWindow: ${typeof polyWindow}`);
    log(`✓ Config loaded - Version: ${version}, Name: ${name}, Native: ${native}`, 'success');
  } catch (e) {
    showResult('config-results', `Error: ${e.message}`, true);
    log(`✗ Config test failed: ${e.message}`, 'error');
  }
}

// Test: Icon
function checkIcon() {
  if (isNative()) {
    log('Check the window titlebar and taskbar for the Poly icon!', 'info');
    showResult('icon-results', 'Look at:\n• Window titlebar (top-left)\n• Taskbar icon\n\nBoth should show the Poly logo!');
  } else {
    log('Icon test requires Native Mode (poly run --native)', 'info');
    showResult('icon-results', '⚠️ Run with: poly run --native\nto see the window icon');
  }
}

// Test: Multi-Window
async function testMultiWindow() {
  log('Creating new window...', 'info');
  try {
    const win = await poly.windows.create({
      title: 'Test Window',
      width: 500,
      height: 400,
      decorations: false,
      html: `<!DOCTYPE html>
<html>
<head>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body { background: #1a1a2e; color: #fff; font-family: system-ui; }
    .titlebar { 
      height: 32px; background: #16213e; display: flex;
      justify-content: space-between; align-items: center; padding: 0 12px;
    }
    .titlebar-buttons button {
      background: none; border: none; color: #888; cursor: pointer; padding: 4px 8px;
    }
    .titlebar-buttons button:hover { color: #fff; }
    .content { padding: 24px; text-align: center; }
    h1 { color: #4ade80; margin-bottom: 16px; }
    p { color: #888; margin-bottom: 16px; }
    .window-id { font-family: monospace; background: #0f0f1a; padding: 8px; border-radius: 4px; }
  </style>
</head>
<body>
  <div class="titlebar" onmousedown="polyWindow.drag()">
    <span>Test Window</span>
    <div class="titlebar-buttons">
      <button onclick="polyWindow.minimize()">─</button>
      <button onclick="polyWindow.maximize()">□</button>
      <button onclick="polyWindow.close()">✕</button>
    </div>
  </div>
  <div class="content">
    <h1>✓ Window Created!</h1>
    <p>This window was created via poly.windows.create()</p>
    <p class="window-id">Window ID: ${Date.now()}</p>
    <p style="margin-top: 24px; font-size: 0.8rem; color: #666;">
      Try dragging, minimizing, maximizing, and closing this window.
    </p>
  </div>
</body>
</html>`
    });
    
    createdWindowId = win.id;
    showResult('window-results', `✓ Window created!\nID: ${win.id}`);
    log(`✓ Window created with ID: ${win.id}`, 'success');
    
    // Get window count
    const count = await poly.windows.count();
    log(`Total windows: ${count}`, 'info');
    
  } catch (e) {
    showResult('window-results', `Error: ${e.message}`, true);
    log(`✗ Multi-window test failed: ${e.message}`, 'error');
  }
}

// Test: Window Control - these always try to use polyWindow
async function testMinimize() {
  log('Minimizing main window...', 'info');
  try {
    if (typeof polyWindow !== 'undefined') {
      polyWindow.minimize();
      log('✓ Minimize command sent via polyWindow', 'success');
      showResult('control-results', '✓ Minimized');
    } else if (typeof poly !== 'undefined' && poly.window && poly.window.minimize) {
      await poly.window.minimize();
      log('✓ Minimize command sent via poly.window', 'success');
      showResult('control-results', '✓ Minimized');
    } else {
      throw new Error('No window control API available');
    }
  } catch (e) {
    showResult('control-results', `Error: ${e.message}`, true);
    log(`✗ Minimize failed: ${e.message}`, 'error');
  }
}

async function testMaximize() {
  log('Toggling maximize...', 'info');
  try {
    if (typeof polyWindow !== 'undefined') {
      polyWindow.maximize();
      log('✓ Maximize command sent via polyWindow', 'success');
      showResult('control-results', '✓ Maximized/Restored');
    } else if (typeof poly !== 'undefined' && poly.window && poly.window.maximize) {
      await poly.window.maximize();
      log('✓ Maximize command sent via poly.window', 'success');
      showResult('control-results', '✓ Maximized/Restored');
    } else {
      throw new Error('No window control API available');
    }
  } catch (e) {
    showResult('control-results', `Error: ${e.message}`, true);
    log(`✗ Maximize failed: ${e.message}`, 'error');
  }
}

async function testRestore() {
  log('Restoring window...', 'info');
  try {
    if (typeof polyWindow !== 'undefined' && polyWindow.restore) {
      polyWindow.restore();
      log('✓ Restore command sent via polyWindow', 'success');
      showResult('control-results', '✓ Restored');
    } else if (typeof poly !== 'undefined' && poly.window && poly.window.restore) {
      await poly.window.restore();
      log('✓ Restore command sent via poly.window', 'success');
      showResult('control-results', '✓ Restored');
    } else {
      // Fallback: unminimize + unmaximize
      if (typeof polyWindow !== 'undefined') {
        polyWindow.maximize(); // Toggle to restore
        log('✓ Restore via maximize toggle', 'success');
        showResult('control-results', '✓ Restored (via toggle)');
      } else {
        throw new Error('No restore API available');
      }
    }
  } catch (e) {
    showResult('control-results', `Error: ${e.message}`, true);
    log(`✗ Restore failed: ${e.message}`, 'error');
  }
}

// ============================================
// Window Manipulation Tests
// ============================================

async function testSetTitle() {
  if (!createdWindowId) {
    log('Create a window first!', 'error');
    showResult('manipulation-results', '⚠️ Create a window first', true);
    return;
  }
  
  const newTitle = 'Updated Title - ' + new Date().toLocaleTimeString();
  log(`Setting window ${createdWindowId} title to: ${newTitle}`, 'info');
  
  try {
    await poly.windows.setTitle(createdWindowId, newTitle);
    log('✓ Title updated!', 'success');
    showResult('manipulation-results', `✓ Title set to:\n"${newTitle}"`);
  } catch (e) {
    log(`✗ Set title failed: ${e.message}`, 'error');
    showResult('manipulation-results', `Error: ${e.message}`, true);
  }
}

async function testSetSize() {
  if (!createdWindowId) {
    log('Create a window first!', 'error');
    showResult('manipulation-results', '⚠️ Create a window first', true);
    return;
  }
  
  // Toggle between two sizes
  const sizes = [[400, 300], [600, 500]];
  const randomSize = sizes[Math.floor(Math.random() * sizes.length)];
  
  log(`Resizing window ${createdWindowId} to ${randomSize[0]}x${randomSize[1]}`, 'info');
  
  try {
    await poly.windows.setSize(createdWindowId, randomSize[0], randomSize[1]);
    log('✓ Size updated!', 'success');
    showResult('manipulation-results', `✓ Size: ${randomSize[0]}x${randomSize[1]}`);
  } catch (e) {
    log(`✗ Set size failed: ${e.message}`, 'error');
    showResult('manipulation-results', `Error: ${e.message}`, true);
  }
}

async function testSetPosition() {
  if (!createdWindowId) {
    log('Create a window first!', 'error');
    showResult('manipulation-results', '⚠️ Create a window first', true);
    return;
  }
  
  // Random position
  const x = Math.floor(Math.random() * 500) + 100;
  const y = Math.floor(Math.random() * 300) + 100;
  
  log(`Moving window ${createdWindowId} to (${x}, ${y})`, 'info');
  
  try {
    await poly.windows.setPosition(createdWindowId, x, y);
    log('✓ Position updated!', 'success');
    showResult('manipulation-results', `✓ Position: (${x}, ${y})`);
  } catch (e) {
    log(`✗ Set position failed: ${e.message}`, 'error');
    showResult('manipulation-results', `Error: ${e.message}`, true);
  }
}

async function testNavigate() {
  if (!createdWindowId) {
    log('Create a window first!', 'error');
    showResult('manipulation-results', '⚠️ Create a window first', true);
    return;
  }
  
  const url = 'https://example.com';
  log(`Navigating window ${createdWindowId} to ${url}`, 'info');
  
  try {
    await poly.windows.navigate(createdWindowId, url);
    log('✓ Navigation started!', 'success');
    showResult('manipulation-results', `✓ Navigated to:\n${url}`);
  } catch (e) {
    log(`✗ Navigate failed: ${e.message}`, 'error');
    showResult('manipulation-results', `Error: ${e.message}`, true);
  }
}

async function testEvalScript() {
  if (!createdWindowId) {
    log('Create a window first!', 'error');
    showResult('manipulation-results', '⚠️ Create a window first', true);
    return;
  }
  
  const script = `
    document.body.style.background = 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)';
    document.body.innerHTML = '<div style="display:flex;align-items:center;justify-content:center;height:100vh;color:white;font-family:system-ui;font-size:24px;text-align:center;">✨ Modified via eval! ✨<br><small style="font-size:14px;opacity:0.7;">JavaScript was executed in this window</small></div>';
  `;
  
  log(`Executing script in window ${createdWindowId}`, 'info');
  
  try {
    await poly.windows.eval(createdWindowId, script);
    log('✓ Script executed!', 'success');
    showResult('manipulation-results', '✓ Script executed!\nCheck the other window');
  } catch (e) {
    log(`✗ Eval failed: ${e.message}`, 'error');
    showResult('manipulation-results', `Error: ${e.message}`, true);
  }
}

// Test: Window State
async function testWindowState() {
  log('Getting window state...', 'info');
  try {
    if (typeof poly !== 'undefined' && poly.window) {
      // Try extended APIs
      const results = [];
      
      try {
        const size = await poly.window.getSize();
        results.push(`Size: ${size.width}x${size.height}`);
      } catch (e) {
        results.push(`Size: ${window.innerWidth}x${window.innerHeight} (browser)`);
      }
      
      try {
        const pos = await poly.window.getPosition();
        results.push(`Position: (${pos.x}, ${pos.y})`);
      } catch (e) {
        results.push(`Position: N/A`);
      }
      
      try {
        const isMax = await poly.window.isMaximized();
        results.push(`Maximized: ${isMax}`);
      } catch (e) {}
      
      try {
        const isMin = await poly.window.isMinimized();
        results.push(`Minimized: ${isMin}`);
      } catch (e) {}
      
      try {
        const isFull = await poly.window.isFullscreen();
        results.push(`Fullscreen: ${isFull}`);
      } catch (e) {}
      
      showResult('state-results', results.join('\n'));
      log(`✓ Window state retrieved`, 'success');
    } else {
      // Dev mode - show browser info
      const state = `Browser Window:
Size: ${window.innerWidth}x${window.innerHeight}
Screen: ${screen.width}x${screen.height}
Mode: Dev Server`;
      showResult('state-results', state);
      log(`✓ Browser state retrieved (dev mode)`, 'success');
    }
  } catch (e) {
    showResult('state-results', `Error: ${e.message}`, true);
    log(`✗ Window state failed: ${e.message}`, 'error');
  }
}

// Test: System Info
async function testSystemInfo() {
  log('Getting system info...', 'info');
  try {
    const platform = await poly.os.platform();
    const arch = await poly.os.arch();
    const hostname = await poly.os.hostname();
    const homedir = await poly.os.homedir();
    
    const info = `Platform: ${platform}
Arch: ${arch}
Hostname: ${hostname}
Home: ${homedir}`;
    
    showResult('system-results', info);
    log(`✓ System info retrieved - ${platform} ${arch}`, 'success');
  } catch (e) {
    showResult('system-results', `Error: ${e.message}`, true);
    log(`✗ System info failed: ${e.message}`, 'error');
  }
}

// Initialize
document.addEventListener('DOMContentLoaded', () => {
  const native = isNative();
  log(`Poly 0.3.1 Feature Test App loaded`, 'success');
  log(`polyWindow available: ${typeof polyWindow !== 'undefined'}`, 'info');
  log(`poly.window available: ${typeof poly !== 'undefined' && poly.window ? 'yes' : 'no'}`, 'info');
  
  if (!native) {
    log('💡 For full testing, run: poly run --native', 'info');
  }
  
  log('Click the buttons to test each feature', 'info');
  
  // Auto-run config test
  setTimeout(testConfig, 500);
});
