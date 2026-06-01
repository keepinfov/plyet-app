import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

// Playground dev server. Port 5180 chosen to avoid clashing with the app's
// strict Tauri port (1420).
export default defineConfig({
  plugins: [sveltekit()],
  server: {
    port: 5180,
    strictPort: true,
  },
});
