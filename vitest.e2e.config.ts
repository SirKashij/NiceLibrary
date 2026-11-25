// End-to-end tests against the built app (e2e/). Run with `npm run test:e2e`
// after building the app; CONTRIBUTING.md explains the setup.
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['e2e/**/*.e2e.ts'],
    environment: 'node',
    // Every test file starts its own tauri-driver on the same fixed port.
    fileParallelism: false,
    testTimeout: 30_000,
    hookTimeout: 60_000,
  },
});
