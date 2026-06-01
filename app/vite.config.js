import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { writeFileSync, mkdirSync } from "fs";
import { resolve } from "path";

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
const d = new Date();
const buildTimestamp = `${String(d.getFullYear()).slice(2)}${String(d.getMonth()+1).padStart(2,'0')}${String(d.getDate()).padStart(2,'0')}${String(d.getHours()).padStart(2,'0')}${String(d.getMinutes()).padStart(2,'0')}`; // yyMMddHHmm local

// Write timestamp to a file so Gradle can read the same value for APK naming
const tsDir = resolve(__dirname, "src-tauri/gen/android/app");
try {
  mkdirSync(tsDir, { recursive: true });
  writeFileSync(resolve(tsDir, "build-timestamp.txt"), buildTimestamp);
} catch (_) { /* ignore in dev mode or if dir doesn't exist yet */ }

export default defineConfig({
  plugins: [sveltekit()],
  define: {
    __BUILD_TIMESTAMP__: JSON.stringify(buildTimestamp),
  },

  // Consume the design kit from source (no pre-build needed in dev or Tauri build).
  resolve: {
    alias: {
      "reglass-material/tokens.css": resolve(__dirname, "../packages/reglass-material/src/lib/tokens.css"),
      "reglass-material/theme": resolve(__dirname, "../packages/reglass-material/src/lib/theme/index.ts"),
      "reglass-material": resolve(__dirname, "../packages/reglass-material/src/lib/index.ts"),
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
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
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    target: 'es2022',
    minify: process.env.TAURI_ENV_DEBUG ? false : 'esbuild',
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});
