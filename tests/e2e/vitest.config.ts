import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    globalSetup: './setup.ts',
    testTimeout: 30000,
    hookTimeout: 30000,
    globals: true,
  },
})
