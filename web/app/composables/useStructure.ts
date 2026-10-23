// Draws chemical structures from SMILES with RDKit.js (RDKit compiled to WASM), served from
// /rdkit so it works offline. Structures can be aligned to a depiction template so every
// member of a set is drawn in the same orientation.

import type { DepictionTemplate } from '~/types/api';

interface RDKitMol {
  get_molblock(): string;
  set_new_coords(): boolean;
  generate_aligned_coords(template: RDKitMol, details: string): string;
  get_svg_with_highlights(details: string): string;
  is_valid(): boolean;
  delete(): void;
}

interface RDKitModule {
  get_mol(input: string): RDKitMol | null;
}

declare global {
  interface Window {
    initRDKitModule?: (opts: { locateFile: () => string }) => Promise<RDKitModule>;
  }
}

let rdkit: Promise<RDKitModule> | null = null;
const templates = new Map<string, RDKitMol>();

function loadRDKit(): Promise<RDKitModule> {
  rdkit ??= new Promise<void>((resolve, reject) => {
    const script = document.createElement('script');
    script.src = '/rdkit/RDKit_minimal.js';
    script.onload = () => resolve();
    script.onerror = () => reject(new Error('could not load RDKit'));
    document.head.appendChild(script);
  }).then(() => window.initRDKitModule!({ locateFile: () => '/rdkit/RDKit_minimal.wasm' }));
  return rdkit;
}

/** A molecule with the template's fixed 2D coordinates, built by rewriting a V2000 molblock. */
function templateMol(RDKit: RDKitModule, t: DepictionTemplate): RDKitMol | null {
  const key = JSON.stringify(t);
  const cached = templates.get(key);
  if (cached) return cached;
  const base = RDKit.get_mol(t.smiles);
  if (!base) return null;
  base.set_new_coords();
  const lines = base.get_molblock().split('\n');
  base.delete();
  const atoms = parseInt(lines[3]!.slice(0, 3), 10);
  if (atoms !== t.coords.length) return null;
  t.coords.forEach(([x, y], i) => {
    const line = lines[4 + i]!;
    lines[4 + i] = x.toFixed(4).padStart(10) + y.toFixed(4).padStart(10) + line.slice(20);
  });
  const mol = RDKit.get_mol(lines.join('\n'));
  if (mol) templates.set(key, mol);
  return mol;
}

/**
 * SVG for a SMILES string. By default bonds have a fixed length so structures can be compared by
 * size; `fit` scales each drawing to fill the box instead (for small tiles).
 */
export async function renderStructure(
  smiles: string,
  template?: DepictionTemplate,
  size = 240,
  fit = false,
): Promise<string> {
  const RDKit = await loadRDKit();
  const mol = RDKit.get_mol(smiles);
  if (!mol || !mol.is_valid()) throw new Error(`invalid SMILES: ${smiles}`);
  try {
    const tmpl = template && templateMol(RDKit, template);
    if (tmpl) mol.generate_aligned_coords(tmpl, JSON.stringify({ acceptFailure: true }));
    // A fixed bond length keeps structures at the same scale, so sizes can be compared.
    const opts = { width: size, height: size, clearBackground: false, ...(fit ? { padding: 0.08 } : { fixedBondLength: 26 }) };
    return mol.get_svg_with_highlights(JSON.stringify(opts));
  } finally {
    mol.delete();
  }
}
