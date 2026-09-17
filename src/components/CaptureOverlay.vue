<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
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

const { t } = useI18n()
const monitorIndex = Number(new URLSearchParams(window.location.search).get('monitor') ?? '0')
const dpr = window.devicePixelRatio || 1

const bg = ref('')
let img: HTMLImageElement | null = null
const start = ref<{ x: number; y: number } | null>(null)
const sel = ref<{ x: number; y: number; w: number; h: number } | null>(null)
const ready = ref(false)

const selStyle = computed(() => {
  const s = sel.value
  if (!s) return {}
  return { left: `${s.x}px`, top: `${s.y}px`, width: `${s.w}px`, height: `${s.h}px` }
})

async function load() {
  try {
    const caps = await invoke<CaptureResult[]>('capture_screens')
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

function normRect(a: { x: number; y: number }, b: { x: number; y: number }) {
  return {
    x: Math.min(a.x, b.x),
    y: Math.min(a.y, b.y),
    w: Math.abs(a.x - b.x),
    h: Math.abs(a.y - b.y),
  }
}

function onDown(e: MouseEvent) {
  start.value = { x: e.clientX, y: e.clientY }
  sel.value = null
}

function onMove(e: MouseEvent) {
  if (!start.value) return
  sel.value = normRect(start.value, { x: e.clientX, y: e.clientY })
}

async function onUp() {
  const s = sel.value
  if (!s || s.w < 3 || s.h < 3 || !ready.value || !img) {
    await cancel()
    return
  }
  // CSS(DIP) 坐标 × dpr = 截图像素坐标
  const sx = Math.round(s.x * dpr)
  const sy = Math.round(s.y * dpr)
  const sw = Math.round(s.w * dpr)
  const sh = Math.round(s.h * dpr)
  const canvas = document.createElement('canvas')
  canvas.width = sw
  canvas.height = sh
  const ctx = canvas.getContext('2d')
  if (!ctx) {
    await cancel()
    return
  }
  ctx.drawImage(img, sx, sy, sw, sh, 0, 0, sw, sh)
  const dataUrl = canvas.toDataURL('image/png')
  const png_base64 = dataUrl.split(',')[1] ?? ''
  await emit('capture-region', { png_base64, width: sw, height: sh, monitor: monitorIndex })
  await getCurrentWindow().close()
}

async function cancel() {
  await emit('capture-cancelled')
  await getCurrentWindow().close()
}

function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape') cancel()
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
    @mousedown="onDown"
    @mousemove="onMove"
    @mouseup="onUp"
  >
    <img v-if="bg" class="bg" :src="bg" alt="" draggable="false" />
    <div v-if="sel" class="sel" :style="selStyle">
      <span class="dim">{{ sel?.w }} × {{ sel?.h }}</span>
    </div>
    <div v-if="!sel && !start" class="hint">
      {{ t('screenshot.overlayHint') }}
      <kbd>Esc</kbd> {{ t('screenshot.overlayCancel') }}
    </div>
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
</style>