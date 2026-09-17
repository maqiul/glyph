<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useSettingsStore } from '../../stores/settings'
import { savePersisted } from '../../stores/persistent'

const { t } = useI18n()
const settings = useSettingsStore()

const text = ref('')
const busy = ref(false)
const error = ref<string | null>(null)
const copied = ref(false)
const sourceName = ref('')

const stats = computed(() => {
  const chars = text.value.length
  const lines = text.value ? text.value.split('\n').length : 0
  return { chars, lines }
})

const hint = computed(() =>
  settings.ocrEngine === 'cloud' ? t('ocr.cloudNote') : t('ocr.hint'),
)

function setEngine(e: 'local' | 'cloud') {
  settings.setOcrEngine(e)
  savePersisted()
}

function setProvider(p: 'baidu' | 'ali' | 'tencent') {
  settings.setOcrProvider(p)
  savePersisted()
}

function onApiKey(e: Event) {
  settings.ocrApiKey = (e.target as HTMLInputElement).value
  savePersisted()
}

function onSecretKey(e: Event) {
  settings.ocrSecretKey = (e.target as HTMLInputElement).value
  savePersisted()
}

async function pickAndRecognize() {
  try {
    const sel = await open({
      multiple: false,
      filters: [
        { name: 'Image', extensions: ['png', 'jpg', 'jpeg', 'bmp', 'webp', 'gif', 'tiff'] },
      ],
    })
    if (!sel) return
    const path = Array.isArray(sel) ? sel[0] : sel
    if (path) await recognize(path)
  } catch (e) {
    error.value = t('ocr.failed', { msg: String(e) })
  }
}

async function recognize(path: string) {
  busy.value = true
  error.value = null
  copied.value = false
  sourceName.value = path.split(/[/\\]/).pop() || path
  try {
    if (settings.ocrEngine === 'cloud') {
      text.value = await invoke<string>('ocr_recognize_cloud', {
        path,
        provider: settings.ocrProvider,
        apiKey: settings.ocrApiKey,
        secretKey: settings.ocrSecretKey,
      })
    } else {
      text.value = await invoke<string>('ocr_recognize_file', { path })
    }
  } catch (e: unknown) {
    const msg = typeof e === 'string' ? e : (e as any)?.message ?? JSON.stringify(e)
    error.value = t('ocr.failed', { msg })
    text.value = ''
  } finally {
    busy.value = false
  }
}

async function copy() {
  try {
    await navigator.clipboard.writeText(text.value)
    copied.value = true
    setTimeout(() => (copied.value = false), 1500)
  } catch (e) {
    console.warn('clipboard failed:', e)
  }
}
</script>

<template>
  <div class="ocr-tool">
    <div class="ocr-toolbar">
      <div class="ocr-head">
        <h1>{{ t('placeholder.ocrTitle') }}</h1>
        <p class="ocr-hint">{{ hint }}</p>
      </div>
      <button class="btn btn-primary" :disabled="busy" @click="pickAndRecognize">
        {{ busy ? t('ocr.recognizing') : t('ocr.pickImage') }}
      </button>
    </div>

    <div class="ocr-config">
      <div class="cfg-group">
        <span class="cfg-label">{{ t('ocr.engine') }}</span>
        <div class="seg">
          <button :class="{ active: settings.ocrEngine === 'local' }" @click="setEngine('local')">
            {{ t('ocr.engineLocal') }}
          </button>
          <button :class="{ active: settings.ocrEngine === 'cloud' }" @click="setEngine('cloud')">
            {{ t('ocr.engineCloud') }}
          </button>
        </div>
      </div>

      <template v-if="settings.ocrEngine === 'cloud'">
        <div class="cfg-group">
          <span class="cfg-label">{{ t('ocr.provider') }}</span>
          <div class="seg">
            <button :class="{ active: settings.ocrProvider === 'baidu' }" @click="setProvider('baidu')">百度</button>
            <button :class="{ active: settings.ocrProvider === 'ali' }" @click="setProvider('ali')">阿里</button>
            <button :class="{ active: settings.ocrProvider === 'tencent' }" @click="setProvider('tencent')">腾讯</button>
          </div>
        </div>
        <div class="cfg-group cfg-keys">
          <input
            class="key-input"
            :placeholder="t('ocr.apiKey')"
            :value="settings.ocrApiKey"
            @input="onApiKey($event)"
          />
          <input
            class="key-input"
            type="password"
            :placeholder="t('ocr.secretKey')"
            :value="settings.ocrSecretKey"
            @input="onSecretKey($event)"
          />
        </div>
      </template>
    </div>

    <div v-if="error" class="ocr-error">⚠ {{ error }}</div>

    <div v-if="busy" class="ocr-busy">
      <div class="spinner"></div>
      <span>{{ t('ocr.recognizing') }}</span>
    </div>

    <div v-else-if="text || sourceName" class="ocr-result">
      <div class="ocr-result-head">
        <span class="ocr-src">{{ sourceName }}</span>
        <span class="ocr-stats">
          {{ t('ocr.chars', { n: stats.chars }) }} · {{ t('ocr.lines', { n: stats.lines }) }}
        </span>
        <button class="btn btn-small" :disabled="!text" @click="copy">
          {{ copied ? t('ocr.copied') : t('ocr.copy') }}
        </button>
      </div>
      <textarea class="ocr-text" readonly :value="text || t('ocr.empty')"></textarea>
    </div>

    <div v-else class="ocr-empty">
      <svg width="72" height="72" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
        <path d="M4 8V5a1 1 0 0 1 1-1h3" />
        <path d="M16 4h3a1 1 0 0 1 1 1v3" />
        <path d="M20 16v3a1 1 0 0 1-1 1h-3" />
        <path d="M8 20H5a1 1 0 0 1-1-1v-3" />
        <path d="M8 10h8M8 14h5" />
      </svg>
    </div>
  </div>
</template>

<style scoped>
.ocr-tool {
  height: 100%;
  overflow-y: auto;
  padding: 24px 28px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.ocr-toolbar {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 20px;
}

.ocr-head h1 {
  font-size: 22px;
  font-weight: 700;
  margin: 0 0 6px;
  color: var(--text);
}

.ocr-hint {
  font-size: 13px;
  color: var(--text-muted);
  margin: 0;
  max-width: 620px;
  line-height: 1.5;
}

.ocr-config {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 16px;
  padding: 12px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface);
}

.cfg-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.cfg-keys {
  flex: 1;
  min-width: 260px;
}

.cfg-label {
  font-size: 12px;
  color: var(--text-muted);
  white-space: nowrap;
}

.seg {
  display: flex;
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
}

.seg button {
  border: 0;
  background: var(--bg);
  color: var(--text-muted);
  padding: 4px 12px;
  font-size: 12px;
  cursor: pointer;
}

.seg button + button {
  border-left: 1px solid var(--border);
}

.seg button.active {
  background: var(--accent);
  color: var(--accent-fg);
}

.key-input {
  flex: 1;
  min-width: 120px;
  padding: 5px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--text);
  font-size: 12px;
  font-family: var(--font-mono);
}

.key-input:focus {
  outline: none;
  border-color: var(--accent);
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s ease;
  white-space: nowrap;
  flex-shrink: 0;
}

.btn:hover {
  background: var(--surface-hover);
  border-color: var(--accent);
}

.btn:disabled {
  opacity: 0.5;
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

.btn-small {
  padding: 4px 12px;
  font-size: 12px;
}

.ocr-error {
  color: var(--error);
  font-size: 13px;
  padding: 10px 14px;
  background: var(--surface);
  border: 1px solid var(--error);
  border-radius: 6px;
}

.ocr-busy {
  display: flex;
  align-items: center;
  gap: 12px;
  color: var(--text-muted);
  font-size: 14px;
  padding: 40px 0;
}

.spinner {
  width: 18px;
  height: 18px;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.ocr-result {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-height: 0;
  flex: 1;
}

.ocr-result-head {
  display: flex;
  align-items: center;
  gap: 12px;
}

.ocr-src {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ocr-stats {
  font-size: 12px;
  color: var(--text-muted);
  font-family: var(--font-mono);
  margin-right: auto;
}

.ocr-text {
  flex: 1;
  min-height: 300px;
  resize: none;
  padding: 14px 16px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg);
  color: var(--text);
  font-size: 14px;
  line-height: 1.6;
  font-family: var(--font-sans);
}

.ocr-text:focus {
  outline: none;
  border-color: var(--accent);
}

.ocr-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--accent);
  opacity: 0.4;
}
</style>