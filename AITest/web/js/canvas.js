// Canvas Mode - Code Editor with Monaco Editor
// Provides a side-by-side code editing experience

let canvasMode = false;
let canvasTabs = [];
let activeTabId = null;
let monacoEditor = null;
let monacoReady = false;
let diagnosticCounts = { errors: 0, warnings: 0 };

const CANVAS_FILE = 'aitest-canvas.json';

// Helper function
function escapeHtml(text) {
  if (!text) return '';
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// =============================================================================
// MONACO EDITOR SETUP
// =============================================================================

// Initialize Monaco
function initMonaco() {
  if (typeof require === 'undefined' || !require.config) {
    console.warn('[Canvas] Monaco loader not ready, retrying...');
    setTimeout(initMonaco, 100);
    return;
  }
  
  require.config({
    paths: {
      'vs': 'https://cdn.jsdelivr.net/npm/monaco-editor@0.45.0/min/vs'
    }
  });
  
  require(['vs/editor/editor.main'], function() {
    // Define custom dark theme
    monaco.editor.defineTheme('canvas-dark', {
      base: 'vs-dark',
      inherit: true,
      rules: [
        { token: 'comment', foreground: '6b7280' },
        { token: 'keyword', foreground: 'c084fc' },
        { token: 'string', foreground: '4ade80' },
        { token: 'number', foreground: 'fb923c' },
        { token: 'function', foreground: '60a5fa' }
      ],
      colors: {
        'editor.background': '#0d0d12',
        'editor.foreground': '#e4e4e7',
        'editor.lineHighlightBackground': '#ffffff08',
        'editor.selectionBackground': '#a78bfa33',
        'editorCursor.foreground': '#a78bfa',
        'editorLineNumber.foreground': '#52525b',
        'editorLineNumber.activeForeground': '#a1a1aa',
        'editor.inactiveSelectionBackground': '#a78bfa22',
        'editorError.foreground': '#f87171',
        'editorWarning.foreground': '#fbbf24',
        'editorInfo.foreground': '#60a5fa'
      }
    });
    
    // Configure JavaScript/TypeScript diagnostics
    monaco.languages.typescript.javascriptDefaults.setDiagnosticsOptions({
      noSemanticValidation: false,
      noSyntaxValidation: false,
      noSuggestionDiagnostics: false
    });
    
    monaco.languages.typescript.typescriptDefaults.setDiagnosticsOptions({
      noSemanticValidation: false,
      noSyntaxValidation: false,
      noSuggestionDiagnostics: false
    });
    
    // Configure compiler options for better checking
    monaco.languages.typescript.javascriptDefaults.setCompilerOptions({
      target: monaco.languages.typescript.ScriptTarget.ESNext,
      allowNonTsExtensions: true,
      moduleResolution: monaco.languages.typescript.ModuleResolutionKind.NodeJs,
      module: monaco.languages.typescript.ModuleKind.ESNext,
      noEmit: true,
      allowJs: true,
      checkJs: true
    });
    
    monaco.languages.typescript.typescriptDefaults.setCompilerOptions({
      target: monaco.languages.typescript.ScriptTarget.ESNext,
      allowNonTsExtensions: true,
      moduleResolution: monaco.languages.typescript.ModuleResolutionKind.NodeJs,
      module: monaco.languages.typescript.ModuleKind.ESNext,
      noEmit: true,
      strict: true
    });
    
    // Configure JSON validation
    monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
      validate: true,
      allowComments: true,
      schemaValidation: 'warning'
    });
    
    monacoReady = true;
    console.log('[Canvas] Monaco ready with diagnostics enabled');
    window.dispatchEvent(new Event('monaco-ready'));
  });
}

// Start loading Monaco
initMonaco();

// =============================================================================
// PERSISTENCE (using Poly FS)
// =============================================================================

async function saveCanvasState() {
  try {
    // Save current tab's code first
    if (activeTabId && monacoEditor) {
      const currentTab = canvasTabs.find(t => t.id === activeTabId);
      if (currentTab) {
        currentTab.code = monacoEditor.getValue();
      }
    }
    
    const state = {
      tabs: canvasTabs,
      activeTabId: activeTabId
    };
    
    if (typeof poly !== 'undefined' && poly.fs) {
      await poly.fs.write(CANVAS_FILE, JSON.stringify(state, null, 2));
    }
  } catch (e) {
    console.error('Failed to save canvas state:', e);
  }
}

async function loadCanvasState() {
  try {
    if (typeof poly === 'undefined' || !poly.fs) {
      return false;
    }
    
    const exists = await poly.fs.exists(CANVAS_FILE);
    if (exists) {
      const content = await poly.fs.read(CANVAS_FILE);
      const state = JSON.parse(content);
      if (state.tabs && state.tabs.length > 0) {
        canvasTabs = state.tabs;
        activeTabId = state.activeTabId;
        return true;
      }
    }
  } catch (e) {
    console.error('Failed to load canvas state:', e);
  }
  return false;
}

// =============================================================================
// EDITOR MANAGEMENT
// =============================================================================

function createEditor(container, code = '', language = 'javascript') {
  if (!monacoReady) {
    console.warn('[Canvas] Monaco not ready, waiting...');
    window.addEventListener('monaco-ready', () => {
      createEditor(container, code, language);
    }, { once: true });
    return;
  }
  
  // Destroy existing editor
  if (monacoEditor) {
    monacoEditor.dispose();
    monacoEditor = null;
  }
  
  // Clear container
  container.innerHTML = '';
  
  // Create editor
  monacoEditor = monaco.editor.create(container, {
    value: code,
    language: getMonacoLanguage(language),
    theme: 'canvas-dark',
    fontSize: 13,
    fontFamily: '"JetBrains Mono", "Fira Code", "SF Mono", Consolas, monospace',
    lineNumbers: 'on',
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    automaticLayout: true,
    tabSize: 2,
    wordWrap: 'on',
    padding: { top: 12, bottom: 12 },
    renderLineHighlight: 'line',
    cursorBlinking: 'smooth',
    smoothScrolling: true,
    contextmenu: true,
    folding: true,
    lineDecorationsWidth: 8,
    lineNumbersMinChars: 3,
    // Enable all validation features
    glyphMargin: true,
    lightbulb: { enabled: true },
    quickSuggestions: true,
    suggestOnTriggerCharacters: true,
    parameterHints: { enabled: true },
    hover: { enabled: true, delay: 300 },
    formatOnPaste: true,
    formatOnType: true
  });
  
  // Listen for changes
  monacoEditor.onDidChangeModelContent(() => {
    onEditorChange();
  });
  
  // Listen for diagnostics/markers changes
  monaco.editor.onDidChangeMarkers((uris) => {
    const model = monacoEditor.getModel();
    if (model && uris.some(uri => uri.toString() === model.uri.toString())) {
      updateDiagnosticDisplay();
    }
  });
  
  // Initial diagnostic update
  setTimeout(updateDiagnosticDisplay, 500);
  
  console.log('[Canvas] Editor created with language:', language);
}

// Update the diagnostic display in header
function updateDiagnosticDisplay() {
  if (!monacoEditor) return;
  
  const model = monacoEditor.getModel();
  if (!model) return;
  
  const markers = monaco.editor.getModelMarkers({ resource: model.uri });
  
  let errors = 0;
  let warnings = 0;
  let infos = 0;
  
  markers.forEach(marker => {
    if (marker.severity === monaco.MarkerSeverity.Error) {
      errors++;
    } else if (marker.severity === monaco.MarkerSeverity.Warning) {
      warnings++;
    } else if (marker.severity === monaco.MarkerSeverity.Info) {
      infos++;
    }
  });
  
  diagnosticCounts = { errors, warnings, infos };
  
  // Update UI
  const display = document.getElementById('canvas-diagnostics');
  if (display) {
    if (errors === 0 && warnings === 0) {
      display.innerHTML = `
        <span class="diag-ok">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
            <polyline points="22 4 12 14.01 9 11.01"></polyline>
          </svg>
          <span>No issues</span>
        </span>
      `;
    } else {
      let html = '';
      if (errors > 0) {
        html += `<span class="diag-error">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="15" y1="9" x2="9" y2="15"></line>
            <line x1="9" y1="9" x2="15" y2="15"></line>
          </svg>
          <span>${errors}</span>
        </span>`;
      }
      if (warnings > 0) {
        html += `<span class="diag-warning">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
            <line x1="12" y1="9" x2="12" y2="13"></line>
            <line x1="12" y1="17" x2="12.01" y2="17"></line>
          </svg>
          <span>${warnings}</span>
        </span>`;
      }
      display.innerHTML = html;
    }
  }
}

function getMonacoLanguage(lang) {
  const map = {
    'js': 'javascript',
    'ts': 'typescript',
    'py': 'python',
    'rb': 'ruby',
    'rs': 'rust',
    'sh': 'shell',
    'yml': 'yaml',
    'md': 'markdown'
  };
  return map[lang] || lang;
}

function setEditorContent(code, language = 'javascript') {
  if (!monacoEditor) return;
  
  const model = monacoEditor.getModel();
  if (model) {
    monaco.editor.setModelLanguage(model, getMonacoLanguage(language));
    monacoEditor.setValue(code);
    // Update diagnostics after content change
    setTimeout(updateDiagnosticDisplay, 300);
  }
}

function getEditorContent() {
  if (!monacoEditor) return '';
  return monacoEditor.getValue();
}

let saveDebounceTimer = null;

function onEditorChange() {
  // Update current tab
  const tab = canvasTabs.find(t => t.id === activeTabId);
  if (tab && monacoEditor) {
    tab.code = monacoEditor.getValue();
  }
  
  // Debounced save
  clearTimeout(saveDebounceTimer);
  saveDebounceTimer = setTimeout(saveCanvasState, 500);
}

// =============================================================================
// CANVAS MODE TOGGLE
// =============================================================================

async function toggleCanvasMode() {
  canvasMode = !canvasMode;
  document.body.classList.toggle('canvas-mode', canvasMode);
  
  const status = document.getElementById('canvas-mode-status');
  if (status) {
    status.textContent = canvasMode ? 'On' : 'Off';
    status.classList.toggle('on', canvasMode);
  }
  
  if (canvasMode) {
    const hasState = await loadCanvasState();
    if (!hasState || canvasTabs.length === 0) {
      canvasTabs = [];
      // Don't create default tab - let user create their own file
      renderCanvasTabs();
      showNewFileInput(); // Show input immediately
    } else {
      renderCanvasTabs();
      if (activeTabId) {
        selectCanvasTab(activeTabId);
      } else if (canvasTabs.length > 0) {
        selectCanvasTab(canvasTabs[0].id);
      }
    }
  }
  
  lucide.createIcons();
  
  if (typeof saveSettings === 'function') {
    saveSettings();
  }
}

function closeCanvas() {
  canvasMode = false;
  document.body.classList.remove('canvas-mode');
  
  const status = document.getElementById('canvas-mode-status');
  if (status) {
    status.textContent = 'Off';
    status.classList.remove('on');
  }
  
  if (typeof saveSettings === 'function') {
    saveSettings();
  }
}


// =============================================================================
// NEW FILE INPUT
// =============================================================================

function showNewFileInput() {
  const dropdown = document.getElementById('new-file-dropdown');
  const input = document.getElementById('new-file-input');
  dropdown.classList.add('show');
  input.value = '';
  input.focus();
}

function hideNewFileInput() {
  const dropdown = document.getElementById('new-file-dropdown');
  dropdown.classList.remove('show');
}

function handleNewFileKey(event) {
  if (event.key === 'Enter') {
    event.preventDefault();
    createNewFile();
  } else if (event.key === 'Escape') {
    hideNewFileInput();
  }
}

function createNewFile() {
  const input = document.getElementById('new-file-input');
  let filename = input.value.trim();
  
  if (!filename) {
    input.focus();
    return;
  }
  
  // Add default extension if none provided
  if (!filename.includes('.')) {
    filename += '.txt';
  }
  
  hideNewFileInput();
  addCanvasTab(filename, '');
}

// =============================================================================
// TAB MANAGEMENT
// =============================================================================

function addCanvasTab(name, code = '', language = null) {
  const id = 'tab-' + Date.now();
  // Auto-detect language from filename if not provided
  const detectedLang = language || detectLanguageFromFilename(name);
  const tab = { id, name, code, language: detectedLang };
  canvasTabs.push(tab);
  
  renderCanvasTabs();
  selectCanvasTab(id);
  saveCanvasState();
  
  return id;
}

function removeCanvasTab(id) {
  const index = canvasTabs.findIndex(t => t.id === id);
  if (index === -1) return;
  
  canvasTabs.splice(index, 1);
  
  if (canvasTabs.length === 0) {
    // No tabs left - show new file input
    activeTabId = null;
    if (monacoEditor) {
      monacoEditor.setValue('');
    }
    showNewFileInput();
  } else if (activeTabId === id) {
    selectCanvasTab(canvasTabs[Math.max(0, index - 1)].id);
  }
  
  renderCanvasTabs();
  saveCanvasState();
}

function selectCanvasTab(id) {
  // Save current tab's code
  if (activeTabId && monacoEditor) {
    const currentTab = canvasTabs.find(t => t.id === activeTabId);
    if (currentTab) {
      currentTab.code = monacoEditor.getValue();
    }
  }
  
  activeTabId = id;
  const tab = canvasTabs.find(t => t.id === id);
  if (!tab) return;
  
  // Update editor
  const container = document.getElementById('canvas-editor');
  if (container) {
    if (!monacoEditor) {
      createEditor(container, tab.code, tab.language);
    } else {
      setEditorContent(tab.code, tab.language);
    }
  }
  
  // Update tab UI
  document.querySelectorAll('.canvas-tab').forEach(el => {
    el.classList.toggle('active', el.dataset.id === id);
  });
  
  saveCanvasState();
}

function renderCanvasTabs() {
  const container = document.getElementById('canvas-tabs');
  if (!container) return;
  
  container.innerHTML = canvasTabs.map(tab => {
    const iconClass = getFileIconClass(tab.name, tab.language);
    return `
      <button class="canvas-tab ${tab.id === activeTabId ? 'active' : ''}" 
              data-id="${tab.id}" 
              onclick="selectCanvasTab('${tab.id}')">
        <i class="${iconClass}"></i>
        <span>${escapeHtml(tab.name)}</span>
        <span class="canvas-tab-close" onclick="event.stopPropagation(); removeCanvasTab('${tab.id}')">
          <i data-lucide="x" class="icon" style="width:12px;height:12px"></i>
        </span>
      </button>
    `;
  }).join('');
  
  lucide.createIcons();
}

function getLanguageIcon(lang) {
  const icons = {
    javascript: 'file-code',
    typescript: 'file-code',
    python: 'file-code',
    html: 'file-code',
    css: 'file-code',
    json: 'file-json',
    default: 'file'
  };
  return icons[lang] || icons.default;
}

// =============================================================================
// LANGUAGE DETECTION
// =============================================================================

// Detect language from filename
function detectLanguageFromFilename(filename) {
  if (!filename) return 'plaintext';
  
  const ext = filename.split('.').pop()?.toLowerCase();
  return getLanguageFromExtension(ext);
}

// Get Monaco language ID from file extension
function getLanguageFromExtension(ext) {
  const map = {
    // JavaScript/TypeScript
    'js': 'javascript',
    'mjs': 'javascript',
    'cjs': 'javascript',
    'jsx': 'javascript',
    'ts': 'typescript',
    'tsx': 'typescript',
    'mts': 'typescript',
    'cts': 'typescript',
    
    // Web
    'html': 'html',
    'htm': 'html',
    'vue': 'html',
    'svelte': 'html',
    'css': 'css',
    'scss': 'scss',
    'sass': 'scss',
    'less': 'less',
    
    // Data
    'json': 'json',
    'jsonc': 'json',
    'json5': 'json',
    'yaml': 'yaml',
    'yml': 'yaml',
    'toml': 'ini',
    'xml': 'xml',
    'svg': 'xml',
    
    // Programming
    'py': 'python',
    'pyw': 'python',
    'rb': 'ruby',
    'rs': 'rust',
    'go': 'go',
    'java': 'java',
    'kt': 'kotlin',
    'kts': 'kotlin',
    'scala': 'scala',
    'c': 'c',
    'h': 'c',
    'cpp': 'cpp',
    'cc': 'cpp',
    'cxx': 'cpp',
    'hpp': 'cpp',
    'cs': 'csharp',
    'swift': 'swift',
    'php': 'php',
    'lua': 'lua',
    'r': 'r',
    'dart': 'dart',
    'ex': 'elixir',
    'exs': 'elixir',
    'erl': 'erlang',
    'clj': 'clojure',
    'hs': 'haskell',
    'fs': 'fsharp',
    'fsx': 'fsharp',
    'pl': 'perl',
    'pm': 'perl',
    
    // Shell/Scripts
    'sh': 'shell',
    'bash': 'shell',
    'zsh': 'shell',
    'fish': 'shell',
    'ps1': 'powershell',
    'psm1': 'powershell',
    'bat': 'bat',
    'cmd': 'bat',
    
    // Config
    'ini': 'ini',
    'conf': 'ini',
    'cfg': 'ini',
    'env': 'ini',
    'properties': 'ini',
    
    // Docs
    'md': 'markdown',
    'markdown': 'markdown',
    'mdx': 'markdown',
    'txt': 'plaintext',
    'log': 'plaintext',
    
    // Database
    'sql': 'sql',
    'mysql': 'sql',
    'pgsql': 'sql',
    
    // Other
    'dockerfile': 'dockerfile',
    'graphql': 'graphql',
    'gql': 'graphql',
    'proto': 'protobuf'
  };
  
  return map[ext] || 'plaintext';
}

// Get file icon class for language/file (uses Devicon)
function getFileIconClass(filename, language) {
  if (filename) {
    const ext = filename.split('.').pop()?.toLowerCase();
    const name = filename.toLowerCase();
    
    // Special filenames
    const specialFiles = {
      'dockerfile': 'devicon-docker-plain colored',
      'docker-compose.yml': 'devicon-docker-plain colored',
      'docker-compose.yaml': 'devicon-docker-plain colored',
      '.gitignore': 'devicon-git-plain colored',
      '.gitattributes': 'devicon-git-plain colored',
      'package.json': 'devicon-npm-original-wordmark colored',
      'tsconfig.json': 'devicon-typescript-plain colored',
      'readme.md': 'devicon-markdown-original',
      'cargo.toml': 'devicon-rust-original colored',
      'go.mod': 'devicon-go-original-wordmark colored',
      'requirements.txt': 'devicon-python-plain colored',
      'gemfile': 'devicon-ruby-plain colored'
    };
    
    if (specialFiles[name]) return specialFiles[name];
    
    // Extension-based icons
    const extIcons = {
      'js': 'devicon-javascript-plain colored',
      'mjs': 'devicon-javascript-plain colored',
      'cjs': 'devicon-javascript-plain colored',
      'jsx': 'devicon-react-original colored',
      'ts': 'devicon-typescript-plain colored',
      'tsx': 'devicon-react-original colored',
      'vue': 'devicon-vuejs-plain colored',
      'svelte': 'devicon-svelte-plain colored',
      'html': 'devicon-html5-plain colored',
      'htm': 'devicon-html5-plain colored',
      'css': 'devicon-css3-plain colored',
      'scss': 'devicon-sass-original colored',
      'sass': 'devicon-sass-original colored',
      'json': 'devicon-json-plain colored',
      'yaml': 'devicon-yaml-plain colored',
      'yml': 'devicon-yaml-plain colored',
      'xml': 'devicon-xml-plain colored',
      'py': 'devicon-python-plain colored',
      'rb': 'devicon-ruby-plain colored',
      'rs': 'devicon-rust-original colored',
      'go': 'devicon-go-original-wordmark colored',
      'java': 'devicon-java-plain colored',
      'kt': 'devicon-kotlin-plain colored',
      'kts': 'devicon-kotlin-plain colored',
      'c': 'devicon-c-original colored',
      'h': 'devicon-c-original colored',
      'cpp': 'devicon-cplusplus-plain colored',
      'cc': 'devicon-cplusplus-plain colored',
      'cs': 'devicon-csharp-plain colored',
      'swift': 'devicon-swift-plain colored',
      'php': 'devicon-php-plain colored',
      'sh': 'devicon-bash-plain colored',
      'bash': 'devicon-bash-plain colored',
      'zsh': 'devicon-bash-plain colored',
      'md': 'devicon-markdown-original',
      'sql': 'devicon-mysql-plain colored',
      'lua': 'devicon-lua-plain colored',
      'r': 'devicon-r-plain colored',
      'dart': 'devicon-dart-plain colored',
      'ex': 'devicon-elixir-plain colored',
      'exs': 'devicon-elixir-plain colored',
      'scala': 'devicon-scala-plain colored',
      'hs': 'devicon-haskell-plain colored',
      'clj': 'devicon-clojure-plain colored',
      'erl': 'devicon-erlang-plain colored',
      'graphql': 'devicon-graphql-plain colored',
      'gql': 'devicon-graphql-plain colored'
    };
    
    if (ext && extIcons[ext]) return extIcons[ext];
  }
  
  // Fallback to language-based icon
  const langIcons = {
    'javascript': 'devicon-javascript-plain colored',
    'typescript': 'devicon-typescript-plain colored',
    'python': 'devicon-python-plain colored',
    'html': 'devicon-html5-plain colored',
    'css': 'devicon-css3-plain colored',
    'json': 'devicon-json-plain colored',
    'markdown': 'devicon-markdown-original',
    'rust': 'devicon-rust-original colored',
    'go': 'devicon-go-original-wordmark colored',
    'java': 'devicon-java-plain colored',
    'kotlin': 'devicon-kotlin-plain colored',
    'cpp': 'devicon-cplusplus-plain colored',
    'c': 'devicon-c-original colored',
    'csharp': 'devicon-csharp-plain colored',
    'ruby': 'devicon-ruby-plain colored',
    'php': 'devicon-php-plain colored',
    'swift': 'devicon-swift-plain colored',
    'shell': 'devicon-bash-plain colored',
    'sql': 'devicon-mysql-plain colored',
    'yaml': 'devicon-yaml-plain colored'
  };
  
  if (language && langIcons[language]) return langIcons[language];
  
  // Default file icon
  return 'devicon-devicon-plain';
}

// =============================================================================
// CODE EXECUTION
// =============================================================================

async function runCanvasCode() {
  const code = getEditorContent();
  const tab = canvasTabs.find(t => t.id === activeTabId);
  const lang = tab?.language || 'javascript';
  
  // Create or show output panel
  let outputWrap = document.getElementById('canvas-output-wrap');
  if (!outputWrap) {
    outputWrap = document.createElement('div');
    outputWrap.id = 'canvas-output-wrap';
    outputWrap.className = 'canvas-output-wrap';
    outputWrap.innerHTML = `
      <div class="canvas-output-header" onclick="toggleCanvasOutput()">
        <span>Output</span>
        <i data-lucide="chevron-down" class="icon"></i>
      </div>
      <pre class="canvas-output" id="canvas-output"></pre>
    `;
    document.querySelector('.canvas-editor-wrap').appendChild(outputWrap);
    lucide.createIcons();
  }
  
  outputWrap.classList.remove('collapsed');
  const output = document.getElementById('canvas-output');
  output.innerHTML = '<span class="output-info">Running...</span>';
  
  try {
    if (lang === 'javascript' || lang === 'js') {
      await runJavaScript(code, output);
    } else if (lang === 'python' || lang === 'py') {
      await runPython(code, output);
    } else {
      output.innerHTML = `<span class="output-info">Language "${lang}" not supported yet.</span>`;
    }
  } catch (e) {
    output.innerHTML = `<span class="output-error">Error: ${escapeHtml(e.message)}</span>`;
  }
}

async function runJavaScript(code, outputEl) {
  const logs = [];
  const origLog = console.log;
  const origError = console.error;
  
  console.log = (...args) => { logs.push(formatArgs(args)); origLog.apply(console, args); };
  console.error = (...args) => { logs.push(`<span class="output-error">${formatArgs(args)}</span>`); origError.apply(console, args); };
  
  try {
    const result = new Function(code)();
    if (logs.length > 0) {
      outputEl.innerHTML = logs.join('\n');
    } else if (result !== undefined) {
      outputEl.innerHTML = `<span class="output-success">${escapeHtml(JSON.stringify(result, null, 2))}</span>`;
    } else {
      outputEl.innerHTML = '<span class="output-success">✓ Done</span>';
    }
  } finally {
    console.log = origLog;
    console.error = origError;
  }
}

async function runPython(code, outputEl) {
  if (typeof poly !== 'undefined' && poly.shell) {
    try {
      const escaped = code.replace(/\\/g, '\\\\').replace(/"/g, '\\"').replace(/\n/g, '\\n');
      const result = await poly.shell.run(`python3 -c "${escaped}"`);
      outputEl.innerHTML = result.error 
        ? `<span class="output-error">${escapeHtml(result.error)}</span>`
        : `<span class="output-success">${escapeHtml(result.output || '✓ Done')}</span>`;
      return;
    } catch (e) {}
  }
  outputEl.innerHTML = '<span class="output-info">Python requires Poly shell.</span>';
}

function formatArgs(args) {
  return args.map(a => a === undefined ? 'undefined' : a === null ? 'null' : typeof a === 'object' ? JSON.stringify(a, null, 2) : String(a)).join(' ');
}

// =============================================================================
// UTILITIES
// =============================================================================

function copyCanvasCode() {
  navigator.clipboard.writeText(getEditorContent()).then(() => {
    const btn = document.querySelector('.canvas-action-btn[onclick*="copyCanvasCode"]');
    if (btn) {
      btn.innerHTML = '<i data-lucide="check" class="icon"></i>';
      lucide.createIcons();
      setTimeout(() => { btn.innerHTML = '<i data-lucide="copy" class="icon"></i>'; lucide.createIcons(); }, 1500);
    }
  });
}

function toggleCanvasOutput() {
  document.getElementById('canvas-output-wrap')?.classList.toggle('collapsed');
}

function setCanvasCode(code, language = 'javascript', name = null) {
  if (!canvasMode) toggleCanvasMode();
  
  const tabName = name || `code.${getFileExtension(language)}`;
  const existingTab = canvasTabs.find(t => t.id === activeTabId);
  
  if (existingTab && existingTab.code.trim() === '') {
    existingTab.code = code;
    existingTab.language = language;
    existingTab.name = tabName;
    renderCanvasTabs();
    setEditorContent(code, language);
  } else {
    addCanvasTab(tabName, code, language);
  }
  
  saveCanvasState();
}

// =============================================================================
// STREAMING API
// =============================================================================

let streamingTabId = null;

function startCanvasStream(language = 'javascript', name = null) {
  if (!canvasMode) toggleCanvasMode();
  
  const tabName = name || `code.${getFileExtension(language)}`;
  const existingTab = canvasTabs.find(t => t.id === activeTabId);
  
  if (existingTab && existingTab.code.trim() === '') {
    existingTab.code = '';
    existingTab.language = language;
    existingTab.name = tabName;
    streamingTabId = existingTab.id;
    renderCanvasTabs();
  } else {
    streamingTabId = addCanvasTab(tabName, '', language);
  }
  
  if (monacoEditor) monacoEditor.setValue('');
  return streamingTabId;
}

function appendCanvasStream(chunk) {
  if (!streamingTabId) return;
  
  const tab = canvasTabs.find(t => t.id === streamingTabId);
  if (!tab) return;
  
  tab.code += chunk;
  
  if (activeTabId === streamingTabId && monacoEditor) {
    const model = monacoEditor.getModel();
    const lastLine = model.getLineCount();
    const lastCol = model.getLineMaxColumn(lastLine);
    monacoEditor.executeEdits('stream', [{
      range: new monaco.Range(lastLine, lastCol, lastLine, lastCol),
      text: chunk
    }]);
    monacoEditor.revealLine(model.getLineCount());
  }
}

function endCanvasStream() {
  if (streamingTabId) {
    saveCanvasState();
    streamingTabId = null;
  }
}

function getFileExtension(lang) {
  const ext = { javascript: 'js', typescript: 'ts', python: 'py', html: 'html', css: 'css', json: 'json', rust: 'rs', java: 'java', cpp: 'cpp', sql: 'sql', bash: 'sh', yaml: 'yml' };
  return ext[lang] || 'txt';
}

// =============================================================================
// TOOL API (for AI)
// =============================================================================

function readCanvas() {
  if (!canvasMode || canvasTabs.length === 0) return { success: false, error: 'Canvas not active' };
  const tab = canvasTabs.find(t => t.id === activeTabId);
  if (!tab) return { success: false, error: 'No active tab' };
  return { success: true, filename: tab.name, language: tab.language, code: getEditorContent() };
}

function writeCanvas(code, language, filename) {
  setCanvasCode(code, language || 'javascript', filename);
  return { success: true };
}

function editCanvas(startLine, endLine, newCode) {
  if (!monacoEditor) return { success: false, error: 'Editor not ready' };
  
  const model = monacoEditor.getModel();
  if (!model) return { success: false, error: 'No model' };
  
  // Validate line numbers
  const totalLines = model.getLineCount();
  if (startLine < 1 || endLine < 1 || startLine > totalLines + 1) {
    return { success: false, error: `Invalid line range: ${startLine}-${endLine}, file has ${totalLines} lines` };
  }
  
  // Ensure newCode ends with newline if replacing whole lines
  let code = newCode || '';
  if (code && !code.endsWith('\n') && endLine < totalLines) {
    code += '\n';
  }
  
  // Handle escaped newlines (in case model sends \\n instead of actual newlines)
  code = code.replace(/\\n/g, '\n');
  
  console.log('[Canvas] Edit:', startLine, '-', endLine, 'with', code.length, 'chars');
  
  const range = new monaco.Range(
    startLine, 
    1, 
    endLine, 
    model.getLineMaxColumn(Math.min(endLine, totalLines))
  );
  
  monacoEditor.executeEdits('ai-edit', [{ range, text: code }]);
  saveCanvasState();
  
  // Update diagnostics after edit
  setTimeout(updateDiagnosticDisplay, 300);
  
  return { success: true, startLine, endLine, charsWritten: code.length };
}

// =============================================================================
// EXPORTS
// =============================================================================

window.Canvas = {
  toggle: toggleCanvasMode, close: closeCanvas, setCode: setCanvasCode, getCode: getEditorContent,
  addTab: addCanvasTab, run: runCanvasCode, isActive: () => canvasMode,
  startStream: startCanvasStream, appendStream: appendCanvasStream, endStream: endCanvasStream,
  read: readCanvas, write: writeCanvas, edit: editCanvas
};

window.setCanvasCode = setCanvasCode;
window.toggleCanvasMode = toggleCanvasMode;
window.closeCanvas = closeCanvas;
window.runCanvasCode = runCanvasCode;
window.copyCanvasCode = copyCanvasCode;
window.selectCanvasTab = selectCanvasTab;
window.removeCanvasTab = removeCanvasTab;
window.addCanvasTab = addCanvasTab;
window.startCanvasStream = startCanvasStream;
window.appendCanvasStream = appendCanvasStream;
window.endCanvasStream = endCanvasStream;
window.readCanvas = readCanvas;
window.toggleCanvasOutput = toggleCanvasOutput;
window.showNewFileInput = showNewFileInput;
window.hideNewFileInput = hideNewFileInput;
window.handleNewFileKey = handleNewFileKey;
window.createNewFile = createNewFile;
window.updateDiagnosticDisplay = updateDiagnosticDisplay;

// Expose editor and state for tools
window.monacoEditor = null;
window.canvasTabs = canvasTabs;
window.activeTabId = activeTabId;

// Keep references updated
Object.defineProperty(window, 'monacoEditor', {
  get: () => monacoEditor,
  set: (v) => { monacoEditor = v; }
});
Object.defineProperty(window, 'canvasTabs', {
  get: () => canvasTabs
});
Object.defineProperty(window, 'activeTabId', {
  get: () => activeTabId
});
