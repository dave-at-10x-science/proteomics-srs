-- Refresh original seed fields individually so edits made in the app survive.
WITH replacements (id, old_value, new_value) AS (
    VALUES
        ('mono-glc', 'Blue circle', '/study/monosaccharides/glc.svg'),
        ('mono-man', 'Green circle', '/study/monosaccharides/man.svg'),
        ('mono-gal', 'Yellow circle', '/study/monosaccharides/gal.svg'),
        ('mono-glcnac', 'Blue square', '/study/monosaccharides/glcnac.svg'),
        ('mono-galnac', 'Yellow square', '/study/monosaccharides/galnac.svg'),
        ('mono-fuc', 'Red triangle', '/study/monosaccharides/fuc.svg'),
        ('mono-neuac', 'Purple diamond', '/study/monosaccharides/neu5ac.svg'),
        ('mono-neugc', 'Light blue diamond', '/study/monosaccharides/neu5gc.svg'),
        ('mono-xyl', 'Orange star', '/study/monosaccharides/xyl.svg')
)
UPDATE notes
SET fields = json_set(fields, '$.symbol', (SELECT new_value FROM replacements WHERE id = notes.id))
WHERE note_type_id = 'Monosaccharide'
  AND EXISTS (
      SELECT 1 FROM replacements
      WHERE id = notes.id AND json_extract(notes.fields, '$.symbol') = old_value
  );

WITH replacements (id, old_value, new_value) AS (
    VALUES
        ('mono-glc',
         'Glucose, mannose and galactose are isomers: all three are "Hex" to the mass spectrometer. Glucose is part of the N-glycan precursor in the ER and is trimmed off before the glycoprotein leaves.',
         'Shares the Hex composition class with mannose and galactose; the symbol distinguishes them.'),
        ('mono-man',
         'Three mannoses form the shared N-glycan core; high-mannose glycans carry 5 to 9.',
         'Three mannose residues form the shared core of animal N-glycans.'),
        ('mono-gal',
         'Extends the antennae of complex N-glycans and forms O-glycan core 1 (Galβ1-3GalNAc).',
         'Often extends the branches of complex N-glycans.'),
        ('mono-glcnac',
         'Two GlcNAcs anchor every N-glycan to asparagine. A single GlcNAc on Ser/Thr is O-GlcNAc.',
         'The first sugar attached to asparagine in animal N-glycans.'),
        ('mono-galnac',
         'The first sugar of mucin-type O-glycans, attached to Ser or Thr. Same mass as GlcNAc.',
         'Starts mucin-type O-glycans on serine or threonine.'),
        ('mono-fuc',
         'A deoxyhexose. Core fucose (α1-6 on the Asn-linked GlcNAc) is common on complex N-glycans; antenna fucose builds Lewis antigens.',
         'A deoxyhexose, commonly found on the core of N-glycans.'),
        ('mono-neuac',
         'The main human sialic acid, capping glycan antennae with a negative charge. Labile in the source and in CID; oxonium ions at m/z 292.103 and 274.092.',
         'The most common sialic acid in humans.'),
        ('mono-neugc',
         'One oxygen heavier than NeuAc (+15.995 Da). Humans can''t make it, so it flags non-human (e.g. mouse or hamster cell) glycosylation in biotherapeutics.',
         'A sialic acid made by many mammals, but not synthesized by humans.'),
        ('mono-xyl',
         'A pentose. Starts the linker of glycosaminoglycan chains on proteoglycans, and appears as β1-2 xylose on plant N-glycans.',
         'Links many glycosaminoglycan chains to their core protein.')
)
UPDATE notes
SET fields = json_set(fields, '$.note', (SELECT new_value FROM replacements WHERE id = notes.id))
WHERE note_type_id = 'Monosaccharide'
  AND EXISTS (
      SELECT 1 FROM replacements
      WHERE id = notes.id AND json_extract(notes.fields, '$.note') = old_value
  );

WITH replacements (id, old_value, new_value) AS (
    VALUES ('mono-neuac', 'NeuAc', 'Neu5Ac'), ('mono-neugc', 'NeuGc', 'Neu5Gc')
)
UPDATE notes
SET fields = json_set(fields, '$.abbreviation', (SELECT new_value FROM replacements WHERE id = notes.id))
WHERE note_type_id = 'Monosaccharide'
  AND EXISTS (
      SELECT 1 FROM replacements
      WHERE id = notes.id AND json_extract(notes.fields, '$.abbreviation') = old_value
  );

WITH aliases (id, canonical, alias) AS (
    VALUES ('mono-neuac', 'Neu5Ac', 'NeuAc'), ('mono-neugc', 'Neu5Gc', 'NeuGc')
)
UPDATE notes
SET fields = json_set(fields, '$.abbreviation_alias', (SELECT alias FROM aliases WHERE id = notes.id))
WHERE note_type_id = 'Monosaccharide'
  AND json_type(fields, '$.abbreviation_alias') IS NULL
  AND EXISTS (
      SELECT 1 FROM aliases
      WHERE id = notes.id AND json_extract(notes.fields, '$.abbreviation') = canonical
  );

WITH originals (id, value) AS (
    VALUES ('mono-neuac', 'NeuAc'), ('mono-neugc', 'NeuGc')
)
UPDATE notes
SET fields = json_set(fields, '$.shorthand', 'Sialic acid')
WHERE note_type_id = 'Monosaccharide'
  AND EXISTS (
      SELECT 1 FROM originals
      WHERE id = notes.id AND json_extract(notes.fields, '$.shorthand') = value
  );

-- Remove only the original reference values; authored replacements remain available.
WITH originals (id, value) AS (
    VALUES
        ('mono-glc', 162.05282), ('mono-man', 162.05282), ('mono-gal', 162.05282),
        ('mono-glcnac', 203.07937), ('mono-galnac', 203.07937), ('mono-fuc', 146.05791),
        ('mono-neuac', 291.09542), ('mono-neugc', 307.09033), ('mono-xyl', 132.04226)
)
UPDATE notes
SET fields = json_remove(fields, '$.residue_mass')
WHERE note_type_id = 'Monosaccharide'
  AND EXISTS (
      SELECT 1 FROM originals
      WHERE id = notes.id AND json_extract(notes.fields, '$.residue_mass') = value
  );

WITH originals (id, value) AS (
    VALUES
        ('mono-glc', 'C6H10O5'), ('mono-man', 'C6H10O5'), ('mono-gal', 'C6H10O5'),
        ('mono-glcnac', 'C8H13NO5'), ('mono-galnac', 'C8H13NO5'), ('mono-fuc', 'C6H10O4'),
        ('mono-neuac', 'C11H17NO8'), ('mono-neugc', 'C11H17NO9'), ('mono-xyl', 'C5H8O4')
)
UPDATE notes
SET fields = json_remove(fields, '$.formula')
WHERE note_type_id = 'Monosaccharide'
  AND EXISTS (
      SELECT 1 FROM originals
      WHERE id = notes.id AND json_extract(notes.fields, '$.formula') = value
  );

-- Retain card identities, schedules and review logs while retiring these directions.
UPDATE cards
SET enabled = 0
WHERE enabled <> 0
  AND template_id IN ('name-to-residue-mass', 'name-to-shorthand')
  AND note_id IN (SELECT id FROM notes WHERE note_type_id = 'Monosaccharide');
