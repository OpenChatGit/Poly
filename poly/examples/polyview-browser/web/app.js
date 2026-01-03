// PolyView Browser - Uses iframe2 (PolyView) instead of multiple WebViews
// This approach has NO z-order issues because everything is in one WebView!

const $ = id => document.getElementById(id);

// Get the current port from the page URL
const PROXY_PORT = window.location.port || '80';

// PolyView proxy URL helper
function getProxyUrl(url) {
  return `http://localhost:${PROXY_PORT}/__polyview/?url=${encodeURIComponent(url)}`;
}

// State
let tabs = new Map(); // id -> { url, title, loading, iframe }
let activeTabId = null;
let tabCounter = 0;

// ============================================
// Tab Management
// ============================================

function createTab(url = 'https://duckduckgo.com') {
  const id = ++tabCounter;
  
  // Create iframe container
  const container = document.createElement('div');
  container.className = 'polyview-container';
  container.id = `tab-${id}`;
  container.innerHTML = `
    <div class="loading-bar"></div>
    <iframe sandbox="allow-same-origin allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox allow-top-navigation"></iframe>
  `;
  $('content').appendChild(container);
  
  const iframe = container.querySelector('iframe');
  const loadingBar = container.querySelector('.loading-bar');
  
  // Store tab data
  tabs.set(id, {
    url: url,
    title: 'New Tab',
    loading: true,
    container: container,
    iframe: iframe,
    loadingBar: loadingBar
  });
  
  // Listen for messages from iframe
  window.addEventListener('message', (e) => {
    if (e.source !== iframe.contentWindow) return;
    handleIframeMessage(id, e.data);
  });
  
  // Track iframe load events
  iframe.addEventListener('load', () => {
    const tab = tabs.get(id);
    if (tab) {
      tab.loading = false;
      tab.loadingBar.classList.remove('active');
      renderTabs();
    }
  });
  
  // Navigate to URL
  navigateTab(id, url);
  
  // Switch to new tab
  switchTab(id);
  
  renderTabs();
  return id;
}

function closeTab(id) {
  if (tabs.size <= 1) return;
  
  const tab = tabs.get(id);
  if (tab) {
    tab.container.remove();
    tabs.delete(id);
    
    // If closing active tab, switch to another
    if (activeTabId === id) {
      const nextId = tabs.keys().next().value;
      if (nextId) switchTab(nextId);
    }
    
    renderTabs();
  }
}

function switchTab(id) {
  if (!tabs.has(id)) return;
  
  // Hide all tabs
  tabs.forEach((tab, tabId) => {
    tab.container.classList.toggle('active', tabId === id);
  });
  
  activeTabId = id;
  
  // Update URL bar
  const tab = tabs.get(id);
  if (tab) {
    $('urlInput').value = tab.url;
    updateSecurityIcon(tab.url);
  }
  
  renderTabs();
}

function navigateTab(id, url) {
  if (!url) url = $('urlInput').value.trim();
  if (!url) return;
  
  // Check if search query or URL
  if (!url.includes('.') && !url.startsWith('http') && !url.startsWith('file:')) {
    url = 'https://www.google.com/search?q=' + encodeURIComponent(url);
  } else if (!url.startsWith('http://') && !url.startsWith('https://') && !url.startsWith('file:')) {
    url = 'https://' + url;
  }
  
  const tab = tabs.get(id);
  if (tab) {
    tab.url = url;
    tab.loading = true;
    tab.loadingBar.classList.add('active');
    
    // Use PolyView proxy URL
    const proxyUrl = getProxyUrl(url);
    tab.iframe.src = proxyUrl;
    
    if (id === activeTabId) {
      $('urlInput').value = url;
      updateSecurityIcon(url);
    }
    
    renderTabs();
  }
}

function handleIframeMessage(tabId, data) {
  if (!data || !data.type?.startsWith('polyview:')) return;
  
  const tab = tabs.get(tabId);
  if (!tab) return;
  
  switch (data.type) {
    case 'polyview:navigate':
      tab.url = data.url;
      if (tabId === activeTabId) {
        $('urlInput').value = data.url;
        updateSecurityIcon(data.url);
      }
      break;
      
    case 'polyview:title':
      tab.title = data.title || 'New Tab';
      renderTabs();
      break;
      
    case 'polyview:loaded':
      tab.loading = false;
      tab.loadingBar.classList.remove('active');
      tab.url = data.url;
      tab.title = data.title || 'New Tab';
      if (tabId === activeTabId) {
        $('urlInput').value = data.url;
        updateSecurityIcon(data.url);
      }
      renderTabs();
      break;
  }
}

// ============================================
// Navigation
// ============================================

function goBack() {
  const tab = tabs.get(activeTabId);
  if (tab) {
    try {
      tab.iframe.contentWindow.history.back();
    } catch (e) {}
  }
}

function goForward() {
  const tab = tabs.get(activeTabId);
  if (tab) {
    try {
      tab.iframe.contentWindow.history.forward();
    } catch (e) {}
  }
}

function reload() {
  const tab = tabs.get(activeTabId);
  if (tab) {
    navigateTab(activeTabId, tab.url);
  }
}

function goHome() {
  navigateTab(activeTabId, 'https://duckduckgo.com');
}

// ============================================
// UI Rendering
// ============================================

function renderTabs() {
  const tabsContainer = $('tabs');
  
  tabsContainer.innerHTML = Array.from(tabs.entries()).map(([id, tab]) => {
    const faviconUrl = getFaviconUrl(tab.url);
    return `
      <div class="tab ${id === activeTabId ? 'active' : ''}" onclick="switchTab(${id})">
        <img class="tab-favicon" src="${faviconUrl}" onerror="this.style.display='none'">
        <span class="tab-title">${escapeHtml(tab.title)}</span>
        <button class="tab-close" onclick="event.stopPropagation(); closeTab(${id})">
          <svg viewBox="0 0 24 24"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
      </div>
    `;
  }).join('');
}

function getFaviconUrl(url) {
  try {
    const urlObj = new URL(url);
    return `https://www.google.com/s2/favicons?domain=${urlObj.hostname}&sz=32`;
  } catch {
    return '';
  }
}

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

function updateSecurityIcon(url) {
  const urlBox = $('urlBox');
  const icon = $('securityIcon');
  
  if (url.startsWith('https://')) {
    urlBox.classList.add('secure');
    icon.innerHTML = '<rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>';
  } else {
    urlBox.classList.remove('secure');
    icon.innerHTML = '<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>';
  }
}

// ============================================
// Event Listeners
// ============================================

$('urlInput').addEventListener('keydown', e => {
  if (e.key === 'Enter') {
    e.preventDefault();
    navigateTab(activeTabId, $('urlInput').value);
    $('urlInput').blur();
  } else if (e.key === 'Escape') {
    const tab = tabs.get(activeTabId);
    if (tab) $('urlInput').value = tab.url;
    $('urlInput').blur();
  }
});

$('urlInput').addEventListener('focus', () => {
  setTimeout(() => $('urlInput').select(), 0);
});

// Keyboard shortcuts
document.addEventListener('keydown', e => {
  if (e.ctrlKey && e.key === 't') {
    e.preventDefault();
    createTab();
  } else if (e.ctrlKey && e.key === 'w') {
    e.preventDefault();
    closeTab(activeTabId);
  } else if (e.ctrlKey && e.key === 'l') {
    e.preventDefault();
    $('urlInput').focus();
  } else if (e.ctrlKey && e.key === 'r') {
    e.preventDefault();
    reload();
  }
});

// Window drag
$('titlebar').querySelector('.titlebar-drag').addEventListener('mousedown', () => {
  if (window.polyWindow) polyWindow.drag();
});

$('titlebar').querySelector('.titlebar-drag').addEventListener('dblclick', () => {
  if (window.polyWindow) polyWindow.maximize();
});

// ============================================
// Initialize
// ============================================

// Create initial tab
createTab('https://duckduckgo.com');

console.log('[PolyView Browser] Ready! Using iframe2 proxy - no z-order issues!');
