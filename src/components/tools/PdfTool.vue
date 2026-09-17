<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'

const { t } = useI18n()
type Tab = 'merge' | 'split' | 'rotate' | 'delete'
const active = ref<Tab>('merge')
const busy = ref(false)
const msg = ref('')
const err = ref('')

async function run(fn: () => Promise<unknown>) {
  busy.value = true
  err.value = ''
  msg.value = ''
  try {
    await fn()
  } catch (e: unknown) {
    err.value = t('pdf.failed', { msg: String(e) })
  } finally {
    busy.value = false
  }
}

// ---- merge ----
const mergeFiles = ref<string[]>([])
async function pickMerge() {
  const sel = await open({ multiple: true, filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (!sel) return
  mergeFiles.value = Array.isArray(sel) ? sel : [sel]
}
async function doMerge() {
  if (mergeFiles.value.length < 2) {
    err.value = t('pdf.needTwo')
    return
  }
  const out = await save({ defaultPath: 'merged.pdf', filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (!out) return
  const paths = mergeFiles.value
  await run(() => invoke('pdf_merge', { paths, out }))
  msg.value = t('pdf.done', { out })
}

// ---- split ----
const splitFile = ref('')
async function pickSplit() {
  const s = await open({ multiple: false, filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (s) splitFile.value = Array.isArray(s) ? s[0] ?? '' : s
}
async function doSplit() {
  if (!splitFile.value) return
  const dir = await open({ directory: true })
  if (!dir) return
  const path = splitFile.value
  await run(() => invoke('pdf_split', { path, outDir: dir }))
  msg.value = t('pdf.splitDone', { dir })
}

// ---- rotate ----
const rotFile = ref('')
const rotPages = ref('')
const rotAngle = ref(90)
async function pickRot() {
  const s = await open({ multiple: false, filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (s) rotFile.value = Array.isArray(s) ? s[0] ?? '' : s
}
async function doRotate() {
  if (!rotFile.value) return
  const out = await save({ defaultPath: 'rotated.pdf', filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (!out) return
  const path = rotFile.value
  const pages = rotPages.value
  const angle = rotAngle.value
  await run(() => invoke('pdf_rotate', { path, pages, angle, out }))
  msg.value = t('pdf.done', { out })
}

// ---- delete ----
const delFile = ref('')
const delPages = ref('')
async function pickDel() {
  const s = await open({ multiple: false, filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (s) delFile.value = Array.isArray(s) ? s[0] ?? '' : s
}
async function doDelete() {
  if (!delFile.value || !delPages.value.trim()) {
    err.value = t('pdf.needPages')
    return
  }
  const out = await save({ defaultPath: 'deleted.pdf', filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (!out) return
  const path = delFile.value
  const pages = delPages.value
  await run(() => invoke('pdf_delete', { path, pages, out }))
  msg.value = t('pdf.done', { out })
}

function base(p: string) {
  return p.split(/[/\\]/).pop() || p
}
</script>

<template>
  <div class="pdf-tool">
    <div class="pdf-tabs">
      <button v-for="tab in (['merge', 'split', 'rotate', 'delete'] as const)" :key="tab" :class="{ active: active === tab }" @click="active = tab">
        {{ t(`pdf.${tab}`) }}
      </button>
    </div>

    <div class="pdf-body">
      <!-- merge -->
      <div v-if="active === 'merge'" class="pdf-panel">
        <button class="btn" @click="pickMerge">{{ t('pdf.pickFiles') }}</button>
        <div v-if="mergeFiles.length" class="pdf-filelist">
          <div class="pdf-count">{{ t('pdf.selected', { n: mergeFiles.length }) }}</div>
          <ul>
            <li v-for="(f, i) in mergeFiles" :key="i" :title="f">{{ base(f) }}</li>
          </ul>
        </div>
        <button class="btn btn-primary" :disabled="busy || mergeFiles.length < 2" @click="doMerge">
          {{ t('pdf.doMerge') }}
        </button>
      </div>

      <!-- split -->
      <div v-else-if="active === 'split'" class="pdf-panel">
        <button class="btn" @click="pickSplit">{{ t('pdf.pickFile') }}</button>
        <div v-if="splitFile" class="pdf-path" :title="splitFile">{{ base(splitFile) }}</div>
        <button class="btn btn-primary" :disabled="busy || !splitFile" @click="doSplit">
          {{ t('pdf.doSplit') }}
        </button>
      </div>

      <!-- rotate -->
      <div v-else-if="active === 'rotate'" class="pdf-panel">
        <button class="btn" @click="pickRot">{{ t('pdf.pickFile') }}</button>
        <div v-if="rotFile" class="pdf-path" :title="rotFile">{{ base(rotFile) }}</div>
        <div class="pdf-field">
          <label>{{ t('pdf.pages') }}</label>
          <input v-model="rotPages" class="pdf-input" placeholder="1,3,5" />
        </div>
        <div class="pdf-field">
          <label>{{ t('pdf.angle') }}</label>
          <div class="seg">
            <button v-for="a in [90, 180, 270]" :key="a" :class="{ active: rotAngle === a }" @click="rotAngle = a">{{ a }}°</button>
          </div>
        </div>
        <button class="btn btn-primary" :disabled="busy || !rotFile" @click="doRotate">
          {{ t('pdf.doRotate') }}
        </button>
      </div>

      <!-- delete -->
      <div v-else-if="active === 'delete'" class="pdf-panel">
        <button class="btn" @click="pickDel">{{ t('pdf.pickFile') }}</button>
        <div v-if="delFile" class="pdf-path" :title="delFile">{{ base(delFile) }}</div>
        <div class="pdf-field">
          <label>{{ t('pdf.pagesDel') }}</label>
          <input v-model="delPages" class="pdf-input" placeholder="2,4" />
        </div>
        <button class="btn btn-primary" :disabled="busy || !delFile" @click="doDelete">
          {{ t('pdf.doDelete') }}
        </button>
      </div>

      <div v-if="msg" class="pdf-msg">✅ {{ msg }}</div>
      <div v-if="err" class="pdf-err">⚠ {{ err }}</div>
    </div>
  </div>
</template>

<style scoped>
.pdf-tool {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.pdf-tabs {
  display: flex;
  gap: 2px;
  padding: 8px 14px;
  border-bottom: 1px solid var(--border);
  background: var(--surface);
}

.pdf-tabs button {
  border: 0;
  background: transparent;
  color: var(--text-muted);
  padding: 6px 14px;
  font-size: 13px;
  border-radius: 6px;
  cursor: pointer;
}

.pdf-tabs button:hover {
  background: var(--surface-hover);
  color: var(--text);
}

.pdf-tabs button.active {
  background: var(--accent);
  color: var(--accent-fg);
}

.pdf-body {
  flex: 1;
  overflow-y: auto;
  padding: 22px 26px;
}

.pdf-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
  max-width: 560px;
}

.btn {
  align-self: flex-start;
  padding: 8px 16px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text);
  font-size: 13px;
  cursor: pointer;
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

.pdf-path {
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--text);
  padding: 6px 10px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pdf-filelist {
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px 12px;
  max-height: 220px;
  overflow-y: auto;
}

.pdf-count {
  font-size: 12px;
  color: var(--text-muted);
  margin-bottom: 4px;
}

.pdf-filelist ul {
  margin: 0;
  padding-left: 18px;
  font-family: var(--font-mono);
  font-size: 12px;
}

.pdf-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.pdf-field label {
  font-size: 12px;
  color: var(--text-muted);
}

.pdf-input {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 13px;
}

.pdf-input:focus {
  outline: none;
  border-color: var(--accent);
}

.seg {
  display: inline-flex;
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
  align-self: flex-start;
}

.seg button {
  border: 0;
  background: var(--bg);
  color: var(--text-muted);
  padding: 6px 14px;
  font-size: 13px;
  cursor: pointer;
}

.seg button + button {
  border-left: 1px solid var(--border);
}

.seg button.active {
  background: var(--accent);
  color: var(--accent-fg);
}

.pdf-msg {
  color: var(--accent);
  font-size: 13px;
  font-family: var(--font-mono);
  word-break: break-all;
}

.pdf-err {
  color: var(--error);
  font-size: 13px;
  padding: 8px 12px;
  background: var(--surface);
  border: 1px solid var(--error);
  border-radius: 6px;
  word-break: break-word;
}
</style>