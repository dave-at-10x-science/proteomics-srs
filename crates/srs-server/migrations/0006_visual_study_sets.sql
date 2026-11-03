-- Content availability is separate from enablement so filling an optional picture can make
-- a card usable without changing the author's enablement choice or deleting its schedule.
ALTER TABLE cards ADD COLUMN content_available INTEGER NOT NULL DEFAULT 1;

-- Keep refresh receipts after note deletion so recreating an ID never reapplies an old edit.
CREATE TABLE seed_content_refreshes (
    refresh_id TEXT NOT NULL,
    note_id TEXT NOT NULL,
    PRIMARY KEY (refresh_id, note_id)
);

-- Retire the old drills in place. Their states and logs remain available.
UPDATE cards SET enabled = 0
WHERE template_id IN ('name-to-mass-shift', 'mass-shift-to-name', 'name-to-ms')
  AND note_id IN (SELECT id FROM notes WHERE note_type_id = 'PTM')
  AND note_id IN (
      'ptm-phospho', 'ptm-acetyl', 'ptm-methyl', 'ptm-dimethyl', 'ptm-trimethyl',
      'ptm-oxidation', 'ptm-deamidation', 'ptm-carbamidomethyl', 'ptm-gg',
      'ptm-citrullination', 'ptm-pyroglu', 'ptm-sulfo', 'ptm-succinyl', 'ptm-nitro'
  );

UPDATE cards SET enabled = 0
WHERE template_id IN ('term-to-definition', 'definition-to-term')
  AND note_id IN (SELECT id FROM notes WHERE note_type_id = 'Term')
  AND note_id IN (
      'term-mz', 'term-charge-state', 'term-isotope-envelope', 'term-monoisotopic-mass',
      'term-esi', 'term-maldi', 'term-lc', 'term-retention-time', 'term-ms1',
      'term-tandem-ms', 'term-precursor', 'term-by-ions', 'term-cid-hcd', 'term-dda',
      'term-dia', 'term-orbitrap', 'term-tof', 'term-quadrupole', 'term-trypsin',
      'term-missed-cleavage', 'term-bottom-up', 'term-psm', 'term-fdr', 'term-ptm'
  );

UPDATE cards SET enabled = 0
WHERE template_id IN ('term-to-definition', 'definition-to-term')
  AND note_id IN (SELECT id FROM notes WHERE note_type_id = 'GlycoConcept')
  AND note_id IN (
      'gly-n-glycosylation', 'gly-n-core', 'gly-high-mannose', 'gly-complex',
      'gly-hybrid', 'gly-core-fucose', 'gly-pngase-f', 'gly-endo-h', 'gly-o-galnac',
      'gly-tn', 'gly-core1', 'gly-core2', 'gly-core3', 'gly-o-glcnac', 'gly-o-release',
      'gly-oxonium', 'gly-y-ions', 'gly-ethcd', 'gly-heterogeneity', 'gly-enrichment',
      'gly-composition', 'gly-snfg'
  );
