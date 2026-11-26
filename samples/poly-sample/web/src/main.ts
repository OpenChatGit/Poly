import { isNativeWindow, poly } from './poly.js';

const NOTE_PATH = 'poly-sample-note.txt';

type StatusKind = 'info' | 'ok' | 'error';

function requireElement<T extends HTMLElement>(id: string): T {
  const element = document.getElementById(id);
  if (!element) throw new Error(`Missing #${id} in index.html`);
  return element as T;
}

const clipboardInput = requireElement<HTMLTextAreaElement>('clipboard-input');
const clipboardOutput = requireElement<HTMLPreElement>('clipboard-output');
const noteInput = requireElement<HTMLTextAreaElement>('note-input');
const noteOutput = requireElement<HTMLPreElement>('note-output');
const counterValue = requireElement<HTMLSpanElement>('counter-value');
const status = requireElement<HTMLParagraphElement>('status');

function setStatus(message: string, kind: StatusKind = 'info'): void {
  status.textContent = message;
  status.dataset['kind'] = kind;
}

function showOutput(element: HTMLPreElement, value: unknown): void {
  element.textContent = typeof value === 'string' ? value : JSON.stringify(value, null, 2);
}

async function guard(label: string, action: () => Promise<void>): Promise<void> {
  try {
    await action();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    setStatus(`${label} failed: ${message}`, 'error');
  }
}

function on(id: string, handler: () => Promise<void> | void): void {
  requireElement<HTMLButtonElement>(id).addEventListener('click', () => {
    void guard(id, async () => {
      await handler();
    });
  });
}

async function loadPlatform(): Promise<void> {
  const [platform, arch, home] = await Promise.all([
    poly.os.platform(),
    poly.os.arch(),
    poly.os.homedir()
  ]);
  requireElement('os-platform').textContent = platform;
  requireElement('os-arch').textContent = arch;
  requireElement('os-home').textContent = home || 'unknown';
  requireElement('os-native').textContent = isNativeWindow() ? 'yes' : 'no (browser)';
}

let count = 0;

requireElement<HTMLButtonElement>('counter-increment').addEventListener('click', () => {
  count += 1;
  counterValue.textContent = String(count);
});

on('counter-notify', async () => {
  await poly.notification.show('Poly Sample', `Counter is at ${count}.`);
  setStatus('Notification sent.', 'ok');
});

on('clipboard-write', async () => {
  await poly.clipboard.write(clipboardInput.value);
  setStatus('Copied to the system clipboard.', 'ok');
});

on('clipboard-read', async () => {
  const text = await poly.clipboard.read();
  showOutput(clipboardOutput, text || '(clipboard is empty)');
  setStatus('Read the system clipboard.', 'ok');
});

on('note-save', async () => {
  await poly.fs.write(NOTE_PATH, noteInput.value);
  showOutput(noteOutput, `wrote ${NOTE_PATH}`);
  setStatus(`Saved ${NOTE_PATH} in the app's working directory.`, 'ok');
});

on('note-load', async () => {
  const text = await poly.fs.read(NOTE_PATH);
  noteInput.value = text;
  showOutput(noteOutput, text);
  setStatus(`Loaded ${NOTE_PATH}.`, 'ok');
});

on('note-delete', async () => {
  if (!(await poly.fs.exists(NOTE_PATH))) {
    showOutput(noteOutput, `${NOTE_PATH} is not on disk`);
    setStatus('Nothing to delete.', 'info');
    return;
  }
  await poly.fs.delete(NOTE_PATH);
  showOutput(noteOutput, `deleted ${NOTE_PATH}`);
  setStatus(`Deleted ${NOTE_PATH}.`, 'ok');
});

setStatus(
  isNativeWindow()
    ? 'Poly APIs connected.'
    : 'Running in a browser; native window commands are disabled.',
  'ok'
);

void guard('Platform info', loadPlatform);
