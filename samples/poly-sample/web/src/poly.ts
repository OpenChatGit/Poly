/**
 * Typed bridge to Poly's JavaScript backend.
 *
 * Poly does not inject a `poly` global (see docs/API.md, "Window Control").
 * Every backend call goes through the framework's HTTP IPC endpoint, which is
 * served by both `poly dev` and the native window's embedded dev server.
 */

export type Platform = 'windows' | 'macos' | 'linux' | 'unknown';

export type InvokeArgs = Record<string, unknown>;

type PolyResponse<T> = { result: T } | { error: string };

async function invoke<T>(name: string, args: InvokeArgs = {}): Promise<T> {
  const response = await fetch('/__poly_invoke', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ fn: name, args })
  });

  let payload: PolyResponse<T>;
  try {
    payload = (await response.json()) as PolyResponse<T>;
  } catch {
    throw new Error(`Poly returned an unreadable response for ${name}.`);
  }

  if (!response.ok || 'error' in payload) {
    const message = 'error' in payload ? payload.error : `HTTP ${response.status}`;
    throw new Error(message || `Poly request ${name} failed with HTTP ${response.status}.`);
  }
  return payload.result;
}

interface NativeIpcHost {
  chrome?: { webview?: { postMessage(message: string): void } };
  webkit?: { messageHandlers?: { ipc?: { postMessage(message: string): void } } };
}

function nativeHost(): NativeIpcHost | undefined {
  return globalThis.window as (Window & NativeIpcHost) | undefined;
}

export type NativeMessage = 'minimize' | 'maximize' | 'restore' | 'close' | 'drag';

export function sendNativeMessage(message: NativeMessage): void {
  const host = nativeHost();
  const webviewPost = host?.chrome?.webview?.postMessage;
  if (typeof webviewPost === 'function') {
    webviewPost.call(host!.chrome!.webview, message);
    return;
  }
  const ipcPost = host?.webkit?.messageHandlers?.ipc?.postMessage;
  if (typeof ipcPost === 'function') {
    ipcPost.call(host!.webkit!.messageHandlers!.ipc, message);
    return;
  }
  throw new Error('Window controls are only available in a Poly desktop window.');
}

export function isNativeWindow(): boolean {
  const host = nativeHost();
  return typeof host?.chrome?.webview?.postMessage === 'function'
    || typeof host?.webkit?.messageHandlers?.ipc?.postMessage === 'function';
}

export const poly = Object.freeze({
  invoke,
  os: {
    platform: () => invoke<Platform>('__poly_os_platform'),
    arch: () => invoke<string>('__poly_os_arch'),
    homedir: () => invoke<string>('__poly_os_homedir')
  },
  clipboard: {
    read: () => invoke<string>('__poly_clipboard_read'),
    write: (text: string) => invoke<boolean>('__poly_clipboard_write', { text })
  },
  notification: {
    show: (title: string, body: string) => invoke<boolean>('__poly_notification_show', { title, body })
  },
  fs: {
    read: (path: string) => invoke<string>('__poly_fs_read', { path }),
    write: (path: string, content: string) => invoke<boolean>('__poly_fs_write', { path, content }),
    exists: (path: string) => invoke<boolean>('__poly_fs_exists', { path }),
    delete: (path: string) => invoke<boolean>('__poly_fs_delete', { path })
  }
});
