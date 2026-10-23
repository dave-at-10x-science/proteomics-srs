<script setup lang="ts">
import type { CheckResponse, Counts, FieldDef, Rating, ReviewCard } from '~/types/api';

const api = useApi();
const card = ref<ReviewCard | null>(null);
const counts = ref<Counts | null>(null);
const loading = ref(true);
const busy = ref(false);
const error = ref('');

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

async function load() {
  loading.value = true;
  try {
    const next = await api.next();
    card.value = next.card;
    counts.value = next.counts;
    resetCard();
  } catch {
    error.value = 'Could not reach the server.';
  } finally {
    loading.value = false;
  }
}

const fieldsByName = computed(() => new Map(card.value?.note_type.fields.map((f) => [f.name, f]) ?? []));
const defs = (names: string[]) => names.map((n) => fieldsByName.value.get(n)).filter((f): f is FieldDef => !!f);
const promptFields = computed(() => defs(card.value?.template.prompt ?? []));
const answerFields = computed(() => defs(card.value?.template.answer ?? []));

// Everything else about the note, shown after answering for context (structure, mass, notes).
// Raw SMILES strings are left out; the structure drawn from them is shown instead.
const detailFields = computed(() => {
  const c = card.value;
  if (!c) return [];
  const shown = new Set([...c.template.prompt, ...c.template.answer]);
  const present = (f: FieldDef) => c.fields[f.from ?? f.name] != null;
  return c.note_type.fields.filter((f) => !shown.has(f.name) && f.type !== 'smiles' && present(f));
});

const mode = computed(() => card.value?.template.mode);

/** e.g. "Name, three-letter code, or one-letter code?" */
const question = computed(() => {
  const labels = answerFields.value.map((f) => fieldLabel(f));
  if (labels.length === 0) return '';
  const lower = labels.map((l, i) => (i === 0 ? l : l.charAt(0).toLowerCase() + l.slice(1)));
  const joined = lower.length > 1 ? `${lower.slice(0, -1).join(', ')}, or ${lower.at(-1)}` : lower[0];
  return `${joined}?`;
});
const correctChoice = computed(() => {
  const c = card.value;
  return c ? String(c.fields[c.template.answer[0]!] ?? '') : '';
});

/** Suggest a rating from an objective check; flip cards are self-graded. */
const suggested = computed<Rating | undefined>(() => {
  if (gaveUp.value) return 'again';
  if (result.value) return result.value.correct ? 'good' : 'again';
  if (picked.value !== null) return picked.value === correctChoice.value ? 'good' : 'again';
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
  if (!c || !revealed.value || busy.value) return;
  busy.value = true;
  try {
    const res = await api.answer({
      note_id: c.note_id,
      template_id: c.template_id,
      rating,
      duration_ms: Date.now() - shownAt,
      answer: typed.value || picked.value || undefined,
    });
    card.value = res.next.card;
    counts.value = res.next.counts;
    resetCard();
  } catch {
    error.value = 'Could not save that answer. Try again.';
  } finally {
    busy.value = false;
  }
}

function onKey(e: KeyboardEvent) {
  if (e.target instanceof HTMLInputElement) return;
  if ((e.key === ' ' || e.key === 'Enter') && mode.value === 'flip' && !revealed.value) {
    e.preventDefault();
    revealed.value = true;
  }
}
onMounted(() => {
  load();
  window.addEventListener('keydown', onKey);
});
onBeforeUnmount(() => window.removeEventListener('keydown', onKey));
</script>

<template>
  <div class="review">
    <div v-if="counts" class="progress muted">
      <span class="learning">{{ counts.learning }}</span> ·
      <span class="review-count">{{ counts.review }}</span> ·
      <span class="new">{{ counts.new }}</span>
    </div>

    <p v-if="error" class="card">{{ error }}</p>
    <p v-if="loading" class="muted center">Loading…</p>

    <div v-else-if="!card" class="card center stack">
      <h2>Done for now</h2>
      <p class="muted">Nothing else is due. Nice work.</p>
      <NuxtLink to="/"><button>Back home</button></NuxtLink>
    </div>

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
        <p class="question muted">{{ question }}</p>
      </section>

      <!-- Answer input -->
      <section v-if="!revealed" class="answer-input">
        <form v-if="mode === 'typed'" class="typed" @submit.prevent="submitTyped">
          <input
            v-model="typed"
            type="text"
            autocomplete="off"
            autocapitalize="off"
            spellcheck="false"
            placeholder="Type your answer"
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

        <section
          class="card stack answer"
          :class="{ correct: suggested === 'good', wrong: suggested === 'again' }"
        >
          <p v-if="result || picked" class="verdict">
            {{ suggested === 'good' ? 'Correct' : `Not quite${typed ? ` (you wrote “${typed}”)` : ''}` }}
          </p>
          <FieldView
            v-for="f in answerFields"
            :key="f.name"
            :def="f"
            :fields="card.fields"
            :note-type="card.note_type"
            label
          />
          <div v-if="detailFields.length" class="details">
            <FieldView
              v-for="f in detailFields"
              :key="f.name"
              :def="f"
              :fields="card.fields"
              :note-type="card.note_type"
              label
            />
          </div>
        </section>

        <RatingButtons :intervals="card.intervals" :suggested="suggested" :disabled="busy" @rate="rate" />
      </template>
    </template>
  </div>
</template>

<style scoped>
.review {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}
.progress {
  text-align: center;
  font-variant-numeric: tabular-nums;
}
.progress .learning {
  color: var(--again);
}
.progress .review-count {
  color: var(--good);
}
.progress .new {
  color: var(--easy);
}
.center {
  text-align: center;
}
.question {
  text-align: center;
  margin: 0.5rem 0 0;
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
.details {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
  border-top: 1px solid var(--border);
  padding-top: 0.75rem;
}
.details :deep(.field:has(.markdown)),
.details :deep(.field:has(.structure)) {
  grid-column: 1 / -1;
}
.details :deep(.value) {
  font-size: 1rem;
}
</style>
