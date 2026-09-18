<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readFile, writeTextFile } from '@tauri-apps/plugin-fs'
import { jsPDF } from 'jspdf'
import Sortable from 'sortablejs'
import { useSettingsStore } from '../../stores/settings'

const { t } = useI18n()
const settings = useSettingsStore()
type Tab =
  'merge' | 'split' | 'rotate' | 'delete' | 'extract' | 'extractPages' | 'pdf2img' | 'img2pdf'
const active = ref<Tab>('merge')
const busy = ref(false)

// 顶部浮动提示：出现后自动消失
const toast = ref<{ kind: 'ok' | 'err'; text: string } | null>(null)
let toastTimer: ReturnType<typeof setTimeout> | null = null
function showToast(kind: 'ok' | 'err', text: string) {
  toast.value = { kind, text }
  if (toastTimer) clearTimeout(toastTimer)
  toastTimer = setTimeout(() => {
    toast.value = null
    toastTimer = null
  }, 2600)
}
const okMsg = (text: string) => showToast('ok', text)
const errMsg = (text: string) => showToast('err', text)

async function run(fn: () => Promise<unknown>): Promise<boolean> {
  busy.value = true
  try {
    await fn()
    return true
  } catch (e: unknown) {
    errMsg(t('pdf.failed', { msg: String(e) }))
    return false
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
    errMsg(t('pdf.needTwo'))
    return
  }
  const out = await save({
    defaultPath: 'merged.pdf',
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (!out) return
  const paths = mergeFiles.value
  if (await run(() => invoke('pdf_merge', { paths, out }))) {
    okMsg(t('pdf.done', { out }))
  }
}

// ---- split ----
const splitFile = ref('')
const splitRanges = ref('')
async function pickSplit() {
  const s = await open({ multiple: false, filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (s) splitFile.value = Array.isArray(s) ? (s[0] ?? '') : s
}
async function doSplit() {
  if (!splitFile.value) return
  const dir = await open({ directory: true })
  if (!dir) return
  const path = splitFile.value
  const ranges = splitRanges.value.trim()
  const call = ranges
    ? invoke('pdf_split_ranges', { path, ranges, outDir: dir })
    : invoke('pdf_split', { path, outDir: dir })
  if (await run(() => call)) {
    okMsg(t('pdf.splitDone', { dir }))
  }
}

// ---- rotate ----
const rotFile = ref('')
const rotPages = ref('')
const rotAngle = ref(90)
async function pickRot() {
  const s = await open({ multiple: false, filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (s) rotFile.value = Array.isArray(s) ? (s[0] ?? '') : s
}
async function doRotate() {
  if (!rotFile.value) return
  const out = await save({
    defaultPath: 'rotated.pdf',
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (!out) return
  const path = rotFile.value
  const pages = rotPages.value
  const angle = rotAngle.value
  if (await run(() => invoke('pdf_rotate', { path, pages, angle, out }))) {
    okMsg(t('pdf.done', { out }))
  }
}

// ---- delete ----
const delFile = ref('')
const delPages = ref('')
async function pickDel() {
  const s = await open({ multiple: false, filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (s) delFile.value = Array.isArray(s) ? (s[0] ?? '') : s
}
async function doDelete() {
  if (!delFile.value || !delPages.value.trim()) {
    errMsg(t('pdf.needPages'))
    return
  }
  const out = await save({
    defaultPath: 'deleted.pdf',
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (!out) return
  const path = delFile.value
  const pages = delPages.value
  if (await run(() => invoke('pdf_delete', { path, pages, out }))) {
    okMsg(t('pdf.done', { out }))
  }
}

// ---- extract text ----
const extractFile = ref('')
const extractText = ref('')
async function pickExtract() {
  const s = await open({ multiple: false, filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (s) extractFile.value = Array.isArray(s) ? (s[0] ?? '') : s
}
async function doExtract() {
  if (!extractFile.value) return
  busy.value = true
  try {
    const text = await invoke<string>('pdf_extract_text', { path: extractFile.value })
    extractText.value = text
    okMsg(t('pdf.extracted', { n: text.length }))
  } catch (e: unknown) {
    errMsg(t('pdf.failed', { msg: String(e) }))
  } finally {
    busy.value = false
  }
}
async function copyExtract() {
  try {
    await navigator.clipboard.writeText(extractText.value)
    okMsg(t('pdf.copied'))
  } catch {
    /* ignore */
  }
}
async function saveExtractTxt() {
  if (!extractText.value) return
  const out = await save({
    defaultPath: 'extracted.txt',
    filters: [{ name: 'TXT', extensions: ['txt'] }],
  })
  if (!out) return
  if (await run(() => writeTextFile(out, extractText.value))) {
    okMsg(t('pdf.done', { out }))
  }
}

// ---- extract pages ----
const extPagesFile = ref('')
const extPages = ref('')
async function pickExtPages() {
  const s = await open({ multiple: false, filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (s) extPagesFile.value = Array.isArray(s) ? (s[0] ?? '') : s
}
async function doExtractPages() {
  if (!extPagesFile.value || !extPages.value.trim()) {
    errMsg(t('pdf.needPagesKeep'))
    return
  }
  const out = await save({
    defaultPath: 'extracted.pdf',
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (!out) return
  const path = extPagesFile.value
  const pages = extPages.value
  if (await run(() => invoke('pdf_extract_pages', { path, pages, out }))) {
    okMsg(t('pdf.done', { out }))
  }
}

// ---- pdf -> images ----
const pdf2imgFile = ref('')
const pdf2imgPages = ref('')
const pdf2imgDpi = ref(150)
const pdf2imgFormat = ref<'png' | 'jpg'>('png')
async function pickPdf2Img() {
  const s = await open({ multiple: false, filters: [{ name: 'PDF', extensions: ['pdf'] }] })
  if (s) pdf2imgFile.value = Array.isArray(s) ? (s[0] ?? '') : s
}
async function doPdf2Img() {
  if (!pdf2imgFile.value) return
  const dir = await open({ directory: true })
  if (!dir) return
  const path = pdf2imgFile.value
  const pages = pdf2imgPages.value.trim()
  const dpi = pdf2imgDpi.value
  const format = pdf2imgFormat.value
  if (
    await run(() => invoke<string[]>('pdf_to_images', { path, pages, dpi, format, outDir: dir }))
  ) {
    okMsg(t('pdf.pdf2imgDone', { dir }))
  }
}

function base(p: string) {
  return p.split(/[/\\]/).pop() || p
}

// ---- 列表操作：Sortable 拖拽 + ✕ 移除 + 清空 ----
function removeAt(idx: number, kind: 'merge' | 'split' | 'rotate' | 'delete') {
  if (kind === 'merge') mergeFiles.value = mergeFiles.value.filter((_, i) => i !== idx)
  else if (kind === 'split') splitFile.value = ''
  else if (kind === 'rotate') rotFile.value = ''
  else if (kind === 'delete') delFile.value = ''
}
function clearMerge() {
  mergeFiles.value = []
}
function moveMergeTop() {
  if (mergeFiles.value.length < 2) return
  const arr = [...mergeFiles.value]
  arr.unshift(arr.pop()!)
  mergeFiles.value = arr
}
function moveMergeBottom() {
  if (mergeFiles.value.length < 2) return
  const arr = [...mergeFiles.value]
  arr.push(arr.shift()!)
  mergeFiles.value = arr
}
function clearImgs() {
  imgFiles.value = []
}
function moveImgsTop() {
  if (imgFiles.value.length < 2) return
  const arr = [...imgFiles.value]
  arr.unshift(arr.pop()!)
  imgFiles.value = arr
}
function moveImgsBottom() {
  if (imgFiles.value.length < 2) return
  const arr = [...imgFiles.value]
  arr.push(arr.shift()!)
  imgFiles.value = arr
}
function removeImg(idx: number) {
  imgFiles.value = imgFiles.value.filter((_, i) => i !== idx)
}

// Sortable 拖拽排序（merge / img 列表）
let mergeSortable: Sortable | null = null
let imgSortable: Sortable | null = null
function setMergeOrder(newOrder: string[]) {
  // newOrder 是拖拽后按 DOM 顺序收集的 basename，重排 mergeFiles 与之对齐
  const map = new Map(mergeFiles.value.map((p) => [base(p), p]))
  mergeFiles.value = newOrder.map((n) => map.get(n)!).filter(Boolean)
}
function setImgOrder(newOrder: string[]) {
  const map = new Map(imgFiles.value.map((p) => [base(p), p]))
  imgFiles.value = newOrder.map((n) => map.get(n)!).filter(Boolean)
}
function createSortable(el: HTMLElement, apply: (order: string[]) => void): Sortable {
  return Sortable.create(el, {
    animation: 160,
    handle: '.drag-handle',
    // Tauri 的 onDragDropEvent 会禁用 webview 内的原生 HTML5 拖拽，
    // 开启 forceFallback 让 SortableJS 用鼠标/指针事件模拟拖拽，绕开该限制
    forceFallback: true,
    fallbackTolerance: 3,
    ghostClass: 'sortable-ghost',
    chosenClass: 'sortable-chosen',
    dragClass: 'sortable-drag',
    onEnd: () => {
      const order = Array.from(el.querySelectorAll<HTMLElement>('[data-name]')).map(
        (item) => item.dataset.name!,
      )
      apply(order)
    },
  })
}
// 列表元素由 v-if 条件渲染，需在元素出现后（重新）绑定 Sortable
function setupMergeSortable() {
  mergeSortable?.destroy()
  mergeSortable = null
  const el = document.getElementById('merge-sortable')
  if (el && active.value === 'merge' && mergeFiles.value.length) {
    mergeSortable = createSortable(el, setMergeOrder)
  }
}
function setupImgSortable() {
  imgSortable?.destroy()
  imgSortable = null
  const el = document.getElementById('img-sortable')
  if (el && active.value === 'img2pdf' && imgFiles.value.length) {
    imgSortable = createSortable(el, setImgOrder)
  }
}
watch(
  () => [active.value, mergeFiles.value.length, imgFiles.value.length] as const,
  () => {
    nextTick(() => {
      setupMergeSortable()
      setupImgSortable()
    })
  },
)
onMounted(() => {
  nextTick(() => {
    setupMergeSortable()
    setupImgSortable()
  })
})
onUnmounted(() => {
  mergeSortable?.destroy()
  imgSortable?.destroy()
})

function onMergeDropFromWindow(_e: DragEvent) {
  /* 占位：拖拽走 App 全局事件 */
}
function pickFromDrop(_kind: string) {
  /* 同上 */
}
function addImgsFromDrop(_e: DragEvent) {
  /* 同上 */
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
          ext === 'jpg' || ext === 'jpeg'
            ? 'jpeg'
            : ext === 'webp'
              ? 'webp'
              : ext === 'bmp'
                ? 'bmp'
                : 'png'
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
    errMsg(t('pdf.needImg'))
    return
  }
  const out = await save({
    defaultPath: 'images.pdf',
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })
  if (!out) return
  const files = imgFiles.value
  const ok = await run(async () => {
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
  if (ok) okMsg(t('pdf.done', { out }))
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
    } else if (active.value === 'extract') {
      extractFile.value = p
    } else if (active.value === 'extractPages') {
      extPagesFile.value = p
    } else if (active.value === 'pdf2img') {
      pdf2imgFile.value = p
    }
  })
})
onUnmounted(() => unDrop?.())
</script>

<template>
  <div class="pdf-tool">
    <Transition name="toast">
      <div v-if="toast" class="pdf-toast" :class="toast.kind">
        <span class="toast-icon">{{ toast.kind === 'ok' ? '✓' : '⚠' }}</span>
        <span class="toast-text">{{ toast.text }}</span>
      </div>
    </Transition>
    <div class="pdf-tabs">
      <button
        v-for="tab in [
          'merge',
          'split',
          'rotate',
          'delete',
          'extract',
          'extractPages',
          'pdf2img',
          'img2pdf',
        ] as const"
        :key="tab"
        :class="{ active: active === tab }"
        @click="active = tab"
      >
        {{ t(`pdf.${tab}`) }}
      </button>
    </div>

    <div class="pdf-body">
      <!-- merge -->
      <div v-if="active === 'merge'" class="pdf-panel">
        <div
          class="pdf-drop"
          @click="pickMerge"
          @dragover.prevent
          @drop.prevent="onMergeDropFromWindow"
        >
          {{ t('pdf.dropOrPick') }}
        </div>
        <div v-if="mergeFiles.length">
          <div id="merge-sortable" class="pdf-list">
            <div class="pdf-list-head">
              <span class="count">{{ t('pdf.selected', { n: mergeFiles.length }) }}</span>
              <div class="list-tools">
                <button @click="moveMergeTop" :disabled="mergeFiles.length < 2">
                  {{ t('pdf.toTop') }}
                </button>
                <button @click="moveMergeBottom" :disabled="mergeFiles.length < 2">
                  {{ t('pdf.toBottom') }}
                </button>
                <button @click="clearMerge" :disabled="!mergeFiles.length">
                  {{ t('pdf.clear') }}
                </button>
              </div>
            </div>
            <div v-for="f in mergeFiles" :key="f" class="pdf-row" :data-name="base(f)">
              <span class="drag-handle" title="拖动排序">⋮⋮</span>
              <span class="order">{{ mergeFiles.indexOf(f) + 1 }}</span>
              <span class="name" :title="f">{{ base(f) }}</span>
              <div class="row-tools">
                <button
                  class="remove"
                  @click="removeAt(mergeFiles.indexOf(f), 'merge')"
                  title="移除"
                >
                  ✕
                </button>
              </div>
            </div>
          </div>
          <div class="btn-row">
            <button
              class="btn btn-primary-lg"
              :disabled="busy || mergeFiles.length < 2"
              @click="doMerge"
            >
              {{ t('pdf.doMerge') }}
            </button>
          </div>
        </div>
      </div>

      <!-- split -->
      <div v-else-if="active === 'split'" class="pdf-panel">
        <div
          class="pdf-drop"
          @click="pickSplit"
          @dragover.prevent
          @drop.prevent="pickFromDrop('split')"
        >
          {{ t('pdf.dropOrPick') }}
        </div>
        <div v-if="splitFile" class="pdf-path" :title="splitFile">{{ base(splitFile) }}</div>
        <div class="pdf-field">
          <label>{{ t('pdf.splitRanges') }}</label>
          <input v-model="splitRanges" class="pdf-input" placeholder="1-3,4-8" />
        </div>
        <div class="btn-row">
          <button class="btn btn-primary-lg" :disabled="busy || !splitFile" @click="doSplit">
            {{ t('pdf.doSplit') }}
          </button>
        </div>
      </div>

      <!-- rotate -->
      <div v-else-if="active === 'rotate'" class="pdf-panel">
        <div
          class="pdf-drop"
          @click="pickRot"
          @dragover.prevent
          @drop.prevent="pickFromDrop('rotate')"
        >
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
            <button
              v-for="a in [90, 180, 270]"
              :key="a"
              :class="{ active: rotAngle === a }"
              @click="rotAngle = a"
            >
              {{ a }}°
            </button>
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
        <div
          class="pdf-drop"
          @click="pickDel"
          @dragover.prevent
          @drop.prevent="pickFromDrop('delete')"
        >
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

      <!-- extract text -->
      <div v-else-if="active === 'extract'" class="pdf-panel">
        <div
          class="pdf-drop"
          @click="pickExtract"
          @dragover.prevent
          @drop.prevent="pickFromDrop('extract')"
        >
          {{ t('pdf.dropOrPick') }}
        </div>
        <div v-if="extractFile" class="pdf-path" :title="extractFile">{{ base(extractFile) }}</div>
        <div class="btn-row">
          <button class="btn btn-primary-lg" :disabled="busy || !extractFile" @click="doExtract">
            {{ t('pdf.doExtract') }}
          </button>
        </div>
        <textarea
          v-model="extractText"
          class="pdf-textarea"
          :placeholder="t('pdf.extractPh')"
          spellcheck="false"
        ></textarea>
        <div class="btn-row">
          <button class="btn" :disabled="!extractText" @click="copyExtract">
            {{ t('pdf.copy') }}
          </button>
          <button class="btn" :disabled="!extractText" @click="saveExtractTxt">
            {{ t('pdf.saveTxt') }}
          </button>
        </div>
      </div>

      <!-- extract pages -->
      <div v-else-if="active === 'extractPages'" class="pdf-panel">
        <div
          class="pdf-drop"
          @click="pickExtPages"
          @dragover.prevent
          @drop.prevent="pickFromDrop('extractPages')"
        >
          {{ t('pdf.dropOrPick') }}
        </div>
        <div v-if="extPagesFile" class="pdf-path" :title="extPagesFile">
          {{ base(extPagesFile) }}
        </div>
        <div class="pdf-field">
          <label>{{ t('pdf.pagesKeep') }}</label>
          <input v-model="extPages" class="pdf-input" placeholder="1,3,5" />
        </div>
        <div class="btn-row">
          <button
            class="btn btn-primary-lg"
            :disabled="busy || !extPagesFile"
            @click="doExtractPages"
          >
            {{ t('pdf.doExtractPages') }}
          </button>
        </div>
      </div>

      <!-- pdf2img -->
      <div v-else-if="active === 'pdf2img'" class="pdf-panel">
        <div
          class="pdf-drop"
          @click="pickPdf2Img"
          @dragover.prevent
          @drop.prevent="pickFromDrop('pdf2img')"
        >
          {{ t('pdf.dropOrPick') }}
        </div>
        <div v-if="pdf2imgFile" class="pdf-path" :title="pdf2imgFile">
          {{ base(pdf2imgFile) }}
        </div>
        <div class="pdf-field">
          <label>{{ t('pdf.pdf2imgPages') }}</label>
          <input v-model="pdf2imgPages" class="pdf-input" :placeholder="t('pdf.pdf2imgPagesPh')" />
        </div>
        <div class="pdf-field">
          <label>{{ t('pdf.pdf2imgFormat') }}</label>
          <select v-model="pdf2imgFormat" class="pdf-input">
            <option value="png">PNG</option>
            <option value="jpg">JPEG</option>
          </select>
        </div>
        <div class="pdf-field">
          <label>{{ t('pdf.pdf2imgDpi') }}</label>
          <select v-model.number="pdf2imgDpi" class="pdf-input">
            <option :value="96">96</option>
            <option :value="150">150</option>
            <option :value="200">200</option>
            <option :value="300">300</option>
          </select>
        </div>
        <div class="btn-row">
          <button class="btn btn-primary-lg" :disabled="busy || !pdf2imgFile" @click="doPdf2Img">
            {{ t('pdf.doPdf2Img') }}
          </button>
        </div>
      </div>

      <!-- img2pdf -->
      <div v-else-if="active === 'img2pdf'" class="pdf-panel">
        <div class="pdf-drop" @click="pickImg" @dragover.prevent @drop.prevent="addImgsFromDrop">
          {{ t('pdf.dropOrPickImg') }}
        </div>
        <div v-if="imgFiles.length">
          <div id="img-sortable" class="pdf-list">
            <div class="pdf-list-head">
              <span class="count">{{ t('pdf.selected', { n: imgFiles.length }) }}</span>
              <div class="list-tools">
                <button @click="moveImgsTop" :disabled="imgFiles.length < 2">
                  {{ t('pdf.toTop') }}
                </button>
                <button @click="moveImgsBottom" :disabled="imgFiles.length < 2">
                  {{ t('pdf.toBottom') }}
                </button>
                <button @click="clearImgs">{{ t('pdf.clear') }}</button>
              </div>
            </div>
            <div v-for="f in imgFiles" :key="f" class="pdf-row" :data-name="base(f)">
              <span class="drag-handle" title="拖动排序">⋮⋮</span>
              <span class="order">{{ imgFiles.indexOf(f) + 1 }}</span>
              <span class="name" :title="f">{{ base(f) }}</span>
              <div class="row-tools">
                <button class="remove" @click="removeImg(imgFiles.indexOf(f))" title="移除">
                  ✕
                </button>
              </div>
            </div>
          </div>
          <div class="btn-row">
            <button
              class="btn btn-primary-lg"
              :disabled="busy || !imgFiles.length"
              @click="doImg2Pdf"
            >
              {{ t('pdf.doImg2Pdf') }}
            </button>
          </div>
        </div>
      </div>
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

/* === Tabs === */
.pdf-tabs {
  display: flex;
  gap: 4px;
  padding: 10px 20px;
  border-bottom: 1px solid var(--border);
  background: var(--surface);
  flex-shrink: 0;
}

.pdf-tabs button {
  border: 0;
  background: transparent;
  color: var(--text-muted);
  padding: 7px 16px;
  font-size: 13px;
  font-weight: 500;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.pdf-tabs button:hover {
  background: var(--surface-hover);
  color: var(--text);
}

.pdf-tabs button.active {
  background: var(--accent);
  color: var(--accent-fg);
}

/* === Body === */
.pdf-body {
  flex: 1;
  overflow-y: auto;
  padding: 24px 28px;
}

.pdf-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 720px;
}

/* === Drop zone === */
.pdf-drop {
  border: 1.5px dashed var(--border);
  border-radius: 10px;
  padding: 32px 20px;
  text-align: center;
  color: var(--text-muted);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s ease;
  background: var(--surface);
}

.pdf-drop:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--bg);
}

/* === File list === */
.pdf-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 8px;
  background: var(--bg);
}

.pdf-list-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px 8px;
  border-bottom: 1px solid var(--border);
  margin-bottom: 4px;
  font-size: 12px;
}

.pdf-list-head .count {
  color: var(--text-muted);
  font-weight: 500;
}

.pdf-list-head .list-tools {
  display: flex;
  gap: 4px;
}

.pdf-list-head .list-tools button {
  border: 0;
  background: transparent;
  color: var(--text-muted);
  font-size: 12px;
  cursor: pointer;
  padding: 3px 10px;
  border-radius: 5px;
  transition: all 0.12s ease;
}

.pdf-list-head .list-tools button:hover:not(:disabled) {
  background: var(--surface-hover);
  color: var(--text);
}

.pdf-list-head .list-tools button:disabled {
  opacity: 0.35;
  cursor: default;
}

.pdf-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 7px;
  transition: border-color 0.12s ease;
}

.pdf-row:hover {
  border-color: var(--accent);
}

.pdf-row .drag-handle {
  flex-shrink: 0;
  cursor: grab;
  color: var(--text-muted);
  font-size: 14px;
  line-height: 1;
  padding: 0 4px;
  user-select: none;
  letter-spacing: -2px;
}

.pdf-row .drag-handle:hover {
  color: var(--accent);
}

.pdf-row .drag-handle:active {
  cursor: grabbing;
}

.pdf-row .order {
  flex-shrink: 0;
  min-width: 24px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  font-family: var(--font-mono);
  color: var(--text-muted);
  background: var(--bg);
  border-radius: 4px;
  padding: 0 6px;
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
  font-size: 13px;
  cursor: pointer;
  border-radius: 5px;
  transition: all 0.12s ease;
}

.pdf-row .row-tools button:hover:not(:disabled) {
  background: var(--bg);
  color: var(--text);
}

.pdf-row .row-tools button.remove:hover {
  background: var(--error);
  color: white;
}

/* Sortable 拖拽状态 */
:deep(.sortable-ghost) {
  opacity: 0.35;
  background: var(--bg);
}

:deep(.sortable-chosen) {
  border-color: var(--accent) !important;
}

:deep(.sortable-drag) {
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  border-color: var(--accent) !important;
  cursor: grabbing;
}

/* === Path (单文件) === */
.pdf-path {
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--text);
  padding: 10px 14px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 7px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* === Form fields === */
.pdf-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.pdf-field label {
  font-size: 12px;
  color: var(--text-muted);
  font-weight: 500;
}

.pdf-input {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 13px;
  transition: border-color 0.12s ease;
}

.pdf-input:focus {
  outline: none;
  border-color: var(--accent);
}

.pdf-textarea {
  flex: 1;
  min-height: 160px;
  resize: vertical;
  padding: 12px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 13px;
  line-height: 1.5;
}

.pdf-textarea:focus {
  outline: none;
  border-color: var(--accent);
}

/* === Segment === */
.seg {
  display: inline-flex;
  border: 1px solid var(--border);
  border-radius: 7px;
  overflow: hidden;
  background: var(--surface);
}

.seg button {
  border: 0;
  background: transparent;
  color: var(--text-muted);
  padding: 8px 18px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.12s ease;
}

.seg button + button {
  border-left: 1px solid var(--border);
}

.seg button:hover {
  background: var(--bg);
  color: var(--text);
}

.seg button.active {
  background: var(--accent);
  color: var(--accent-fg);
}

/* === Buttons === */
.btn-row {
  display: flex;
  gap: 10px;
  align-items: center;
  flex-wrap: wrap;
  margin-top: 4px;
}

.btn-primary-lg {
  background: var(--accent);
  color: var(--accent-fg);
  border: 0;
  padding: 10px 28px;
  font-size: 14px;
  font-weight: 500;
  border-radius: 7px;
  cursor: pointer;
  transition: background 0.12s ease;
}

.btn-primary-lg:hover:not(:disabled) {
  background: var(--accent-hover);
}

.btn-primary-lg:disabled {
  background: var(--border);
  color: var(--text-muted);
  cursor: default;
}

/* === 顶部浮动提示 toast === */
.pdf-toast {
  position: fixed;
  top: 18px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 1000;
  display: flex;
  align-items: center;
  gap: 8px;
  max-width: 80vw;
  padding: 10px 18px;
  border-radius: 8px;
  font-size: 13px;
  font-family: var(--font-mono);
  background: var(--surface);
  border: 1px solid var(--border);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  word-break: break-all;
}

.pdf-toast.ok {
  color: var(--accent);
  border-color: var(--accent);
}

.pdf-toast.err {
  color: var(--error);
  border-color: var(--error);
}

.pdf-toast .toast-icon {
  flex-shrink: 0;
}

.toast-enter-active,
.toast-leave-active {
  transition:
    opacity 0.25s ease,
    transform 0.25s ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translate(-50%, -12px);
}

.toast-enter-to,
.toast-leave-from {
  opacity: 1;
  transform: translate(-50%, 0);
}
</style>
