import { fileURLToPath, URL } from "node:url";

import vue from "@vitejs/plugin-vue";
import { defineConfig, loadEnv } from "vite";

export default defineConfig(({ mode }) => {
  const environment = loadEnv(mode, process.cwd(), "VITE_");
  return {
    plugins: [vue()],
    resolve: {
      alias: {
        "@": fileURLToPath(new URL("./src", import.meta.url)),
      },
    },
    clearScreen: false,
    server: {
      host: "127.0.0.1",
      port: 1420,
      strictPort: true,
      proxy: {
        "/api": {
          target: environment.VITE_DSH_DESKTOP_BRIDGE_URL ?? "http://127.0.0.1:1421",
          changeOrigin: true,
          ws: true,
        },
      },
      watch: {
        ignored: ["**/apps/desktop/**"],
      },
    },
  };
});
