<script setup lang="ts">
import type { Board, GameBest, GameOption, Pairing } from '~/types/api';

type TileState = 'hidden' | 'shown' | 'matched';

const api = useApi();
const options = ref<GameOption[] | null>(null);
const setId = ref('');
const pairingIdx = ref(0);
const pairs = ref(6);
const error = ref('');

const board = ref<Board | null>(null);
const states = ref<Record<string, TileState>>({});
const open = ref<string[]>([]);
const flips = ref(0);
const startedAt = ref<number | null>(null);
const elapsed = ref(0);
const best = ref<GameBest | null>(null);
let timer: ReturnType<typeof setInterval> | undefined;

onMounted(async () => {
  options.value = await api.gameOptions();
  setId.value = options.value[0]?.set_id ?? '';
});
onBeforeUnmount(() => clearInterval(timer));

const option = computed(() => options.value?.find((o) => o.set_id === setId.value));
const pairing = computed<Pairing | undefined>(() => option.value?.pairings[pairingIdx.value]);
watch(setId, () => (pairingIdx.value = 0));

const matched = computed(() => Object.values(states.value).filter((s) => s === 'matched').length / 2);
const total = computed(() => (board.value?.tiles.length ?? 0) / 2);
const done = computed(() => !!board.value && matched.value === total.value);
const columns = computed(() => {
  // Structures need bigger tiles than short text.
  const t = board.value;
  if (!t) return 4;
  const hasStructure = [pairing.value?.a, pairing.value?.b].some((f) => {
    const d = t.note_type.fields.find((x) => x.name === f);
    return d?.type === 'smiles' || (d?.type === 'image' && !!d.from);
  });
  return hasStructure ? 3 : 4;
});

async function start() {
  if (!pairing.value) return;
  error.value = '';
  try {
    board.value = await api.board(setId.value, pairing.value.a, pairing.value.b, pairs.value);
  } catch (e) {
    error.value = apiErrors(e).join(' ');
    return;
  }
  states.value = Object.fromEntries(board.value.tiles.map((t) => [t.id, 'hidden' as TileState]));
  open.value = [];
  flips.value = 0;
  startedAt.value = null;
  elapsed.value = 0;
  best.value = null;
  clearInterval(timer);
}

function flip(id: string) {
  const b = board.value;
  if (!b || states.value[id] !== 'hidden' || open.value.length >= 2) return;
  if (startedAt.value === null) {
    startedAt.value = Date.now();
    timer = setInterval(() => (elapsed.value = Date.now() - startedAt.value!), 250);
  }
  flips.value++;
  states.value[id] = 'shown';
  open.value.push(id);
  if (open.value.length < 2) return;

  const [x, y] = open.value.map((tid) => b.tiles.find((t) => t.id === tid)!);
  if (x!.note_id === y!.note_id) {
    states.value[x!.id] = 'matched';
    states.value[y!.id] = 'matched';
    open.value = [];
    if (done.value) finish();
  } else {
    setTimeout(() => {
      states.value[x!.id] = 'hidden';
      states.value[y!.id] = 'hidden';
      open.value = [];
    }, 900);
  }
}

async function finish() {
  clearInterval(timer);
  elapsed.value = Date.now() - (startedAt.value ?? Date.now());
  const b = board.value!;
  best.value = await api.gameResult({
    set_id: b.set_id,
    a: pairing.value!.a,
    b: pairing.value!.b,
    pairs: total.value,
    flips: flips.value,
    duration_ms: elapsed.value,
    completed: true,
  });
}

const seconds = (ms: number) => `${(ms / 1000).toFixed(1)}s`;
</script>

<template>
  <div class="stack">
    <!-- Setup -->
    <section v-if="!board" class="card stack">
      <h1>Memory game</h1>
      <p class="muted">Find the matching pairs. Only uses cards you've already studied, and doesn't affect your reviews.</p>
      <template v-if="options && options.length">
        <label>
          Set
          <select v-model="setId">
            <option v-for="o in options" :key="o.set_id" :value="o.set_id">
              {{ o.set_name }} ({{ o.introduced }} studied)
            </option>
          </select>
        </label>
        <label v-if="option">
          Match
          <select v-model.number="pairingIdx">
            <option v-for="(p, i) in option.pairings" :key="p.label" :value="i">{{ p.label }}</option>
          </select>
        </label>
        <label>
          Pairs
          <select v-model.number="pairs">
            <option v-for="n in [4, 6, 8, 10]" :key="n" :value="n">{{ n }}</option>
          </select>
        </label>
        <p v-if="error" class="error">{{ error }}</p>
        <button class="primary" @click="start">Play</button>
      </template>
      <p v-else-if="options" class="muted">Study a few cards first, then come back to play.</p>
    </section>

    <!-- Game -->
    <template v-else>
      <div class="status">
        <span>{{ matched }} / {{ total }} pairs</span>
        <span>{{ flips }} flips</span>
        <span>{{ seconds(elapsed) }}</span>
      </div>
      <div class="grid" :style="{ gridTemplateColumns: `repeat(${columns}, 1fr)` }">
        <GameTile
          v-for="t in board.tiles"
          :key="t.id"
          :field="t.field"
          :fields="board.notes[t.note_id]!"
          :note-type="board.note_type"
          :state="states[t.id]!"
          @flip="flip(t.id)"
        />
      </div>
      <section v-if="done" class="card stack result">
        <h2>All matched!</h2>
        <p>{{ total }} pairs in {{ seconds(elapsed) }} with {{ flips }} flips.</p>
        <p v-if="best && best.games > 1" class="muted">
          Best: {{ seconds(best.best_ms ?? 0) }}, fewest flips {{ best.fewest_flips }} ({{ best.games }} games)
        </p>
        <div class="actions">
          <button class="primary" @click="start">Play again</button>
          <button @click="board = null">Change game</button>
        </div>
      </section>
      <button v-else class="quit" @click="board = null">Quit</button>
    </template>
  </div>
</template>

<style scoped>
h1,
h2 {
  margin: 0;
  font-size: 1.1rem;
}
label {
  display: grid;
  gap: 0.35rem;
  font-weight: 600;
}
select {
  font: inherit;
  padding: 0.6rem;
  border-radius: var(--radius);
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text);
  min-height: 44px;
}
.status {
  display: flex;
  justify-content: space-between;
  color: var(--muted);
  font-variant-numeric: tabular-nums;
}
.grid {
  display: grid;
  gap: 0.5rem;
}
.result {
  text-align: center;
}
.result p {
  margin: 0;
}
.actions {
  display: flex;
  gap: 0.5rem;
  justify-content: center;
}
.quit {
  display: block;
  margin-left: auto;
  margin-right: auto;
  background: none;
  border: none;
  color: var(--muted);
}
.error {
  color: var(--again);
  margin: 0;
}
</style>
