<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readFile } from '@tauri-apps/plugin-fs'
import { jsPDF } from 'jspdf'
import { useSettingsStore } from '../../stores/settings'

const { t } = useI18n()
const settings = useSettingsStore()
type Tab = 'merge' | 'split' | 'rotate' | 'delete' | 'img2pdf'
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

// ---- 列表操作：拖拽排序 + 上/下移 + 删除 + 清空 ----
function removeAt(idx: number, kind: 'merge' | 'split' | 'rotate' | 'delete') {
  if (kind === 'split') splitFile.value = ''
  else if (kind === 'rotate') rotFile.value = ''
  else if (kind === 'delete') delFile.value = ''
}
function moveOne(idx: number, delta: number) {
  const j = idx + delta
  if (j < 0 || j >= mergeFiles.value.length) return
  const arr = [...mergeFiles.value]
  ;[arr[idx], arr[j]] = [arr[j]!, arr[idx]!]
  mergeFiles.value = arr
}
function moveMerge(delta: number) {
  if (mergeFiles.value.length < 2) return
  const arr = [...mergeFiles.value]
  arr.unshift(arr.pop()!)
  arr.push(arr.shift()!)
  mergeFiles.value = arr
}
function clearMerge() {
  mergeFiles.value = []
}

// HTML5 drag-drop reorder（merge）
const dragIdx = ref<number | null>(null)
function dragStart(i: number, e: DragEvent) {
  dragIdx.value = i
  e.dataTransfer?.setData('text/plain', String(i))
}
function dragOver(i: number, _e: DragEvent) {
  // placeholder 视觉
}
function dragDrop(i: number, e: DragEvent) {
  e.preventDefault()
  const fromStr = e.dataTransfer?.getData('text/plain') ?? ''
  const from = Number(fromStr)
  if (Number.isNaN(from) || from === i) return
  const arr = [...mergeFiles.value]
  const [moved] = arr.splice(from, 1)
  if (!moved) return
  arr.splice(i, 0, moved)
  mergeFiles.value = arr
}
function dragEnd() {
  dragIdx.value = null
}

function moveOneImg(idx: number, delta: number) {
  const j = idx + delta
  if (j < 0 || j >= imgFiles.value.length) return
  const arr = [...imgFiles.value]
  ;[arr[idx], arr[j]] = [arr[j]!, arr[idx]!]
  imgFiles.value = arr
}
function moveImgs(delta: number) {
  if (imgFiles.value.length < 2) return
  const arr = [...imgFiles.value]
  arr.unshift(arr.pop()!)
  arr.push(arr.shift()!)
  imgFiles.value = arr
}
function clearImgs() {
  imgFiles.value = []
}
function removeImg(idx: number) {
  imgFiles.value = imgFiles.value.filter((_, i) => i !== idx)
}
const imgDragIdx = ref<number | null>(null)
function imgDragStart(i: number, e: DragEvent) {
  imgDragIdx.value = i
  e.dataTransfer?.setData('text/plain', 'img:' + i)
}
function imgDragOver(_i: number, _e: DragEvent) {}
function imgDragDrop(i: number, e: DragEvent) {
  e.preventDefault()
  const data = e.dataTransfer?.getData('text/plain') ?? ''
  const from = Number(data.replace('img:', ''))
  if (Number.isNaN(from) || from === i) return
  const arr = [...imgFiles.value]
  const [moved] = arr.splice(from, 1)
  if (!moved) return
  arr.splice(i, 0, moved)
  imgFiles.value = arr
}
function imgDragEnd() {
  imgDragIdx.value = null
}

// 兜底：drop 区接住窗口拖进来的文件（防止 dragdrop 只走上面的列表）
function onMergeDropFromWindow(_e: DragEvent) {
  // 走 App 全局事件已经 emit，这里不再做
}
function pickFromDrop(kind: 'split' | 'rotate' | 'delete') {
  // 同上占位
}
function addImgsFromDrop(_e: DragEvent) {
  // 同上占位
}

// ---- images -> pdf (jsPDF) ----
const imgFiles = ref<string[]>([])
async function pickImg() {
  const sel = await open({
    multiple: true,
    filters: [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp'] }],
  })
  if (!sel) return
  imgFiles.value = Array.isArray(sel) ? sel : [sel]
}
function toBase64(bytes: Uint8Array): string {
  let bin = ''
  const chunk = 0x8000
  for (let i = 0; i < bytes.length; i += chunk) {
    bin += String.fromCharCode(...Array.from(bytes.subarray(i, i + chunk)))
  }
  return btoa(bin)
}
function imgToPng(path: string): Promise<{ dataUrl: string; w: number; h: number }> {
  return new Promise((resolve, reject) => {
    readFile(path)
      .then((bytes) => {
        const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes)
        const ext = (path.split('.').pop() || 'png').toLowerCase()
        const mime =
          ext === 'jpg' || ext === 'jpeg' ? 'jpeg' : ext === 'webp' ? 'webp' : ext === 'bmp' ? 'bmp' : 'png'
        const url = `data:image/${mime};base64,${toBase64(u8)}`
        const img = new Image()
        img.onload = () => {
          const canvas = document.createElement('canvas')
          canvas.width = img.naturalWidth
          canvas.height = img.naturalHeight
          const ctx = canvas.getContext('2d')
          if (!ctx) {
            reject(new Error('canvas ctx'))
            return
          }
          ctx.drawImage(img, 0, 0)
          resolve({ dataUrl: canvas.toDataURL('image/png'), w: canvas.width, h: canvas.height })
        }
        img.onerror = () => reject(new Error('load image failed'))
        img.src = url
      })
      .catch(reject)
  })
}
async function doImg2Pdf() {
  if (imgFiles.value.length === 0) {
    err.value = t('pdf.needImg')
    return
  }
  const out = await save({ defaultPath: 'images.pdf', filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (!out) return
  const files = imgFiles.value
  await run(async () => {
    let pdf: jsPDF | null = null
    for (const p of files) {
      const { dataUrl, w, h } = await imgToPng(p)
      if (!pdf) pdf = new jsPDF({ unit: 'pt', format: [w, h] })
      else pdf.addPage([w, h])
      pdf.addImage(dataUrl, 'PNG', 0, 0, w, h)
    }
    if (!pdf) throw new Error('no image')
    const uri = pdf.output('datauristring')
    const b64 = uri.split(',')[1] ?? ''
    await invoke('write_file_base64', { path: out, dataBase64: b64 })
  })
  msg.value = t('pdf.done', { out })
}

let unDrop: (() => void) | null = null
onMounted(async () => {
  unDrop = await listen<{ path: string }>('app-file-drop', (e) => {
    if (settings.activeTool !== 'pdf') return
    const p = e.payload.path
    if (active.value === 'img2pdf' && /\.(png|jpe?g|webp|bmp)$/i.test(p)) {
      if (!imgFiles.value.includes(p)) imgFiles.value = [...imgFiles.value, p]
      return
    }
    if (!/\.pdf$/i.test(p)) return
    if (active.value === 'merge') {
      if (!mergeFiles.value.includes(p)) mergeFiles.value = [...mergeFiles.value, p]
    } else if (active.value === 'split') {
      splitFile.value = p
    } else if (active.value === 'rotate') {
      rotFile.value = p
    } else if (active.value === 'delete') {
      delFile.value = p
    }
  })
})
onUnmounted(() => unDrop?.())
</script>

<template>
  <div class="pdf-tool">
    <div class="pdf-tabs">
      <button v-for="tab in (['merge', 'split', 'rotate', 'delete', 'img2pdf'] as const)" :key="tab" :class="{ active: active === tab }" @click="active = tab">
        {{ t(`pdf.${tab}`) }}
      </button>
    </div>

    <div class="pdf-body">
      <!-- merge -->
      <div v-if="active === 'merge'" class="pdf-panel">
        <div class="pdf-drop" @click="pickMerge" @dragover.prevent @drop.prevent="onMergeDropFromWindow">
          {{ t('pdf.dropOrPick') }}
        </div>
        <div v-if="mergeFiles.length">
          <div class="pdf-list">
            <div class="pdf-list-head">
              <span class="count">{{ t('pdf.selected', { n: mergeFiles.length }) }}</span>
              <div class="list-tools">
                <button @click="moveMerge(-1)" :disabled="mergeFiles.length < 2">↑↑</button>
                <button @click="moveMerge(1)" :disabled="mergeFiles.length < 2">↓↓</button>
                <button @click="clearMerge" :disabled="!mergeFiles.length">{{ t('pdf.clear') }}</button>
              </div>
            </div>
            <div
              v-for="(f, i) in mergeFiles"
              :key="i"
              class="pdf-row"
            >
              <span class="order">{{ i + 1 }}</span>
              <span class="name" :title="f">{{ base(f) }}</span>
              <div class="row-tools">
                <button @click="moveOne(i, -1)" :disabled="i === 0" title="上移">↑</button>
                <button @click="moveOne(i, 1)" :disabled="i === mergeFiles.length - 1" title="下移">↓</button>
                <button class="remove" @click="removeAt(i, 'merge')" title="移除">✕</button>
              </div>
            </div>
          </div>
          <div class="btn-row">
            <button class="btn btn-primary-lg" :disabled="busy || mergeFiles.length < 2" @click="doMerge">
              {{ t('pdf.doMerge') }}
            </button>
          </div>
        </div>
      </div>

      <!-- split -->
      <div v-else-if="active === 'split'" class="pdf-panel">
        <div class="pdf-drop" @click="pickSplit" @dragover.prevent @drop.prevent="pickFromDrop('split')">
          {{ t('pdf.dropOrPick') }}
        </div>
        <div v-if="splitFile" class="pdf-path" :title="splitFile">{{ base(splitFile) }}</div>
        <div class="btn-row">
          <button class="btn btn-primary-lg" :disabled="busy || !splitFile" @click="doSplit">
            {{ t('pdf.doSplit') }}
          </button>
        </div>
      </div>

      <!-- rotate -->
      <div v-else-if="active === 'rotate'" class="pdf-panel">
        <div class="pdf-drop" @click="pickRot" @dragover.prevent @drop.prevent="pickFromDrop('rotate')">
          {{ t('pdf.dropOrPick') }}
        </div>
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
        <div class="btn-row">
          <button class="btn btn-primary-lg" :disabled="busy || !rotFile" @click="doRotate">
            {{ t('pdf.doRotate') }}
          </button>
        </div>
      </div>

      <!-- delete -->
      <div v-else-if="active === 'delete'" class="pdf-panel">
        <div class="pdf-drop" @click="pickDel" @dragover.prevent @drop.prevent="pickFromDrop('delete')">
          {{ t('pdf.dropOrPick') }}
        </div>
        <div v-if="delFile" class="pdf-path" :title="delFile">{{ base(delFile) }}</div>
        <div class="pdf-field">
          <label>{{ t('pdf.pagesDel') }}</label>
          <input v-model="delPages" class="pdf-input" placeholder="2,4" />
        </div>
        <div class="btn-row">
          <button class="btn btn-primary-lg" :disabled="busy || !delFile" @click="doDelete">
            {{ t('pdf.doDelete') }}
          </button>
        </div>
      </div>

      <!-- img2pdf -->
      <div v-else-if="active === 'img2pdf'" class="pdf-panel">
        <div class="pdf-drop" @click="pickImg" @dragover.prevent @drop.prevent="addImgsFromDrop">
          {{ t('pdf.dropOrPickImg') }}
        </div>
        <div v-if="imgFiles.length">
          <div class="pdf-list">
            <div class="pdf-list-head">
              <span class="count">{{ t('pdf.selected', { n: imgFiles.length }) }}</span>
              <div class="list-tools">
                <button @click="moveImgs(-1)" :disabled="imgFiles.length < 2">↑↑</button>
                <button @click="moveImgs(1)" :disabled="imgFiles.length < 2">↓↓</button>
                <button @click="clearImgs">{{ t('pdf.clear') }}</button>
              </div>
            </div>
            <div
              v-for="(f, i) in imgFiles"
              :key="i"
              class="pdf-row"
            >
              <span class="order">{{ i + 1 }}</span>
              <span class="name" :title="f">{{ base(f) }}</span>
              <div class="row-tools">
                <button @click="moveOneImg(i, -1)" :disabled="i === 0">↑</button>
                <button @click="moveOneImg(i, 1)" :disabled="i === imgFiles.length - 1">↓</button>
                <button class="remove" @click="removeImg(i)">✕</button>
              </div>
            </div>
          </div>
          <div class="btn-row">
            <button class="btn btn-primary-lg" :disabled="busy || !imgFiles.length" @click="doImg2Pdf">
              {{ t('pdf.doImg2Pdf') }}
            </button>
          </div>
        </div>
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
  padding: 18px 20px;
}

.pdf-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  max-width: 720px;
}

/* 文件列表 */
.pdf-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 6px;
  background: var(--surface);
}

.pdf-list-empty {
  padding: 24px 12px;
  text-align: center;
  color: var(--text-muted);
  font-size: 13px;
}

.pdf-list-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px 6px;
  border-bottom: 1px dashed var(--border);
  margin-bottom: 4px;
  font-size: 12px;
}

.pdf-list-head .count {
  color: var(--text-muted);
}

.pdf-list-head .list-tools {
  display: flex;
  gap: 6px;
}

.pdf-list-head .list-tools button {
  border: 0;
  background: transparent;
  color: var(--text-muted);
  font-size: 12px;
  cursor: pointer;
  padding: 2px 8px;
  border-radius: 4px;
}

.pdf-list-head .list-tools button:hover {
  background: var(--surface-hover);
  color: var(--text);
}

.pdf-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 8px;
}

.pdf-row.dragging {
  opacity: 0.5;
}

.pdf-row .order {
  flex-shrink: 0;
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  font-family: var(--font-mono);
  color: var(--text-muted);
  background: var(--surface);
  border-radius: 4px;
}

.pdf-row .name {
  flex: 1;
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pdf-row .name:hover {
  color: var(--accent);
  cursor: pointer;
}

.pdf-row .row-tools {
  display: flex;
  gap: 2px;
  flex-shrink: 0;
}

.pdf-row .row-tools button {
  width: 26px;
  height: 26px;
  border: 0;
  background: transparent;
  color: var(--text-muted);
  font-size: 14px;
  cursor: pointer;
  border-radius: 4px;
}

.pdf-row .row-tools button:hover {
  background: var(--surface-hover);
  color: var(--text);
}

.pdf-row .row-tools button.remove:hover {
  color: var(--error);
}

.pdf-drop {
  border: 1.5px dashed var(--border);
  border-radius: 10px;
  padding: 28px 16px;
  text-align: center;
  color: var(--text-muted);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.pdf-drop:hover {
  border-color: var(--accent);
  color: var(--accent);
}

/* 按钮组统一风格 */
.btn-row {
  display: flex;
  gap: 10px;
  align-items: center;
  flex-wrap: wrap;
}

.btn-primary-lg {
  background: var(--accent);
  color: var(--accent-fg);
  border-color: var(--accent);
  padding: 10px 22px;
  font-size: 14px;
  font-weight: 500;
}

.btn-primary-lg:hover {
  background: var(--accent-hover);
}

.btn-primary-lg:disabled {
  background: var(--border);
  border-color: var(--border);
  color: var(--text-muted);
}

/* 单文件选择后路径展示 */
.pdf-path {
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--text);
  padding: 10px 14px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
}

.seg button {
  border: 0;
  background: var(--bg);
  color: var(--text-muted);
  padding: 8px 18px;
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
  padding: 8px 12px;
  background: var(--surface);
  border-radius: 6px;
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