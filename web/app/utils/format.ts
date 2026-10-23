import type { FieldDef } from '~/types/api';

/** The field's label, or its name made readable: "three_letter" -> "Three letter". */
export function fieldLabel(def: FieldDef): string {
  if (def.label) return def.label;
  const words = def.name.replace(/_/g, ' ');
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** Compact interval for answer buttons: 10m, 3h, 2d, 1.5mo, 2y. */
export function formatInterval(seconds: number): string {
  const m = seconds / 60;
  if (m < 60) return `${Math.max(1, Math.round(m))}m`;
  const h = m / 60;
  if (h < 24) return `${Math.round(h)}h`;
  const d = h / 24;
  if (d < 30) return `${Math.round(d)}d`;
  const mo = d / 30.44;
  if (mo < 12) return `${Number(mo.toFixed(1))}mo`;
  return `${Number((d / 365.25).toFixed(1))}y`;
}
