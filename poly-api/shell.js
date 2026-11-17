import { invoke } from './invoke.js';

export function openUrl(url) {
  return invoke('__poly_shell_open', { url: String(url) });
}
