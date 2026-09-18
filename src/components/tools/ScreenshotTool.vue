<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open, save } from '@tauri-apps/plugin-dialog'
import { join, downloadDir } from '@tauri-apps/api/path'
import { useSettingsStore } from '../../stores/settings'
import { savePersisted } from '../../stores/persistent'

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
const settings = useSettingsStore()
const captures = ref<CaptureResult[]>([])
const capturing = ref(false)
const error = ref<string | null>(null)
const saveToast = ref<string | null>(null)

async function capture() {
  capturing.value = true
  error.value = null
  try {
    const caps = await invoke<CaptureResult[]>('capture_screens')
    captures.value = caps
    // 默认自动存到目标目录（未设保存目录则存系统下载）
    for (const c of caps) await saveToDir(c)
  } catch (e: unknown) {
    const msg = typeof e === 'string' ? e : ((e as any)?.message ?? JSON.stringify(e))
    error.value = t('screenshot.failed', { msg })
  } finally {
    capturing.value = false
  }
}

function tsName(): string {
  const d = new Date()
  const pad = (n: number) => String(n).padStart(2, '0')
  return `glyph-${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}-${pad(d.getHours())}${pad(d.getMinutes())}${pad(d.getSeconds())}.png`
}

async function targetDir(): Promise<string> {
  return settings.screenshotDir || (await downloadDir())
}

async function saveToDir(c: CaptureResult) {
  try {
    const dir = await targetDir()
    const path = await join(dir, tsName())
    await invoke('write_file_base64', { path, dataBase64: c.png_base64 })
    saveToast.value = t('screenshot.savedTo', { path })
    setTimeout(() => (saveToast.value = null), 2500)
  } catch (e: unknown) {
    error.value = t('screenshot.saveFailed', { msg: String(e) })
  }
}

async function saveAs(c: CaptureResult) {
  try {
    const target = await save({
      title: t('screenshot.saveAs'),
      defaultPath: tsName(),
      filters: [{ name: 'PNG', extensions: ['png'] }],
    })
    if (!target) return
    await invoke('write_file_base64', { path: target, dataBase64: c.png_base64 })
    saveToast.value = t('screenshot.savedTo', { path: target })
    setTimeout(() => (saveToast.value = null), 2500)
  } catch (e: unknown) {
    error.value = t('screenshot.saveFailed', { msg: String(e) })
  }
}

async function copyShot(c: CaptureResult) {
  try {
    await invoke('copy_image_to_clipboard', { imageBase64: c.png_base64 })
    saveToast.value = t('screenshot.copiedImg')
    setTimeout(() => (saveToast.value = null), 2500)
  } catch (e: unknown) {
    error.value = t('screenshot.copyFailed', { msg: String(e) })
  }
}

async function chooseDir() {
  try {
    const dir = await open({
      directory: true,
      multiple: false,
      defaultPath: settings.screenshotDir || undefined,
    })
    if (dir && typeof dir === 'string') {
      settings.setScreenshotDir(dir)
      savePersisted()
    }
  } catch (e) {
    console.warn('choose dir failed:', e)
  }
}

function clearDir() {
  settings.setScreenshotDir('')
  savePersisted()
}

const dirLabel = computed(() => {
  if (!settings.screenshotDir) return t('screenshot.dirDefault')
  const parts = settings.screenshotDir.split(/[/\\]/)
  return parts[parts.length - 1] || settings.screenshotDir
})

const ocrBusy = ref(false)
const ocrResults = ref<Record<number, string>>({})
const ocrError = ref<string | null>(null)

async function recognizeShot(c: CaptureResult) {
  ocrBusy.value = true
  ocrError.value = null
  try {
    const txt =
      settings.ocrEngine === 'cloud'
        ? await invoke<string>('ocr_recognize_cloud_base64', {
            imageBase64: c.png_base64,
            provider: settings.ocrProvider,
            apiKey: settings.ocrApiKey,
            secretKey: settings.ocrSecretKey,
          })
        : await invoke<string>('ocr_recognize_base64', { imageBase64: c.png_base64 })
    ocrResults.value = { ...ocrResults.value, [c.index]: txt || '' }
  } catch (e: unknown) {
    ocrError.value = t('screenshot.ocrFailed', { msg: String(e) })
  } finally {
    ocrBusy.value = false
  }
}

async function restoreMain() {
  const w = getCurrentWindow()
  await w.show()
  await w.setFocus()
}

async function regionCapture() {
  if (capturing.value) return
  capturing.value = true
  error.value = null
  const label = 'capture-overlay'
  try {
    await getCurrentWindow().hide()
    // 等主窗从屏幕消失，避免截到主窗残影
    await new Promise((r) => setTimeout(r, 250))
    // 预截屏并缓存，overlay 只读缓存（不再自我截屏 → 不白屏）
    await invoke('capture_screens')
    const existing = await WebviewWindow.getByLabel(label)
    if (existing) await existing.close()

    const unRegion = await listen<{
      png_base64: string
      width: number
      height: number
      monitor: number
    }>('capture-region', async (e) => {
      const p = e.payload
      const newCap: CaptureResult = {
        index: captures.value.length,
        name: `Region ${p.width}×${p.height}`,
        x: 0,
        y: 0,
        width: p.width,
        height: p.height,
        png_base64: p.png_base64,
      }
      captures.value = [...captures.value, newCap]
      unRegion()
      unCancel()
      await restoreMain()
      await saveToDir(newCap)
      capturing.value = false
    })
    const unCancel = await listen('capture-cancelled', async () => {
      unRegion()
      unCancel()
      await restoreMain()
      capturing.value = false
    })

    new WebviewWindow(label, {
      url: '/?mode=capture&monitor=0',
      transparent: true,
      decorations: false,
      alwaysOnTop: true,
      skipTaskbar: true,
      fullscreen: true,
      resizable: false,
      focus: true,
    })
  } catch (e: unknown) {
    error.value = t('screenshot.failed', { msg: String(e) })
    await restoreMain()
    capturing.value = false
  }
}

// 全局截图快捷键：无论组件是否刚挂载，消费 pendingCapture 标记后自动发起框选
function consumePendingCapture() {
  if (settings.pendingCapture) {
    settings.pendingCapture = false
    void regionCapture()
  }
}
onMounted(consumePendingCapture)
watch(() => settings.pendingCapture, consumePendingCapture)
</script>

<template>
  <div class="shot-tool">
    <div class="shot-toolbar">
      <div class="shot-head">
        <h1>{{ t('placeholder.screenshotTitle') }}</h1>
        <p class="shot-hint">{{ t('screenshot.hint') }}</p>
      </div>
      <div class="shot-actions">
        <button class="btn" :disabled="capturing" @click="regionCapture">
          {{ t('screenshot.region') }}
        </button>
        <button class="btn btn-primary" :disabled="capturing" @click="capture">
          {{ capturing ? t('screenshot.capturing') : t('screenshot.capture') }}
        </button>
      </div>
    </div>

    <div class="shot-config">
      <span class="cfg-label">{{ t('screenshot.dirLabel') }}</span>
      <button class="btn btn-small dir-btn" :title="settings.screenshotDir" @click="chooseDir">
        {{ dirLabel }}
      </button>
      <button v-if="settings.screenshotDir" class="btn btn-small" @click="clearDir">✕</button>
    </div>

    <div v-if="error" class="shot-error">⚠ {{ error }}</div>

    <div v-if="captures.length" class="shot-results">
      <div class="shot-results-head">
        {{ t('screenshot.resultTitle') }} · {{ t('screenshot.screens', { n: captures.length }) }}
      </div>
      <div class="shot-grid">
        <div v-for="c in captures" :key="c.index" class="shot-card">
          <div class="shot-card-head">
            <span class="shot-name">{{ c.name }}</span>
            <span class="shot-dim">{{ c.width }}×{{ c.height }}</span>
          </div>
          <div class="shot-thumb">
            <img :src="`data:image/png;base64,${c.png_base64}`" :alt="c.name" />
          </div>
          <div class="shot-card-foot">
            <span class="shot-pos">@ {{ c.x }}, {{ c.y }}</span>
            <div class="shot-card-actions">
              <button class="btn btn-small" @click="copyShot(c)">
                {{ t('screenshot.copyImg') }}
              </button>
              <button class="btn btn-small" @click="saveToDir(c)">
                {{ t('screenshot.saveToDir') }}
              </button>
              <button class="btn btn-small" @click="saveAs(c)">{{ t('screenshot.saveAs') }}</button>
              <button class="btn btn-small btn-ocr" :disabled="ocrBusy" @click="recognizeShot(c)">
                {{ ocrBusy ? t('screenshot.ocrBusy') : t('screenshot.ocrBtn') }}
              </button>
            </div>
          </div>
          <div v-if="ocrResults[c.index] !== undefined" class="shot-ocr">
            <div class="shot-ocr-head">{{ t('screenshot.ocrResult') }}</div>
            <textarea
              class="shot-ocr-text"
              readonly
              :value="ocrResults[c.index] || '（未识别到文字）'"
            ></textarea>
          </div>
        </div>
      </div>
    </div>

    <div v-if="saveToast" class="save-toast">{{ saveToast }}</div>
  </div>
</template>

<style scoped>
.shot-tool {
  height: 100%;
  overflow-y: auto;
  padding: 24px 28px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.shot-toolbar {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 20px;
}

.shot-head h1 {
  font-size: 22px;
  font-weight: 700;
  margin: 0 0 6px;
  color: var(--text);
}

.shot-hint {
  font-size: 13px;
  color: var(--text-muted);
  margin: 0;
  max-width: 560px;
  line-height: 1.5;
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

.shot-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.shot-error {
  color: var(--error);
  font-size: 13px;
  padding: 10px 14px;
  background: var(--surface);
  border: 1px solid var(--error);
  border-radius: 6px;
}

.shot-results-head {
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-muted);
  margin-bottom: 12px;
}

.shot-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
}

.shot-card {
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
  background: var(--surface);
  display: flex;
  flex-direction: column;
}

.shot-card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
}

.shot-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

.shot-dim {
  font-size: 12px;
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.shot-thumb {
  background: repeating-conic-gradient(var(--code-bg) 0% 25%, transparent 0% 50%) 50% / 16px 16px;
  border-top: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
  aspect-ratio: 16 / 10;
  overflow: hidden;
}

.shot-thumb img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
}

.shot-card-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
}

.shot-pos {
  font-size: 11px;
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.shot-card-actions {
  display: flex;
  gap: 6px;
}

.save-toast {
  position: fixed;
  bottom: 24px;
  right: 24px;
  background: var(--accent);
  color: var(--accent-fg);
  padding: 10px 16px;
  border-radius: 6px;
  font-size: 13px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
  z-index: 100;
  max-width: 70%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.shot-config {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface);
}

.cfg-label {
  font-size: 12px;
  color: var(--text-muted);
  white-space: nowrap;
}

.dir-btn {
  max-width: 360px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.btn-ocr {
  border-color: var(--accent);
  color: var(--accent);
}

.btn-ocr:hover {
  background: var(--accent);
  color: var(--accent-fg);
}

.shot-ocr {
  border-top: 1px solid var(--border);
  padding: 8px 12px;
}

.shot-ocr-head {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-muted);
  margin-bottom: 6px;
}

.shot-ocr-text {
  width: 100%;
  min-height: 80px;
  resize: vertical;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--text);
  font-size: 13px;
  line-height: 1.5;
  font-family: var(--font-sans);
}
</style>
