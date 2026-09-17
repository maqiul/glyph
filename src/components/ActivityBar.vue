<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { TOOLS } from '../tools'
import { useSettingsStore, type ActiveTool } from '../stores/settings'
import { savePersisted } from '../stores/persistent'

const { t } = useI18n()
const settings = useSettingsStore()

function select(id: ActiveTool) {
  if (settings.activeTool !== id) {
    settings.setActiveTool(id)
    savePersisted()
  }
}
</script>

<template>
  <nav class="activity-bar">
    <button
      v-for="tool in TOOLS"
      :key="tool.id"
      class="ab-item"
      :class="{ active: settings.activeTool === tool.id }"
      :title="t(tool.labelKey)"
      @click="select(tool.id)"
    >
      <svg
        width="22"
        height="22"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="1.6"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path :d="tool.icon" />
      </svg>
    </button>
  </nav>
</template>

<style scoped>
.activity-bar {
  width: 48px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 8px 0;
  background: var(--surface);
  border-right: 1px solid var(--border);
  flex-shrink: 0;
}

.ab-item {
  width: 38px;
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 0;
  background: transparent;
  color: var(--text-muted);
  border-radius: 8px;
  cursor: pointer;
  position: relative;
  transition: all 0.15s ease;
}

.ab-item:hover {
  background: var(--surface-hover);
  color: var(--text);
}

.ab-item.active {
  color: var(--accent);
  background: var(--surface-hover);
}

.ab-item.active::before {
  content: '';
  position: absolute;
  left: -5px;
  top: 8px;
  bottom: 8px;
  width: 3px;
  border-radius: 0 2px 2px 0;
  background: var(--accent);
}
</style>