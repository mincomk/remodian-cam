import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { VitePWA } from 'vite-plugin-pwa'
import icons from './public/icons.json' assert { type: 'json' }

// https://vite.dev/config/
export default defineConfig({
    plugins: [tailwindcss(), react(),
    VitePWA({
        registerType: 'autoUpdate',
        includeAssets: ['favicon.ico', 'robots.txt', 'apple-touch-icon.png'],
        manifest: {
            name: 'Remodian',
            short_name: 'Remodian',
            description: 'Meridian Remote Controller',
            theme_color: '#ffffff',
            background_color: '#ffffff',
            display: 'standalone',
            start_url: '/',
            icons: icons.icons
        },
        workbox: {
            globPatterns: ['**/*.{js,css,html,ico,png,svg}']
        },
        devOptions: {
            enabled: true
        }
    })
    ],
})


