// AI Chat - Poly AI Test App

const chatMessages = document.getElementById('chat-messages');
const userInput = document.getElementById('user-input');
const sendBtn = document.getElementById('send-btn');
const modelsCount = document.getElementById('models-count');
const modelBtn = document.getElementById('model-btn');
const modelName = document.getElementById('model-name');
const modelMenu = document.getElementById('model-menu');
const modelList = document.getElementById('model-list');
const toolsBtn = document.getElementById('tools-btn');
const toolsMenu = document.getElementById('tools-menu');
const thinkStatus = document.getElementById('think-status');
const streamStatus = document.getElementById('stream-status');
const inputIsland = document.getElementById('input-island');
const sidebar = document.getElementById('sidebar');
const chatHistory = document.getElementById('chat-history');

let conversationHistory = [];
let selectedModel = null;
let models = [];
let thinkMode = false;
let streamMode = true;
let webSearchMode = false;
let chats = [];
let currentChatId = null;

// =============================================================================
// SETTINGS PERSISTENCE (using Poly File System API)
// =============================================================================

const SETTINGS_FILE = 'aitest-settings.json';
const CHATS_FILE = 'aitest-chats.json';

// Settings are loaded async, so we need to track if they're ready
let settingsLoaded = false;

async function loadSettings() {
  try {
    // Check if Poly is available
    if (typeof poly === 'undefined' || !poly.fs) {
      console.log('[Settings] Poly FS not available, using defaults');
      settingsLoaded = true;
      return;
    }
    
    // Check if settings file exists
    const exists = await poly.fs.exists(SETTINGS_FILE);
    console.log('[Settings] File exists:', exists);
    
    if (exists) {
      const content = await poly.fs.read(SETTINGS_FILE);
      console.log('[Settings] Loading:', content);
      const settings = JSON.parse(content);
      
      thinkMode = settings.thinkMode ?? false;
      streamMode = settings.streamMode ?? true;
      webSearchMode = settings.webSearchMode ?? false;
      selectedModel = settings.selectedModel || null;
      currentChatId = settings.currentChatId || null;
      
      console.log('[Settings] Loaded - thinkMode:', thinkMode, 'streamMode:', streamMode, 'selectedModel:', selectedModel, 'currentChatId:', currentChatId);
      
      // Canvas mode - just restore the setting, don't activate yet
      if (settings.canvasMode && typeof toggleCanvasMode === 'function') {
        setTimeout(() => {
          if (settings.canvasMode) toggleCanvasMode();
        }, 200);
      }
      
      // Update UI to reflect loaded settings
      updateSettingsUI();
    } else {
      console.log('[Settings] No settings file, using defaults');
    }
    
    settingsLoaded = true;
  } catch (e) {
    console.error('Failed to load settings:', e);
    settingsLoaded = true;
  }
}

async function saveSettings() {
  try {
    // Check if Poly is available
    if (typeof poly === 'undefined' || !poly.fs) {
      console.log('[Settings] Poly FS not available, cannot save');
      return;
    }
    
    const settings = {
      thinkMode,
      streamMode,
      webSearchMode,
      selectedModel,
      currentChatId,
      canvasMode: typeof canvasMode !== 'undefined' ? canvasMode : false
    };
    console.log('[Settings] Saving:', settings);
    await poly.fs.write(SETTINGS_FILE, JSON.stringify(settings, null, 2));
  } catch (e) {
    console.error('Failed to save settings:', e);
  }
}

function updateSettingsUI() {
  // Update toggle states
  thinkStatus.textContent = thinkMode ? 'On' : 'Off';
  thinkStatus.classList.toggle('on', thinkMode);
  
  streamStatus.textContent = streamMode ? 'On' : 'Off';
  streamStatus.classList.toggle('on', streamMode);
  
  const webSearchStatus = document.getElementById('web-search-status');
  if (webSearchStatus) {
    webSearchStatus.textContent = webSearchMode ? 'On' : 'Off';
    webSearchStatus.classList.toggle('on', webSearchMode);
  }
  
  const canvasModeStatus = document.getElementById('canvas-mode-status');
  if (canvasModeStatus) {
    const isCanvasActive = typeof canvasMode !== 'undefined' && canvasMode;
    canvasModeStatus.textContent = isCanvasActive ? 'On' : 'Off';
    canvasModeStatus.classList.toggle('on', isCanvasActive);
  }
}

async function loadChats() {
  try {
    if (typeof poly === 'undefined' || !poly.fs) {
      console.log('[Chats] Poly FS not available');
      return;
    }
    
    const exists = await poly.fs.exists(CHATS_FILE);
    if (exists) {
      const content = await poly.fs.read(CHATS_FILE);
      chats = JSON.parse(content);
      updateChatHistory();
      console.log('[Chats] Loaded', chats.length, 'chats');
    }
  } catch (e) {
    console.error('Failed to load chats:', e);
  }
}

async function saveChats() {
  try {
    if (typeof poly === 'undefined' || !poly.fs) {
      return;
    }
    await poly.fs.write(CHATS_FILE, JSON.stringify(chats, null, 2));
  } catch (e) {
    console.error('Failed to save chats:', e);
  }
}

// Function to send message from UI button clicks
function sendMessageFromUI(message) {
  if (!message || !selectedModel) return;
  userInput.value = message;
  sendMessage();
}
window.sendMessageFromUI = sendMessageFromUI;

// Sidebar functions
function toggleSidebar() {
  sidebar.classList.toggle('collapsed');
  document.body.classList.toggle('sidebar-collapsed');
}

function newChat() {
  if (conversationHistory.length > 0 && currentChatId) {
    saveCurrentChat();
  }
  currentChatId = Date.now().toString();
  conversationHistory = [];
  chatMessages.innerHTML = '';
  inputIsland.classList.add('centered');
  updateChatHistory();
  saveSettings(); // Save new chat ID
}

function saveCurrentChat() {
  if (!currentChatId || conversationHistory.length === 0) return;
  const firstMessage = conversationHistory.find(m => m.role === 'user');
  const title = firstMessage ? firstMessage.content.slice(0, 30) + (firstMessage.content.length > 30 ? '...' : '') : 'New Chat';

  const existingIndex = chats.findIndex(c => c.id === currentChatId);
  const chatData = { id: currentChatId, title, messages: [...conversationHistory], timestamp: Date.now() };
  if (existingIndex >= 0) { chats[existingIndex] = chatData; } else { chats.unshift(chatData); }
  updateChatHistory();
  saveChats();
}

function loadChat(chatId) {
  const chat = chats.find(c => c.id === chatId);
  if (!chat) return;
  if (currentChatId && conversationHistory.length > 0) { saveCurrentChat(); }
  currentChatId = chat.id;
  conversationHistory = [...chat.messages];
  chatMessages.innerHTML = '';
  inputIsland.classList.remove('centered');
  for (const msg of conversationHistory) { addMessage(msg.role, msg.content, null, false); }
  updateChatHistory();
  saveSettings(); // Save current chat ID
}

function updateChatHistory() {
  chatHistory.innerHTML = '';
  for (const chat of chats) {
    const item = document.createElement('div');
    item.className = `chat-history-item${chat.id === currentChatId ? ' active' : ''}`;
    item.innerHTML = `
      <i data-lucide="message-square" class="icon"></i>
      <span>${escapeHtml(chat.title)}</span>
      <button class="chat-menu-btn" onclick="event.stopPropagation(); toggleChatMenu('${chat.id}')">
        <i data-lucide="more-horizontal" class="icon"></i>
      </button>
      <div class="chat-item-menu" id="chat-menu-${chat.id}">
        <div class="chat-item-menu-item" onclick="event.stopPropagation(); renameChat('${chat.id}')">
          <i data-lucide="pencil" class="icon"></i><span>Rename</span>
        </div>
        <div class="chat-item-menu-item danger" onclick="event.stopPropagation(); deleteChat('${chat.id}')">
          <i data-lucide="trash-2" class="icon"></i><span>Delete</span>
        </div>
      </div>`;
    item.onclick = () => loadChat(chat.id);
    chatHistory.appendChild(item);
  }
  lucide.createIcons();
}

function toggleChatMenu(chatId) {
  document.querySelectorAll('.chat-item-menu').forEach(menu => {
    if (menu.id !== `chat-menu-${chatId}`) menu.classList.remove('show');
  });
  document.getElementById(`chat-menu-${chatId}`).classList.toggle('show');
}

function renameChat(chatId) {
  const chat = chats.find(c => c.id === chatId);
  if (!chat) return;
  const newTitle = prompt('Rename chat:', chat.title);
  if (newTitle && newTitle.trim()) { chat.title = newTitle.trim(); updateChatHistory(); saveChats(); }
  document.getElementById(`chat-menu-${chatId}`).classList.remove('show');
}

function deleteChat(chatId) {
  const menu = document.getElementById(`chat-menu-${chatId}`);
  if (menu) menu.classList.remove('show');
  const chatIndex = chats.findIndex(c => c.id === chatId);
  if (chatIndex === -1) return;
  chats.splice(chatIndex, 1);
  saveChats();
  if (currentChatId === chatId) {
    if (chats.length > 0) { loadChat(chats[0].id); }
    else { currentChatId = null; conversationHistory = []; chatMessages.innerHTML = ''; inputIsland.classList.add('centered'); updateChatHistory(); }
  } else { updateChatHistory(); }
}

// Close menus when clicking outside
document.addEventListener('click', (e) => {
  if (!e.target.closest('.chat-menu-btn') && !e.target.closest('.chat-item-menu')) {
    document.querySelectorAll('.chat-item-menu').forEach(menu => menu.classList.remove('show'));
  }
  if (!e.target.closest('.account-dropdown')) {
    const accountMenu = document.getElementById('account-menu');
    if (accountMenu) accountMenu.classList.remove('show');
  }
  if (!e.target.closest('.model-selector')) { modelMenu.classList.remove('show'); }
  if (!e.target.closest('.tools-dropdown')) { toolsMenu.classList.remove('show'); toolsBtn.parentElement.classList.remove('open'); }
});

// Account menu functions
function toggleAccountMenu() { document.getElementById('account-menu').classList.toggle('show'); lucide.createIcons(); }
function showSettings() { document.getElementById('account-menu').classList.remove('show'); console.log('Settings clicked'); }
function showAbout() { document.getElementById('account-menu').classList.remove('show'); console.log('About clicked'); }

// Configure marked if available
function renderMarkdown(text) { 
  if (!text) return '';
  
  // Try to use marked
  const lib = window.markedLib || (typeof marked !== 'undefined' ? marked : null);
  
  if (lib) {
    try {
      // marked v9+ uses marked.parse()
      if (typeof lib.parse === 'function') {
        return lib.parse(text);
      }
      // Older versions use marked() directly
      if (typeof lib === 'function') {
        return lib(text);
      }
    } catch (e) {
      console.error('[Markdown] Parse error:', e);
    }
  }
  
  // Fallback: basic formatting with proper escaping
  let html = escapeHtml(text);
  
  // Code blocks (must be before inline code)
  html = html.replace(/```(\w*)\n([\s\S]*?)```/g, (match, lang, code) => {
    return `<pre><code class="language-${lang || 'text'}">${code}</code></pre>`;
  });
  
  // Inline code
  html = html.replace(/`([^`]+)`/g, '<code>$1</code>');
  
  // Bold and italic
  html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
  html = html.replace(/\*(.+?)\*/g, '<em>$1</em>');
  
  // Headers
  html = html.replace(/^### (.+)$/gm, '<h3>$1</h3>');
  html = html.replace(/^## (.+)$/gm, '<h2>$1</h2>');
  html = html.replace(/^# (.+)$/gm, '<h1>$1</h1>');
  
  // Lists
  html = html.replace(/^- (.+)$/gm, '<li>$1</li>');
  html = html.replace(/(<li>.*<\/li>\n?)+/g, '<ul>$&</ul>');
  
  // Paragraphs
  html = html.replace(/\n\n/g, '</p><p>');
  html = html.replace(/\n/g, '<br>');
  
  return `<p>${html}</p>`;
}
window.renderMarkdown = renderMarkdown; // Make available for ui-components.js

// =============================================================================
// TOOL RESULT BOX RENDERING
// =============================================================================

function renderToolResultBox(toolName, result, filename = null) {
  const isSuccess = result && result.success !== false && !result.error;
  const statusClass = isSuccess ? 'success' : 'error';
  
  // Get file icon class
  const iconClass = filename ? getToolFileIconClass(filename) : 'devicon-javascript-plain colored';
  
  // Success/error icon SVG
  const statusIcon = isSuccess 
    ? `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>`
    : `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="15" y1="9" x2="9" y2="15"></line><line x1="9" y1="9" x2="15" y2="15"></line></svg>`;
  
  // Handle errors
  if (!isSuccess) {
    const errorMsg = result?.error || 'Tool failed';
    return `<div class="tool-result-box error">
      <span class="tool-icon">${statusIcon}</span>
      <span class="tool-text">${escapeHtml(toolName.replace(/_/g, ' '))} failed: ${escapeHtml(errorMsg)}</span>
    </div>`;
  }
  
  if (toolName === 'edit_canvas') {
    const displayName = filename || 'code';
    return `<div class="tool-result-box ${statusClass}">
      <span class="tool-icon">${statusIcon}</span>
      <span class="tool-text">Edited</span>
      <span class="tool-file"><i class="${iconClass}"></i>${escapeHtml(displayName)}</span>
    </div>`;
  }
  
  if (toolName === 'check_diagnostics' && result.diagnostics) {
    const d = result.diagnostics;
    const hasIssues = d.errors.length > 0 || d.warnings.length > 0;
    return `<div class="tool-result-box diagnostics ${hasIssues ? '' : 'success'}">
      <div class="diag-header">
        <span class="tool-icon">${statusIcon}</span>
        <span class="tool-text">Checked</span>
        <span class="tool-file"><i class="${iconClass}"></i>${escapeHtml(result.filename || 'code')}</span>
        <div class="diag-summary">
          ${d.errors.length > 0 ? `<span class="diag-errors">${d.errors.length} errors</span>` : ''}
          ${d.warnings.length > 0 ? `<span class="diag-warnings">${d.warnings.length} warnings</span>` : ''}
          ${!hasIssues ? `<span class="diag-ok">No issues</span>` : ''}
        </div>
      </div>
    </div>`;
  }
  
  if (toolName === 'read_canvas') {
    const displayName = result.filename || filename || 'code';
    const lineCount = result.code ? result.code.split('\n').length : 0;
    return `<div class="tool-result-box success">
      <span class="tool-icon">${statusIcon}</span>
      <span class="tool-text">Read ${lineCount} lines</span>
      <span class="tool-file"><i class="${iconClass}"></i>${escapeHtml(displayName)}</span>
    </div>`;
  }
  
  if (toolName === 'start_task') {
    return `<div class="tool-result-box">
      <span class="tool-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline></svg></span>
      <span class="tool-text">Started: <strong>${escapeHtml(result.message?.replace('Started: ', '') || 'Task')}</strong></span>
    </div>`;
  }
  
  if (toolName === 'end_task') {
    return `<div class="tool-result-box success">
      <span class="tool-icon">${statusIcon}</span>
      <span class="tool-text">Completed: <strong>${escapeHtml(result.summary || 'Task finished')}</strong></span>
    </div>`;
  }
  
  // Generic fallback
  return `<div class="tool-result-box ${statusClass}">
    <span class="tool-icon">${statusIcon}</span>
    <span class="tool-text">${escapeHtml(toolName.replace(/_/g, ' '))}</span>
  </div>`;
}

function getToolFileIconClass(filename) {
  if (!filename) return 'devicon-devicon-plain';
  const ext = filename.split('.').pop()?.toLowerCase();
  const icons = {
    'js': 'devicon-javascript-plain colored',
    'ts': 'devicon-typescript-plain colored',
    'py': 'devicon-python-plain colored',
    'html': 'devicon-html5-plain colored',
    'css': 'devicon-css3-plain colored',
    'json': 'devicon-json-plain colored',
    'rs': 'devicon-rust-original colored',
    'go': 'devicon-go-original-wordmark colored'
  };
  return icons[ext] || 'devicon-devicon-plain';
}

window.renderToolResultBox = renderToolResultBox;

// Auto-resize textarea
userInput.addEventListener('input', () => {
  userInput.style.height = 'auto';
  userInput.style.height = Math.min(userInput.scrollHeight, 120) + 'px';
  updateSendButton();
});

function updateSendButton() { sendBtn.disabled = !userInput.value.trim().length || !selectedModel; }

// Toggle functions
function toggleThink() { thinkMode = !thinkMode; thinkStatus.textContent = thinkMode ? 'On' : 'Off'; thinkStatus.classList.toggle('on', thinkMode); saveSettings(); }
function toggleStream() { streamMode = !streamMode; streamStatus.textContent = streamMode ? 'On' : 'Off'; streamStatus.classList.toggle('on', streamMode); saveSettings(); }
function toggleWebSearch() { webSearchMode = !webSearchMode; const status = document.getElementById('web-search-status'); status.textContent = webSearchMode ? 'On' : 'Off'; status.classList.toggle('on', webSearchMode); saveSettings(); }
function toggleToolsMenu() { toolsMenu.classList.toggle('show'); toolsBtn.parentElement.classList.toggle('open'); }
function toggleModelMenu() { modelMenu.classList.toggle('show'); }

function selectModel(model) {
  selectedModel = model;
  modelName.textContent = model;
  modelName.classList.remove('disconnected');
  modelMenu.classList.remove('show');
  document.querySelectorAll('.model-item').forEach(item => item.classList.toggle('selected', item.dataset.model === model));
  updateSendButton();
  saveSettings();
}

function clearWelcome() { const welcome = document.querySelector('.welcome'); if (welcome) welcome.remove(); inputIsland.classList.remove('centered'); }
function scrollToBottom() { window.scrollTo(0, document.body.scrollHeight); }
function escapeHtml(text) { const div = document.createElement('div'); div.textContent = text; return div.innerHTML; }

// Chain of Thought step management - updates the reasoning toggle label and content
const COT_STEP_LABELS = {
  analyzing: 'Analyzing',
  executing: 'Executing',
  fetching: 'Fetching',
  thinking: 'Thinking',
  generating: 'Generating',
  running: 'Running Code',
  reasoning: 'Reasoning',
  writing: 'Writing to Canvas',
  processing: 'Processing'
};

// Labels for final state based on what happened
const FINAL_LABELS = {
  reasoning: 'Finished Reasoning',
  thinking: 'Finished Thinking', 
  writing: 'Code Written',
  processing: 'Done',
  default: 'Done'
};

function updateReasoningLabel(div, stepId, isActive = true) {
  const reasoningToggle = div.querySelector('.reasoning-toggle');
  if (!reasoningToggle) return;
  
  const label = COT_STEP_LABELS[stepId] || stepId;
  const labelSpan = reasoningToggle.querySelector('span');
  
  // Show the toggle but don't open the content (just show label)
  reasoningToggle.style.display = 'inline-flex';
  
  // Track what type of processing we did (for final label)
  if (stepId === 'thinking' || stepId === 'reasoning') {
    div.dataset.hadReasoning = 'true';
  }
  
  // Update label with current step
  if (labelSpan) {
    labelSpan.textContent = isActive ? `${label}...` : label;
  }
  
  // Add/remove active class for animation
  if (isActive) {
    reasoningToggle.classList.add('active');
  } else {
    reasoningToggle.classList.remove('active');
  }
}

function addReasoningStep(div, icon, text) {
  const reasoningInner = div.querySelector('.reasoning-inner');
  const reasoningToggle = div.querySelector('.reasoning-toggle');
  const reasoningContent = div.querySelector('.reasoning-content');
  if (!reasoningInner) return;
  
  // Show and open reasoning block only when adding actual content
  if (reasoningToggle) {
    reasoningToggle.style.display = 'inline-flex';
    reasoningToggle.classList.add('open');
  }
  if (reasoningContent) {
    reasoningContent.classList.add('show');
  }
  
  // Add step line
  const stepLine = `${icon} ${text}\n`;
  reasoningInner.textContent += stepLine;
  reasoningInner.scrollTop = reasoningInner.scrollHeight;
  scrollToBottom();
}

function getStepIcon(stepId) {
  const icons = {
    analyzing: '🔍',
    executing: '⚡',
    fetching: '🌐',
    thinking: '💭',
    generating: '✨',
    running: '▶️',
    reasoning: '🧠',
    completed: '✓',
    error: '❌'
  };
  return icons[stepId] || '•';
}

function addMessage(role, content, thinking = null, scroll = true) {
  clearWelcome();
  const div = document.createElement('div');
  div.className = `message ${role}`;
  let html = '';
  if (thinking) {
    html += `<div class="reasoning-toggle" onclick="this.classList.toggle('open'); this.nextElementSibling.classList.toggle('show')">
      <i data-lucide="chevron-right" class="chevron"></i><span>Reasoning</span></div>
      <div class="reasoning-content"><div class="reasoning-inner">${escapeHtml(thinking)}</div></div>`;
  }
  
  // Check for JSON UI in assistant messages
  let renderedContent;
  if (role === 'assistant') {
    renderedContent = renderMarkdown(content);
  } else {
    renderedContent = escapeHtml(content);
  }
  
  html += `<div class="content">${renderedContent}</div>`;
  div.innerHTML = html;
  chatMessages.appendChild(div);
  lucide.createIcons();
  if (scroll) scrollToBottom();
  return div;
}

function createStreamingMessage() {
  clearWelcome();
  const div = document.createElement('div');
  div.className = 'message assistant streaming';
  div.innerHTML = `<div class="reasoning-toggle" style="display: none;" onclick="this.classList.toggle('open'); this.nextElementSibling.classList.toggle('show')">
    <i data-lucide="chevron-right" class="chevron"></i><span>Reasoning</span></div>
    <div class="reasoning-content"><div class="reasoning-inner"></div></div>
    <div class="content"><span class="cursor"></span></div>`;
  chatMessages.appendChild(div);
  lucide.createIcons();
  scrollToBottom();
  return div;
}

function updateStreamingMessage(div, content, thinking) {
  const contentEl = div.querySelector('.content');
  const reasoningToggle = div.querySelector('.reasoning-toggle');
  const reasoningContent = div.querySelector('.reasoning-content');
  const reasoningInner = div.querySelector('.reasoning-inner');
  
  // Handle thinking/reasoning content (only for models with actual thinking)
  if (thinking && thinking.trim()) {
    reasoningToggle.style.display = 'inline-flex';
    // Only set text content if it's actual thinking (not tool boxes)
    if (!reasoningInner.querySelector('.tool-result-box')) {
      reasoningInner.textContent = thinking;
    }
    if (!reasoningToggle.classList.contains('open')) { 
      reasoningToggle.classList.add('open'); 
      reasoningContent.classList.add('show'); 
    }
    reasoningInner.scrollTop = reasoningInner.scrollHeight;
  }
  
  // Clean tool call JSON from display
  let displayContent = content || '';
  displayContent = displayContent
    .replace(/\{"tool"\s*:\s*"[^"]*"\s*,?\s*"arguments"\s*:\s*\{[^}]*\}\s*\}/g, '')
    .replace(/\{"tool"\s*:\s*"[^"]*"\s*\}/g, '')
    .replace(/```json\s*$/g, '')
    .replace(/```\s*$/g, '')
    .replace(/```json\s*\n?\s*$/g, '')
    .trim();
  
  // If only tool call was in content, don't update text
  if (!displayContent && content && content.includes('"tool"')) {
    return;
  }
  
  // Find or create the CURRENT text segment (the one being streamed)
  let currentSegment = contentEl.querySelector('.text-segment.streaming');
  if (!currentSegment) {
    currentSegment = document.createElement('div');
    currentSegment.className = 'text-segment streaming';
    contentEl.appendChild(currentSegment);
  }
  
  // Clean any remaining tool JSON patterns more aggressively
  displayContent = displayContent
    .replace(/```json\s*\{[\s\S]*?"tool"[\s\S]*?\}\s*```/g, '')
    .replace(/```\s*\{[\s\S]*?"tool"[\s\S]*?\}\s*```/g, '')
    .replace(/\{[\s\S]*?"tool"\s*:\s*"[^"]*"[\s\S]*?"arguments"[\s\S]*?\}/g, '')
    .replace(/```json\s*\{[\s\S]*$/g, '') // Incomplete JSON block
    .replace(/```\s*\{[\s\S]*$/g, '') // Incomplete block
    .replace(/```json\s*$/gm, '')
    .replace(/```\s*$/gm, '')
    .replace(/^\s*```\s*$/gm, '')
    .trim();
  
  // Update only the current streaming segment
  currentSegment.innerHTML = renderMarkdown(displayContent);
  
  // Ensure cursor is at the end
  let cursor = contentEl.querySelector('.cursor');
  if (cursor) cursor.remove();
  cursor = document.createElement('span');
  cursor.className = 'cursor';
  contentEl.appendChild(cursor);
  
  scrollToBottom();
}

// Commit current streaming segment and add tool box inline
function commitStreamingSegmentAndAddToolBox(div, toolName, result, filename) {
  const contentEl = div.querySelector('.content');
  
  // Find current streaming segment and commit it (remove streaming class)
  const currentSegment = contentEl.querySelector('.text-segment.streaming');
  if (currentSegment) {
    // Clean any tool JSON from the segment before committing
    let segmentHtml = currentSegment.innerHTML;
    segmentHtml = segmentHtml
      .replace(/```json\s*\{[\s\S]*?"tool"[\s\S]*?\}\s*```/g, '')
      .replace(/```\s*\{[\s\S]*?"tool"[\s\S]*?\}\s*```/g, '')
      .replace(/\{[\s\S]*?"tool"\s*:\s*"[^"]*"[\s\S]*?"arguments"[\s\S]*?\}/g, '')
      .replace(/<code[^>]*>\s*\{[\s\S]*?"tool"[\s\S]*?\}\s*<\/code>/g, '')
      .replace(/<pre[^>]*>[\s\S]*?<\/pre>/g, (match) => {
        // Only remove pre blocks that contain tool JSON
        if (match.includes('"tool"')) return '';
        return match;
      })
      .replace(/```json\s*$/g, '')
      .replace(/```\s*$/g, '')
      .replace(/^\s*```\s*$/gm, '')
      .trim();
    currentSegment.innerHTML = segmentHtml;
    
    currentSegment.classList.remove('streaming');
    // If segment is empty, remove it
    if (!currentSegment.textContent.trim()) {
      currentSegment.remove();
    }
  }
  
  // Remove cursor temporarily
  const cursor = contentEl.querySelector('.cursor');
  if (cursor) cursor.remove();
  
  // Only add tool box if we have a valid result
  if (toolName && result) {
    const boxHtml = renderToolResultBox(toolName, result, filename);
    const boxWrapper = document.createElement('div');
    boxWrapper.className = 'tool-box-inline';
    boxWrapper.innerHTML = boxHtml;
    contentEl.appendChild(boxWrapper);
  }
  
  // Create new streaming segment for next text
  const newSegment = document.createElement('div');
  newSegment.className = 'text-segment streaming';
  contentEl.appendChild(newSegment);
  
  // Add cursor back
  const newCursor = document.createElement('span');
  newCursor.className = 'cursor';
  contentEl.appendChild(newCursor);
  
  scrollToBottom();
}

function finalizeStreamingMessage(div, content, thinking) {
  div.classList.remove('streaming');
  const contentEl = div.querySelector('.content');
  const reasoningToggle = div.querySelector('.reasoning-toggle');
  const reasoningContent = div.querySelector('.reasoning-content');
  const reasoningInner = div.querySelector('.reasoning-inner');
  
  // Check if we have any reasoning content (from CoT steps or thinking)
  const existingReasoning = reasoningInner ? reasoningInner.textContent.trim() : '';
  
  // Only add thinking if it's new (not already in the reasoning block)
  if (thinking && reasoningInner) {
    const thinkingTrimmed = thinking.trim();
    // Check if this thinking is already displayed (avoid duplicates)
    if (!existingReasoning.includes(thinkingTrimmed.substring(0, 50))) {
      if (existingReasoning) {
        reasoningInner.textContent = existingReasoning + '\n\n' + thinkingTrimmed;
      } else {
        reasoningInner.textContent = thinkingTrimmed;
      }
    }
  }
  
  const hasReasoningContent = reasoningInner && reasoningInner.textContent.trim().length > 0;
  
  // Only show reasoning toggle if there's actual thinking content (not just tool boxes)
  if (hasReasoningContent) {
    reasoningToggle.style.display = 'inline-flex';
    reasoningToggle.classList.remove('active');
    const labelSpan = reasoningToggle.querySelector('span');
    if (labelSpan) {
      labelSpan.textContent = 'Finished Reasoning';
    }
    // Collapse the reasoning block
    reasoningToggle.classList.remove('open');
    reasoningContent.classList.remove('show');
  } else { 
    // Hide reasoning toggle if no actual thinking content
    if (reasoningToggle) reasoningToggle.style.display = 'none';
  }
  
  // Remove cursor
  const cursor = contentEl.querySelector('.cursor');
  if (cursor) cursor.remove();
  
  // Check if we have inline tool boxes
  const hasInlineBoxes = contentEl.querySelector('.tool-box-inline');
  if (hasInlineBoxes) {
    // Find the last streaming segment and finalize it
    const lastSegment = contentEl.querySelector('.text-segment.streaming');
    if (lastSegment) {
      // Clean any tool JSON from the segment
      let segmentHtml = lastSegment.innerHTML;
      segmentHtml = segmentHtml
        .replace(/```json\s*\{[\s\S]*?"tool"[\s\S]*?\}\s*```/g, '')
        .replace(/```\s*\{[\s\S]*?"tool"[\s\S]*?\}\s*```/g, '')
        .replace(/\{[\s\S]*?"tool"\s*:\s*"[^"]*"[\s\S]*?"arguments"[\s\S]*?\}/g, '')
        .replace(/<code[^>]*>\s*\{[\s\S]*?"tool"[\s\S]*?\}\s*<\/code>/g, '')
        .replace(/<pre[^>]*>\s*<code[^>]*>\s*\{[\s\S]*?"tool"[\s\S]*?\}\s*<\/code>\s*<\/pre>/g, '')
        .replace(/<pre[^>]*>[\s\S]*?<\/pre>/g, (match) => {
          if (match.includes('"tool"')) return '';
          return match;
        })
        .replace(/```json\s*$/g, '')
        .replace(/```\s*$/g, '')
        .replace(/^\s*```\s*$/gm, '')
        .trim();
      lastSegment.innerHTML = segmentHtml;
      
      lastSegment.classList.remove('streaming');
      // If empty, remove it
      if (!lastSegment.textContent.trim()) {
        lastSegment.remove();
      }
    }
    
    // Also clean any remaining text segments
    contentEl.querySelectorAll('.text-segment').forEach(seg => {
      let html = seg.innerHTML;
      const cleaned = html
        .replace(/```json\s*\{[\s\S]*?"tool"[\s\S]*?\}\s*```/g, '')
        .replace(/```\s*\{[\s\S]*?"tool"[\s\S]*?\}\s*```/g, '')
        .replace(/\{[\s\S]*?"tool"\s*:\s*"[^"]*"[\s\S]*?"arguments"[\s\S]*?\}/g, '')
        .replace(/<pre[^>]*>\s*<code[^>]*>\s*\{[\s\S]*?"tool"[\s\S]*?\}\s*<\/code>\s*<\/pre>/g, '')
        .replace(/<pre[^>]*>[\s\S]*?<\/pre>/g, (match) => {
          if (match.includes('"tool"')) return '';
          return match;
        })
        .replace(/```json\s*$/g, '')
        .replace(/```\s*$/g, '')
        .replace(/^\s*```\s*$/gm, '')
        .trim();
      if (cleaned !== html) {
        seg.innerHTML = cleaned;
      }
      if (!seg.textContent.trim()) {
        seg.remove();
      }
    });
  } else {
    // No inline boxes - just set the content normally
    const cleanContent = content
      .replace(/\{"tool"\s*:\s*"[^"]*"\s*,?\s*"arguments"\s*:\s*\{[^}]*\}\s*\}/g, '')
      .replace(/\{"tool"\s*:\s*"[^"]*"\s*\}/g, '')
      .replace(/```\s*$/g, '')
      .trim();
    contentEl.innerHTML = renderMarkdown(cleanContent);
  }
}

async function init() {
  // Load saved settings and chats first (async with Poly FS)
  await loadSettings();
  await loadChats();
  
  // Restore active chat if we have one saved
  if (currentChatId && chats.length > 0) {
    const savedChat = chats.find(c => c.id === currentChatId);
    if (savedChat) {
      loadChat(currentChatId);
    }
  }
  
  if (typeof poly === 'undefined') { modelName.textContent = 'Poly not ready'; modelName.classList.add('disconnected'); return; }
  try {
    const isRunning = await poly.ai.checkOllama();
    if (isRunning) { await loadModels(); }
    else { modelName.textContent = 'Ollama offline'; modelName.classList.add('disconnected'); }
  } catch (e) { console.error('Init error:', e); modelName.textContent = 'Connection error'; modelName.classList.add('disconnected'); }
}

async function loadModels() {
  try {
    const allModels = await poly.ai.listModels();
    
    // Filter to only supported models
    models = ModelAdapters.filterSupported(allModels);
    
    const unsupportedCount = allModels.length - models.length;
    modelsCount.textContent = models.length;
    if (unsupportedCount > 0) {
      console.log(`[Models] Filtered out ${unsupportedCount} unsupported models`);
    }
    
    modelList.innerHTML = '';
    
    if (models.length === 0) {
      modelList.innerHTML = '<div class="model-empty">No supported models found.<br>Install a Qwen3 model in Ollama.</div>';
      modelName.textContent = 'No models';
      modelName.classList.add('disconnected');
      return;
    }
    
    models.forEach(model => {
      const item = document.createElement('div');
      item.className = 'model-item';
      item.dataset.model = model;
      const parts = model.split(':');
      const name = parts[0];
      const tag = parts[1] || 'latest';
      
      // Extract size from tag (e.g., "480b-cloud" -> "480B")
      const sizeMatch = tag.match(/(\d+\.?\d*)b/i);
      const size = sizeMatch ? sizeMatch[1] + 'B' : '';
      
      // Check if it's a cloud model
      const isCloud = tag.toLowerCase().includes('cloud');
      
      // Build HTML: Name | Cloud tag (if cloud) | Size
      let html = `<span class="model-name">${name}</span>`;
      if (isCloud) {
        html += `<span class="model-cloud">Cloud</span>`;
      }
      html += `<span class="model-size">${size}</span>`;
      
      item.innerHTML = html;
      item.onclick = () => selectModel(model);
      modelList.appendChild(item);
    });
    
    // Restore previously selected model if it exists and is supported, otherwise select first
    const savedModel = selectedModel;
    if (savedModel && models.includes(savedModel)) {
      selectModel(savedModel);
    } else if (models.length > 0) {
      selectModel(models[0]);
    }
  } catch (e) { 
    console.error('Failed to load models:', e);
    modelName.textContent = 'No models'; 
    modelName.classList.add('disconnected');
  }
}

async function sendMessage() {
  const message = userInput.value.trim();
  if (!message || !selectedModel) return;
  if (!currentChatId) { currentChatId = Date.now().toString(); }
  addMessage('user', message);
  userInput.value = '';
  userInput.style.height = 'auto';
  conversationHistory.push({ role: 'user', content: message });
  sendBtn.disabled = true;
  
  // Build messages
  let messagesWithSystem = [...conversationHistory];
  
  // Get model adapter for model-specific behavior
  const adapter = ModelAdapters.getAdapter(selectedModel);
  
  console.log('[App] sendMessage - thinkMode:', thinkMode, 'adapter:', adapter?.name);
  
  // Check if canvas mode is active - if so, add tools to system prompt
  const canvasActive = typeof canvasMode !== 'undefined' && canvasMode;
  if (canvasActive && adapter && adapter.buildToolSystemPrompt) {
    const tools = window.AITools?.definitions || [];
    const toolPrompt = adapter.buildToolSystemPrompt(tools);
    if (toolPrompt) {
      // Add system message with tools
      messagesWithSystem = [
        { role: 'system', content: 'You are a helpful coding assistant.' + toolPrompt },
        ...messagesWithSystem
      ];
    }
  }
  
  // Format messages with adapter (adds /think or /no_think for Qwen3)
  if (adapter && adapter.formatMessages) {
    messagesWithSystem = adapter.formatMessages(messagesWithSystem, thinkMode);
  }
  
  try {
    if (streamMode) {
      await streamResponse(messagesWithSystem, adapter);
    } else {
      // Get options from adapter
      const options = adapter ? adapter.formatOptions({ think: thinkMode, temperature: 0.7 }) : { temperature: 0.7 };
      const response = await poly.ai.ollama(selectedModel, messagesWithSystem, options);
      
      // Parse response with adapter
      let content = response.content;
      let thinking = response.thinking;
      
      if (adapter && adapter.parseThinking) {
        const parsed = adapter.parseThinking(content, thinkMode);
        thinking = parsed.thinking || thinking;
        content = parsed.content;
      }
      
      // Check for tool calls
      if (adapter && adapter.parseToolCalls) {
        const toolCalls = adapter.parseToolCalls(content);
        if (toolCalls.length > 0) {
          content = await handleToolCalls(toolCalls, content, adapter);
        }
      }
      
      addMessage('assistant', content, thinking);
      conversationHistory.push({ role: 'assistant', content });
    }
  } catch (e) { addMessage('assistant', `Error: ${e.message}`); }
  finally { updateSendButton(); saveCurrentChat(); }
}

async function streamResponse(messages, adapter) {
  const streamDiv = createStreamingMessage();
  let fullContent = '';
  let fullThinking = '';
  
  // Get options from adapter (includes thinking mode params)
  const thinkingEnabled = thinkMode;
  const options = adapter ? adapter.formatOptions({ think: thinkingEnabled, temperature: 0.7 }) : { think: thinkingEnabled, temperature: 0.7 };
  
  // Check if canvas mode is active - if so, add tools to system prompt
  const canvasActive = typeof canvasMode !== 'undefined' && canvasMode;
  let messagesWithTools = messages;
  
  if (canvasActive && adapter && adapter.buildToolSystemPrompt) {
    const tools = window.AITools?.definitions || [];
    const toolPrompt = adapter.buildToolSystemPrompt(tools);
    if (toolPrompt && !messages.some(m => m.role === 'system')) {
      messagesWithTools = [
        { role: 'system', content: 'You are a helpful coding assistant.' + toolPrompt },
        ...messages
      ];
      console.log('[App] Added tool system prompt, tools:', tools.map(t => t.name));
      console.log('[App] System prompt:', messagesWithTools[0].content.substring(0, 500));
    }
  }
  
  const result = await poly.ai.stream.start(selectedModel, messagesWithTools, options);
  if (!result || !result.streamId) { throw new Error('Failed to start stream'); }
  
  let done = false;
  while (!done) {
    const pollResult = await poly.ai.stream.poll(result.streamId);
    if (pollResult.error) throw new Error(pollResult.error);
    
    for (const chunk of pollResult.chunks) {
      if (chunk.type === 'thinking') { 
        // Only collect thinking if thinkMode is enabled
        if (thinkingEnabled) {
          fullThinking += chunk.delta; 
          updateStreamingMessage(streamDiv, fullContent, fullThinking); 
        }
      }
      else if (chunk.type === 'content') { 
        fullContent += chunk.delta;
        
        // Use adapter to parse thinking from content (for models that embed it)
        if (adapter && adapter.extractPartialThinking) {
          const parsed = adapter.extractPartialThinking(fullContent);
          // Only show thinking if thinkMode is enabled
          if (parsed.thinking && thinkingEnabled) {
            fullThinking = parsed.thinking;
          }
          updateStreamingMessage(streamDiv, parsed.mainContent, thinkingEnabled ? fullThinking : '');
        } else {
          updateStreamingMessage(streamDiv, fullContent, thinkingEnabled ? fullThinking : '');
        }
      }
      else if (chunk.type === 'done') { 
        fullContent = chunk.content; 
        // Only keep thinking if thinkMode is enabled
        fullThinking = thinkingEnabled ? (chunk.thinking || fullThinking) : ''; 
      }
    }
    done = pollResult.done;
    if (!done) await new Promise(r => setTimeout(r, 16));
  }
  
  // Final parse with adapter
  let finalContent = fullContent;
  let finalThinking = thinkingEnabled ? fullThinking : null;
  
  if (adapter && adapter.parseThinking) {
    const parsed = adapter.parseThinking(fullContent, thinkingEnabled);
    if (parsed.thinking && thinkingEnabled) {
      finalThinking = parsed.thinking;
    }
    finalContent = parsed.content;
  }
  
  // Check for tool calls in the response
  if (canvasActive && adapter && adapter.parseToolCalls) {
    const toolCalls = adapter.parseToolCalls(finalContent);
    if (toolCalls.length > 0) {
      // Agentic loop - keep executing tools until model calls end_task or max loops
      let currentContent = finalContent;
      let loopCount = 0;
      const maxLoops = 15; // Allow more loops since we have explicit end_task
      
      // Clean initial content - this is what was streamed BEFORE the first tool call
      const initialCleanContent = finalContent
        .replace(/\{"tool"\s*:\s*"[^"]*"\s*,?\s*"arguments"\s*:\s*\{[^}]*\}\s*\}/g, '')
        .replace(/\{"tool"\s*:\s*"[^"]*"\s*\}/g, '')
        .replace(/```json\s*\{[\s\S]*$/g, '')
        .replace(/```\s*\{[\s\S]*$/g, '')
        .replace(/```json\s*$/gm, '')
        .replace(/```\s*$/gm, '')
        .trim();
      
      // Commit the initial text segment (the text before first tool call)
      const contentEl = streamDiv.querySelector('.content');
      const initialSegment = contentEl.querySelector('.text-segment.streaming');
      if (initialSegment && initialCleanContent) {
        initialSegment.innerHTML = renderMarkdown(initialCleanContent);
        initialSegment.classList.remove('streaming');
        // Create new streaming segment for after tool boxes
        const newSegment = document.createElement('div');
        newSegment.className = 'text-segment streaming';
        contentEl.appendChild(newSegment);
      }
      
      // Track accumulated content for history (not for display)
      let accumulatedContent = initialCleanContent;
      
      while (loopCount < maxLoops) {
        let calls = adapter.parseToolCalls(currentContent);
        if (calls.length === 0) break;
        
        // Filter out duplicate start_task if already active
        let filteredStartTask = false;
        if (window.AITools?.isAgentLoopActive()) {
          const beforeCount = calls.length;
          calls = calls.filter(c => c.name !== 'start_task');
          if (calls.length < beforeCount) {
            console.log('[Agent] Filtered out duplicate start_task');
            filteredStartTask = true;
          }
        }
        
        // If all tools were filtered out, send a prompt to continue
        if (calls.length === 0 && filteredStartTask) {
          console.log('[Agent] Only start_task was called, prompting model to use edit_canvas');
          loopCount++;
          // Send a message telling the model to use edit_canvas instead
          const promptMessages = [
            ...messagesWithTools,
            { role: 'assistant', content: currentContent.replace(/```[\s\S]*?```/g, '').trim() || 'I will fix the errors.' },
            { role: 'user', content: 'Task already started. Now use edit_canvas to fix each error. Do not call start_task again.' }
          ];
          
          const promptResult = await poly.ai.stream.start(selectedModel, promptMessages, options);
          if (!promptResult || !promptResult.streamId) break;
          
          let promptContent = '';
          let promptDone = false;
          while (!promptDone) {
            const pollResult = await poly.ai.stream.poll(promptResult.streamId);
            if (pollResult.error) break;
            for (const chunk of pollResult.chunks) {
              if (chunk.type === 'content') {
                promptContent += chunk.delta;
                if (adapter && adapter.extractPartialThinking) {
                  const parsed = adapter.extractPartialThinking(promptContent);
                  updateStreamingMessage(streamDiv, parsed.mainContent, thinkingEnabled ? parsed.thinking : '');
                }
              } else if (chunk.type === 'done') {
                promptContent = chunk.content;
              }
            }
            promptDone = pollResult.done;
            if (!promptDone) await new Promise(r => setTimeout(r, 16));
          }
          
          if (adapter && adapter.parseThinking) {
            const parsed = adapter.parseThinking(promptContent, thinkingEnabled);
            currentContent = parsed.content;
          } else {
            currentContent = promptContent;
          }
          continue;
        }
        
        if (calls.length === 0) break;
        
        loopCount++;
        const toolNames = calls.map(c => c.name).join(', ');
        console.log(`[Agent] Loop ${loopCount}, tools: ${toolNames}`);
        
        // Check if this is end_task - if so, execute and stop
        const hasEndTask = calls.some(c => c.name === 'end_task');
        
        const toolResults = await handleToolCalls(calls, currentContent, adapter, streamDiv);
        
        // Add tool result boxes INLINE in the content (text → box → text pattern)
        const activeTab = window.canvasTabs?.find(t => t.id === window.activeTabId);
        const filename = activeTab?.name || 'code';
        
        // Add each tool result as an inline box (except start_task/end_task - AI explains itself)
        for (const { tool, result } of toolResults) {
          if (tool !== 'end_task' && tool !== 'start_task') {
            commitStreamingSegmentAndAddToolBox(streamDiv, tool, result, filename);
          }
        }
        
        // Update reasoning label if we have thinking content
        const reasoningToggle = streamDiv.querySelector('.reasoning-toggle');
        if (reasoningToggle) {
          const labelSpan = reasoningToggle.querySelector('span');
          if (labelSpan) {
            const lastTool = toolResults[toolResults.length - 1];
            if (lastTool.tool === 'check_diagnostics') {
              labelSpan.textContent = 'Checking code...';
            } else if (lastTool.tool === 'edit_canvas') {
              labelSpan.textContent = 'Editing...';
            } else if (lastTool.tool === 'start_task') {
              labelSpan.textContent = lastTool.result.message?.replace('Started: ', '') || 'Working...';
            } else if (lastTool.tool === 'end_task') {
              labelSpan.textContent = 'Completed';
            } else {
              labelSpan.textContent = 'Working...';
            }
          }
        }
        
        // If end_task was called, get final response from model
        if (hasEndTask) {
          console.log('[Agent] end_task called, getting final response');
          const endResult = toolResults.find(r => r.tool === 'end_task');
          const summary = endResult?.result?.summary || 'Task completed';
          
          // Ask model for a final summary response
          const finalMessages = [
            ...messagesWithTools,
            { role: 'assistant', content: 'I have completed the task.' },
            { role: 'user', content: `Task completed: "${summary}". Now write a brief, friendly summary for the user explaining what you did. Do NOT use any tools, just respond with text.` }
          ];
          
          const finalResult = await poly.ai.stream.start(selectedModel, finalMessages, options);
          if (finalResult && finalResult.streamId) {
            let finalResponseContent = '';
            let finalDone = false;
            
            while (!finalDone) {
              const pollResult = await poly.ai.stream.poll(finalResult.streamId);
              if (pollResult.error) break;
              
              for (const chunk of pollResult.chunks) {
                if (chunk.type === 'content') {
                  finalResponseContent += chunk.delta;
                  // Clean any accidental tool calls
                  let cleanFinal = finalResponseContent
                    .replace(/\{"tool"[\s\S]*?\}/g, '')
                    .replace(/```json[\s\S]*?```/g, '')
                    .replace(/```[\s\S]*?```/g, '')
                    .trim();
                  updateStreamingMessage(streamDiv, cleanFinal, '');
                } else if (chunk.type === 'done') {
                  finalResponseContent = chunk.content;
                }
              }
              finalDone = pollResult.done;
              if (!finalDone) await new Promise(r => setTimeout(r, 16));
            }
            
            // Clean final content
            accumulatedContent = (accumulatedContent ? accumulatedContent + '\n\n' : '') + 
              finalResponseContent
                .replace(/\{"tool"[\s\S]*?\}/g, '')
                .replace(/```json[\s\S]*?```/g, '')
                .trim();
          }
          
          break;
        }
        
        // Update label based on task
        const labelSpan = streamDiv.querySelector('.reasoning-toggle span');
        if (labelSpan) {
          const startResult = toolResults.find(r => r.tool === 'start_task');
          if (startResult && startResult.result.message) {
            labelSpan.textContent = startResult.result.message.replace('Agent loop started: ', '');
          } else {
            labelSpan.textContent = `Working (${loopCount})...`;
          }
        }
        
        // Clean the tool call JSON from the content
        let cleanContent = currentContent
          .replace(/\{"tool"\s*:\s*"[^"]*"\s*,?\s*"arguments"\s*:\s*\{[^}]*\}\s*\}/g, '')
          .replace(/\{"tool"\s*:\s*"[^"]*"\s*\}/g, '')
          .replace(/^\s*\}\s*$/gm, '')
          .trim();
        
        // Format tool results for the model
        let toolResultMessage = '';
        for (const { tool, result } of toolResults) {
          // Handle errors first
          if (result.error || result.success === false) {
            toolResultMessage += `❌ ${tool} failed: ${result.error || 'Unknown error'}. Try a different approach.\n`;
            continue;
          }
          
          if (tool === 'start_task') {
            if (result.alreadyActive) {
              toolResultMessage += `Already working. Use edit_canvas now.\n`;
            } else {
              toolResultMessage += `Task started. Now tell the user what you'll fix, then use edit_canvas.\n`;
            }
          } else if (tool === 'check_diagnostics' && result.success) {
            const d = result.diagnostics;
            if (d.errors.length > 0) {
              toolResultMessage += `Found ${d.errors.length} errors:\n`;
              d.errors.slice(0, 10).forEach(e => {
                toolResultMessage += `Line ${e.line}: ${e.message}\n`;
              });
            } else {
              toolResultMessage += `No errors found.\n`;
            }
          } else if (tool === 'read_canvas' && result.success) {
            toolResultMessage += `Code (${result.code.split('\n').length} lines):\n\`\`\`\n${result.code}\n\`\`\`\n`;
          } else if (tool === 'edit_canvas' && result.success) {
            toolResultMessage += `✓ Line ${result.startLine} edited. Tell user what you fixed, then continue or end_task.\n`;
          } else if (tool === 'end_task') {
            toolResultMessage += `Task completed. Now write your final summary for the user.\n`;
          } else {
            toolResultMessage += `${tool}: ${JSON.stringify(result)}\n`;
          }
        }
        
        // Guide based on state
        if (window.AITools?.isAgentLoopActive()) {
          toolResultMessage += `\nREMEMBER: Write a message explaining what you just did before the next tool call.`;
        }
        
        // Send tool results back to model for next action
        const followUpMessages = [
          ...messagesWithTools,
          { role: 'assistant', content: cleanContent || 'Using tool.' },
          { role: 'user', content: toolResultMessage }
        ];
        
        // Stream the follow-up response
        const followUpResult = await poly.ai.stream.start(selectedModel, followUpMessages, options);
        if (!followUpResult || !followUpResult.streamId) break;
        
        let followUpContent = '';
        let followUpDone = false;
        
        // Track content for THIS iteration only (not accumulated)
        let iterationStreamContent = '';
        
        while (!followUpDone) {
          const pollResult = await poly.ai.stream.poll(followUpResult.streamId);
          if (pollResult.error) break;
          
          for (const chunk of pollResult.chunks) {
            if (chunk.type === 'content') {
              followUpContent += chunk.delta;
              // Parse thinking if present
              if (adapter && adapter.extractPartialThinking) {
                const parsed = adapter.extractPartialThinking(followUpContent);
                iterationStreamContent = parsed.mainContent;
                updateStreamingMessage(streamDiv, iterationStreamContent, thinkingEnabled ? parsed.thinking : '');
              } else {
                iterationStreamContent = followUpContent;
                updateStreamingMessage(streamDiv, iterationStreamContent, '');
              }
            } else if (chunk.type === 'done') {
              followUpContent = chunk.content;
            }
          }
          followUpDone = pollResult.done;
          if (!followUpDone) await new Promise(r => setTimeout(r, 16));
        }
        
        // Parse final content for next iteration
        let iterationContent = followUpContent;
        if (adapter && adapter.parseThinking) {
          const parsed = adapter.parseThinking(followUpContent, thinkingEnabled);
          iterationContent = parsed.content;
          if (parsed.thinking && thinkingEnabled) {
            finalThinking = (finalThinking || '') + '\n' + parsed.thinking;
          }
        }
        
        // Clean tool JSON from the iteration content
        const cleanIterationContent = iterationContent
          .replace(/\{"tool"\s*:\s*"[^"]*"\s*,?\s*"arguments"\s*:\s*\{[^}]*\}\s*\}/g, '')
          .replace(/\{"tool"\s*:\s*"[^"]*"\s*\}/g, '')
          .trim();
        
        // Accumulate all non-tool content for final display
        if (cleanIterationContent) {
          accumulatedContent = (accumulatedContent ? accumulatedContent + '\n\n' : '') + cleanIterationContent;
        }
        
        currentContent = iterationContent;
      }
      
      // Use accumulated content as final
      finalContent = accumulatedContent || finalContent;
      
      console.log(`[Agent] Finished after ${loopCount} loop(s)`);
    }
  }
  
  // If thinking mode is off, ensure no thinking is shown
  if (!thinkingEnabled) {
    finalThinking = null;
  }
  
  finalizeStreamingMessage(streamDiv, finalContent, finalThinking);
  
  // Store clean content in history (without thinking)
  const historyContent = adapter && adapter.cleanResponseForHistory 
    ? adapter.cleanResponseForHistory(fullContent) 
    : finalContent;
  conversationHistory.push({ role: 'assistant', content: historyContent });
}

// Enter to send
userInput.addEventListener('keydown', (e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); sendMessage(); } });

// =============================================================================
// TOOL CALLING
// =============================================================================

async function handleToolCalls(toolCalls, originalContent, adapter, streamDiv = null) {
  console.log('[Tools] Handling tool calls:', toolCalls);
  
  // Collect tool results to send back to the model
  const toolResults = [];
  
  for (const toolCall of toolCalls) {
    const { name, arguments: args } = toolCall;
    
    // Update the label to show tool execution
    if (streamDiv) {
      updateReasoningLabel(streamDiv, 'executing', true);
      const labelSpan = streamDiv.querySelector('.reasoning-toggle span');
      if (labelSpan) {
        if (name === 'start_task') {
          labelSpan.textContent = args?.task || 'Starting task...';
        } else if (name === 'end_task') {
          labelSpan.textContent = 'Finishing...';
        } else {
          const toolLabel = name.replace(/_/g, ' ');
          labelSpan.textContent = `${toolLabel}...`;
        }
      }
    }
    
    // Execute the tool
    let result;
    try {
      result = window.AITools?.execute(name, args || {});
      console.log('[Tools] Result:', result);
    } catch (e) {
      console.error('[Tools] Error executing:', name, e);
      result = { success: false, error: e.message };
    }
    
    // Always add a result, even if it failed
    if (!result) {
      result = { success: false, error: 'Tool returned no result' };
    }
    
    toolResults.push({ tool: name, result });
    
    // Mark tool type for UI
    if (streamDiv) {
      if (name === 'start_task') {
        streamDiv.dataset.hadToolCall = 'task';
        streamDiv.dataset.taskName = args?.task || 'Working';
      } else if (name === 'end_task') {
        streamDiv.dataset.hadToolCall = 'completed';
      } else if (name === 'check_diagnostics') {
        streamDiv.dataset.hadToolCall = 'diagnostics';
      } else if (name === 'read_canvas') {
        streamDiv.dataset.hadToolCall = 'read';
      } else if (name === 'edit_canvas') {
        streamDiv.dataset.hadToolCall = 'edit';
      }
    }
  }
  
  // Return tool results so the model can respond
  return toolResults;
}

// Init
if (document.readyState === 'loading') { document.addEventListener('DOMContentLoaded', () => setTimeout(init, 100)); }
else { setTimeout(init, 100); }

// Initial button state
sendBtn.disabled = true;
