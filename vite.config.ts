import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri drives this dev server; the port must match `build.devUrl` in tauri.conf.json.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // src-tauri is watched by cargo, not by vite.
      ignored: ["**/src-tauri/**"],
    },
  },
  // Tauri targets a fixed, modern WebView2 - no need to down-level.
  build: {
    target: "esnext",
    minify: "esbuild",
    sourcemap: false,
  },
});
