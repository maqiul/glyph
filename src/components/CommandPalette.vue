<script setup lang="ts">
import { ref, computed, nextTick, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { TOOLS } from '../tools'
import { useSettingsStore } from '../stores/settings'
import { savePersisted } from '../stores/persistent'

const { t } = useI18n()
const settings = useSettingsStore()
const emit = defineEmits<{ (e: 'close'): void }>()

const query = ref('')
const idx = ref(0)
const input = ref<HTMLInputElement | null>(null)

interface Cmd {
  id: string
  label: string
  hint: string
  run: () => void
}

const commands = computed<Cmd[]>(() => {
  const list: Cmd[] = TOOLS.map((tool) => ({
    id: 'tool-' + tool.id,
    label: t(tool.labelKey),
    hint: t('cmd.goTool'),
    run: () => {
      settings.setActiveTool(tool.id)
      savePersisted()
    },
  }))
  list.push({
    id: 'theme',
    label: t('cmd.toggleTheme'),
    hint: 'Ctrl+D',
    run: () => {
      settings.toggleTheme()
      savePersisted()
    },
  })
  return list
})

const filtered = computed<Cmd[]>(() => {
  const q = query.value.trim().toLowerCase()
  if (!q) return commands.value
  return commands.value.filter((c) => c.label.toLowerCase().includes(q))
})

function run(c: Cmd) {
  c.run()
  emit('close')
}

function onKey(e: KeyboardEvent) {
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    idx.value = Math.min(idx.value + 1, filtered.value.length - 1)
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    idx.value = Math.max(idx.value - 1, 0)
  } else if (e.key === 'Enter') {
    e.preventDefault()
    const c = filtered.value[idx.value]
    if (c) run(c)
  } else if (e.key === 'Escape') {
    e.preventDefault()
    emit('close')
  }
}

onMounted(() => nextTick(() => input.value?.focus()))
</script>

<template>
  <div class="palette-mask" @click.self="emit('close')">
    <div class="palette">
      <input
        ref="input"
        v-model="query"
        class="palette-input"
        :placeholder="t('cmd.placeholder')"
        @keydown="onKey"
      />
      <ul class="palette-list">
        <li
          v-for="(c, i) in filtered"
          :key="c.id"
          :class="['palette-item', { active: i === idx }]"
          @mouseenter="idx = i"
          @click="run(c)"
        >
          <span class="palette-label">{{ c.label }}</span>
          <span class="palette-hint">{{ c.hint }}</span>
        </li>
        <li v-if="!filtered.length" class="palette-empty">{{ t('cmd.empty') }}</li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.palette-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.35);
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 12vh;
  z-index: 200;
}

.palette {
  width: 520px;
  max-width: 90vw;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.28);
  overflow: hidden;
}

.palette-input {
  width: 100%;
  border: 0;
  border-bottom: 1px solid var(--border);
  background: var(--surface);
  color: var(--text);
  padding: 14px 16px;
  font-size: 15px;
}

.palette-input:focus {
  outline: none;
}

.palette-list {
  list-style: none;
  margin: 0;
  padding: 6px;
  max-height: 320px;
  overflow-y: auto;
}

.palette-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 9px 12px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
  color: var(--text);
}

.palette-item.active {
  background: var(--accent);
  color: var(--accent-fg);
}

.palette-hint {
  font-size: 11px;
  opacity: 0.6;
  font-family: var(--font-mono);
}

.palette-empty {
  padding: 16px;
  text-align: center;
  color: var(--text-muted);
  font-size: 13px;
}
</style>