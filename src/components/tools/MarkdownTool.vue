<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import Reader from '../Reader.vue'
import { useSettingsStore } from '../../stores/settings'
import { savePersisted } from '../../stores/persistent'

const { t } = useI18n()
const settings = useSettingsStore()
const errorMessage = ref<string | null>(null)
const readerRef = ref<InstanceType<typeof Reader> | null>(null)
const showRecent = ref(false)

const currentTitle = computed(() => {
  if (!settings.currentPath) return 'Markdown'
  const parts = settings.currentPath.split(/[/\\]/)
  return parts[parts.length - 1] || 'Markdown'
})

function isMdPath(path: string): boolean {
  return /\.(md|markdown|mdown|mkd|mkdn)$/i.test(path)
}

async function openFileDialog() {
  try {
    errorMessage.value = null
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Markdown', extensions: ['md', 'markdown', 'mdown', 'mkd', 'mkdn'] }],
    })
    if (!selected) return
    const path = Array.isArray(selected) ? selected[0] : selected
    if (path) loadPath(path)
  } catch (e) {
    errorMessage.value = t('errors.openFailed', { e: String(e) })
  }
}

function loadPath(path: string) {
  if (!isMdPath(path)) {
    errorMessage.value = t('errors.notMarkdown', { path })
    return
  }
  if (settings.dirty && !confirm(t('confirm.discard'))) return
  errorMessage.value = null
  settings.setCurrentPath(path)
  showRecent.value = false
  savePersisted()
}

async function newFile() {
  if (settings.dirty && !confirm(t('confirm.discard'))) return
  try {
    const target = await save({
      title: t('dialog.newTitle'),
      defaultPath: 'untitled.md',
      filters: [{ name: 'Markdown', extensions: ['md', 'markdown'] }],
    })
    if (!target) return
    await invoke('create_markdown_file', { path: target })
    settings.setDirty(false)
    settings.setCurrentPath(target)
  } catch (e) {
    errorMessage.value = t('errors.newFailed', { e: String(e) })
  }
}

async function saveFile() {
  if (!settings.currentPath) {
    await saveAs()
    return
  }
  await readerRef.value?.save()
}

async function saveAs() {
  try {
    const target = await save({
      title: t('dialog.saveAsTitle'),
      defaultPath: currentTitle.value,
      filters: [{ name: 'Markdown', extensions: ['md', 'markdown'] }],
    })
    if (!target) return
    settings.setCurrentPath(target)
    await readerRef.value?.save()
  } catch (e) {
    errorMessage.value = t('errors.saveAsFailed', { e: String(e) })
  }
}

function cycleMode() {
  settings.cycleMode()
  savePersisted()
}

function setMode(m: 'edit' | 'split' | 'preview') {
  settings.setMode(m)
  savePersisted()
}

function toggleImmersive() {
  settings.setImmersive(!settings.immersive)
}

function removeRecent(path: string) {
  settings.removeRecent(path)
  savePersisted()
}

function handleKeydown(e: KeyboardEvent) {
  // keep-alive 下本组件切走时不销毁，避免快捷键误触发其他工具
  if (settings.activeTool !== 'markdown') return
  const ctrl = e.ctrlKey || e.metaKey
  if (ctrl && !e.shiftKey && e.key.toLowerCase() === 'o') {
    e.preventDefault()
    openFileDialog()
  } else if (ctrl && !e.shiftKey && e.key.toLowerCase() === 'n') {
    e.preventDefault()
    newFile()
  } else if (ctrl && !e.shiftKey && e.key.toLowerCase() === 's') {
    e.preventDefault()
    saveFile()
  } else if (ctrl && e.shiftKey && e.key.toLowerCase() === 's') {
    e.preventDefault()
    saveAs()
  } else if (ctrl && !e.shiftKey && e.key.toLowerCase() === 'e') {
    e.preventDefault()
    cycleMode()
  } else if (e.key === 'F11') {
    e.preventDefault()
    toggleImmersive()
  } else if (e.key === 'Escape' && settings.immersive) {
    settings.setImmersive(false)
  }
}

let unlistenDrag: (() => void) | null = null

onMounted(async () => {
  try {
    unlistenDrag = await getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === 'drop') {
        const first = event.payload.paths[0]
        if (first) loadPath(first)
      }
    })
  } catch (e) {
    console.warn('drag-drop listener failed:', e)
  }
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  unlistenDrag?.()
  window.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <div class="md-tool">
    <header v-show="!settings.immersive" class="toolbar">
      <div class="tb-left">
        <span class="file-name">{{ currentTitle }}</span>
        <span v-if="settings.dirty" class="dirty-dot" title="Unsaved">●</span>
        <div class="mode-switch">
          <button :class="{ active: settings.mode === 'edit' }" @click="setMode('edit')">{{ t('mode.edit') }}</button>
          <button :class="{ active: settings.mode === 'split' }" @click="setMode('split')">{{ t('mode.split') }}</button>
          <button :class="{ active: settings.mode === 'preview' }" @click="setMode('preview')">{{ t('mode.preview') }}</button>
        </div>
      </div>
      <div class="tb-right">
        <div class="menu-anchor">
          <button class="btn" @click="showRecent = !showRecent">{{ t('toolbar.recent') }} ▾</button>
          <div v-if="showRecent" class="dropdown">
            <div v-if="!settings.recentFiles.length" class="dropdown-empty">{{ t('recent.empty') }}</div>
            <div v-for="p in settings.recentFiles" :key="p" class="dropdown-item-wrap">
              <button class="dropdown-item" @click="loadPath(p)">{{ p.split(/[/\\]/).pop() }}</button>
              <button class="dropdown-x" @click.stop="removeRecent(p)">✕</button>
            </div>
            <button v-if="settings.recentFiles.length" class="dropdown-clear" @click="settings.clearRecent(); savePersisted()">{{ t('recent.clearAll') }}</button>
          </div>
        </div>
        <button class="btn" @click="newFile" title="Ctrl+N">{{ t('toolbar.new') }}</button>
        <button class="btn btn-primary" @click="openFileDialog" title="Ctrl+O">
          <span class="btn-icon">📂</span>
          <span>{{ t('toolbar.open') }}</span>
        </button>
        <button class="btn" :disabled="!settings.dirty" @click="saveFile" title="Ctrl+S">{{ t('toolbar.save') }}</button>
      </div>
    </header>

    <main class="md-main" @click="showRecent = false">
      <Reader v-if="settings.currentPath" ref="readerRef" :path="settings.currentPath" />
      <div v-else class="empty-state">
        <div class="empty-icon">
          <svg width="72" height="72" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M6 3h8l5 5v12a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z" />
            <path d="M13 3v6h6" />
            <path d="M8 13h6M8 16h4" />
          </svg>
        </div>
        <h1>Markdown</h1>
        <p class="subtitle">{{ t('empty.subtitle') }}</p>
        <div class="empty-actions">
          <button class="btn btn-primary btn-large" @click.stop="openFileDialog">
            {{ t('empty.openFile') }} <kbd class="kbd">Ctrl+O</kbd>
          </button>
          <button class="btn btn-large" @click.stop="newFile">
            {{ t('empty.newFile') }} <kbd class="kbd">Ctrl+N</kbd>
          </button>
        </div>
        <p class="hint">{{ t('empty.dragHint') }}</p>
        <div v-if="settings.recentFiles.length" class="recent-block">
          <div class="recent-title">{{ t('empty.recent') }}</div>
          <button v-for="p in settings.recentFiles" :key="p" class="recent-item" @click.stop="loadPath(p)">
            {{ p.split(/[/\\]/).pop() }}
          </button>
        </div>
      </div>
      <div v-if="errorMessage" class="error-banner" @click.stop>
        <span>⚠</span>
        <span>{{ errorMessage }}</span>
        <button class="error-close" @click="errorMessage = null">✕</button>
      </div>
    </main>

    <footer v-show="!settings.immersive" class="statusbar">
      <span>{{ settings.currentPath || t('status.noFile') }}</span>
      <span class="status-right">
        <span v-if="settings.dirty" class="dirty-indicator">{{ t('status.unsaved') }}</span>
        <span v-else-if="settings.currentPath">{{ t('status.saved') }}</span>
        <span>{{ t('mode.' + settings.mode) }}</span>
      </span>
    </footer>
  </div>
</template>

<style scoped>
.md-tool {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-width: 0;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 14px;
  border-bottom: 1px solid var(--border);
  background: var(--surface);
  user-select: none;
  gap: 12px;
}

.tb-left,
.tb-right {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.file-name {
  font-size: 13px;
  color: var(--text);
  font-weight: 500;
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dirty-dot {
  color: var(--accent);
  font-size: 12px;
}

.mode-switch {
  display: flex;
  margin-left: 8px;
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
}

.mode-switch button {
  border: 0;
  background: var(--bg);
  color: var(--text-muted);
  padding: 5px 12px;
  font-size: 12px;
  cursor: pointer;
}

.mode-switch button + button {
  border-left: 1px solid var(--border);
}

.mode-switch button.active {
  background: var(--accent);
  color: var(--accent-fg);
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 12px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s ease;
  white-space: nowrap;
}

.btn:hover {
  background: var(--surface-hover);
  border-color: var(--accent);
}

.btn:disabled {
  opacity: 0.45;
  cursor: default;
}

.btn-primary {
  background: var(--accent);
  color: var(--accent-fg);
  border-color: var(--accent);
}

.btn-primary:hover {
  background: var(--accent-hover);
}

.btn-large {
  padding: 10px 20px;
  font-size: 14px;
}

.btn-icon {
  font-size: 14px;
}

.kbd {
  display: inline-block;
  padding: 1px 6px;
  font-size: 10px;
  font-family: var(--font-mono);
  background: rgba(0, 0, 0, 0.15);
  border-radius: 3px;
  margin-left: 6px;
  opacity: 0.8;
}

.btn-primary .kbd {
  background: rgba(255, 255, 255, 0.2);
  color: var(--accent-fg);
}

.menu-anchor {
  position: relative;
}

.dropdown {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  min-width: 240px;
  max-width: 360px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  padding: 6px;
  z-index: 50;
}

.dropdown-empty {
  padding: 12px;
  color: var(--text-muted);
  font-size: 13px;
  text-align: center;
}

.dropdown-item-wrap {
  display: flex;
  align-items: center;
}

.dropdown-item {
  flex: 1;
  text-align: left;
  background: transparent;
  border: 0;
  color: var(--text);
  padding: 7px 10px;
  font-size: 13px;
  border-radius: 5px;
  cursor: pointer;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dropdown-item:hover {
  background: var(--surface-hover);
}

.dropdown-x {
  background: transparent;
  border: 0;
  color: var(--text-muted);
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
}

.dropdown-x:hover {
  color: var(--error);
}

.dropdown-clear {
  width: 100%;
  margin-top: 4px;
  padding: 6px;
  background: transparent;
  border: 0;
  border-top: 1px solid var(--border);
  color: var(--text-muted);
  font-size: 12px;
  cursor: pointer;
}

.dropdown-clear:hover {
  color: var(--error);
}

.md-main {
  flex: 1;
  overflow: hidden;
  position: relative;
  min-width: 0;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 12px;
  color: var(--text-muted);
  position: relative;
}

.empty-icon {
  color: var(--accent);
  opacity: 0.6;
  margin-bottom: 8px;
}

.empty-state h1 {
  font-size: 30px;
  font-weight: 700;
  margin: 0;
  color: var(--text);
  letter-spacing: -0.5px;
}

.subtitle {
  font-size: 14px;
  margin: 0 0 8px 0;
}

.empty-actions {
  display: flex;
  gap: 10px;
}

.hint {
  font-size: 12px;
  margin: 0;
  opacity: 0.7;
}

.recent-block {
  margin-top: 16px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  max-width: 420px;
}

.recent-title {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-muted);
  margin-bottom: 4px;
}

.recent-item {
  background: transparent;
  border: 0;
  color: var(--accent);
  font-size: 13px;
  cursor: pointer;
  padding: 3px 10px;
  border-radius: 4px;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recent-item:hover {
  background: var(--surface);
}

.statusbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 14px;
  border-top: 1px solid var(--border);
  background: var(--surface);
  font-size: 11px;
  color: var(--text-muted);
  font-family: var(--font-mono);
  overflow: hidden;
  white-space: nowrap;
}

.statusbar > span:first-child {
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}

.status-right {
  display: flex;
  gap: 14px;
  flex-shrink: 0;
}

.dirty-indicator {
  color: var(--accent);
}

.error-banner {
  position: absolute;
  bottom: 16px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 16px;
  background: var(--error);
  color: white;
  border-radius: 6px;
  font-size: 13px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  max-width: 80%;
  z-index: 100;
}

.error-close {
  background: transparent;
  border: 0;
  color: white;
  cursor: pointer;
  padding: 0 4px;
  opacity: 0.8;
}

.error-close:hover {
  opacity: 1;
}
</style>