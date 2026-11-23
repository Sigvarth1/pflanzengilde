import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = new URL('../', import.meta.url);
const cargoDir = new URL('rust/garden-core/', root);
const target = 'wasm32-unknown-unknown';
const result = spawnSync('cargo', [
  'build', '--locked', '--release', '--target', target, '--target-dir', 'target',
], { cwd: cargoDir, stdio: 'inherit' });
if (result.error) throw result.error;
if (result.status !== 0) process.exit(result.status ?? 1);

const output = new URL('src/generated/', root);
mkdirSync(output, { recursive: true });
copyFileSync(
  fileURLToPath(new URL(`target/${target}/release/garden_core.wasm`, cargoDir)),
  fileURLToPath(new URL('garden-core.wasm', output)),
);
