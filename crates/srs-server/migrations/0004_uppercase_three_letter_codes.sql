-- Three-letter amino acid codes are written in uppercase (SER, not Ser). Seed import never updates
-- existing notes, so bring them in line here. Answer checking ignores case either way.
UPDATE notes
SET fields = json_set(fields, '$.three_letter', upper(json_extract(fields, '$.three_letter')))
WHERE note_type_id = 'AminoAcid' AND json_type(fields, '$.three_letter') = 'text';
