import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

import { resolve } from 'path'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@bufbuild/protobuf/wire': resolve(__dirname, 'node_modules/@bufbuild/protobuf'),
      '@bufbuild/protobuf': resolve(__dirname, 'node_modules/@bufbuild/protobuf'),
      '@shared': resolve(__dirname, '../../shared/ts-core/src')
    }
  },
  server: {
    fs: {
      allow: ['.', '../../shared']
    }
  }
})
