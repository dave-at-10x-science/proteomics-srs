import type {
  Board,
  CardRef,
  CheckResponse,
  DraftResult,
  DraftStatus,
  GameBest,
  GameOption,
  FieldValue,
  NextResponse,
  NoteType,
  Rating,
  ReviewCard,
  RoundResponse,
  SetSummary,
  Settings,
  StoredNote,
} from '~/types/api';

type Fields = Record<string, FieldValue>;
type Status = 'draft' | 'published';

export function useApi() {
  return {
    next: () => $fetch<NextResponse>('/api/review/next'),
    round: (size?: number) => $fetch<RoundResponse>('/api/review/round', { query: size ? { size } : {} }),
    card: (ref: CardRef) => $fetch<ReviewCard>('/api/review/card', { query: { ...ref } }),
    check: (note_id: string, template_id: string, input: string) =>
      $fetch<CheckResponse>('/api/review/check', { method: 'POST', body: { note_id, template_id, input } }),
    answer: (body: { note_id: string; template_id: string; rating: Rating; duration_ms?: number; answer?: string }) =>
      $fetch<{ next: NextResponse }>('/api/review/answer', { method: 'POST', body }),
    sets: () => $fetch<SetSummary[]>('/api/sets'),
    subscribe: (id: string, subscribed: boolean) =>
      $fetch(`/api/sets/${encodeURIComponent(id)}/subscription`, { method: 'PUT', body: { subscribed } }),
    settings: () => $fetch<Settings>('/api/settings'),
    saveSettings: (s: Settings) => $fetch<Settings>('/api/settings', { method: 'PUT', body: s }),
    createSet: (name: string) => $fetch<SetSummary>('/api/sets', { method: 'POST', body: { name } }),
    setNotes: (id: string) => $fetch<StoredNote[]>(`/api/sets/${encodeURIComponent(id)}/notes`),
    noteTypes: () => $fetch<NoteType[]>('/api/note-types'),
    note: (id: string) => $fetch<StoredNote>(`/api/notes/${encodeURIComponent(id)}`),
    createNote: (body: { set_id: string; note_type_id: string; fields: Fields; status: Status }) =>
      $fetch<StoredNote>('/api/notes', { method: 'POST', body }),
    updateNote: (id: string, body: { fields: Fields; status: Status }) =>
      $fetch<StoredNote>(`/api/notes/${encodeURIComponent(id)}`, { method: 'PUT', body }),
    deleteNote: (id: string) => $fetch(`/api/notes/${encodeURIComponent(id)}`, { method: 'DELETE' }),
    gameOptions: () => $fetch<GameOption[]>('/api/game/options'),
    board: (set: string, a: string, b: string, pairs: number) =>
      $fetch<Board>('/api/game/board', { query: { set, a, b, pairs } }),
    gameResult: (body: {
      set_id: string;
      a: string;
      b: string;
      pairs: number;
      flips: number;
      duration_ms: number;
      completed: boolean;
    }) => $fetch<GameBest>('/api/game/results', { method: 'POST', body }),
    draftStatus: () => $fetch<DraftStatus>('/api/draft/status'),
    draft: (setId: string, body: { source: string; count: number; note_type_id?: string }) =>
      $fetch<DraftResult>(`/api/sets/${encodeURIComponent(setId)}/draft`, { method: 'POST', body }),
    publishDrafts: (setId: string) =>
      $fetch<{ published: number }>(`/api/sets/${encodeURIComponent(setId)}/publish-drafts`, { method: 'POST' }),
    uploadImage: (file: File) =>
      $fetch<{ url: string }>('/api/media', { method: 'POST', body: file, headers: { 'content-type': file.type } }),
  };
}

/** The server's validation messages from a failed request, or a generic one. */
export function apiErrors(e: unknown): string[] {
  const data = (e as { data?: { error?: string; errors?: string[] } }).data;
  return data?.errors ?? (data?.error ? [data.error] : ['Something went wrong. Is the server running?']);
}
