import { defineConfig } from 'vite';

// Poly starts its own backend for `/__poly_invoke` and project `assets/`.
// Point Vite's proxy at it so the app works identically in the browser and
// in the native window.
export default defineConfig({
  root: 'web',
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    proxy: {
      '/__poly_invoke': { target: process.env.POLY_BACKEND_URL || 'http://127.0.0.1:3000' },
      '/__poly_invoke_stream': { target: process.env.POLY_BACKEND_URL || 'http://127.0.0.1:3000' },
      '/assets': { target: process.env.POLY_BACKEND_URL || 'http://127.0.0.1:3000' }
    }
  },
  build: { outDir: '../dist/web', emptyOutDir: true }
});
