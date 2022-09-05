import { defineConfig } from 'vite'

export default defineConfig({
    server: {
      port: 3001,
      proxy: {
        // with options
        '/api': {
          target: 'http://localhost:8080',
        }
      }
    }
  })
  