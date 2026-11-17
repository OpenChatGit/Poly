import { invoke } from './invoke.js';

export function request(url, options = {}) {
  const { method = 'GET', headers = {}, body, timeout } = options;
  return invoke('__poly_http_request', {
    url: String(url),
    method,
    headers,
    ...(body === undefined ? {} : { body }),
    ...(timeout === undefined ? {} : { timeout })
  });
}

export function get(url, options = {}) {
  return request(url, { ...options, method: 'GET' });
}

export function post(url, body, options = {}) {
  return request(url, { ...options, method: 'POST', body });
}
