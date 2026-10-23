#!/usr/bin/env python3
"""Validate SMILES in a set file and render preview structures.

For each note it checks that the SMILES parses, that the residue mass computed from it matches
`residue_mass_mono` (if present), and draws the structure aligned to the note type's depiction
template so every structure has the same orientation.

The app renders structures from SMILES at runtime; these SVGs and the contact sheet are
previews for reviewing content.

Usage:
    pip install rdkit pyyaml
    python tools/render_structures.py seed/amino-acids/amino-acids.yaml
"""

import argparse
import pathlib
import sys

import yaml
from rdkit import Chem
from rdkit.Chem import AllChem, Draw
from rdkit.Chem.Descriptors import ExactMolWt
from rdkit.Chem.Draw import rdMolDraw2D
from rdkit.Geometry import Point3D

WATER = 18.010565
MASS_TOLERANCE = 1e-4


def template_mol(tmpl):
    mol = Chem.MolFromSmiles(tmpl["smiles"])
    if mol.GetNumAtoms() != len(tmpl["coords"]):
        sys.exit(f"depiction template has {mol.GetNumAtoms()} atoms but {len(tmpl['coords'])} coords")
    conf = Chem.Conformer(mol.GetNumAtoms())
    for i, (x, y) in enumerate(tmpl["coords"]):
        conf.SetAtomPosition(i, Point3D(x, y, 0))
    mol.AddConformer(conf)
    return mol


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("set_file", type=pathlib.Path)
    args = ap.parse_args()

    data = yaml.safe_load(args.set_file.read_text())
    nt = data["note_type"]
    structure = next((f for f in nt["fields"] if f.get("from") == "smiles"), None)
    tmpl = None
    if structure and structure.get("depiction_template"):
        tmpl = template_mol(nt["depiction_templates"][structure["depiction_template"]])

    media = args.set_file.parent / "media"
    media.mkdir(exist_ok=True)
    mols, legends, problems = [], [], []

    for note in data["notes"]:
        mol = Chem.MolFromSmiles(note.get("smiles", ""))
        if mol is None:
            problems.append(f"{note['id']}: SMILES does not parse")
            continue
        if "residue_mass_mono" in note:
            calc = ExactMolWt(mol) - WATER
            if abs(calc - note["residue_mass_mono"]) > MASS_TOLERANCE:
                problems.append(f"{note['id']}: residue mass {note['residue_mass_mono']} but SMILES gives {calc:.5f}")
        if tmpl is not None:
            AllChem.GenerateDepictionMatching2DStructure(mol, tmpl)
        else:
            AllChem.Compute2DCoords(mol)

        drawer = rdMolDraw2D.MolDraw2DSVG(300, 300)
        drawer.drawOptions().clearBackground = False
        drawer.drawOptions().fixedBondLength = 30
        drawer.DrawMolecule(mol)
        drawer.FinishDrawing()
        (media / f"{note['id']}.svg").write_text(drawer.GetDrawingText())
        mols.append(mol)
        legends.append(note.get("name", note["id"]))

    if mols:
        sheet = Draw.MolsToGridImage(mols, molsPerRow=5, subImgSize=(220, 220), legends=legends)
        sheet.save(args.set_file.parent / "preview.png")

    for p in problems:
        print(f"PROBLEM {p}")
    print(f"{len(mols)} structures rendered, {len(problems)} problems")
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()
