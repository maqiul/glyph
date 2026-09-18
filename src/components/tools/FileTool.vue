<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readFile } from '@tauri-apps/plugin-fs'
import { useSettingsStore } from '../../stores/settings'

const { t } = useI18n()
const settings = useSettingsStore()

// 目标/源编码候选（value = encoding_rs 标签）
const ENCODINGS = [
  { label: 'UTF-8', value: 'UTF-8' },
  { label: 'UTF-8 with BOM', value: 'UTF-8 BOM' },
  { label: 'GBK', value: 'GBK' },
  { label: 'GB18030', value: 'GB18030' },
  { label: 'Big5', value: 'BIG5' },
  { label: 'Shift_JIS', value: 'SHIFT_JIS' },
  { label: 'EUC-KR', value: 'EUC-KR' },
  { label: 'Latin-1 (Windows-1252)', value: 'windows-1252' },
  { label: 'UTF-16 LE', value: 'UTF-16LE' },
  { label: 'UTF-16 BE', value: 'UTF-16BE' },
]

const filePath = ref('')
const fileName = ref('')
const detected = ref('')
const preview = ref('')
const fromSel = ref('')
const toSel = ref('UTF-8')
const status = ref('')
const error = ref('')
const busy = ref(false)

function fileNameFromPath(p: string) {
  return p.split(/[\\/]/).pop() || p
}

// 用浏览器 TextDecoder 预览解码结果（Chromium 支持 gbk/big5/shift_jis/euc-kr 等）
async function makePreview(path: string, enc: string) {
  preview.value = ''
  try {
    const bytes = await readFile(path)
    const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes)
    const head = u8.subarray(0, 64 * 1024)
    const label = /utf-?8/i.test(enc) ? 'utf-8' : enc.replace(/\s*\(BOM\)/i, '')
    let text = ''
    try {
      text = new TextDecoder(label, { fatal: false }).decode(head)
    } catch {
      text = new TextDecoder('utf-8', { fatal: false }).decode(head)
    }
    preview.value = text.slice(0, 5000)
  } catch (e) {
    console.warn('preview failed:', e)
  }
}

async function loadPath(path: string) {
  error.value = ''
  status.value = ''
  filePath.value = path
  fileName.value = fileNameFromPath(path)
  busy.value = true
  try {
    detected.value = await invoke<string>('detect_encoding', { path })
    await makePreview(path, detected.value)
  } catch (e) {
    error.value = String(e)
    detected.value = ''
  } finally {
    busy.value = false
  }
}

async function pick() {
  const sel = await open({ multiple: false })
  if (typeof sel === 'string') await loadPath(sel)
}

async function convert() {
  if (!filePath.value) return
  error.value = ''
  status.value = ''
  const to = toSel.value
  const out = await save({
    defaultPath: fileName.value || 'converted.txt',
    filters: [{ name: 'All Files', extensions: ['*'] }],
  })
  if (!out) return
  busy.value = true
  try {
    const from = fromSel.value
    // “UTF-8 BOM” 后端按 UTF-8 解码/编码，再补 BOM：这里交给后端，目标用 UTF-8
    const realTo = to === 'UTF-8 BOM' ? 'UTF-8' : to
    const result = await invoke<string>('convert_file_encoding', {
      path: filePath.value,
      from,
      to: realTo,
      out,
    })
    status.value = t('file.done', { result, out })
  } catch (e) {
    error.value = String(e)
  } finally {
    busy.value = false
  }
}

let unDrop: (() => void) | null = null
onMounted(async () => {
  try {
    unDrop = await listen<{ path: string }>('app-file-drop', (e) => {
      if (settings.activeTool !== 'file') return
      void loadPath(e.payload.path)
    })
  } catch {
    /* 忽略 */
  }
})
onUnmounted(() => unDrop?.())
</script>

<template>
  <div class="file-tool">
    <div class="file-body">
      <div class="file-drop" @click="pick" @dragover.prevent @drop.prevent>
        <div v-if="fileName" class="file-picked">
          <div class="file-name" :title="filePath">{{ fileName }}</div>
          <div class="file-enc">
            {{ t('file.detected') }}：<b>{{ detected || '—' }}</b>
          </div>
        </div>
        <span v-else class="file-hint">{{ t('file.dropHint') }}</span>
      </div>

      <div v-if="error" class="file-err">⚠ {{ error }}</div>

      <div v-if="filePath" class="file-row">
        <label class="file-label">{{ t('file.from') }}</label>
        <select v-model="fromSel" class="file-select">
          <option value="">{{ t('file.auto') }}</option>
          <option v-for="e in ENCODINGS" :key="'f' + e.value" :value="e.value">
            {{ e.label }}
          </option>
        </select>
        <label class="file-label">{{ t('file.to') }}</label>
        <select v-model="toSel" class="file-select">
          <option v-for="e in ENCODINGS" :key="'t' + e.value" :value="e.value">
            {{ e.label }}
          </option>
        </select>
        <button class="btn" :disabled="busy" @click="convert">{{ t('file.convert') }}</button>
      </div>

      <div v-if="status" class="file-ok">✓ {{ status }}</div>

      <div class="file-preview-head">{{ t('file.preview') }}</div>
      <pre class="file-preview">{{ preview || '—' }}</pre>
    </div>
  </div>
</template>

<style scoped>
.file-tool {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.file-body {
  flex: 1;
  overflow-y: auto;
  padding: 18px 22px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.file-drop {
  border: 1.5px dashed var(--border);
  border-radius: 10px;
  padding: 26px;
  text-align: center;
  cursor: pointer;
  transition:
    border-color 0.15s ease,
    background 0.15s ease;
}

.file-drop:hover {
  border-color: var(--accent);
  background: var(--surface);
}

.file-hint {
  color: var(--text-muted);
  font-size: 14px;
}

.file-picked {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.file-name {
  font-weight: 600;
  font-size: 14px;
  word-break: break-all;
}

.file-enc {
  font-size: 13px;
  color: var(--text-muted);
}

.file-enc b {
  color: var(--accent);
}

.file-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.file-label {
  font-size: 12px;
  color: var(--text-muted);
}

.file-select {
  padding: 7px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--text);
  font-size: 13px;
}

.file-preview-head {
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-muted);
}

.file-preview {
  margin: 0;
  flex: 1;
  min-height: 160px;
  padding: 12px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
  overflow: auto;
}

.btn {
  padding: 7px 16px;
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
  opacity: 0.5;
  cursor: default;
}

.file-err {
  color: var(--error);
  font-size: 13px;
  padding: 8px 12px;
  background: var(--surface);
  border: 1px solid var(--error);
  border-radius: 6px;
  white-space: pre-wrap;
  word-break: break-word;
}

.file-ok {
  color: var(--accent);
  font-size: 13px;
  font-family: var(--font-mono);
  word-break: break-all;
}
</style>
