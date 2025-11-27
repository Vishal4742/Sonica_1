import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'
import { fileURLToPath } from 'url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

// https://vite.dev/config/
export default defineConfig({
    plugins: [react()],
    resolve: {
        alias: {
            "@": path.resolve(__dirname, "./src"),
        },
    },
    build: {
        // Optimize build output
        target: 'es2015',
        minify: 'esbuild', // Use esbuild (default, faster, no extra deps)
        rollupOptions: {
            output: {
                // Code splitting for better caching
                manualChunks: {
                    'react-vendor': ['react', 'react-dom'],
                    'framer': ['framer-motion'],
                },
            },
        },
        chunkSizeWarningLimit: 1000, // Warn for chunks > 1MB
    },
    server: {
        proxy: {
            '/api': {
                target: 'http://localhost:8000',
                changeOrigin: true,
                rewrite: (path) => path.replace(/^\/api/, ''),
            },
        },
    },
})
