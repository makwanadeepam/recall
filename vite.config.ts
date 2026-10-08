import { defineConfig } from "vite";

// Vite config tuned for Tauri.
// - Fixed dev port so Tauri can point its webview at it.
// - Don't clear the screen so Rust compiler errors stay visible.
// - Ignore the src-tauri folder to avoid needless HMR reloads.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  // Produce a small, modern bundle. Tauri ships a recent WebView, so we can
  // target modern JS and skip legacy transforms.
  build: {
    target: "es2022",
    minify: "esbuild",
    sourcemap: false,
    chunkSizeWarningLimit: 1000,
  },
  // Only expose env vars prefixed with VITE_ or TAURI_ to the client.
  envPrefix: ["VITE_", "TAURI_"],
});
