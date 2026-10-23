import type { FieldValue, NoteType, StoredNote } from '~/types/api';

/** A stored note's field values, without its bookkeeping keys. */
export function noteFields(n: StoredNote): Record<string, FieldValue> {
  const { id: _id, note_type_id: _type, status: _status, ...fields } = n;
  return fields;
}

/** Short display title: the first text field's value. */
export function noteTitle(n: StoredNote, type?: NoteType): string {
  const first = type?.fields.find((f) => f.type === 'text') ?? type?.fields[0];
  const v = first ? n[first.name] : undefined;
  return v != null ? String(v) : n.id;
}
