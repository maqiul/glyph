<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { readFile } from '@tauri-apps/plugin-fs'
import QRCode from 'qrcode'
import jsQR from 'jsqr'
import { useSettingsStore } from '../../stores/settings'

const { t } = useI18n()
const settings = useSettingsStore()
type Sub =
  | 'json'
  | 'codec'
  | 'hash'
  | 'time'
  | 'uuid'
  | 'regex'
  | 'jwt'
  | 'color'
  | 'qr'
  | 'text'
  | 'radix'
  | 'icon'
  | 'svg'
const active = ref<Sub>('json')

// ---- JSON ----
const jsonIn = ref('')
const jsonOut = ref('')
const jsonErr = ref('')
function jsonFormat() {
  try {
    jsonOut.value = JSON.stringify(JSON.parse(jsonIn.value), null, 2)
    jsonErr.value = ''
  } catch (e) {
    jsonOut.value = ''
    jsonErr.value = String(e)
  }
}
function jsonMinify() {
  try {
    jsonOut.value = JSON.stringify(JSON.parse(jsonIn.value))
    jsonErr.value = ''
  } catch (e) {
    jsonOut.value = ''
    jsonErr.value = String(e)
  }
}

// ---- Codec ----
const codecIn = ref('')
const codecOut = ref('')
function b64Encode(s: string) {
  const bytes = new TextEncoder().encode(s)
  let bin = ''
  bytes.forEach((c) => (bin += String.fromCharCode(c)))
  return btoa(bin)
}
function b64Decode(s: string) {
  const bin = atob(s.trim())
  const bytes = Uint8Array.from(bin, (c) => c.charCodeAt(0))
  return new TextDecoder().decode(bytes)
}
function hexEncode(s: string) {
  return Array.from(new TextEncoder().encode(s), (b) => b.toString(16).padStart(2, '0')).join('')
}
function codec(mode: 'b64e' | 'b64d' | 'urle' | 'urld' | 'hexe') {
  try {
    const i = codecIn.value
    if (mode === 'b64e') codecOut.value = b64Encode(i)
    else if (mode === 'b64d') codecOut.value = b64Decode(i)
    else if (mode === 'urle') codecOut.value = encodeURIComponent(i)
    else if (mode === 'urld') codecOut.value = decodeURIComponent(i)
    else codecOut.value = hexEncode(i)
  } catch (e) {
    codecOut.value = String(e)
  }
}

// ---- Hash ----
const hashIn = ref('')
const hashOut = ref('')
async function doHash(algo: string) {
  const buf = await crypto.subtle.digest(algo, new TextEncoder().encode(hashIn.value))
  hashOut.value = Array.from(new Uint8Array(buf), (b) => b.toString(16).padStart(2, '0')).join('')
}

// ---- Time ----
const timeIn = ref('')
const timeOut = ref('')
function nowStamp() {
  timeIn.value = String(Date.now())
  renderTime()
}
function renderTime() {
  const v = timeIn.value.trim()
  if (!v) {
    timeOut.value = ''
    return
  }
  const num = Number(v)
  if (!Number.isNaN(num)) {
    const ms = v.length <= 10 ? num * 1000 : num
    const d = new Date(ms)
    timeOut.value = `${d.toLocaleString()}\nISO: ${d.toISOString()}`
  } else {
    const parsed = Date.parse(v)
    timeOut.value = Number.isNaN(parsed)
      ? t('dev.invalid')
      : `${parsed} (ms)\n${new Date(parsed).toISOString()}`
  }
}

// ---- UUID ----
const uuidList = ref<string[]>([])
const uuidCount = ref(5)
function genUuid() {
  const n = Math.max(1, Math.min(100, uuidCount.value || 1))
  uuidList.value = Array.from({ length: n }, () => crypto.randomUUID())
}

// ---- Regex ----
const rePattern = ref('')
const reFlags = ref('g')
const reText = ref('')
const reMatches = computed<string[] | null>(() => {
  if (!rePattern.value) return []
  try {
    const flags = reFlags.value.includes('g') ? reFlags.value : reFlags.value + 'g'
    return Array.from(reText.value.matchAll(new RegExp(rePattern.value, flags))).map((m) => m[0])
  } catch {
    return null
  }
})
const reError = computed(() => reMatches.value === null)

// ---- JWT ----
const jwtIn = ref('')
function b64urlDecode(s: string): string {
  let t2 = s.replace(/-/g, '+').replace(/_/g, '/')
  while (t2.length % 4) t2 += '='
  const bin = atob(t2)
  const bytes = Uint8Array.from(bin, (c) => c.charCodeAt(0))
  return new TextDecoder().decode(bytes)
}
const jwt = computed(() => {
  const parts = jwtIn.value.trim().split('.')
  if (parts.length < 2) return null
  try {
    const header = JSON.parse(b64urlDecode(parts[0]!))
    const payload = JSON.parse(b64urlDecode(parts[1]!))
    let expInfo = ''
    if (typeof payload.exp === 'number') {
      const d = new Date(payload.exp * 1000)
      expInfo = `${d.toLocaleString()} · ${d.getTime() > Date.now() ? '未过期' : '已过期'}`
    }
    return {
      header: JSON.stringify(header, null, 2),
      payload: JSON.stringify(payload, null, 2),
      expInfo,
      signed: parts.length >= 3,
    }
  } catch (e) {
    return { error: String(e) }
  }
})

// ---- Color ----
const fgHex = ref('#0969da')
const bgHex = ref('#ffffff')
function hexToRgb(hex: string): [number, number, number] | null {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim())
  if (!m) return null
  const n = parseInt(m[1]!, 16)
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255]
}
function relLum(rgb: [number, number, number]): number {
  const ch = rgb.map((v) => {
    const s = v / 255
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4)
  })
  return 0.2126 * ch[0]! + 0.7152 * ch[1]! + 0.0722 * ch[2]!
}
const color = computed(() => {
  const fg = hexToRgb(fgHex.value)
  const bg = hexToRgb(bgHex.value)
  if (!fg || !bg) return null
  const l1 = relLum(fg)
  const l2 = relLum(bg)
  const ratio = (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05)
  return {
    ratio: ratio.toFixed(2),
    aa: ratio >= 4.5,
    aaLarge: ratio >= 3,
    aaa: ratio >= 7,
    fgRgb: `rgb(${fg.join(', ')})`,
    bgRgb: `rgb(${bg.join(', ')})`,
  }
})

// ---- QR ----
const qrText = ref('')
const qrDataUrl = ref('')
const qrDecoded = ref('')
const qrErr = ref('')
watch(qrText, async (v) => {
  if (!v) {
    qrDataUrl.value = ''
    return
  }
  try {
    qrDataUrl.value = await QRCode.toDataURL(v, { margin: 1, width: 280 })
  } catch (e) {
    qrErr.value = String(e)
  }
})
async function onQrFile(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  if (!file) return
  qrErr.value = ''
  qrDecoded.value = ''
  const url = URL.createObjectURL(file)
  const img = new Image()
  img.onload = () => {
    const canvas = document.createElement('canvas')
    canvas.width = img.naturalWidth
    canvas.height = img.naturalHeight
    const ctx = canvas.getContext('2d')!
    ctx.drawImage(img, 0, 0)
    const data = ctx.getImageData(0, 0, canvas.width, canvas.height)
    const res = jsQR(data.data, data.width, data.height)
    if (res?.data) qrDecoded.value = res.data
    else qrErr.value = t('dev.qrNoCode')
    URL.revokeObjectURL(url)
  }
  img.src = url
}

// ---- Text ----
const textIn = ref('')
const textOut = ref('')
const TEXT_OPS: Record<string, (s: string) => string> = {
  upper: (s) => s.toUpperCase(),
  lower: (s) => s.toLowerCase(),
  capitalize: (s) => s.replace(/\b\w/g, (c) => c.toUpperCase()),
  trimLines: (s) =>
    s
      .split('\n')
      .map((l) => l.trim())
      .join('\n'),
  dropEmpty: (s) =>
    s
      .split('\n')
      .filter((l) => l.trim() !== '')
      .join('\n'),
  dedup: (s) => Array.from(new Set(s.split('\n'))).join('\n'),
  sortAsc: (s) =>
    s
      .split('\n')
      .sort((a, b) => a.localeCompare(b))
      .join('\n'),
  sortDesc: (s) =>
    s
      .split('\n')
      .sort((a, b) => b.localeCompare(a))
      .join('\n'),
  reverse: (s) => s.split('\n').reverse().join('\n'),
}
function applyTextOp(op: keyof typeof TEXT_OPS) {
  textOut.value = TEXT_OPS[op]!(textIn.value)
}
const textStats = computed(() => {
  const s = textIn.value
  return {
    chars: [...s].length,
    words: (s.match(/\S+/g) || []).length,
    lines: s ? s.split('\n').length : 0,
    bytes: new TextEncoder().encode(s).length,
  }
})

// ---- Radix ----
const radixIn = ref('')
const radixBase = ref(10)
function parseBase(str: string, base: number): bigint {
  let s = str.trim()
  let neg = false
  if (s.startsWith('-')) {
    neg = true
    s = s.slice(1)
  }
  if (s === '') throw new Error('empty')
  const digits = '0123456789abcdefghijklmnopqrstuvwxyz'
  const B = BigInt(base)
  let acc = 0n
  for (const ch of s.toLowerCase()) {
    const d = digits.indexOf(ch)
    if (d < 0 || d >= base) throw new Error('bad digit')
    acc = acc * B + BigInt(d)
  }
  return neg ? -acc : acc
}
const radixOut = computed(() => {
  const s = radixIn.value.trim()
  if (!s) return null
  try {
    const n = parseBase(s, radixBase.value)
    return {
      dec: n.toString(10),
      hex: n.toString(16).toUpperCase(),
      oct: n.toString(8),
      bin: n.toString(2),
    }
  } catch {
    return { error: true as const }
  }
})

// ---- 通用：图片加载 ----
function toBase64(bytes: Uint8Array): string {
  let bin = ''
  const chunk = 0x8000
  for (let i = 0; i < bytes.length; i += chunk) {
    bin += String.fromCharCode(...Array.from(bytes.subarray(i, i + chunk)))
  }
  return btoa(bin)
}
function loadImageFromSrc(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image()
    img.onload = () => resolve(img)
    img.onerror = () => reject(new Error('load image failed'))
    img.src = src
  })
}
function loadImage(file: File): Promise<HTMLImageElement> {
  return loadImageFromSrc(URL.createObjectURL(file))
}
// 从绝对路径读图（Tauri 原生拖拽只能拿到路径）
async function loadImageFromPath(path: string): Promise<{ img: HTMLImageElement; name: string }> {
  const bytes = await readFile(path)
  const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes)
  const ext = (path.split('.').pop() || 'png').toLowerCase()
  const mime =
    ext === 'jpg' || ext === 'jpeg'
      ? 'image/jpeg'
      : ext === 'webp'
        ? 'image/webp'
        : ext === 'gif'
          ? 'image/gif'
          : ext === 'bmp'
            ? 'image/bmp'
            : 'image/png'
  const img = await loadImageFromSrc(`data:${mime};base64,${toBase64(u8)}`)
  const name = path.split(/[\\/]/).pop() || 'image'
  return { img, name }
}
function baseName(name: string) {
  return name.replace(/\.[^.]+$/, '') || 'image'
}

// ---- 图片转 ICO ----
const iconImg = ref<HTMLImageElement | null>(null)
const iconFileName = ref('')
const iconSizes = ref('16,32,48,64,128,256')
const iconStatus = ref('')
async function onIconFile(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  if (!file) return
  iconStatus.value = ''
  try {
    iconImg.value = await loadImage(file)
    iconFileName.value = file.name
  } catch {
    iconStatus.value = t('dev.iconFail')
  }
}
function buildIco(images: { size: number; data: Uint8Array }[]): Blob {
  const count = images.length
  const headerSize = 6 + count * 16
  const header = new Uint8Array(headerSize)
  const dv = new DataView(header.buffer)
  dv.setUint16(2, 1, true) // type = icon
  dv.setUint16(4, count, true)
  let offset = headerSize
  let p = 6
  for (const im of images) {
    const dim = im.size >= 256 ? 0 : im.size
    header[p] = dim
    header[p + 1] = dim
    dv.setUint16(p + 4, 1, true) // planes
    dv.setUint16(p + 6, 32, true) // bit count
    dv.setUint32(p + 8, im.data.length, true)
    dv.setUint32(p + 12, offset, true)
    offset += im.data.length
    p += 16
  }
  const out = new Uint8Array(offset)
  out.set(header, 0)
  let o = headerSize
  for (const im of images) {
    out.set(im.data, o)
    o += im.data.length
  }
  return new Blob([out], { type: 'image/x-icon' })
}
async function doIcon() {
  const img = iconImg.value
  if (!img) return
  iconStatus.value = ''
  const parsed = iconSizes.value
    .split(',')
    .map((s) => parseInt(s.trim(), 10))
    .filter((n) => n >= 16 && n <= 256)
  const sizes = parsed.length ? Array.from(new Set(parsed)) : [16, 32, 48, 64, 128, 256]
  const images: { size: number; data: Uint8Array }[] = []
  for (const s of sizes) {
    const cv = document.createElement('canvas')
    cv.width = s
    cv.height = s
    const ctx = cv.getContext('2d')
    if (!ctx) continue
    ctx.clearRect(0, 0, s, s)
    ctx.imageSmoothingEnabled = true
    ctx.imageSmoothingQuality = 'high'
    const scale = Math.min(s / img.naturalWidth, s / img.naturalHeight)
    const w = img.naturalWidth * scale
    const h = img.naturalHeight * scale
    ctx.drawImage(img, (s - w) / 2, (s - h) / 2, w, h)
    const blob = await new Promise<Blob | null>((res) => cv.toBlob(res, 'image/png'))
    if (blob) images.push({ size: s, data: new Uint8Array(await blob.arrayBuffer()) })
  }
  if (!images.length) {
    iconStatus.value = t('dev.iconFail')
    return
  }
  const ico = new Uint8Array(await buildIco(images).arrayBuffer())
  const out = await save({
    defaultPath: `${baseName(iconFileName.value) || 'icon'}.ico`,
    filters: [{ name: 'Icon', extensions: ['ico'] }],
  })
  if (!out) return
  try {
    await invoke('write_file_base64', { path: out, dataBase64: toBase64(ico) })
    iconStatus.value = t('dev.iconSaved', { out, n: images.length, sizes: sizes.join('/') })
  } catch (e) {
    iconStatus.value = `${t('dev.saveFailed')}: ${String(e)}`
  }
}

// ---- 图片转 SVG（矢量化：中位切分量化 + 轮廓提取 + RDP 简化）----
const svgImg = ref<HTMLImageElement | null>(null)
const svgFileName = ref('')
const svgMaxDim = ref(128)
const svgColors = ref(8)
const svgEps = ref(1)
const svgBusy = ref(false)
const svgCode = ref('')
const svgError = ref('')
const svgSaved = ref('')
const svgDataUrl = computed(() =>
  svgCode.value ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svgCode.value)}` : '',
)
async function onSvgFile(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  if (!file) return
  svgError.value = ''
  try {
    svgImg.value = await loadImage(file)
    svgFileName.value = file.name
  } catch {
    svgError.value = t('dev.svgFail')
  }
}
function bounds(box: number[][]): [number[], number[]] {
  const lo = [255, 255, 255]
  const hi = [0, 0, 0]
  for (const p of box) {
    for (let c = 0; c < 3; c++) {
      if (p[c]! < lo[c]!) lo[c] = p[c]!
      if (p[c]! > hi[c]!) hi[c] = p[c]!
    }
  }
  return [lo, hi]
}
function boxRange(box: number[][]): number {
  const [lo, hi] = bounds(box)
  return Math.max(hi[0]! - lo[0]!, hi[1]! - lo[1]!, hi[2]! - lo[2]!)
}
function longestChannel(box: number[][]): number {
  const [lo, hi] = bounds(box)
  const dr = hi[0]! - lo[0]!
  const dg = hi[1]! - lo[1]!
  const db = hi[2]! - lo[2]!
  return dr >= dg && dr >= db ? 0 : dg >= db ? 1 : 2
}
function avgColor(box: number[][]): number[] {
  let r = 0
  let g = 0
  let b = 0
  for (const p of box) {
    r += p[0]!
    g += p[1]!
    b += p[2]!
  }
  const n = box.length
  return [Math.round(r / n), Math.round(g / n), Math.round(b / n)]
}
function quantize(pixels: number[][], k: number): number[][] {
  if (!pixels.length) return []
  const boxes: number[][][] = [pixels]
  while (boxes.length < k) {
    let target = -1
    let best = -1
    for (let i = 0; i < boxes.length; i++) {
      const b = boxes[i]!
      if (b.length < 2) continue
      const r = boxRange(b)
      if (r > best) {
        best = r
        target = i
      }
    }
    if (target < 0) break
    const box = boxes.splice(target, 1)[0]!
    const chan = longestChannel(box)
    box.sort((a, b) => a[chan]! - b[chan]!)
    const mid = box.length >> 1
    boxes.push(box.slice(0, mid), box.slice(mid))
  }
  return boxes.filter((b) => b.length > 0).map(avgColor)
}
function extractLoops(mask: Uint8Array, w: number, h: number): number[][][] {
  const on = (x: number, y: number) => x >= 0 && y >= 0 && x < w && y < h && mask[y * w + x] === 1
  const edges: number[][] = []
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      if (!on(x, y)) continue
      if (!on(x, y - 1)) edges.push([x, y, x + 1, y])
      if (!on(x + 1, y)) edges.push([x + 1, y, x + 1, y + 1])
      if (!on(x, y + 1)) edges.push([x + 1, y + 1, x, y + 1])
      if (!on(x - 1, y)) edges.push([x, y + 1, x, y])
    }
  }
  const key = (x: number, y: number) => `${x},${y}`
  const startMap = new Map<string, number[]>()
  edges.forEach((e, i) => {
    const k = key(e[0]!, e[1]!)
    const arr = startMap.get(k)
    if (arr) arr.push(i)
    else startMap.set(k, [i])
  })
  const used = new Uint8Array(edges.length)
  const loops: number[][][] = []
  for (let i = 0; i < edges.length; i++) {
    if (used[i]) continue
    const startE = edges[i]!
    const loop: number[][] = []
    let cur: number | undefined = i
    while (cur !== undefined) {
      const e = edges[cur]!
      used[cur] = 1
      loop.push([e[0]!, e[1]!])
      const k = key(e[2]!, e[3]!)
      if (k === key(startE[0]!, startE[1]!)) break
      cur = (startMap.get(k) || []).find((j) => !used[j])
    }
    if (loop.length > 2) loops.push(loop)
  }
  return loops
}
function rdp(pts: number[][], eps: number): number[][] {
  const n = pts.length
  if (n < 3 || eps <= 0) return pts
  const keep = new Uint8Array(n)
  keep[0] = 1
  keep[n - 1] = 1
  const stack: number[][] = [[0, n - 1]]
  while (stack.length) {
    const seg = stack.pop()!
    const s = seg[0]!
    const e = seg[1]!
    const a = pts[s]!
    const b = pts[e]!
    const x1 = a[0]!
    const y1 = a[1]!
    const dx = b[0]! - x1
    const dy = b[1]! - y1
    const len2 = dx * dx + dy * dy
    let maxD = -1
    let idx = -1
    for (let i = s + 1; i < e; i++) {
      const p = pts[i]!
      let d: number
      if (len2 === 0) d = Math.hypot(p[0]! - x1, p[1]! - y1)
      else {
        const t = ((p[0]! - x1) * dx + (p[1]! - y1) * dy) / len2
        const tc = Math.max(0, Math.min(1, t))
        d = Math.hypot(p[0]! - (x1 + tc * dx), p[1]! - (y1 + tc * dy))
      }
      if (d > maxD) {
        maxD = d
        idx = i
      }
    }
    if (maxD > eps && idx > 0) {
      keep[idx] = 1
      stack.push([s, idx], [idx, e])
    }
  }
  return pts.filter((_, i) => keep[i] === 1)
}
function polyArea(pts: number[][]): number {
  let a = 0
  for (let i = 0; i < pts.length; i++) {
    const p = pts[i]!
    const q = pts[(i + 1) % pts.length]!
    a += p[0]! * q[1]! - q[0]! * p[1]!
  }
  return Math.abs(a) / 2
}
function vectorize(img: HTMLImageElement, maxDim: number, colorCount: number, eps: number): string {
  const natW = img.naturalWidth || img.width
  const natH = img.naturalHeight || img.height
  const scale = Math.min(1, maxDim / Math.max(natW, natH))
  const w = Math.max(1, Math.round(natW * scale))
  const h = Math.max(1, Math.round(natH * scale))
  const cv = document.createElement('canvas')
  cv.width = w
  cv.height = h
  const ctx = cv.getContext('2d', { willReadFrequently: true })
  if (!ctx) throw new Error('no 2d')
  ctx.imageSmoothingEnabled = true
  ctx.imageSmoothingQuality = 'high'
  ctx.drawImage(img, 0, 0, w, h)
  const data = ctx.getImageData(0, 0, w, h).data
  const total = w * h
  const samples: number[][] = []
  for (let i = 0; i < total; i++) {
    if (data[i * 4 + 3]! < 128) continue
    samples.push([data[i * 4]!, data[i * 4 + 1]!, data[i * 4 + 2]!])
  }
  if (!samples.length) throw new Error('empty')
  const palette = quantize(samples, Math.max(1, Math.min(24, colorCount)))
  const labels = new Int16Array(total).fill(-1)
  for (let i = 0; i < total; i++) {
    if (data[i * 4 + 3]! < 128) continue
    const r = data[i * 4]!
    const g = data[i * 4 + 1]!
    const b = data[i * 4 + 2]!
    let best = 0
    let bd = Infinity
    for (let c = 0; c < palette.length; c++) {
      const p = palette[c]!
      const dr = r - p[0]!
      const dg = g - p[1]!
      const db = b - p[2]!
      const d = dr * dr + dg * dg + db * db
      if (d < bd) {
        bd = d
        best = c
      }
    }
    labels[i] = best
  }
  const minArea = 3
  let paths = ''
  for (let c = 0; c < palette.length; c++) {
    const mask = new Uint8Array(total)
    for (let i = 0; i < total; i++) if (labels[i] === c) mask[i] = 1
    let d = ''
    for (const loop of extractLoops(mask, w, h)) {
      const simp = rdp(loop, eps)
      if (simp.length < 3 || polyArea(simp) < minArea) continue
      d += 'M' + simp.map((p) => `${p[0]} ${p[1]}`).join('L') + 'Z'
    }
    if (d) {
      const col = palette[c]!
      paths += `<path fill="rgb(${col[0]},${col[1]},${col[2]})" fill-rule="evenodd" d="${d}"/>`
    }
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" width="${w}" height="${h}">${paths}</svg>`
}
async function doSvg() {
  const img = svgImg.value
  if (!img) return
  svgError.value = ''
  svgBusy.value = true
  svgCode.value = ''
  svgSaved.value = ''
  await new Promise((r) => setTimeout(r, 20))
  try {
    svgCode.value = vectorize(img, svgMaxDim.value, svgColors.value, svgEps.value)
  } catch (e) {
    svgError.value = t('dev.svgFail')
    console.error(e)
  } finally {
    svgBusy.value = false
  }
}
async function downloadSvg() {
  if (!svgCode.value) return
  const out = await save({
    defaultPath: `${baseName(svgFileName.value) || 'image'}.svg`,
    filters: [{ name: 'SVG', extensions: ['svg'] }],
  })
  if (!out) return
  try {
    await invoke('write_file_base64', {
      path: out,
      dataBase64: toBase64(new TextEncoder().encode(svgCode.value)),
    })
    svgSaved.value = out
    svgError.value = ''
  } catch (e) {
    svgError.value = `${t('dev.saveFailed')}: ${String(e)}`
  }
}

// ---- 拖拽（Tauri 原生文件拖入，App.vue 转发 app-file-drop）----
const iconInputRef = ref<HTMLInputElement | null>(null)
const svgInputRef = ref<HTMLInputElement | null>(null)
async function applyIconPath(path: string) {
  iconStatus.value = ''
  try {
    const { img, name } = await loadImageFromPath(path)
    iconImg.value = img
    iconFileName.value = name
  } catch {
    iconStatus.value = t('dev.iconFail')
  }
}
async function applySvgPath(path: string) {
  svgError.value = ''
  try {
    const { img, name } = await loadImageFromPath(path)
    svgImg.value = img
    svgFileName.value = name
  } catch {
    svgError.value = t('dev.svgFail')
  }
}
let unDrop: (() => void) | null = null
onMounted(async () => {
  try {
    unDrop = await listen<{ path: string }>('app-file-drop', (e) => {
      if (settings.activeTool !== 'dev') return
      const p = e.payload.path
      if (!/\.(png|jpe?g|webp|bmp|gif)$/i.test(p)) return
      if (active.value === 'icon') void applyIconPath(p)
      else if (active.value === 'svg') void applySvgPath(p)
    })
  } catch {
    /* 拖拽监听不可用时忽略 */
  }
})
onUnmounted(() => unDrop?.())

async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text)
  } catch (e) {
    console.warn(e)
  }
}
</script>

<template>
  <div class="dev-tool">
    <div class="dev-tabs">
      <button
        v-for="tab in [
          'json',
          'codec',
          'hash',
          'time',
          'uuid',
          'regex',
          'jwt',
          'color',
          'qr',
          'text',
          'radix',
          'icon',
          'svg',
        ] as const"
        :key="tab"
        :class="{ active: active === tab }"
        @click="active = tab"
      >
        {{ t(`dev.${tab}`) }}
      </button>
    </div>

    <div class="dev-body">
      <!-- JSON -->
      <div v-if="active === 'json'" class="dev-panel">
        <textarea
          v-model="jsonIn"
          class="dev-io"
          :placeholder="t('dev.input')"
          spellcheck="false"
        ></textarea>
        <div class="dev-actions">
          <button class="btn" @click="jsonFormat">{{ t('dev.format') }}</button>
          <button class="btn" @click="jsonMinify">{{ t('dev.minify') }}</button>
          <button class="btn" :disabled="!jsonOut" @click="copy(jsonOut)">
            {{ t('dev.copy') }}
          </button>
        </div>
        <div v-if="jsonErr" class="dev-err">⚠ {{ jsonErr }}</div>
        <textarea
          :value="jsonOut"
          class="dev-io"
          readonly
          :placeholder="t('dev.output')"
          spellcheck="false"
        ></textarea>
      </div>

      <!-- Codec -->
      <div v-else-if="active === 'codec'" class="dev-panel">
        <textarea
          v-model="codecIn"
          class="dev-io"
          :placeholder="t('dev.input')"
          spellcheck="false"
        ></textarea>
        <div class="dev-actions">
          <button class="btn" @click="codec('b64e')">Base64 ↑</button>
          <button class="btn" @click="codec('b64d')">Base64 ↓</button>
          <button class="btn" @click="codec('urle')">URL ↑</button>
          <button class="btn" @click="codec('urld')">URL ↓</button>
          <button class="btn" @click="codec('hexe')">Hex</button>
          <button class="btn" :disabled="!codecOut" @click="copy(codecOut)">
            {{ t('dev.copy') }}
          </button>
        </div>
        <textarea
          :value="codecOut"
          class="dev-io"
          readonly
          :placeholder="t('dev.output')"
          spellcheck="false"
        ></textarea>
      </div>

      <!-- Hash -->
      <div v-else-if="active === 'hash'" class="dev-panel">
        <textarea
          v-model="hashIn"
          class="dev-io"
          :placeholder="t('dev.input')"
          spellcheck="false"
        ></textarea>
        <div class="dev-actions">
          <button class="btn" @click="doHash('SHA-1')">SHA-1</button>
          <button class="btn" @click="doHash('SHA-256')">SHA-256</button>
          <button class="btn" @click="doHash('SHA-384')">SHA-384</button>
          <button class="btn" @click="doHash('SHA-512')">SHA-512</button>
          <button class="btn" :disabled="!hashOut" @click="copy(hashOut)">
            {{ t('dev.copy') }}
          </button>
        </div>
        <input :value="hashOut" class="dev-line" readonly spellcheck="false" />
      </div>

      <!-- Time -->
      <div v-else-if="active === 'time'" class="dev-panel">
        <div class="dev-actions">
          <input
            v-model="timeIn"
            class="dev-line"
            :placeholder="t('dev.timePh')"
            @input="renderTime"
          />
          <button class="btn" @click="nowStamp">{{ t('dev.now') }}</button>
        </div>
        <textarea
          :value="timeOut"
          class="dev-io"
          readonly
          :placeholder="t('dev.output')"
        ></textarea>
      </div>

      <!-- UUID -->
      <div v-else-if="active === 'uuid'" class="dev-panel">
        <div class="dev-actions">
          <label class="cfg-label">{{ t('dev.count') }}</label>
          <input v-model.number="uuidCount" type="number" min="1" max="100" class="dev-num" />
          <button class="btn" @click="genUuid">{{ t('dev.generate') }}</button>
          <button class="btn" :disabled="!uuidList.length" @click="copy(uuidList.join('\n'))">
            {{ t('dev.copy') }}
          </button>
        </div>
        <textarea
          :value="uuidList.join('\n')"
          class="dev-io"
          readonly
          spellcheck="false"
        ></textarea>
      </div>

      <!-- Regex -->
      <div v-else-if="active === 'regex'" class="dev-panel">
        <div class="dev-actions">
          <input
            v-model="rePattern"
            class="dev-line"
            :placeholder="t('dev.pattern')"
            spellcheck="false"
          />
          <input
            v-model="reFlags"
            class="dev-num"
            :placeholder="t('dev.flags')"
            style="width: 70px"
          />
        </div>
        <textarea
          v-model="reText"
          class="dev-io"
          :placeholder="t('dev.testText')"
          spellcheck="false"
        ></textarea>
        <div v-if="reError" class="dev-err">⚠ {{ t('dev.invalidRegex') }}</div>
        <div v-else-if="rePattern" class="dev-matches">
          <div class="dev-matches-head">{{ t('dev.matches') }}: {{ reMatches?.length ?? 0 }}</div>
          <ul class="dev-match-list">
            <li v-for="(m, i) in reMatches" :key="i">{{ m }}</li>
          </ul>
        </div>
      </div>

      <!-- JWT -->
      <div v-else-if="active === 'jwt'" class="dev-panel">
        <textarea
          v-model="jwtIn"
          class="dev-io"
          placeholder="eyJhbGciOi..."
          spellcheck="false"
          style="min-height: 70px; flex: none"
        ></textarea>
        <template v-if="jwt && !('error' in jwt)">
          <div class="dev-section-head">Header</div>
          <textarea :value="jwt.header" class="dev-io" readonly spellcheck="false"></textarea>
          <div class="dev-section-head">
            Payload <span v-if="jwt.expInfo" class="jwt-exp">· exp: {{ jwt.expInfo }}</span>
          </div>
          <textarea :value="jwt.payload" class="dev-io" readonly spellcheck="false"></textarea>
        </template>
        <div v-else-if="jwt && 'error' in jwt" class="dev-err">⚠ {{ jwt.error }}</div>
      </div>

      <!-- Color -->
      <div v-else-if="active === 'color'" class="dev-panel">
        <div class="dev-actions">
          <label class="cfg-label">FG</label>
          <input v-model="fgHex" class="dev-line" spellcheck="false" style="max-width: 140px" />
          <input v-model="fgHex" type="color" class="dev-color" />
          <label class="cfg-label">BG</label>
          <input v-model="bgHex" class="dev-line" spellcheck="false" style="max-width: 140px" />
          <input v-model="bgHex" type="color" class="dev-color" />
        </div>
        <div v-if="color" class="color-preview" :style="{ background: bgHex, color: fgHex }">
          示例文字 Aa Bb 123 · Sample
        </div>
        <div v-if="color" class="color-info">
          <div class="ci-row">
            <span>对比度</span><span class="ci-ratio">{{ color.ratio }}:1</span>
          </div>
          <div class="ci-row">
            <span>WCAG AA (4.5)</span><span>{{ color.aa ? '✅ 通过' : '❌ 不通过' }}</span>
          </div>
          <div class="ci-row">
            <span>WCAG AA 大字 (3)</span><span>{{ color.aaLarge ? '✅' : '❌' }}</span>
          </div>
          <div class="ci-row">
            <span>WCAG AAA (7)</span><span>{{ color.aaa ? '✅' : '❌' }}</span>
          </div>
          <div class="ci-row">
            <span>FG RGB</span><span class="mono">{{ color.fgRgb }}</span>
          </div>
          <div class="ci-row">
            <span>BG RGB</span><span class="mono">{{ color.bgRgb }}</span>
          </div>
        </div>
        <div v-else class="dev-err">⚠ 请输入合法的 6 位十六进制颜色（如 #0969da）</div>
      </div>

      <!-- QR -->
      <div v-else-if="active === 'qr'" class="dev-panel">
        <div class="dev-section-head">{{ t('dev.qrGen') }}</div>
        <textarea
          v-model="qrText"
          class="dev-io"
          :placeholder="t('dev.input')"
          style="min-height: 60px; flex: none"
          spellcheck="false"
        ></textarea>
        <div v-if="qrDataUrl" class="qr-preview">
          <img :src="qrDataUrl" alt="QR" />
          <button class="btn" @click="copy(qrText)">{{ t('dev.copy') }}</button>
        </div>
        <div class="dev-section-head" style="margin-top: 8px">{{ t('dev.qrDecode') }}</div>
        <input type="file" accept="image/*" @change="onQrFile" class="dev-file" />
        <div v-if="qrErr" class="dev-err">⚠ {{ qrErr }}</div>
        <textarea
          v-if="qrDecoded"
          :value="qrDecoded"
          class="dev-io"
          readonly
          spellcheck="false"
          style="min-height: 60px"
        ></textarea>
      </div>

      <!-- Text -->
      <div v-else-if="active === 'text'" class="dev-panel">
        <textarea
          v-model="textIn"
          class="dev-io"
          :placeholder="t('dev.input')"
          spellcheck="false"
        ></textarea>
        <div class="dev-actions">
          <button class="btn" @click="applyTextOp('upper')">{{ t('dev.upper') }}</button>
          <button class="btn" @click="applyTextOp('lower')">{{ t('dev.lower') }}</button>
          <button class="btn" @click="applyTextOp('capitalize')">{{ t('dev.capitalize') }}</button>
          <button class="btn" @click="applyTextOp('trimLines')">{{ t('dev.trimLines') }}</button>
          <button class="btn" @click="applyTextOp('dropEmpty')">{{ t('dev.dropEmpty') }}</button>
          <button class="btn" @click="applyTextOp('dedup')">{{ t('dev.dedup') }}</button>
          <button class="btn" @click="applyTextOp('sortAsc')">{{ t('dev.sortAsc') }}</button>
          <button class="btn" @click="applyTextOp('sortDesc')">{{ t('dev.sortDesc') }}</button>
          <button class="btn" @click="applyTextOp('reverse')">{{ t('dev.reverse') }}</button>
          <button class="btn" :disabled="!textOut" @click="copy(textOut)">
            {{ t('dev.copy') }}
          </button>
        </div>
        <div class="dev-stats">{{ t('dev.stats', textStats) }}</div>
        <textarea
          :value="textOut"
          class="dev-io"
          readonly
          :placeholder="t('dev.output')"
          spellcheck="false"
        ></textarea>
      </div>

      <!-- Radix -->
      <div v-else-if="active === 'radix'" class="dev-panel">
        <div class="dev-actions">
          <input
            v-model="radixIn"
            class="dev-line"
            :placeholder="t('dev.radixPh')"
            spellcheck="false"
          />
          <label class="cfg-label">{{ t('dev.base') }}</label>
          <select v-model.number="radixBase" class="dev-num" style="width: 80px">
            <option :value="2">2</option>
            <option :value="8">8</option>
            <option :value="10">10</option>
            <option :value="16">16</option>
          </select>
        </div>
        <div v-if="radixOut && !('error' in radixOut)" class="radix-grid">
          <div class="ci-row">
            <span>{{ t('dev.dec') }}</span
            ><code class="mono">{{ radixOut.dec }}</code
            ><button class="btn" @click="copy(radixOut.dec)">{{ t('dev.copy') }}</button>
          </div>
          <div class="ci-row">
            <span>{{ t('dev.hex') }}</span
            ><code class="mono">{{ radixOut.hex }}</code
            ><button class="btn" @click="copy(radixOut.hex)">{{ t('dev.copy') }}</button>
          </div>
          <div class="ci-row">
            <span>{{ t('dev.oct') }}</span
            ><code class="mono">{{ radixOut.oct }}</code
            ><button class="btn" @click="copy(radixOut.oct)">{{ t('dev.copy') }}</button>
          </div>
          <div class="ci-row">
            <span>{{ t('dev.bin') }}</span
            ><code class="mono">{{ radixOut.bin }}</code
            ><button class="btn" @click="copy(radixOut.bin)">{{ t('dev.copy') }}</button>
          </div>
        </div>
        <div v-else-if="radixOut && 'error' in radixOut" class="dev-err">
          ⚠ {{ t('dev.invalidNumber') }}
        </div>
      </div>

      <!-- 图片转 ICO -->
      <div v-else-if="active === 'icon'" class="dev-panel">
        <div class="dev-drop" @click="iconInputRef?.click()" @dragover.prevent @drop.prevent>
          <div v-if="iconImg" class="icon-preview">
            <img :src="iconImg.src" alt="src" />
            <span class="dev-stats">
              {{ iconFileName }} · {{ iconImg.naturalWidth }} × {{ iconImg.naturalHeight }}
            </span>
          </div>
          <span v-else class="dev-drop-hint">{{ t('dev.dropImage') }}</span>
        </div>
        <input
          ref="iconInputRef"
          type="file"
          accept="image/*"
          class="dev-file-hidden"
          @change="onIconFile"
        />
        <div class="dev-actions">
          <label class="cfg-label">{{ t('dev.iconSizes') }}</label>
          <input v-model="iconSizes" class="dev-line" spellcheck="false" style="max-width: 280px" />
          <button class="btn" :disabled="!iconImg" @click="doIcon">{{ t('dev.iconGen') }}</button>
        </div>
        <div v-if="iconStatus" class="dev-stats">{{ iconStatus }}</div>
      </div>

      <!-- 图片转 SVG -->
      <div v-else-if="active === 'svg'" class="dev-panel">
        <div class="dev-drop" @click="svgInputRef?.click()" @dragover.prevent @drop.prevent>
          <div v-if="svgImg" class="icon-preview">
            <img :src="svgImg.src" alt="src" />
            <span class="dev-stats">
              {{ svgFileName }} · {{ svgImg.naturalWidth }} × {{ svgImg.naturalHeight }}
            </span>
          </div>
          <span v-else class="dev-drop-hint">{{ t('dev.dropImage') }}</span>
        </div>
        <input
          ref="svgInputRef"
          type="file"
          accept="image/*"
          class="dev-file-hidden"
          @change="onSvgFile"
        />
        <div class="dev-actions">
          <label class="cfg-label">{{ t('dev.svgMaxDim') }}</label>
          <input v-model.number="svgMaxDim" type="number" min="32" max="400" class="dev-num" />
          <label class="cfg-label">{{ t('dev.svgColors') }}</label>
          <input v-model.number="svgColors" type="number" min="2" max="24" class="dev-num" />
          <label class="cfg-label">{{ t('dev.svgDetail') }}</label>
          <input v-model.number="svgEps" type="number" min="0" max="5" step="0.5" class="dev-num" />
          <button class="btn" :disabled="!svgImg || svgBusy" @click="doSvg">
            {{ svgBusy ? t('dev.svgBusy') : t('dev.svgGen') }}
          </button>
        </div>
        <div v-if="svgError" class="dev-err">⚠ {{ svgError }}</div>
        <div v-if="svgDataUrl" class="svg-preview">
          <img :src="svgDataUrl" alt="svg" />
        </div>
        <div v-if="svgCode" class="dev-actions">
          <button class="btn" @click="copy(svgCode)">{{ t('dev.copy') }}</button>
          <button class="btn" @click="downloadSvg">{{ t('dev.svgDownload') }}</button>
        </div>
        <div v-if="svgSaved" class="dev-stats">{{ t('dev.svgSaved', { out: svgSaved }) }}</div>
        <textarea
          v-if="svgCode"
          :value="svgCode"
          class="dev-io"
          readonly
          spellcheck="false"
        ></textarea>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dev-tool {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.dev-tabs {
  display: flex;
  gap: 2px;
  padding: 8px 14px;
  border-bottom: 1px solid var(--border);
  background: var(--surface);
  flex-wrap: wrap;
}

.dev-tabs button {
  border: 0;
  background: transparent;
  color: var(--text-muted);
  padding: 6px 14px;
  font-size: 13px;
  border-radius: 6px;
  cursor: pointer;
}

.dev-tabs button:hover {
  background: var(--surface-hover);
  color: var(--text);
}

.dev-tabs button.active {
  background: var(--accent);
  color: var(--accent-fg);
}

.dev-body {
  flex: 1;
  overflow-y: auto;
  padding: 18px 22px;
}

.dev-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
}

.dev-io {
  flex: 1;
  min-height: 120px;
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

.dev-io:focus,
.dev-line:focus,
.dev-num:focus {
  outline: none;
  border-color: var(--accent);
}

.dev-line {
  flex: 1;
  min-width: 0;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 13px;
}

.dev-num {
  width: 64px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 13px;
}

.dev-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.cfg-label {
  font-size: 12px;
  color: var(--text-muted);
}

.btn {
  padding: 6px 14px;
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

.dev-err {
  color: var(--error);
  font-size: 13px;
  font-family: var(--font-mono);
  padding: 8px 12px;
  background: var(--surface);
  border: 1px solid var(--error);
  border-radius: 6px;
  white-space: pre-wrap;
  word-break: break-word;
}

.dev-matches {
  border-top: 1px solid var(--border);
  padding-top: 10px;
}

.dev-matches-head {
  font-size: 12px;
  color: var(--text-muted);
  margin-bottom: 6px;
}

.dev-match-list {
  margin: 0;
  padding-left: 20px;
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--text);
  max-height: 200px;
  overflow-y: auto;
}

.dev-section-head {
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-muted);
}

.jwt-exp {
  text-transform: none;
  letter-spacing: 0;
  color: var(--accent);
}

.dev-color {
  width: 34px;
  height: 32px;
  padding: 0;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  cursor: pointer;
}

.color-preview {
  padding: 20px;
  border-radius: 8px;
  border: 1px solid var(--border);
  font-size: 18px;
  font-weight: 600;
  text-align: center;
}

.color-info {
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 6px 14px;
}

.ci-row {
  display: flex;
  justify-content: space-between;
  padding: 6px 0;
  font-size: 13px;
  border-bottom: 1px dashed var(--border);
}

.ci-row:last-child {
  border-bottom: 0;
}

.ci-ratio {
  font-family: var(--font-mono);
  font-weight: 700;
  color: var(--accent);
}

.mono {
  font-family: var(--font-mono);
}

.qr-preview {
  display: flex;
  align-items: center;
  gap: 16px;
}

.qr-preview img {
  border: 1px solid var(--border);
  border-radius: 8px;
  background: #fff;
  padding: 6px;
  width: 160px;
  height: 160px;
}

.dev-file {
  font-size: 13px;
  color: var(--text-muted);
}

.dev-stats {
  font-size: 12px;
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.radix-grid .ci-row {
  gap: 12px;
}

.radix-grid .ci-row code {
  flex: 1;
  min-width: 0;
  overflow-x: auto;
  text-align: right;
  white-space: nowrap;
}

.radix-grid .ci-row .btn {
  flex-shrink: 0;
}

.icon-preview {
  display: flex;
  align-items: center;
  gap: 12px;
}

.icon-preview img {
  max-width: 96px;
  max-height: 96px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: repeating-conic-gradient(#ccc 0 25%, #fff 0 50%) 50% / 16px 16px;
}

.svg-preview {
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px;
  display: flex;
  justify-content: center;
  background: repeating-conic-gradient(#eee 0 25%, #fff 0 50%) 50% / 16px 16px;
}

.svg-preview img {
  max-width: 100%;
  max-height: 280px;
}

.dev-drop {
  border: 1.5px dashed var(--border);
  border-radius: 10px;
  padding: 20px;
  text-align: center;
  cursor: pointer;
  transition:
    border-color 0.15s ease,
    background 0.15s ease;
}

.dev-drop:hover {
  border-color: var(--accent);
  background: var(--surface);
}

.dev-drop-hint {
  color: var(--text-muted);
  font-size: 13px;
}

.dev-file-hidden {
  display: none;
}

.dev-drop .icon-preview {
  flex-direction: column;
  align-items: center;
  gap: 8px;
}
</style>
