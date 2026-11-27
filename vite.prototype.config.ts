// PROTOTYPE, throwaway: serves prototype-accent.html outside Tauri. node_modules may live above this
// folder (agent worktree), so the dev server may read files outside the root.
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  server: { port: 1431, strictPort: true, fs: { strict: false } },
});
