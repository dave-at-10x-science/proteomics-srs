<script setup lang="ts">
import type { CardRef, CheckResponse, Counts, FieldDef, Rating, ReviewCard } from '~/types/api';

const api = useApi();
const error = ref('');
const busy = ref(false);

// A round is a fixed list of cards picked at the start. Each card is shown once, even if a rating
// makes it due again before the round ends; it comes back in a later round.
interface Round {
  cards: CardRef[];
  index: number;
  correct: number;
  missed: number;
}
const ROUND_KEY = 'srs.round';
const SIZE_KEY = 'srs.roundSize';
const SIZES = [5, 10, 20, 50];

const phase = ref<'setup' | 'playing' | 'finished'>('setup');
const available = ref<Counts | null>(null);
const size = ref(10);
const round = ref<Round | null>(null);
const card = ref<ReviewCard | null>(null);

function storage(): Storage | null {
  try {
    return window.sessionStorage;
  } catch {
    return null;
  }
}
function saveRound() {
  try {
    if (round.value && phase.value === 'playing') storage()?.setItem(ROUND_KEY, JSON.stringify(round.value));
    else storage()?.removeItem(ROUND_KEY);
  } catch {
    // Resuming after a reload is a convenience; the round still works without it.
  }
}

const unstudied = ref(0);
const dueCount = computed(() => {
  const c = available.value;
  return c ? c.learning + c.review + c.new : 0;
});
const roundSize = computed(() => Math.min(size.value, dueCount.value));

async function loadSetup() {
  phase.value = 'setup';
  card.value = null;
  try {
    const res = await api.round(1);
    available.value = res.counts;
    unstudied.value = res.unstudied;
    error.value = '';
  } catch {
    error.value = 'Could not reach the server.';
  }
}

async function start() {
  busy.value = true;
  try {
    try {
      localStorage.setItem(SIZE_KEY, String(size.value));
    } catch {
      // Not remembering the size is fine.
    }
    const res = await api.round(size.value);
    round.value = { cards: res.cards, index: 0, correct: 0, missed: 0 };
    phase.value = 'playing';
    await showCurrent();
  } catch {
    error.value = 'Could not start a round.';
  } finally {
    busy.value = false;
  }
}

async function showCurrent() {
  const r = round.value;
  saveRound();
  if (!r || r.index >= r.cards.length) {
    phase.value = 'finished';
    card.value = null;
    saveRound();
    return;
  }
  try {
    card.value = await api.card(r.cards[r.index]!);
  } catch {
    // The card was deleted or unsubscribed since the round started: skip it.
    r.index += 1;
    return showCurrent();
  }
  resetCard();
}

// Per-card state.
const revealed = ref(false);
const typed = ref('');
const result = ref<CheckResponse | null>(null);
const picked = ref<string | null>(null);
const gaveUp = ref(false);
let shownAt = Date.now();

function resetCard() {
  revealed.value = false;
  typed.value = '';
  result.value = null;
  picked.value = null;
  gaveUp.value = false;
  shownAt = Date.now();
}

const fieldsByName = computed(() => new Map(card.value?.note_type.fields.map((f) => [f.name, f]) ?? []));
const defs = (names: string[]) => names.map((n) => fieldsByName.value.get(n)).filter((f): f is FieldDef => !!f);
const promptFields = computed(() => defs(card.value?.template.prompt ?? []));
const answerFields = computed(() => defs(card.value?.template.answer ?? []));
// Images in the prompt are already on screen; text prompts stay in the facts row for context.
const shownInPrompt = computed(() => promptFields.value.filter((f) => f.type === 'image').map((f) => f.name));

const mode = computed(() => card.value?.template.mode);

/** What to answer with, e.g. "Type the three-letter code". */
const question = computed(() => {
  const labels = answerFields.value.map((f) => fieldLabel(f).toLowerCase());
  if (labels.length === 0) return '';
  const what = labels.length > 1 ? `${labels.slice(0, -1).join(', ')} and ${labels.at(-1)}` : labels[0]!;
  if (mode.value === 'typed') return `Type the ${what}`;
  if (mode.value === 'multiple_choice') return `Pick the ${what}`;
  return `${what.charAt(0).toUpperCase()}${what.slice(1)}?`;
});
const correctChoice = computed(() => {
  const c = card.value;
  return c ? String(c.fields[c.template.answer[0]!] ?? '') : '';
});

/** Typed and multiple-choice answers grade themselves; flip cards are self-graded. */
const verdict = computed<'correct' | 'wrong' | undefined>(() => {
  if (gaveUp.value) return 'wrong';
  if (result.value) return result.value.correct ? 'correct' : 'wrong';
  if (picked.value !== null) return picked.value === correctChoice.value ? 'correct' : 'wrong';
  return undefined;
});

async function submitTyped() {
  const c = card.value;
  if (!c || !typed.value.trim() || busy.value) return;
  busy.value = true;
  try {
    result.value = await api.check(c.note_id, c.template_id, typed.value);
    revealed.value = true;
  } finally {
    busy.value = false;
  }
}

function giveUp() {
  gaveUp.value = true;
  revealed.value = true;
}

function pick(choice: string) {
  if (revealed.value) return;
  picked.value = choice;
  revealed.value = true;
}

async function rate(rating: Rating) {
  const c = card.value;
  const r = round.value;
  if (!c || !r || !revealed.value || busy.value) return;
  busy.value = true;
  try {
    await api.answer({
      note_id: c.note_id,
      template_id: c.template_id,
      rating,
      duration_ms: Date.now() - shownAt,
      answer: typed.value || picked.value || undefined,
    });
    if (rating === 'again') r.missed += 1;
    else r.correct += 1;
    r.index += 1;
    await showCurrent();
  } catch {
    error.value = 'Could not save that answer. Try again.';
  } finally {
    busy.value = false;
  }
}

// A wrong answer is recorded as Again; Continue is focused so Enter moves on.
const continueButton = useTemplateRef<{ $el: HTMLElement }>('continueButton');
watch(continueButton, (b) => b?.$el.focus());

function onKey(e: KeyboardEvent) {
  if (e.target instanceof HTMLInputElement) return;
  if ((e.key === ' ' || e.key === 'Enter') && mode.value === 'flip' && !revealed.value) {
    e.preventDefault();
    revealed.value = true;
  }
}

onMounted(async () => {
  window.addEventListener('keydown', onKey);
  try {
    const saved = Number(localStorage.getItem(SIZE_KEY));
    if (saved > 0) size.value = saved;
  } catch {
    // Use the default size.
  }
  let resumed: Round | null = null;
  try {
    resumed = JSON.parse(storage()?.getItem(ROUND_KEY) ?? 'null');
  } catch {
    resumed = null;
  }
  if (resumed && resumed.index < resumed.cards.length) {
    round.value = resumed;
    phase.value = 'playing';
    await showCurrent();
  } else {
    await loadSetup();
  }
});
onBeforeUnmount(() => window.removeEventListener('keydown', onKey));
</script>


<template>
  <div class="flex flex-col gap-4">
    <ErrorAlert :errors="error" />

    <!-- Choosing a round -->
    <template v-if="phase === 'setup'">
      <UCard v-if="!available">
        <p class="text-muted">Loading…</p>
      </UCard>
      <UCard v-else-if="dueCount > 0" :ui="{ body: 'flex flex-col gap-5' }">
        <div class="flex items-center gap-3">
          <span class="flex size-11 items-center justify-center rounded-xl bg-primary/12 text-primary">
            <UIcon name="i-lucide-repeat" class="size-6" />
          </span>
          <div>
            <h1 class="text-lg font-semibold text-highlighted">New round</h1>
            <p class="text-sm text-muted">
              <span class="font-semibold text-default tabular-nums">{{ dueCount }}</span>
              {{ dueCount === 1 ? 'card' : 'cards' }} due
            </p>
          </div>
        </div>
        <SegmentedPicker v-model="size" :items="SIZES" legend="Cards in this round" />
        <div class="flex flex-col gap-2">
          <UButton
            :label="`Start round of ${roundSize} ${roundSize === 1 ? 'card' : 'cards'}`"
            trailing-icon="i-lucide-arrow-right"
            block
            :loading="busy"
            @click="start"
          />
          <p class="text-center text-xs text-muted">If more are due than fit, the round is a random pick of them.</p>
        </div>
      </UCard>
      <UCard v-else>
        <UEmpty
          icon="i-lucide-circle-check"
          title="Nothing is due right now"
          description="You're all caught up. Study something new, or browse what you know."
          variant="naked"
          :actions="[
            ...(unstudied
              ? [{ label: `Study new notes (${unstudied} left)`, icon: 'i-lucide-book-open', to: '/study' }]
              : []),
            { label: 'Browse cards', icon: 'i-lucide-layers', to: '/browse', color: 'neutral', variant: 'outline' },
          ]"
        />
      </UCard>
    </template>

    <!-- Round finished -->
    <UCard v-else-if="phase === 'finished' && round" :ui="{ body: 'flex flex-col items-center gap-4 py-8 text-center' }">
      <span
        class="flex size-16 items-center justify-center rounded-full"
        :class="round.missed ? 'bg-primary/12 text-primary' : 'bg-success/12 text-success'"
      >
        <UIcon :name="round.missed ? 'i-lucide-target' : 'i-lucide-trophy'" class="size-8" />
      </span>
      <div>
        <h1 class="text-xl font-semibold text-highlighted">{{ round.missed ? 'Round done' : 'Perfect round' }}</h1>
        <p class="mt-1 text-muted">
          <span class="font-semibold text-default">{{ round.correct }}</span> of {{ round.correct + round.missed }} right.
          <template v-if="round.missed">The {{ round.missed }} you missed will come back in a later round.</template>
        </p>
      </div>
      <div class="grid w-full gap-2">
        <UButton label="Another round" icon="i-lucide-repeat" block @click="loadSetup" />
        <UButton label="Back to today" icon="i-lucide-house" to="/" color="neutral" variant="outline" block />
      </div>
    </UCard>

    <template v-else-if="phase === 'playing' && round">
      <div class="flex items-center gap-3">
        <UProgress :model-value="round.index" :max="round.cards.length" size="sm" class="flex-1" />
        <span class="text-sm font-medium text-muted tabular-nums">
          {{ Math.min(round.index + 1, round.cards.length) }} / {{ round.cards.length }}
        </span>
      </div>

      <p v-if="!card" class="text-center text-muted">Loading…</p>
      <template v-else>
        <UCard :ui="{ body: 'flex flex-col gap-4' }">
          <FieldView
            v-for="f in promptFields"
            :key="f.name"
            :def="f"
            :fields="card.fields"
            :note-type="card.note_type"
            large
          />
        </UCard>

        <div class="flex justify-center">
          <UBadge :label="question" color="primary" variant="soft" size="xl" class="rounded-full px-4" />
        </div>

        <!-- Answer input -->
        <template v-if="!revealed">
          <form v-if="mode === 'typed'" class="flex flex-col gap-2" @submit.prevent="submitTyped">
            <UInput
              v-model="typed"
              :placeholder="question"
              :aria-label="question"
              autocomplete="off"
              autocapitalize="off"
              :spellcheck="false"
              autofocus
              class="w-full"
            />
            <UButton type="submit" label="Check" block :disabled="!typed.trim()" :loading="busy" />
            <UButton label="I don't know" color="neutral" variant="ghost" block @click="giveUp" />
          </form>

          <div v-else-if="mode === 'multiple_choice'" class="grid grid-cols-2 gap-2">
            <UButton
              v-for="c in card.choices"
              :key="c"
              color="neutral"
              variant="outline"
              block
              class="min-h-14 text-center"
              @click="pick(c)"
            >
              {{ c }}
            </UButton>
          </div>

          <UButton v-else label="Show answer" icon="i-lucide-eye" block @click="revealed = true" />
        </template>

        <!-- Revealed -->
        <template v-else>
          <div v-if="mode === 'multiple_choice'" class="grid grid-cols-2 gap-2">
            <UButton
              v-for="c in card.choices"
              :key="c"
              :color="c === correctChoice ? 'success' : c === picked ? 'error' : 'neutral'"
              :variant="c === correctChoice || c === picked ? 'soft' : 'outline'"
              :trailing-icon="c === correctChoice ? 'i-lucide-check' : c === picked ? 'i-lucide-x' : undefined"
              block
              disabled
              class="min-h-14 text-center disabled:cursor-default"
              :class="c === correctChoice || c === picked ? 'disabled:opacity-100' : 'disabled:opacity-50'"
            >
              {{ c }}
            </UButton>
          </div>

          <UCard
            :class="verdict === 'correct' ? 'bg-success/8 ring-success/40' : verdict === 'wrong' ? 'bg-error/8 ring-error/40' : ''"
            :ui="{ body: 'flex flex-col gap-4' }"
          >
            <p
              v-if="verdict"
              class="flex items-center gap-2 font-semibold"
              :class="verdict === 'correct' ? 'text-success' : 'text-error'"
            >
              <UIcon :name="verdict === 'correct' ? 'i-lucide-circle-check' : 'i-lucide-circle-x'" class="size-5 shrink-0" />
              <template v-if="verdict === 'correct'">Correct</template>
              <template v-else-if="gaveUp">The answer is {{ answerFields.map((f) => card!.fields[f.name]).join(', ') }}</template>
              <template v-else>Not quite{{ typed ? ` (you wrote “${typed}”)` : '' }}</template>
            </p>
            <NoteFacts
              :note-type="card.note_type"
              :fields="card.fields"
              :exclude="shownInPrompt"
              :highlight="card.template.answer"
            />
          </UCard>

          <UButton
            v-if="verdict === 'wrong'"
            ref="continueButton"
            block
            class="flex-col gap-0"
            :disabled="busy"
            @click="rate('again')"
          >
            <span class="font-semibold">Continue</span>
            <span class="text-xs font-normal opacity-85">recorded as Again</span>
          </UButton>
          <RatingButtons
            v-else
            :key="`${card.note_id}/${card.template_id}`"
            :intervals="card.intervals"
            :suggested="verdict === 'correct' ? 'good' : undefined"
            :only="verdict === 'correct' ? ['hard', 'good', 'easy'] : undefined"
            :disabled="busy"
            @rate="rate"
          />
        </template>
      </template>
    </template>
  </div>
</template>
