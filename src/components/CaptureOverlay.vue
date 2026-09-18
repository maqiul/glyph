<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { emit } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

interface CaptureResult {
  index: number
  name: string
  x: number
  y: number
  width: number
  height: number
  png_base64: string
}

interface Pt {
  x: number
  y: number
}

interface Shape {
  tool: ToolName
  color: string
  width: number
  pts?: Pt[]
  x?: number
  y?: number
  text?: string
  size?: number
  n?: number
}

type ToolName = 'rect' | 'ellipse' | 'arrow' | 'pen' | 'highlight' | 'mosaic' | 'text' | 'number'

const { t } = useI18n()
const monitorIndex = Number(new URLSearchParams(window.location.search).get('monitor') ?? '0')
const dpr = window.devicePixelRatio || 1

const bg = ref('')
let img: HTMLImageElement | null = null
const ready = ref(false)

type Phase = 'select' | 'edit'
const phase = ref<Phase>('select')
const start = ref<Pt | null>(null)
const sel = ref<{ x: number; y: number; w: number; h: number } | null>(null)

// 标注状态
const annoRef = ref<HTMLCanvasElement | null>(null)
const shapes = ref<Shape[]>([])
const tempShape = ref<Shape | null>(null)
const drawing = ref(false)
const tool = ref<ToolName>('rect')
const color = ref('#ff3b30')
const width = ref(4)
const textEditing = ref(false)
const textInput = ref('')
const textPos = ref<Pt>({ x: 0, y: 0 })
const textRef = ref<HTMLInputElement | null>(null)
const numberSeq = ref(0)

let crop: { sx: number; sy: number; sw: number; sh: number } | null = null
let baseCanvas: HTMLCanvasElement | null = null
const scratch: HTMLCanvasElement = document.createElement('canvas')

const TOOLS: { name: ToolName; icon: string; key: string }[] = [
  { name: 'rect', icon: '▭', key: 'annotateRect' },
  { name: 'ellipse', icon: '◯', key: 'annotateEllipse' },
  { name: 'arrow', icon: '↗', key: 'annotateArrow' },
  { name: 'pen', icon: '✎', key: 'annotatePen' },
  { name: 'highlight', icon: '🖍', key: 'annotateHighlight' },
  { name: 'mosaic', icon: '▦', key: 'annotateMosaic' },
  { name: 'text', icon: 'T', key: 'annotateText' },
  { name: 'number', icon: '①', key: 'annotateNumber' },
]
const COLORS = ['#ff3b30', '#ffcc00', '#34c759', '#0a84ff', '#ff2d92', '#111111', '#ffffff']
const WIDTHS = [2, 4, 8]

const selStyle = computed(() => {
  const s = sel.value
  if (!s) return {}
  return { left: `${s.x}px`, top: `${s.y}px`, width: `${s.w}px`, height: `${s.h}px` }
})

const toolbarStyle = computed(() => {
  const s = sel.value
  if (!s) return {}
  const below = s.y + s.h + 48 < window.innerHeight
  const top = below ? s.y + s.h + 8 : Math.max(8, s.y - 48)
  const left = Math.max(8, Math.min(s.x, window.innerWidth - 680))
  return { top: `${top}px`, left: `${left}px` }
})

const textStyle = computed(() => {
  const s = sel.value
  if (!s) return {}
  return {
    left: `${s.x + textPos.value.x}px`,
    top: `${s.y + textPos.value.y}px`,
    color: color.value,
    fontSize: `${textSize()}px`,
  }
})

function textSize(): number {
  return Math.max(16, width.value * 4)
}

async function load() {
  try {
    // 读主窗预截屏的缓存，而非自己再截（避免 overlay 白底被截入 → 白屏）
    const caps = await invoke<CaptureResult[]>('get_cached_capture')
    const c = caps.find((x) => x.index === monitorIndex) ?? caps[0]
    if (!c) return
    bg.value = `data:image/png;base64,${c.png_base64}`
    img = new Image()
    img.onload = () => {
      ready.value = true
    }
    img.src = bg.value
  } catch (e) {
    console.error('overlay capture failed:', e)
  }
}

function normRect(a: Pt, b: Pt) {
  return {
    x: Math.min(a.x, b.x),
    y: Math.min(a.y, b.y),
    w: Math.abs(a.x - b.x),
    h: Math.abs(a.y - b.y),
  }
}

function clamp(v: number, lo: number, hi: number) {
  return Math.max(lo, Math.min(hi, v))
}

function relPoint(e: MouseEvent): Pt {
  const s = sel.value!
  return { x: clamp(e.clientX - s.x, 0, s.w), y: clamp(e.clientY - s.y, 0, s.h) }
}

function normPts(shp: Shape) {
  const pts = shp.pts!
  const a = pts[0]!
  const b = pts[pts.length - 1]!
  return {
    x: Math.min(a.x, b.x),
    y: Math.min(a.y, b.y),
    w: Math.abs(a.x - b.x),
    h: Math.abs(a.y - b.y),
  }
}

function makeShape(p: Pt): Shape {
  if (tool.value === 'pen' || tool.value === 'highlight')
    return { tool: tool.value, color: color.value, width: width.value, pts: [p] }
  return { tool: tool.value, color: color.value, width: width.value, pts: [p, { ...p }] }
}

// —— 选区 -> 进入标注编辑态 ——
function enterEdit() {
  const s = sel.value
  if (!s || !img) return
  crop = {
    sx: Math.round(s.x * dpr),
    sy: Math.round(s.y * dpr),
    sw: Math.max(1, Math.round(s.w * dpr)),
    sh: Math.max(1, Math.round(s.h * dpr)),
  }
  baseCanvas = document.createElement('canvas')
  baseCanvas.width = crop.sw
  baseCanvas.height = crop.sh
  baseCanvas
    .getContext('2d')
    ?.drawImage(img, crop.sx, crop.sy, crop.sw, crop.sh, 0, 0, crop.sw, crop.sh)
  shapes.value = []
  tempShape.value = null
  numberSeq.value = 0
  phase.value = 'edit'
  nextTick(() => {
    const cv = annoRef.value
    if (cv) {
      cv.width = crop!.sw
      cv.height = crop!.sh
    }
    redraw()
  })
}

function drawArrow(ctx: CanvasRenderingContext2D, a: Pt, b: Pt, w: number) {
  ctx.beginPath()
  ctx.moveTo(a.x, a.y)
  ctx.lineTo(b.x, b.y)
  ctx.stroke()
  const ang = Math.atan2(b.y - a.y, b.x - a.x)
  const len = Math.max(10, w * 4)
  ctx.beginPath()
  ctx.moveTo(b.x, b.y)
  ctx.lineTo(b.x - len * Math.cos(ang - Math.PI / 6), b.y - len * Math.sin(ang - Math.PI / 6))
  ctx.moveTo(b.x, b.y)
  ctx.lineTo(b.x - len * Math.cos(ang + Math.PI / 6), b.y - len * Math.sin(ang + Math.PI / 6))
  ctx.stroke()
}

function drawMosaic(
  ctx: CanvasRenderingContext2D,
  r: { x: number; y: number; w: number; h: number },
) {
  if (!baseCanvas) return
  const block = 12
  const sw2 = Math.max(1, Math.round(r.w / block))
  const sh2 = Math.max(1, Math.round(r.h / block))
  scratch.width = sw2
  scratch.height = sh2
  const sctx = scratch.getContext('2d')
  if (!sctx) return
  sctx.imageSmoothingEnabled = true
  sctx.clearRect(0, 0, sw2, sh2)
  sctx.drawImage(baseCanvas, r.x * dpr, r.y * dpr, r.w * dpr, r.h * dpr, 0, 0, sw2, sh2)
  ctx.imageSmoothingEnabled = false
  ctx.drawImage(scratch, r.x, r.y, r.w, r.h)
  ctx.imageSmoothingEnabled = true
}

function drawShape(ctx: CanvasRenderingContext2D, shp: Shape) {
  ctx.save()
  ctx.strokeStyle = shp.color
  ctx.fillStyle = shp.color
  ctx.lineWidth = shp.width
  ctx.lineCap = 'round'
  ctx.lineJoin = 'round'
  if (shp.tool === 'rect') {
    const r = normPts(shp)
    ctx.strokeRect(r.x, r.y, r.w, r.h)
  } else if (shp.tool === 'ellipse') {
    const r = normPts(shp)
    ctx.beginPath()
    ctx.ellipse(r.x + r.w / 2, r.y + r.h / 2, r.w / 2, r.h / 2, 0, 0, Math.PI * 2)
    ctx.stroke()
  } else if (shp.tool === 'arrow') {
    const pts = shp.pts!
    drawArrow(ctx, pts[0]!, pts[pts.length - 1]!, shp.width)
  } else if (shp.tool === 'pen') {
    const pts = shp.pts!
    ctx.beginPath()
    pts.forEach((p, i) => (i ? ctx.lineTo(p.x, p.y) : ctx.moveTo(p.x, p.y)))
    ctx.stroke()
  } else if (shp.tool === 'highlight') {
    const pts = shp.pts!
    ctx.globalAlpha = 0.35
    ctx.lineWidth = (shp.width || 4) * 3
    ctx.beginPath()
    pts.forEach((p, i) => (i ? ctx.lineTo(p.x, p.y) : ctx.moveTo(p.x, p.y)))
    ctx.stroke()
  } else if (shp.tool === 'number') {
    const r = Math.max(11, shp.width * 2 + 6)
    ctx.beginPath()
    ctx.arc(shp.x ?? 0, shp.y ?? 0, r, 0, Math.PI * 2)
    ctx.fill()
    ctx.fillStyle = '#ffffff'
    ctx.font = `bold ${Math.round(r * 1.1)}px system-ui, "Segoe UI", sans-serif`
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    ctx.fillText(String(shp.n ?? 1), shp.x ?? 0, shp.y ?? 0)
  } else if (shp.tool === 'mosaic') {
    const r = normPts(shp)
    if (r.w > 1 && r.h > 1) drawMosaic(ctx, r)
  } else if (shp.tool === 'text') {
    ctx.font = `${shp.size ?? 16}px system-ui, "Segoe UI", "Microsoft YaHei", sans-serif`
    ctx.textBaseline = 'top'
    ctx.fillText(shp.text ?? '', shp.x ?? 0, shp.y ?? 0)
  }
  ctx.restore()
}

function redraw() {
  const cv = annoRef.value
  const s = sel.value
  if (!cv || !s) return
  const ctx = cv.getContext('2d')
  if (!ctx) return
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  ctx.clearRect(0, 0, s.w, s.h)
  for (const shp of shapes.value) drawShape(ctx, shp)
  if (tempShape.value) drawShape(ctx, tempShape.value)
}

function onDown(e: MouseEvent) {
  if (phase.value === 'select') {
    start.value = { x: e.clientX, y: e.clientY }
    sel.value = null
    return
  }
  // edit 阶段
  if (textEditing.value) {
    commitText()
    return
  }
  if (tool.value === 'text') {
    textPos.value = relPoint(e)
    textInput.value = ''
    textEditing.value = true
    nextTick(() => textRef.value?.focus())
    return
  }
  if (tool.value === 'number') {
    const p = relPoint(e)
    numberSeq.value += 1
    shapes.value.push({
      tool: 'number',
      color: color.value,
      width: width.value,
      x: p.x,
      y: p.y,
      n: numberSeq.value,
    })
    redraw()
    return
  }
  drawing.value = true
  tempShape.value = makeShape(relPoint(e))
}

function onMove(e: MouseEvent) {
  if (phase.value === 'select') {
    if (!start.value) return
    sel.value = normRect(start.value, { x: e.clientX, y: e.clientY })
  } else if (drawing.value && tempShape.value) {
    const p = relPoint(e)
    if (tempShape.value.tool === 'pen' || tempShape.value.tool === 'highlight')
      tempShape.value.pts!.push(p)
    else tempShape.value.pts![1] = p
    redraw()
  }
}

function onUp() {
  if (phase.value === 'select') {
    const s = sel.value
    if (!s || s.w < 3 || s.h < 3 || !ready.value || !img) {
      cancel()
      return
    }
    enterEdit()
    return
  }
  if (drawing.value) {
    drawing.value = false
    const shp = tempShape.value
    tempShape.value = null
    if (shp) {
      const freeform = shp.tool === 'pen' || shp.tool === 'highlight'
      const r = freeform ? bboxOf(shp.pts!) : normPts(shp)
      if (freeform || r.w > 2 || r.h > 2) shapes.value.push(shp)
    }
    redraw()
  }
}

function bboxOf(pts: Pt[]) {
  const xs = pts.map((p) => p.x)
  const ys = pts.map((p) => p.y)
  return {
    x: Math.min(...xs),
    y: Math.min(...ys),
    w: Math.max(...xs) - Math.min(...xs),
    h: Math.max(...ys) - Math.min(...ys),
  }
}

function commitText() {
  if (textInput.value.trim()) {
    shapes.value.push({
      tool: 'text',
      color: color.value,
      width: width.value,
      x: textPos.value.x,
      y: textPos.value.y,
      text: textInput.value,
      size: textSize(),
    })
  }
  textEditing.value = false
  textInput.value = ''
  redraw()
}

function undo() {
  shapes.value.pop()
  redraw()
}

function clearAll() {
  shapes.value = []
  numberSeq.value = 0
  redraw()
}

function compositeDataUrl(): string | null {
  if (!img || !crop) return null
  const final = document.createElement('canvas')
  final.width = crop.sw
  final.height = crop.sh
  const fctx = final.getContext('2d')
  if (!fctx) return null
  fctx.drawImage(img, crop.sx, crop.sy, crop.sw, crop.sh, 0, 0, crop.sw, crop.sh)
  const cv = annoRef.value
  if (cv) fctx.drawImage(cv, 0, 0)
  return final.toDataURL('image/png')
}

const copied = ref(false)

async function copyAndClose() {
  if (textEditing.value) commitText()
  const dataUrl = compositeDataUrl()
  if (!dataUrl) return
  try {
    await invoke('copy_image_to_clipboard', { imageBase64: dataUrl.split(',')[1] ?? '' })
    copied.value = true
    setTimeout(() => getCurrentWindow().close(), 450)
  } catch (e) {
    console.error('copy to clipboard failed:', e)
    await getCurrentWindow().close()
  }
}

async function confirm() {
  if (textEditing.value) commitText()
  if (!img || !crop) return
  try {
    const dataUrl = compositeDataUrl()
    if (!dataUrl) throw new Error('canvas 2d unavailable')
    const png_base64 = dataUrl.split(',')[1] ?? ''
    await emit('capture-region', {
      png_base64,
      width: crop.sw,
      height: crop.sh,
      monitor: monitorIndex,
    })
  } catch (e) {
    console.error('annotate export failed:', e)
    await emit('capture-cancelled')
  } finally {
    await getCurrentWindow().close()
  }
}

function backToSelect() {
  phase.value = 'select'
  sel.value = null
  start.value = null
  shapes.value = []
  tempShape.value = null
  numberSeq.value = 0
  textEditing.value = false
  crop = null
  baseCanvas = null
}

async function cancel() {
  try {
    await emit('capture-cancelled')
  } finally {
    await getCurrentWindow().close()
  }
}

function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    if (phase.value === 'edit') backToSelect()
    else cancel()
  } else if (e.key === 'Enter' && phase.value === 'edit' && !textEditing.value) {
    confirm()
  } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z' && phase.value === 'edit') {
    e.preventDefault()
    undo()
  }
}

onMounted(() => {
  load()
  window.addEventListener('keydown', onKey)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKey)
})
</script>

<template>
  <div
    class="overlay"
    :class="{ editing: phase === 'edit' }"
    @mousedown="onDown"
    @mousemove="onMove"
    @mouseup="onUp"
  >
    <img v-if="bg" class="bg" :src="bg" alt="" draggable="false" />

    <!-- 选区阶段 -->
    <template v-if="phase === 'select'">
      <div v-if="sel" class="sel" :style="selStyle">
        <span class="dim">{{ Math.round(sel.w) }} × {{ Math.round(sel.h) }}</span>
      </div>
      <div v-if="!sel && !start" class="hint">
        {{ t('screenshot.overlayHint') }}
        <kbd>Esc</kbd> {{ t('screenshot.overlayCancel') }}
      </div>
    </template>

    <!-- 标注编辑态 -->
    <template v-if="phase === 'edit' && sel">
      <div class="sel edit" :style="selStyle">
        <canvas ref="annoRef" class="anno"></canvas>
      </div>

      <div class="toolbar" :style="toolbarStyle" @mousedown.stop @mouseup.stop>
        <div class="tb-group">
          <button
            v-for="tb in TOOLS"
            :key="tb.name"
            class="tb-btn"
            :class="{ active: tool === tb.name }"
            :title="t('screenshot.' + tb.key)"
            @click="tool = tb.name"
          >
            <span class="tb-icon">{{ tb.icon }}</span>
          </button>
        </div>
        <span class="tb-sep"></span>
        <div class="tb-group">
          <button
            v-for="c in COLORS"
            :key="c"
            class="tb-color"
            :class="{ active: color === c }"
            :style="{ background: c }"
            @click="color = c"
          ></button>
        </div>
        <span class="tb-sep"></span>
        <div class="tb-group">
          <button
            v-for="w in WIDTHS"
            :key="w"
            class="tb-btn tb-width"
            :class="{ active: width === w }"
            @click="width = w"
          >
            <span class="dot" :style="{ width: w + 3 + 'px', height: w + 3 + 'px' }"></span>
          </button>
        </div>
        <span class="tb-sep"></span>
        <div class="tb-group">
          <button class="tb-btn" :title="t('screenshot.annotateUndo')" @click="undo">↶</button>
          <button class="tb-btn" :title="t('screenshot.annotateClear')" @click="clearAll">⟲</button>
        </div>
        <span class="tb-sep"></span>
        <div class="tb-group">
          <button
            class="tb-btn tb-copy"
            :class="{ active: copied }"
            :title="t('screenshot.annotateCopy')"
            @click="copyAndClose"
          >
            {{ copied ? '✓' : '⧉' }}
          </button>
        </div>
        <span class="tb-sep"></span>
        <div class="tb-group">
          <button
            class="tb-btn tb-cancel"
            :title="t('screenshot.annotateCancel')"
            @click="backToSelect"
          >
            ✕
          </button>
          <button
            class="tb-btn tb-confirm"
            :title="t('screenshot.annotateConfirm')"
            @click="confirm"
          >
            ✓
          </button>
        </div>
      </div>

      <input
        v-if="textEditing"
        ref="textRef"
        v-model="textInput"
        class="text-input"
        :style="textStyle"
        :placeholder="t('screenshot.annotateTextPh')"
        @mousedown.stop
        @keyup.enter="commitText"
        @blur="commitText"
      />
    </template>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  overflow: hidden;
  cursor: crosshair;
  user-select: none;
  background: transparent;
}

.overlay.editing {
  cursor: default;
}

.bg {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  display: block;
  pointer-events: none;
}

.sel {
  position: absolute;
  border: 1.5px solid var(--accent);
  box-shadow: 0 0 0 9999px rgba(0, 0, 0, 0.45);
  pointer-events: none;
}

.sel.edit {
  border-color: var(--accent);
}

.anno {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  display: block;
  pointer-events: none;
}

.dim {
  position: absolute;
  top: -22px;
  left: 0;
  font-size: 12px;
  color: #fff;
  background: var(--accent);
  padding: 1px 6px;
  border-radius: 3px;
  font-family: var(--font-mono);
  white-space: nowrap;
}

.hint {
  position: absolute;
  top: 20px;
  left: 50%;
  transform: translateX(-50%);
  color: #fff;
  background: rgba(0, 0, 0, 0.6);
  padding: 8px 16px;
  border-radius: 8px;
  font-size: 13px;
  pointer-events: none;
}

.hint kbd {
  font-family: var(--font-mono);
  background: rgba(255, 255, 255, 0.2);
  padding: 1px 6px;
  border-radius: 3px;
  margin: 0 4px;
}

.toolbar {
  position: absolute;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  background: rgba(28, 28, 30, 0.96);
  border: 1px solid rgba(255, 255, 255, 0.14);
  border-radius: 10px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
  z-index: 10;
  cursor: default;
}

.tb-group {
  display: flex;
  align-items: center;
  gap: 4px;
}

.tb-sep {
  width: 1px;
  height: 20px;
  background: rgba(255, 255, 255, 0.16);
  margin: 0 2px;
}

.tb-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  border: 1px solid transparent;
  background: transparent;
  color: #eee;
  font-size: 15px;
  cursor: pointer;
  transition: background 0.12s ease;
}

.tb-btn:hover {
  background: rgba(255, 255, 255, 0.12);
}

.tb-btn.active {
  background: var(--accent);
  color: #fff;
}

.tb-icon {
  line-height: 1;
}

.tb-color {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  border: 2px solid rgba(255, 255, 255, 0.35);
  cursor: pointer;
  padding: 0;
}

.tb-color.active {
  border-color: #fff;
  box-shadow: 0 0 0 2px var(--accent);
}

.tb-width .dot {
  display: inline-block;
  border-radius: 50%;
  background: currentColor;
}

.tb-cancel {
  color: #ff6b6b;
}

.tb-confirm {
  background: var(--accent);
  color: #fff;
}

.tb-confirm:hover {
  filter: brightness(1.1);
}

.text-input {
  position: absolute;
  z-index: 11;
  min-width: 120px;
  background: rgba(0, 0, 0, 0.35);
  border: 1px dashed var(--accent);
  border-radius: 4px;
  padding: 2px 4px;
  outline: none;
  font-weight: 600;
}
</style>
