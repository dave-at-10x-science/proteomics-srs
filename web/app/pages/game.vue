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
  <div class="flex flex-col gap-4">
    <!-- Setup -->
    <template v-if="!board">
      <UCard v-if="options && !options.length">
        <UEmpty
          icon="i-lucide-puzzle"
          title="Nothing to play with yet"
          description="The memory game only uses cards you've studied. Study a few, then come back."
          variant="naked"
          :actions="[{ label: 'Study new notes', icon: 'i-lucide-book-open', to: '/study' }]"
        />
      </UCard>
      <UCard v-else :ui="{ body: 'flex flex-col gap-5' }">
        <div class="flex items-center gap-3">
          <span class="flex size-11 items-center justify-center rounded-xl bg-primary/12 text-primary">
            <UIcon name="i-lucide-puzzle" class="size-6" />
          </span>
          <div>
            <h1 class="text-lg font-semibold text-highlighted">Memory game</h1>
            <p class="text-sm text-muted">Find the matching pairs.</p>
          </div>
        </div>
        <p class="text-sm text-toned">Only uses cards you've already studied, and doesn't affect your reviews.</p>
        <template v-if="options">
          <UFormField label="Set" :hint="option ? `${option.introduced} studied` : undefined">
            <USelect
              v-model="setId"
              :items="options.map((o) => ({ label: o.set_name, value: o.set_id }))"
              class="w-full"
            />
          </UFormField>
          <UFormField v-if="option" label="Match">
            <USelect
              v-model="pairingIdx"
              :items="option.pairings.map((p, i) => ({ label: p.label, value: i }))"
              class="w-full"
            />
          </UFormField>
          <SegmentedPicker v-model="pairs" :items="[4, 6, 8, 10]" legend="Pairs" />
          <ErrorAlert :errors="error" />
          <UButton label="Play" icon="i-lucide-play" block @click="start" />
        </template>
      </UCard>
    </template>

    <!-- Game -->
    <template v-else>
      <div class="flex items-center justify-between text-sm font-medium text-muted tabular-nums">
        <span class="flex items-center gap-1.5"><UIcon name="i-lucide-check-check" class="size-4" />{{ matched }} / {{ total }} pairs</span>
        <span class="flex items-center gap-1.5"><UIcon name="i-lucide-mouse-pointer-click" class="size-4" />{{ flips }} flips</span>
        <span class="flex items-center gap-1.5"><UIcon name="i-lucide-timer" class="size-4" />{{ seconds(elapsed) }}</span>
      </div>
      <div class="grid gap-2" :style="{ gridTemplateColumns: `repeat(${columns}, 1fr)` }">
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
      <UCard v-if="done" :ui="{ body: 'flex flex-col items-center gap-4 py-6 text-center' }">
        <span class="flex size-14 items-center justify-center rounded-full bg-success/12 text-success">
          <UIcon name="i-lucide-trophy" class="size-7" />
        </span>
        <div>
          <h2 class="text-xl font-semibold text-highlighted">All matched!</h2>
          <p class="mt-1 text-muted">{{ total }} pairs in {{ seconds(elapsed) }} with {{ flips }} flips.</p>
          <p v-if="best && best.games > 1" class="mt-1 text-sm text-muted">
            Best: {{ seconds(best.best_ms ?? 0) }}, fewest flips {{ best.fewest_flips }} ({{ best.games }} games)
          </p>
        </div>
        <div class="grid w-full grid-cols-2 gap-2">
          <UButton label="Play again" icon="i-lucide-rotate-ccw" block @click="start" />
          <UButton label="Change game" color="neutral" variant="outline" block @click="board = null" />
        </div>
      </UCard>
      <UButton v-else label="Quit" color="neutral" variant="ghost" class="self-center" @click="board = null" />
    </template>
  </div>
</template>
