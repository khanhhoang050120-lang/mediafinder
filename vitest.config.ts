import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import path from "node:path";

// Các module của Tauri được thay bằng bản giả nối vào một recorder mà từng
// bài kiểm thử tự gắn lên `globalThis.__ipc` — nhờ vậy mỗi bài tự quyết định
// backend trả lời gì, và mọi lệnh gửi đi đều đếm được.
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    // Bài kiểm thử chạy trong Node nhưng component là mã trình duyệt; thiếu
    // điều kiện này thì `svelte` phân giải sang bản server, nơi không có
    // `mount()`.
    conditions: ["browser"],
    alias: {
      "@tauri-apps/api/core": path.resolve(__dirname, "tests/stubs/tauri-core.ts"),
      "@tauri-apps/api/event": path.resolve(__dirname, "tests/stubs/tauri-event.ts"),
      "@tauri-apps/api/webviewWindow": path.resolve(__dirname, "tests/stubs/tauri-webview-window.ts"),
      "@tauri-apps/plugin-updater": path.resolve(__dirname, "tests/stubs/tauri-updater.ts"),
      "@tauri-apps/plugin-process": path.resolve(__dirname, "tests/stubs/tauri-process.ts"),
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["tests/vitest.setup.ts"],
    include: ["tests/**/*.test.ts"],
    // Các kịch bản dùng đồng hồ thật (nhịp poll 250–3000ms của app), nên một
    // nhóm có thể chạy tới chục giây.
    testTimeout: 60_000,
    // Các nhóm dùng chung globalThis.__ipc; chạy tuần tự để không giẫm nhau.
    fileParallelism: false,
  },
});
