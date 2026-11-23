import { createServer } from 'vite';

const vite = await createServer({
  server: { middlewareMode: true, hmr: false, ws: false, watch: null },
  appType: 'custom',
  optimizeDeps: { noDiscovery: true, include: [] },
});
try {
  await vite.ssrLoadModule('/scripts/postbuild.ts');
} finally {
  await vite.close();
}
