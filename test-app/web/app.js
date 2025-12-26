function app() {
  return {
    // State
    testsRun: 0,
    
    // Clipboard
    clipboardText: '',
    clipboardResult: '',
    
    // Multi-Window
    windowTitle: 'New Window',
    windowResult: '',
    
    // File Dialog
    fileResult: '',
    
    // File System
    fsPath: 'test.txt',
    fsResult: '',
    
    // Dialog
    dialogResult: '',
    
    // AI
    aiResult: '',

    // ========== Clipboard Tests ==========
    async copyToClipboard() {
      try {
        await poly.clipboard.write(this.clipboardText || 'Hello from Poly!');
        this.clipboardResult = '✓ Copied to clipboard: ' + (this.clipboardText || 'Hello from Poly!');
        this.testsRun++;
      } catch (e) {
        this.clipboardResult = '✗ Error: ' + e.message;
      }
    },

    async pasteFromClipboard() {
      try {
        const text = await poly.clipboard.read();
        this.clipboardResult = '✓ Clipboard content: ' + text;
        this.clipboardText = text;
        this.testsRun++;
      } catch (e) {
        this.clipboardResult = '✗ Error: ' + e.message;
      }
    },

    async clearClipboard() {
      try {
        await poly.clipboard.clear();
        this.clipboardResult = '✓ Clipboard cleared';
        this.testsRun++;
      } catch (e) {
        this.clipboardResult = '✗ Error: ' + e.message;
      }
    },

    // ========== Multi-Window Tests ==========
    async createWindow() {
      try {
        const result = await poly.windows.create({
          title: this.windowTitle || 'Test Window',
          width: 500,
          height: 400,
          html: `
            <!DOCTYPE html>
            <html>
            <head>
              <style>
                * { margin: 0; padding: 0; box-sizing: border-box; }
                body { 
                  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                  color: white;
                  font-family: system-ui;
                  min-height: 100vh;
                }
                /* Custom Titlebar */
                .titlebar {
                  height: 32px; background: rgba(0,0,0,0.3); display: flex;
                  justify-content: space-between; align-items: center;
                  padding: 0 12px; -webkit-app-region: drag;
                }
                .titlebar-title { font-size: 12px; opacity: 0.8; }
                .titlebar-buttons { display: flex; gap: 4px; -webkit-app-region: no-drag; }
                .titlebar-btn {
                  width: 28px; height: 24px; border: none; background: transparent;
                  color: white; cursor: pointer; font-size: 12px; border-radius: 4px;
                  opacity: 0.7;
                }
                .titlebar-btn:hover { background: rgba(255,255,255,0.2); opacity: 1; }
                .titlebar-btn.close:hover { background: #e81123; }
                .content { 
                  display: flex; align-items: center; justify-content: center;
                  height: calc(100vh - 32px); text-align: center;
                }
                h1 { font-size: 32px; margin-bottom: 8px; }
                p { opacity: 0.8; }
                button {
                  margin-top: 16px; padding: 10px 20px; border: none;
                  background: rgba(255,255,255,0.2); color: white;
                  border-radius: 8px; cursor: pointer; font-size: 14px;
                }
                button:hover { background: rgba(255,255,255,0.3); }
              </style>
            </head>
            <body>
              <div class="titlebar" onmousedown="poly.window.drag()">
                <div class="titlebar-title">${this.windowTitle || 'Test Window'}</div>
                <div class="titlebar-buttons">
                  <button class="titlebar-btn" onclick="poly.window.minimize()">─</button>
                  <button class="titlebar-btn" onclick="poly.window.maximize()">□</button>
                  <button class="titlebar-btn close" onclick="poly.window.close()">✕</button>
                </div>
              </div>
              <div class="content">
                <div>
                  <h1>🎉 Custom Window!</h1>
                  <p>Frameless window with custom titlebar</p>
                  <p>Created via poly.windows.create()</p>
                  <button onclick="poly.window.close()">Close Window</button>
                </div>
              </div>
            </body>
            </html>
          `
        });
        this.windowResult = '✓ Window created with ID: ' + result.id;
        this.testsRun++;
      } catch (e) {
        this.windowResult = '✗ Error: ' + (e.message || e);
      }
    },

    async getWindowCount() {
      try {
        const count = await poly.windows.count();
        this.windowResult = '✓ Window count: ' + count;
        this.testsRun++;
      } catch (e) {
        this.windowResult = '✗ Error: ' + e.message;
      }
    },

    async listWindows() {
      try {
        const ids = await poly.windows.list();
        this.windowResult = '✓ Window IDs: ' + JSON.stringify(ids);
        this.testsRun++;
      } catch (e) {
        this.windowResult = '✗ Error: ' + e.message;
      }
    },

    async closeAllWindows() {
      try {
        await poly.windows.closeAll();
        this.windowResult = '✓ All windows closed';
        this.testsRun++;
      } catch (e) {
        this.windowResult = '✗ Error: ' + e.message;
      }
    },

    // ========== File Dialog Tests ==========
    async openFile() {
      try {
        const file = await poly.dialog.open({ title: 'Select a file' });
        this.fileResult = file ? '✓ Selected: ' + file : '✓ Cancelled';
        this.testsRun++;
      } catch (e) {
        this.fileResult = '✗ Error: ' + e.message;
      }
    },

    async saveFile() {
      try {
        const path = await poly.dialog.save({ 
          title: 'Save file',
          defaultName: 'test.txt'
        });
        this.fileResult = path ? '✓ Save to: ' + path : '✓ Cancelled';
        this.testsRun++;
      } catch (e) {
        this.fileResult = '✗ Error: ' + e.message;
      }
    },

    async pickFolder() {
      try {
        const folder = await poly.dialog.folder({ title: 'Select folder' });
        this.fileResult = folder ? '✓ Folder: ' + folder : '✓ Cancelled';
        this.testsRun++;
      } catch (e) {
        this.fileResult = '✗ Error: ' + e.message;
      }
    },

    // ========== File System Tests ==========
    async readFile() {
      try {
        const content = await poly.fs.read(this.fsPath);
        this.fsResult = '✓ Content: ' + content.substring(0, 100) + (content.length > 100 ? '...' : '');
        this.testsRun++;
      } catch (e) {
        this.fsResult = '✗ Error: ' + e.message;
      }
    },

    async writeFile() {
      try {
        await poly.fs.write(this.fsPath, 'Hello from Poly! Written at: ' + new Date().toISOString());
        this.fsResult = '✓ Written to: ' + this.fsPath;
        this.testsRun++;
      } catch (e) {
        this.fsResult = '✗ Error: ' + e.message;
      }
    },

    async checkExists() {
      try {
        const exists = await poly.fs.exists(this.fsPath);
        this.fsResult = exists ? '✓ File exists' : '✓ File does not exist';
        this.testsRun++;
      } catch (e) {
        this.fsResult = '✗ Error: ' + e.message;
      }
    },

    // ========== Dialog Tests ==========
    async showMessage() {
      try {
        await poly.dialog.message('Test Message', 'This is a test message from Poly!', 'info');
        this.dialogResult = '✓ Message shown';
        this.testsRun++;
      } catch (e) {
        this.dialogResult = '✗ Error: ' + e.message;
      }
    },

    async showConfirm() {
      try {
        const result = await poly.dialog.confirm('Confirm Test', 'Do you want to continue?');
        this.dialogResult = '✓ Confirm result: ' + (result ? 'Yes' : 'No');
        this.testsRun++;
      } catch (e) {
        this.dialogResult = '✗ Error: ' + e.message;
      }
    },

    async showCustom() {
      try {
        const result = await poly.dialog.custom({
          type: 'warning',
          title: 'Custom Dialog',
          message: 'This is a custom dialog with multiple buttons!',
          buttons: [
            { text: 'Cancel', value: 'cancel' },
            { text: 'Maybe', value: 'maybe' },
            { text: 'OK', value: 'ok', primary: true }
          ]
        });
        this.dialogResult = '✓ Custom result: ' + result;
        this.testsRun++;
      } catch (e) {
        this.dialogResult = '✗ Error: ' + e.message;
      }
    },

    // ========== AI Tests ==========
    async checkOllama() {
      try {
        // This would need the AI API to be exposed
        this.aiResult = '⚠ AI API test - check console for poly.ai availability';
        console.log('poly.ai:', typeof poly.ai);
        this.testsRun++;
      } catch (e) {
        this.aiResult = '✗ Error: ' + e.message;
      }
    },

    async listModels() {
      try {
        this.aiResult = '⚠ AI API test - Ollama models would be listed here';
        this.testsRun++;
      } catch (e) {
        this.aiResult = '✗ Error: ' + e.message;
      }
    }
  };
}
