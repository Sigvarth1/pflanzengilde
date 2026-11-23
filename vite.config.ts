import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { existsSync } from 'node:fs';

export default defineConfig({
  plugins: [
    react(),
    {
      name: 'production-site-entry',
      apply: 'build',
      transformIndexHtml: {
        order: 'pre',
        handler(html) {
          const hasLegalPages = ['Impressum', 'Datenschutz'].some(name =>
            existsSync(new URL(`./src/legal/${name}.tsx`, import.meta.url))
          );
          const script = hasLegalPages ? '<script type="module" src="/src/legal-entry.tsx"></script>' : '';
          return {
            html: html.replace('<script type="module" src="/src/main.tsx"></script>', script),
            tags: [{ tag: 'link', attrs: { rel: 'stylesheet', href: '/src/index.css' }, injectTo: 'head' }],
          };
        },
      },
    },
  ],
  server: {
    port: 3000,
    open: false,
  },
  build: {
    emptyOutDir: true,
  },
});
