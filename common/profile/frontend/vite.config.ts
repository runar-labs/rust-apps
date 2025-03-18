import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'path';

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      '@kagi/ui': path.resolve(__dirname, '../../../common/ui/src/lib/components/ui'),
      '@kagi/auth': path.resolve(__dirname, '../../auth/frontend/src')
    }
  }
}); 