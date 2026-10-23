// Copy RDKit's WASM build into public/ so structures render offline, served by our own server.
import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
// The package's main entry is dist/RDKit_minimal.js; the .wasm sits beside it.
const dist = dirname(require.resolve('@rdkit/rdkit'));
const out = join(dirname(fileURLToPath(import.meta.url)), '..', 'public', 'rdkit');
mkdirSync(out, { recursive: true });
for (const f of ['RDKit_minimal.js', 'RDKit_minimal.wasm']) {
  copyFileSync(join(dist, f), join(out, f));
}
console.log(`copied RDKit to ${out}`);
