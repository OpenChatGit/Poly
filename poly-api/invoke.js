export async function invoke(fn, args = {}) {
  const response = await fetch('/__poly_invoke', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ fn, args })
  });

  let payload;
  try {
    payload = await response.json();
  } catch {
    throw new Error(`Poly API returned an invalid response for ${fn}.`);
  }
  if (!response.ok || payload.error) {
    throw new Error(payload.error || `Poly API request failed with HTTP ${response.status}.`);
  }
  return Object.prototype.hasOwnProperty.call(payload, 'result') ? payload.result : payload;
}
