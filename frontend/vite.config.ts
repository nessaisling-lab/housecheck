import path from "path"
import react from "@vitejs/plugin-react"
import { defineConfig, loadEnv } from "vite"
import { inspectAttr } from 'kimi-plugin-inspect-react'

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, __dirname, "")
  // Backend (Rust/Axum) origin — override in .env.local as VITE_BACKEND_URL
  const backend = env.VITE_BACKEND_URL || "http://localhost:8080"
  return {
    base: '/',
    plugins: [inspectAttr(), react()],
    server: {
      port: 3000,
      proxy: {
        // Frontend calls /api/* → backend /* (avoids CORS in dev)
        "/api": {
          target: backend,
          changeOrigin: true,
          rewrite: (p) => p.replace(/^\/api/, ""),
        },
      },
    },
    resolve: {
      alias: {
        "@": path.resolve(__dirname, "./src"),
      },
    },
  }
});
