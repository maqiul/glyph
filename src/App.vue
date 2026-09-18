<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { emit } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import ActivityBar from './components/ActivityBar.vue'
import CommandPalette from './components/CommandPalette.vue'
import { TOOLS } from './tools'
import { useSettingsStore } from './stores/settings'
import { loadPersisted, savePersisted } from './stores/persistent'
import { setLocale, type Lang } from './i18n'

interface AppInfo {
  name: string
  version: string
  tauri_version: string
  platform: string
}

const { t } = useI18n()
const settings = useSettingsStore()
const appInfo = ref<AppInfo | null>(null)
const showSettings = ref(false)
const showPalette = ref(false)

const activeToolDef = computed(
  () => TOOLS.find((tool) => tool.id === settings.activeTool) ?? TOOLS[0]!,
)
const activeToolLabel = computed(() => t(activeToolDef.value.labelKey))

async function bootstrap() {
  try {
    appInfo.value = await invoke<AppInfo>('get_app_info')
    await loadPersisted()
  } catch (e) {
    console.warn('bootstrap failed:', e)
  }
}

function toggleTheme() {
  settings.toggleTheme()
  savePersisted()
}

function changeLang(l: Lang) {
  settings.setLanguage(l)
  setLocale(l)
  savePersisted()
}

function setTheme(theme: 'light' | 'dark') {
  settings.setTheme(theme)
  savePersisted()
}

function handleKeydown(e: KeyboardEvent) {
  const ctrl = e.ctrlKey || e.metaKey
  if (ctrl && e.shiftKey && e.key.toLowerCase() === 'p') {
    e.preventDefault()
    showPalette.value = !showPalette.value
  } else if (ctrl && !e.shiftKey && e.key.toLowerCase() === 'd') {
    e.preventDefault()
    toggleTheme()
  }
}

let unlistenDrag: (() => void) | null = null

onMounted(async () => {
  await bootstrap()
  document.documentElement.setAttribute('data-theme', settings.theme)
  window.addEventListener('keydown', handleKeydown)
  try {
    unlistenDrag = await getCurrentWebview().onDragDropEvent((e) => {
      if (e.payload.type === 'drop') {
        const first = e.payload.paths[0]
        if (first) emit('app-file-drop', { path: first })
      }
    })
  } catch (e) {
    console.warn('drag listener failed:', e)
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
  unlistenDrag?.()
})
</script>

<template>
  <div class="app-shell">
    <header class="global-bar">
      <div class="gb-left">
        <svg class="logo" width="22" height="22" viewBox="0 0 32 32" aria-label="Glyph">
          <rect x="2" y="2" width="28" height="28" rx="7" fill="currentColor" />
          <path d="M22 11.5a6 6 0 1 0 0 9" stroke="var(--bg)" stroke-width="2.4" stroke-linecap="round" fill="none" />
          <line x1="16" y1="16" x2="22" y2="16" stroke="var(--bg)" stroke-width="2.4" stroke-linecap="round" />
          <line x1="22" y1="16" x2="22" y2="20" stroke="var(--bg)" stroke-width="2.4" stroke-linecap="round" />
        </svg>
        <span class="app-name">Glyph</span>
        <span class="divider">·</span>
        <span class="tool-name">{{ activeToolLabel }}</span>
      </div>

      <div class="gb-right">
        <button class="btn btn-icon-only" :title="t('toolbar.themeTip')" @click="toggleTheme">
          {{ settings.theme === 'light' ? '🌙' : '☀️' }}
        </button>
        <div class="menu-anchor">
          <button class="btn btn-icon-only" :title="t('toolbar.settingsTip')" @click="showSettings = !showSettings">⚙</button>
          <div v-if="showSettings" class="dropdown dropdown-settings" @click.stop>
            <div class="setting-group">
              <div class="setting-label">{{ t('settings.language') }}</div>
              <div class="setting-row">
                <button :class="{ active: settings.language === 'zh' }" @click="changeLang('zh')">中文</button>
                <button :class="{ active: settings.language === 'en' }" @click="changeLang('en')">English</button>
              </div>
            </div>
            <div class="setting-group">
              <div class="setting-label">{{ t('settings.theme') }}</div>
              <div class="setting-row">
                <button :class="{ active: settings.theme === 'light' }" @click="setTheme('light')">☀️ Light</button>
                <button :class="{ active: settings.theme === 'dark' }" @click="setTheme('dark')">🌙 Dark</button>
              </div>
            </div>
            <p v-if="appInfo" class="about">Glyph v{{ appInfo.version }} · Tauri {{ appInfo.tauri_version }} · {{ appInfo.platform }}</p>
          </div>
        </div>
      </div>
    </header>

    <div class="body" @click="showSettings = false">
      <ActivityBar />
      <main class="tool-area">
        <keep-alive>
          <component :is="activeToolDef.component" />
        </keep-alive>
      </main>
    </div>

    <CommandPalette v-if="showPalette" @close="showPalette = false" />
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--bg);
  color: var(--text);
}

.global-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 14px;
  border-bottom: 1px solid var(--border);
  background: var(--surface);
  user-select: none;
  -webkit-app-region: drag;
  gap: 12px;
}

.gb-left,
.gb-right {
  display: flex;
  align-items: center;
  gap: 8px;
  -webkit-app-region: no-drag;
  min-width: 0;
}

.logo {
  color: var(--accent);
  flex-shrink: 0;
}

.app-name {
  font-weight: 600;
  font-size: 14px;
  letter-spacing: 0.5px;
}

.divider {
  color: var(--text-muted);
  margin: 0 2px;
}

.tool-name {
  font-size: 13px;
  color: var(--text-muted);
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

.btn-icon-only {
  padding: 5px 9px;
  font-size: 14px;
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

.dropdown-settings {
  min-width: 280px;
}

.setting-group {
  padding: 8px 10px;
}

.setting-group + .setting-group {
  border-top: 1px solid var(--border);
}

.setting-label {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-muted);
  margin-bottom: 6px;
}

.setting-row {
  display: flex;
  gap: 6px;
}

.setting-row button {
  flex: 1;
  padding: 4px 8px;
  font-size: 12px;
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text);
  border-radius: 5px;
  cursor: pointer;
}

.setting-row button.active {
  background: var(--accent);
  color: var(--accent-fg);
  border-color: var(--accent);
}

.about {
  margin: 6px 10px 4px;
  font-size: 10px;
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.body {
  flex: 1;
  display: flex;
  min-height: 0;
}

.tool-area {
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.dir-btn {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: left;
}

.dir-clear {
  flex: 0 0 auto;
  width: 32px;
}
</style>