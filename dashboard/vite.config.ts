import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		proxy: {
			// Proxy API calls to your Rust backend during development
			'/api': {
				target: 'http://localhost:8000',
				changeOrigin: true
			},
			// Proxy metrics endpoint
			'/metrics': {
				target: 'http://localhost:8000',
				changeOrigin: true
			}
		}
	}
});
