import { defineConfig } from "vite";
import { VitePWA } from "vite-plugin-pwa";

export default defineConfig({
  server: {
    proxy: {
      "/api": "http://localhost:8000",
      "/health": "http://localhost:8000"
    }
  },
  plugins: [
    VitePWA({
      registerType: "autoUpdate",
      includeAssets: ["favicon.svg", "icon.svg"],
      manifest: {
        name: "Fruitbox",
        short_name: "Fruitbox",
        description: "Offline-capable Fruitbox singleplayer and solver playground.",
        theme_color: "#ffb347",
        background_color: "#1b1025",
        display: "standalone",
        scope: "/",
        start_url: "/",
        icons: [
          {
            src: "/icon.svg",
            sizes: "any",
            type: "image/svg+xml",
            purpose: "any maskable"
          }
        ]
      },
      workbox: {
        globPatterns: ["**/*.{js,css,html,svg,png,ico,webmanifest}"],
        runtimeCaching: [
          {
            urlPattern: ({ url }) => url.pathname.startsWith("/api/v1/modes"),
            handler: "NetworkFirst",
            options: {
              cacheName: "fruitbox-api"
            }
          }
        ]
      }
    })
  ]
});
