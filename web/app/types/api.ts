// Mirrors the JSON returned by srs-server.

export type Rating = 'again' | 'hard' | 'good' | 'easy';
export type Phase = 'new' | 'learning' | 'review' | 'relearning';
export type FieldType = 'text' | 'markdown' | 'number' | 'smiles' | 'image';
export type AnswerMode = 'flip' | 'typed' | 'multiple_choice';
export type FieldValue = string | number;

export interface FieldDef {
  name: string;
  label?: string;
  type: FieldType;
  unit?: string;
  optional?: boolean;
  from?: string;
  depiction_template?: string;
}

export interface DepictionTemplate {
  smiles: string;
  coords: [number, number][];
}

export interface CardTemplate {
  id: string;
  prompt: string[];
  answer: string[];
  mode: AnswerMode;
  accept?: string[];
  tolerance?: number;
  enabled_by_default: boolean;
}

export interface NoteType {
  id: string;
  fields: FieldDef[];
  depiction_templates?: Record<string, DepictionTemplate>;
  templates: CardTemplate[];
}

export interface Counts {
  learning: number;
  review: number;
  new: number;
}

export interface ReviewCard {
  note_id: string;
  template_id: string;
  set_id: string;
  phase: Phase;
  note_type: NoteType;
  template: CardTemplate;
  fields: Record<string, FieldValue>;
  choices: string[] | null;
  /** Seconds until due again, per rating. */
  intervals: Record<Rating, number>;
}

export interface NextResponse {
  counts: Counts;
  card: ReviewCard | null;
}

export interface CheckResponse {
  correct: boolean;
  expected: Record<string, FieldValue>;
}

export interface SetSummary {
  id: string;
  name: string;
  is_default: boolean;
  tags: string[];
  subscribed: boolean;
  note_count: number;
  card_count: number;
}

export interface Settings {
  new_per_day: number;
  reviews_per_day: number | null;
  desired_retention: number;
}
