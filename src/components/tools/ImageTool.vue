<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'
import { readFile } from '@tauri-apps/plugin-fs'
import { useSettingsStore } from '../../stores/settings'

const { t } = useI18n()
const settings = useSettingsStore()

type Fmt = 'png' | 'jpeg' | 'webp'
type WmPos = 'se' | 'sw' | 'ne' | 'nw' | 'center'

const img = ref<HTMLImageElement | null>(null)
const fileName = ref('')
const busy = ref(false)
const status = ref('')
const error = ref('')

const fmt = ref<Fmt>('png')
const quality = ref(0.85)
const resizeW = ref('')
const resizeH = ref('')
const keepRatio = ref(true)

const wmOn = ref(false)
const wmText = ref('')
const wmSize = ref(24)
const wmOpacity = ref(0.5)
const wmPos = ref<WmPos>('se')

const inputRef = ref<HTMLInputElement | null>(null)

const srcSize = computed(() =>
  img.value ? `${img.value.naturalWidth}×${img.value.naturalHeight}` : '—',
)

// 目标尺寸：留空则用原尺寸；按比例时另一边按原图比例推导
const target = computed(() => {
  if (!img.value) return { w: 0, h: 0 }
  const ow = img.value.naturalWidth
  const oh = img.value.naturalHeight
  let w = parseInt(resizeW.value, 10)
  let h = parseInt(resizeH.value, 10)
  if (!Number.isFinite(w) || w <= 0) w = 0
  if (!Number.isFinite(h) || h <= 0) h = 0
  if (keepRatio.value) {
    if (w && !h) h = Math.round((w / ow) * oh)
    else if (h && !w) w = Math.round((h / oh) * ow)
    else if (!w && !h) {
      w = ow
      h = oh
    }
  } else if (!w || !h) {
    w = w || ow
    h = h || oh
  }
  return { w: Math.max(1, w || ow), h: Math.max(1, h || oh) }
})

const mime = computed(() =>
  fmt.value === 'jpeg' ? 'image/jpeg' : fmt.value === 'webp' ? 'image/webp' : 'image/png',
)

function render(): HTMLCanvasElement | null {
  const source = img.value
  if (!source) return null
  const { w, h } = target.value
  const canvas = document.createElement('canvas')
  canvas.width = w
  canvas.height = h
  const ctx = canvas.getContext('2d')
  if (!ctx) return null
  if (fmt.value === 'jpeg') {
    ctx.fillStyle = '#ffffff'
    ctx.fillRect(0, 0, w, h)
  }
  ctx.imageSmoothingQuality = 'high'
  ctx.drawImage(source, 0, 0, w, h)
  if (wmOn.value && wmText.value) {
    const fs = Math.max(
      8,
      Math.round(wmSize.value * (h / Math.max(1, source.naturalHeight)) || wmSize.value),
    )
    ctx.globalAlpha = Math.min(1, Math.max(0.05, wmOpacity.value))
    ctx.font = `${fs}px sans-serif`
    ctx.fillStyle = '#ffffff'
    ctx.shadowColor = 'rgba(0,0,0,0.6)'
    ctx.shadowBlur = Math.max(2, fs / 6)
    const pad = Math.round(fs * 0.8)
    const m = ctx.measureText(wmText.value)
    const tw = m.width
    const th = fs
    let x = pad
    let y = h - pad
    if (wmPos.value === 'se') {
      x = pad
      y = h - pad
    } else if (wmPos.value === 'sw') {
      x = pad
      y = h - pad
    } else if (wmPos.value === 'ne') {
      x = w - tw - pad
      y = pad + th
    } else if (wmPos.value === 'nw') {
      x = pad
      y = pad + th
    } else {
      x = (w - tw) / 2
      y = (h + th) / 2
    }
    if (wmPos.value === 'sw') x = w - tw - pad
    ctx.fillText(wmText.value, x, y)
    ctx.globalAlpha = 1
    ctx.shadowBlur = 0
  }
  return canvas
}

const previewUrl = ref('')
const outSize = ref(0)

function refresh() {
  const canvas = render()
  if (!canvas) {
    previewUrl.value = ''
    outSize.value = 0
    return
  }
  const url = canvas.toDataURL(mime.value, quality.value)
  previewUrl.value = url
  // base64 → 估算字节数
  const b64 = url.split(',')[1] || ''
  outSize.value = Math.round((b64.length * 3) / 4)
}

function canvasBase64(): string {
  const canvas = render()
  if (!canvas) return ''
  const url = canvas.toDataURL(mime.value, quality.value)
  return url.split(',')[1] || ''
}

const fmtLabel = computed(() => (fmt.value === 'jpeg' ? 'jpg' : fmt.value))
const outName = computed(() => {
  const base = fileName.value.replace(/\.[^.]+$/, '') || 'image'
  return `${base}.${fmtLabel.value}`
})

function humanSize(n: number): string {
  if (!n) return '—'
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / 1024 / 1024).toFixed(2)} MB`
}

function loadImageFromSrc(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const im = new Image()
    im.onload = () => resolve(im)
    im.onerror = () => reject(new Error('decode failed'))
    im.src = src
  })
}

async function applySrc(src: string, name: string) {
  error.value = ''
  status.value = ''
  try {
    img.value = await loadImageFromSrc(src)
    fileName.value = name
    resizeW.value = String(img.value.naturalWidth)
    resizeH.value = String(img.value.naturalHeight)
    refresh()
  } catch (e) {
    error.value = String(e)
  }
}

function extOf(path: string): string {
  const m = path.toLowerCase().match(/\.([a-z0-9]+)$/)
  return m?.[1] ?? ''
}
function mimeOf(path: string): string {
  const e = extOf(path)
  if (e === 'jpg' || e === 'jpeg') return 'image/jpeg'
  if (e === 'webp') return 'image/webp'
  if (e === 'gif') return 'image/gif'
  if (e === 'bmp') return 'image/bmp'
  return 'image/png'
}
function toBase64(bytes: Uint8Array): string {
  let s = ''
  const chunk = 0x8000
  for (let i = 0; i < bytes.length; i += chunk) {
    s += String.fromCharCode(...bytes.subarray(i, i + chunk))
  }
  return btoa(s)
}

async function onFile(file: File) {
  await applySrc(URL.createObjectURL(file), file.name)
}
async function loadPath(path: string) {
  try {
    const bytes = await readFile(path)
    const dataUrl = `data:${mimeOf(path)};base64,${toBase64(bytes)}`
    await applySrc(dataUrl, path.split(/[\\/]/).pop() || path)
  } catch (e) {
    error.value = String(e)
  }
}

async function pick() {
  try {
    const picked = await open({
      multiple: false,
      filters: [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'gif'] }],
    })
    if (typeof picked === 'string') await loadPath(picked)
  } catch (e) {
    error.value = String(e)
  }
}

async function doSave() {
  if (!img.value) return
  busy.value = true
  error.value = ''
  status.value = ''
  try {
    const out = await save({
      defaultPath: outName.value,
      filters: [{ name: fmtLabel.value.toUpperCase(), extensions: [fmtLabel.value] }],
    })
    if (!out) return
    const data = canvasBase64()
    if (!data) return
    await invoke('write_file_base64', { path: out, dataBase64: data })
    status.value = t('image.saved', { out, size: humanSize(outSize.value) })
  } catch (e) {
    error.value = String(e)
  } finally {
    busy.value = false
  }
}

function reset() {
  resizeW.value = img.value ? String(img.value.naturalWidth) : ''
  resizeH.value = img.value ? String(img.value.naturalHeight) : ''
  wmOn.value = false
  wmText.value = ''
  quality.value = 0.85
  refresh()
}

let unDrop: (() => void) | null = null
onMounted(async () => {
  try {
    unDrop = await listen<{ path: string }>('app-file-drop', (e) => {
      if (settings.activeTool !== 'image') return
      const p = e.payload.path
      if (!/\.(png|jpe?g|webp|bmp|gif)$/i.test(p)) return
      void loadPath(p)
    })
  } catch {
    /* 忽略 */
  }
})
onUnmounted(() => unDrop?.())
</script>

<template>
  <div class="image-tool">
    <div class="image-body">
      <div class="dev-drop" @click="inputRef?.click()" @dragover.prevent @drop.prevent>
        <img v-if="previewUrl" :src="previewUrl" class="image-preview" alt="preview" />
        <div v-if="img" class="dev-drop-hint">
          {{ fileName }} · {{ srcSize }} · → {{ target.w }}×{{ target.h }} ·
          {{ humanSize(outSize) }}
        </div>
        <span v-else class="dev-drop-hint">{{ t('image.dropHint') }}</span>
        <input
          ref="inputRef"
          type="file"
          accept="image/*"
          class="dev-file-hidden"
          @change="onFile(($event.target as HTMLInputElement).files?.[0]!)"
        />
      </div>

      <div v-if="error" class="image-err">⚠ {{ error }}</div>

      <div v-if="img" class="image-grid">
        <div class="image-field">
          <label>{{ t('image.format') }}</label>
          <select v-model="fmt" class="image-select" @change="refresh">
            <option value="png">PNG</option>
            <option value="jpeg">JPEG</option>
            <option value="webp">WebP</option>
          </select>
        </div>
        <div class="image-field">
          <label>{{ t('image.quality') }} · {{ Math.round(quality * 100) }}%</label>
          <input
            v-model.number="quality"
            type="range"
            min="0.3"
            max="1"
            step="0.01"
            :disabled="fmt === 'png'"
            @input="refresh"
          />
        </div>
        <div class="image-field">
          <label>{{ t('image.width') }}</label>
          <input v-model="resizeW" type="number" min="1" class="image-input" @input="refresh" />
        </div>
        <div class="image-field">
          <label>{{ t('image.height') }}</label>
          <input v-model="resizeH" type="number" min="1" class="image-input" @input="refresh" />
        </div>
        <label class="image-check">
          <input v-model="keepRatio" type="checkbox" @change="refresh" />
          {{ t('image.keepRatio') }}
        </label>
      </div>

      <div v-if="img" class="image-section">
        <label class="image-check">
          <input v-model="wmOn" type="checkbox" @change="refresh" />
          {{ t('image.watermark') }}
        </label>
        <div v-if="wmOn" class="image-grid">
          <div class="image-field grow">
            <label>{{ t('image.wmText') }}</label>
            <input
              v-model="wmText"
              class="image-input"
              :placeholder="t('image.wmTextPh')"
              @input="refresh"
            />
          </div>
          <div class="image-field">
            <label>{{ t('image.wmSize') }} · {{ wmSize }}</label>
            <input v-model.number="wmSize" type="range" min="8" max="120" @input="refresh" />
          </div>
          <div class="image-field">
            <label>{{ t('image.wmOpacity') }} · {{ Math.round(wmOpacity * 100) }}%</label>
            <input
              v-model.number="wmOpacity"
              type="range"
              min="0.05"
              max="1"
              step="0.05"
              @input="refresh"
            />
          </div>
          <div class="image-field">
            <label>{{ t('image.wmPos') }}</label>
            <select v-model="wmPos" class="image-select" @change="refresh">
              <option value="se">{{ t('image.posSe') }}</option>
              <option value="sw">{{ t('image.posSw') }}</option>
              <option value="ne">{{ t('image.posNe') }}</option>
              <option value="nw">{{ t('image.posNw') }}</option>
              <option value="center">{{ t('image.posCenter') }}</option>
            </select>
          </div>
        </div>
      </div>

      <div v-if="img" class="image-actions">
        <button class="btn" @click="reset">{{ t('image.reset') }}</button>
        <button class="btn primary" :disabled="busy" @click="doSave">
          {{ busy ? t('image.saving') : t('image.save') }}
        </button>
      </div>

      <div v-if="status" class="image-ok">✓ {{ status }}</div>
    </div>
  </div>
</template>

<style scoped>
.image-tool {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.image-body {
  flex: 1;
  overflow: auto;
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.dev-drop {
  border: 1.5px dashed var(--border, #ccc);
  border-radius: 10px;
  padding: 20px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  cursor: pointer;
  min-height: 160px;
  justify-content: center;
}
.dev-drop:hover {
  border-color: var(--accent, #4a90d9);
}
.dev-file-hidden {
  display: none;
}
.dev-drop-hint {
  color: var(--muted, #888);
  font-size: 13px;
  text-align: center;
}
.image-preview {
  max-width: 100%;
  max-height: 300px;
  object-fit: contain;
  background-image:
    linear-gradient(45deg, #ddd 25%, transparent 25%),
    linear-gradient(-45deg, #ddd 25%, transparent 25%),
    linear-gradient(45deg, transparent 75%, #ddd 75%),
    linear-gradient(-45deg, transparent 75%, #ddd 75%);
  background-size: 16px 16px;
  background-position:
    0 0,
    0 8px,
    8px -8px,
    -8px 0;
}
.image-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 12px 16px;
  align-items: flex-end;
}
.image-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  color: var(--muted, #777);
  min-width: 120px;
}
.image-field.grow {
  flex: 1;
  min-width: 200px;
}
.image-input,
.image-select {
  padding: 6px 8px;
  border: 1px solid var(--border, #ccc);
  border-radius: 6px;
  background: var(--bg, #fff);
  color: inherit;
  font-size: 13px;
}
.image-check {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
}
.image-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px;
  border: 1px solid var(--border, #eee);
  border-radius: 8px;
}
.image-actions {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
}
.btn.primary {
  background: var(--accent, #4a90d9);
  color: #fff;
  border-color: transparent;
}
.image-err {
  color: #d9534f;
  font-size: 13px;
}
.image-ok {
  color: #4a9d5b;
  font-size: 13px;
}
</style>
