// Poly Browser - Main Application Script
// Uses the new Multi-Tab Browser API

const $ = id => document.getElementById(id);

// Elements
const tabsContainer = $('tabs');
const urlInput = $('urlInput');
const urlBox = $('urlBox');
const securityIcon = $('securityIcon');
const loadingBar = $('loadingBar');
const content = $('content');

// Tab State - maps native tabId to tab data
let tabs = new Map(); // tabId -> { url, title, loading }
let activeTabId = null;

// ============================================
// Favicon Helper
// ============================================

function getFaviconUrl(url) {
  try {
    const urlObj = new URL(url);
    return `https://www.google.com/s2/favicons?domain=${urlObj.hostname}&sz=32`;
  } catch {
    return null;
  }
}

// ============================================
// Tab API (communicates with native)
// ============================================

function createTab(url = 'https://google.com') {
  // Send command to native - it will create the WebView and send back tabCreated event
  if (window.ipc) {
    window.ipc.postMessage('createTab:' + url);
  }
}

function closeTab(tabId) {
  if (window.ipc && tabs.size > 1) {
    window.ipc.postMessage('closeTab:' + tabId);
  }
}

function switchTab(tabId) {
  if (window.ipc && tabId !== activeTabId) {
    window.ipc.postMessage('switchTab:' + tabId);
  }
}

function navigateTab(tabId, url) {
  if (!url) url = urlInput.value.trim();
  if (!url) return;
  
  // Check if search query or URL
  if (!url.includes('.') && !url.startsWith('http') && !url.startsWith('file:')) {
    url = 'https://www.google.com/search?q=' + encodeURIComponent(url);
  } else if (!url.startsWith('http://') && !url.startsWith('https://') && !url.startsWith('file:')) {
    url = 'https://' + url;
  }
  
  if (window.ipc) {
    window.ipc.postMessage('navigate:' + tabId + ':' + url);
  }
  
  // Update local state
  const tab = tabs.get(tabId);
  if (tab) {
    tab.url = url;
    tab.loading = true;
    renderTabs();
  }
  
  if (tabId === activeTabId) {
    urlInput.value = url;
    updateSecurityIcon(url);
    startLoading();
  }
}

function goBack() {
  if (window.ipc && activeTabId) {
    window.ipc.postMessage('goBack:' + activeTabId);
  }
}

function goForward() {
  if (window.ipc && activeTabId) {
    window.ipc.postMessage('goForward:' + activeTabId);
  }
}

function reload() {
  if (activeTabId) {
    const tab = tabs.get(activeTabId);
    if (tab) {
      navigateTab(activeTabId, tab.url);
    }
  }
}

function goHome() {
  if (activeTabId) {
    navigateTab(activeTabId, 'https://google.com');
  }
}

// ============================================
// Native Event Handlers (called by Poly)
// ============================================

// Called when a new tab WebView is created
window.onTabCreated = function(tabId) {
  console.log('[Browser] Tab created:', tabId);
  tabs.set(tabId, {
    url: 'about:blank',
    title: 'New Tab',
    loading: true
  });
  renderTabs();
};

// Called when a tab becomes active
window.onTabActivated = function(tabId) {
  console.log('[Browser] Tab activated:', tabId);
  activeTabId = tabId;
  content.classList.add('hidden');
  
  const tab = tabs.get(tabId);
  if (tab) {
    urlInput.value = tab.url;
    updateSecurityIcon(tab.url);
    if (tab.loading) {
      startLoading();
    } else {
      stopLoading();
    }
  }
  
  renderTabs();
};

// Called when a tab is closed
window.onTabClosed = function(tabId) {
  console.log('[Browser] Tab closed:', tabId);
  tabs.delete(tabId);
  renderTabs();
};

// Called when navigation starts in a tab
window.onTabNavStart = function(tabId, url) {
  console.log('[Browser] Tab', tabId, 'navigating to:', url);
  const tab = tabs.get(tabId);
  if (tab) {
    tab.url = url;
    tab.loading = true;
    renderTabs();
  }
  
  if (tabId === activeTabId) {
    urlInput.value = url;
    updateSecurityIcon(url);
    startLoading();
  }
};

// Called when page title changes (usually means loaded)
window.onTabTitleChange = function(tabId, title) {
  console.log('[Browser] Tab', tabId, 'title:', title);
  const tab = tabs.get(tabId);
  if (tab) {
    tab.title = title || 'New Tab';
    tab.loading = false;
    renderTabs();
  }
  
  if (tabId === activeTabId) {
    stopLoading();
  }
};

// Called when page finishes loading
window.onTabLoadEnd = function(tabId) {
  console.log('[Browser] Tab', tabId, 'loaded');
  const tab = tabs.get(tabId);
  if (tab) {
    tab.loading = false;
    renderTabs();
  }
  
  if (tabId === activeTabId) {
    stopLoading();
  }
};

// ============================================
// UI Rendering
// ============================================

function renderTabs() {
  const tabsArray = Array.from(tabs.entries());
  
  tabsContainer.innerHTML = tabsArray.map(([tabId, tab]) => {
    const faviconUrl = getFaviconUrl(tab.url);
    const faviconHtml = tab.loading
      ? `<svg class="tab-favicon-svg loading" viewBox="0 0 24 24" fill="none" stroke="#52525b" stroke-width="2">
           <circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/>
         </svg>`
      : faviconUrl
        ? `<img class="tab-favicon" src="${faviconUrl}" onerror="this.style.display='none';this.nextElementSibling.style.display='block'"/>
           <svg class="tab-favicon-svg" style="display:none" viewBox="0 0 24 24" fill="none" stroke="#52525b" stroke-width="2">
             <circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/>
             <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
           </svg>`
        : `<svg class="tab-favicon-svg" viewBox="0 0 24 24" fill="none" stroke="#52525b" stroke-width="2">
             <circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/>
             <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
           </svg>`;
    
    return `
      <div class="tab ${tabId === activeTabId ? 'active' : ''}" data-id="${tabId}">
        ${faviconHtml}
        <span class="tab-title">${escapeHtml(tab.title)}</span>
        <button class="tab-close" data-close="${tabId}">
          <svg viewBox="0 0 24 24"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
      </div>
    `;
  }).join('');
  
  // Add click handlers
  tabsContainer.querySelectorAll('.tab').forEach(el => {
    el.addEventListener('click', (e) => {
      if (e.target.closest('.tab-close')) return;
      switchTab(parseInt(el.dataset.id));
    });
  });
  
  tabsContainer.querySelectorAll('.tab-close').forEach(el => {
    el.addEventListener('click', (e) => {
      e.stopPropagation();
      closeTab(parseInt(el.dataset.close));
    });
  });
}

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// ============================================
// Window Controls
// ============================================

$('minBtn').onclick = () => {
  if (window.ipc) window.ipc.postMessage('minimize');
};

$('maxBtn').onclick = () => {
  if (window.ipc) window.ipc.postMessage('maximize');
};

$('closeBtn').onclick = () => {
  if (window.ipc) window.ipc.postMessage('close');
};

// Window drag - on drag region
$('dragRegion').addEventListener('mousedown', (e) => {
  if (window.ipc) window.ipc.postMessage('drag');
});

$('dragRegion').addEventListener('dblclick', (e) => {
  if (window.ipc) window.ipc.postMessage('maximize');
});

// ============================================
// Loading State
// ============================================

let loadingTimeout = null;

function startLoading() {
  if (loadingTimeout) clearTimeout(loadingTimeout);
  loadingBar.classList.add('active');
  
  // Safety timeout
  loadingTimeout = setTimeout(stopLoading, 5000);
}

function stopLoading() {
  if (loadingTimeout) {
    clearTimeout(loadingTimeout);
    loadingTimeout = null;
  }
  loadingBar.classList.remove('active');
}

// ============================================
// UI Helpers
// ============================================

function updateSecurityIcon(url) {
  if (url.startsWith('https://')) {
    urlBox.classList.add('secure');
    securityIcon.innerHTML = '<rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>';
  } else {
    urlBox.classList.remove('secure');
    securityIcon.innerHTML = '<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>';
  }
}

// ============================================
// Event Listeners
// ============================================

urlInput.addEventListener('keydown', e => {
  if (e.key === 'Enter') {
    e.preventDefault();
    if (activeTabId) {
      navigateTab(activeTabId, urlInput.value);
    }
    urlInput.blur();
  } else if (e.key === 'Escape') {
    const tab = tabs.get(activeTabId);
    if (tab) urlInput.value = tab.url;
    urlInput.blur();
  }
});

urlInput.addEventListener('focus', () => {
  setTimeout(() => urlInput.select(), 0);
});

$('backBtn').onclick = goBack;
$('forwardBtn').onclick = goForward;
$('reloadBtn').onclick = reload;
$('homeBtn').onclick = goHome;
$('newTabBtn').onclick = () => createTab();

// Keyboard shortcuts
document.addEventListener('keydown', e => {
  if (e.ctrlKey && e.key === 't') {
    e.preventDefault();
    createTab();
  } else if (e.ctrlKey && e.key === 'w') {
    e.preventDefault();
    if (activeTabId) closeTab(activeTabId);
  } else if (e.ctrlKey && e.key === 'l') {
    e.preventDefault();
    urlInput.focus();
  }
});

// ============================================
// Initialize
// ============================================

function initBrowser() {
  if (window.ipc) {
    content.classList.add('hidden');
    console.log('[Browser] IPC ready, waiting for initial tab...');
    // The native side creates the initial tab automatically
    // We just wait for onTabCreated and onTabActivated events
  } else {
    setTimeout(initBrowser, 50);
  }
}

setTimeout(initBrowser, 100);
