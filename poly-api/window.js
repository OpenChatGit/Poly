export function sendNativeMessage(message) {
  const host = globalThis.window;
  if (typeof host?.chrome?.webview?.postMessage === 'function') {
    host.chrome.webview.postMessage(message);
    return;
  }
  if (typeof host?.webkit?.messageHandlers?.ipc?.postMessage === 'function') {
    host.webkit.messageHandlers.ipc.postMessage(message);
    return;
  }
  throw new Error('Current-window controls are only available in a native Poly window.');
}

export function isNativeWindow() {
  const host = globalThis.window;
  return typeof host?.chrome?.webview?.postMessage === 'function'
    || typeof host?.webkit?.messageHandlers?.ipc?.postMessage === 'function';
}

export function getCurrentWindow() {
  return Object.freeze({
    minimize: () => sendNativeMessage('minimize'),
    maximize: () => sendNativeMessage('maximize'),
    restore: () => sendNativeMessage('restore'),
    close: () => sendNativeMessage('close'),
    startDragging: () => sendNativeMessage('drag')
  });
}
