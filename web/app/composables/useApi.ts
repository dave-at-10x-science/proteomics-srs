import type { CheckResponse, NextResponse, Rating, SetSummary, Settings } from '~/types/api';

export function useApi() {
  return {
    next: () => $fetch<NextResponse>('/api/review/next'),
    check: (note_id: string, template_id: string, input: string) =>
      $fetch<CheckResponse>('/api/review/check', { method: 'POST', body: { note_id, template_id, input } }),
    answer: (body: { note_id: string; template_id: string; rating: Rating; duration_ms?: number; answer?: string }) =>
      $fetch<{ next: NextResponse }>('/api/review/answer', { method: 'POST', body }),
    sets: () => $fetch<SetSummary[]>('/api/sets'),
    subscribe: (id: string, subscribed: boolean) =>
      $fetch(`/api/sets/${encodeURIComponent(id)}/subscription`, { method: 'PUT', body: { subscribed } }),
    settings: () => $fetch<Settings>('/api/settings'),
    saveSettings: (s: Settings) => $fetch<Settings>('/api/settings', { method: 'PUT', body: s }),
  };
}
