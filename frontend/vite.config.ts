import { defineConfig } from 'vite'

export default defineConfig({
    server: {
      proxy: {
        // with options
        '/api': {
          target: 'http://localhost:3030',
        }
      }
    }
  })
  