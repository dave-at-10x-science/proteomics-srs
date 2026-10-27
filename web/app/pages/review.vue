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

const dueCount = computed(() => (available.value ? available.value.learning + available.value.review : 0));
const availableTotal = computed(() => dueCount.value + (available.value?.new ?? 0));
const roundSize = computed(() => Math.min(size.value, availableTotal.value));

async function loadSetup() {
  phase.value = 'setup';
  card.value = null;
  try {
    available.value = (await api.round(1)).counts;
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
const continueButton = ref<HTMLButtonElement | null>(null);
watch(continueButton, (b) => b?.focus());

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
  <div class="review">
    <p v-if="error" class="card">{{ error }}</p>

    <!-- Choosing a round -->
    <section v-if="phase === 'setup'" class="card stack setup">
      <h2>New round</h2>
      <template v-if="available">
        <p class="muted">
          <strong>{{ dueCount }}</strong> due and <strong>{{ available.new }}</strong> new
          {{ available.new === 1 ? 'card' : 'cards' }} available.
        </p>
        <template v-if="availableTotal > 0">
          <div class="sizes" role="radiogroup" aria-label="Cards in this round">
            <button
              v-for="s in SIZES"
              :key="s"
              role="radio"
              :aria-checked="size === s"
              :class="{ selected: size === s }"
              @click="size = s"
            >
              {{ s }}
            </button>
          </div>
          <button class="primary wide" :disabled="busy" @click="start">
            Start round of {{ roundSize }} {{ roundSize === 1 ? 'card' : 'cards' }}
          </button>
          <p class="muted small">Due cards come first, then new ones fill the round.</p>
        </template>
        <template v-else>
          <p>Nothing else is due. Nice work.</p>
          <NuxtLink to="/browse"><button class="wide">Browse cards</button></NuxtLink>
        </template>
      </template>
      <p v-else class="muted">Loading…</p>
    </section>

    <!-- Round finished -->
    <section v-else-if="phase === 'finished' && round" class="card stack center">
      <h2>Round done</h2>
      <p>
        <strong>{{ round.correct }}</strong> of {{ round.correct + round.missed }} right.
        <template v-if="round.missed">The {{ round.missed }} you missed will come back in a later round.</template>
      </p>
      <div class="actions">
        <button class="primary wide" @click="loadSetup">Another round</button>
        <NuxtLink to="/"><button class="wide">Back home</button></NuxtLink>
      </div>
    </section>

    <template v-else-if="phase === 'playing' && round">
      <div class="progress">
        <div class="bar"><div :style="{ width: `${(round.index / round.cards.length) * 100}%` }" /></div>
        <span class="muted">{{ Math.min(round.index + 1, round.cards.length) }} / {{ round.cards.length }}</span>
      </div>

      <p v-if="!card" class="muted center">Loading…</p>
      <template v-else>
        <section class="card stack prompt">
          <FieldView
            v-for="f in promptFields"
            :key="f.name"
            :def="f"
            :fields="card.fields"
            :note-type="card.note_type"
            large
          />
        </section>

        <p class="question">{{ question }}</p>

        <!-- Answer input -->
        <section v-if="!revealed" class="answer-input">
          <form v-if="mode === 'typed'" class="typed" @submit.prevent="submitTyped">
            <input
              v-model="typed"
              type="text"
              autocomplete="off"
              autocapitalize="off"
              spellcheck="false"
              :placeholder="question"
              autofocus
            />
            <button class="primary" :disabled="busy || !typed.trim()">Check</button>
            <button type="button" class="link" @click="giveUp">I don't know</button>
          </form>

          <div v-else-if="mode === 'multiple_choice'" class="choices">
            <button v-for="c in card.choices" :key="c" @click="pick(c)">{{ c }}</button>
          </div>

          <button v-else class="primary wide" @click="revealed = true">Show answer</button>
        </section>

        <!-- Revealed -->
        <template v-else>
          <div v-if="mode === 'multiple_choice'" class="choices">
            <button
              v-for="c in card.choices"
              :key="c"
              disabled
              :class="{ correct: c === correctChoice, wrong: c === picked && c !== correctChoice }"
            >
              {{ c }}
            </button>
          </div>

          <section class="card stack answer" :class="verdict">
            <p v-if="verdict" class="verdict">
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
          </section>

          <button
            v-if="verdict === 'wrong'"
            ref="continueButton"
            class="primary wide continue"
            :disabled="busy"
            @click="rate('again')"
          >
            Continue
            <span class="small">recorded as Again</span>
          </button>
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

<style scoped>
.review {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
h2 {
  margin: 0;
  font-size: 1.1rem;
}
.center {
  text-align: center;
}
.small {
  font-size: 0.85rem;
}
.actions {
  display: grid;
  gap: 0.5rem;
}
.setup p {
  margin: 0;
}
.sizes {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 0.5rem;
}
.sizes .selected {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--focus);
  font-weight: 600;
}
.progress {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  font-variant-numeric: tabular-nums;
}
.bar {
  flex: 1;
  height: 6px;
  border-radius: 3px;
  background: var(--border);
  overflow: hidden;
}
.bar div {
  height: 100%;
  background: var(--accent);
  transition: width 0.2s;
}
.question {
  text-align: center;
  margin: 0;
  font-weight: 600;
  font-size: 1.1rem;
  color: var(--accent);
}
.typed {
  display: grid;
  gap: 0.5rem;
}
.link {
  background: none;
  border: none;
  color: var(--muted);
  min-height: 40px;
}
.choices {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.5rem;
}
.choices button {
  min-height: 56px;
}
.choices button:disabled {
  opacity: 1;
}
.choices .correct,
.answer.correct {
  background: var(--correct-bg);
  border-color: var(--good);
}
.choices .wrong,
.answer.wrong {
  background: var(--wrong-bg);
  border-color: var(--again);
}
.wide {
  width: 100%;
}
.verdict {
  font-weight: 600;
  margin: 0;
}
.continue {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.1rem;
}
.continue .small {
  opacity: 0.85;
  font-weight: 400;
}
</style>
