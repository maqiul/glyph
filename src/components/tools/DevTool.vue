<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import QRCode from 'qrcode'
import jsQR from 'jsqr'

const { t } = useI18n()
type Sub = 'json' | 'codec' | 'hash' | 'time' | 'uuid' | 'regex' | 'jwt' | 'color' | 'qr'
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
    timeOut.value = Number.isNaN(parsed) ? t('dev.invalid') : `${parsed} (ms)\n${new Date(parsed).toISOString()}`
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
    return { header: JSON.stringify(header, null, 2), payload: JSON.stringify(payload, null, 2), expInfo, signed: parts.length >= 3 }
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
      <button v-for="tab in (['json','codec','hash','time','uuid','regex','jwt','color','qr'] as const)" :key="tab" :class="{ active: active === tab }" @click="active = tab">{{ t(`dev.${tab}`) }}</button>
    </div>

    <div class="dev-body">
      <!-- JSON -->
      <div v-if="active === 'json'" class="dev-panel">
        <textarea v-model="jsonIn" class="dev-io" :placeholder="t('dev.input')" spellcheck="false"></textarea>
        <div class="dev-actions">
          <button class="btn" @click="jsonFormat">{{ t('dev.format') }}</button>
          <button class="btn" @click="jsonMinify">{{ t('dev.minify') }}</button>
          <button class="btn" :disabled="!jsonOut" @click="copy(jsonOut)">{{ t('dev.copy') }}</button>
        </div>
        <div v-if="jsonErr" class="dev-err">⚠ {{ jsonErr }}</div>
        <textarea :value="jsonOut" class="dev-io" readonly :placeholder="t('dev.output')" spellcheck="false"></textarea>
      </div>

      <!-- Codec -->
      <div v-else-if="active === 'codec'" class="dev-panel">
        <textarea v-model="codecIn" class="dev-io" :placeholder="t('dev.input')" spellcheck="false"></textarea>
        <div class="dev-actions">
          <button class="btn" @click="codec('b64e')">Base64 ↑</button>
          <button class="btn" @click="codec('b64d')">Base64 ↓</button>
          <button class="btn" @click="codec('urle')">URL ↑</button>
          <button class="btn" @click="codec('urld')">URL ↓</button>
          <button class="btn" @click="codec('hexe')">Hex</button>
          <button class="btn" :disabled="!codecOut" @click="copy(codecOut)">{{ t('dev.copy') }}</button>
        </div>
        <textarea :value="codecOut" class="dev-io" readonly :placeholder="t('dev.output')" spellcheck="false"></textarea>
      </div>

      <!-- Hash -->
      <div v-else-if="active === 'hash'" class="dev-panel">
        <textarea v-model="hashIn" class="dev-io" :placeholder="t('dev.input')" spellcheck="false"></textarea>
        <div class="dev-actions">
          <button class="btn" @click="doHash('SHA-1')">SHA-1</button>
          <button class="btn" @click="doHash('SHA-256')">SHA-256</button>
          <button class="btn" @click="doHash('SHA-384')">SHA-384</button>
          <button class="btn" @click="doHash('SHA-512')">SHA-512</button>
          <button class="btn" :disabled="!hashOut" @click="copy(hashOut)">{{ t('dev.copy') }}</button>
        </div>
        <input :value="hashOut" class="dev-line" readonly spellcheck="false" />
      </div>

      <!-- Time -->
      <div v-else-if="active === 'time'" class="dev-panel">
        <div class="dev-actions">
          <input v-model="timeIn" class="dev-line" :placeholder="t('dev.timePh')" @input="renderTime" />
          <button class="btn" @click="nowStamp">{{ t('dev.now') }}</button>
        </div>
        <textarea :value="timeOut" class="dev-io" readonly :placeholder="t('dev.output')"></textarea>
      </div>

      <!-- UUID -->
      <div v-else-if="active === 'uuid'" class="dev-panel">
        <div class="dev-actions">
          <label class="cfg-label">{{ t('dev.count') }}</label>
          <input v-model.number="uuidCount" type="number" min="1" max="100" class="dev-num" />
          <button class="btn" @click="genUuid">{{ t('dev.generate') }}</button>
          <button class="btn" :disabled="!uuidList.length" @click="copy(uuidList.join('\n'))">{{ t('dev.copy') }}</button>
        </div>
        <textarea :value="uuidList.join('\n')" class="dev-io" readonly spellcheck="false"></textarea>
      </div>

      <!-- Regex -->
      <div v-else-if="active === 'regex'" class="dev-panel">
        <div class="dev-actions">
          <input v-model="rePattern" class="dev-line" :placeholder="t('dev.pattern')" spellcheck="false" />
          <input v-model="reFlags" class="dev-num" :placeholder="t('dev.flags')" style="width: 70px" />
        </div>
        <textarea v-model="reText" class="dev-io" :placeholder="t('dev.testText')" spellcheck="false"></textarea>
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
        <textarea v-model="jwtIn" class="dev-io" placeholder="eyJhbGciOi..." spellcheck="false" style="min-height: 70px; flex: none"></textarea>
        <template v-if="jwt && !('error' in jwt)">
          <div class="dev-section-head">Header</div>
          <textarea :value="jwt.header" class="dev-io" readonly spellcheck="false"></textarea>
          <div class="dev-section-head">Payload <span v-if="jwt.expInfo" class="jwt-exp">· exp: {{ jwt.expInfo }}</span></div>
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
          <div class="ci-row"><span>对比度</span><span class="ci-ratio">{{ color.ratio }}:1</span></div>
          <div class="ci-row"><span>WCAG AA (4.5)</span><span>{{ color.aa ? '✅ 通过' : '❌ 不通过' }}</span></div>
          <div class="ci-row"><span>WCAG AA 大字 (3)</span><span>{{ color.aaLarge ? '✅' : '❌' }}</span></div>
          <div class="ci-row"><span>WCAG AAA (7)</span><span>{{ color.aaa ? '✅' : '❌' }}</span></div>
          <div class="ci-row"><span>FG RGB</span><span class="mono">{{ color.fgRgb }}</span></div>
          <div class="ci-row"><span>BG RGB</span><span class="mono">{{ color.bgRgb }}</span></div>
        </div>
        <div v-else class="dev-err">⚠ 请输入合法的 6 位十六进制颜色（如 #0969da）</div>
      </div>

      <!-- QR -->
      <div v-else-if="active === 'qr'" class="dev-panel">
        <div class="dev-section-head">{{ t('dev.qrGen') }}</div>
        <textarea v-model="qrText" class="dev-io" :placeholder="t('dev.input')" style="min-height: 60px; flex: none" spellcheck="false"></textarea>
        <div v-if="qrDataUrl" class="qr-preview">
          <img :src="qrDataUrl" alt="QR" />
          <button class="btn" @click="copy(qrText)">{{ t('dev.copy') }}</button>
        </div>
        <div class="dev-section-head" style="margin-top: 8px">{{ t('dev.qrDecode') }}</div>
        <input type="file" accept="image/*" @change="onQrFile" class="dev-file" />
        <div v-if="qrErr" class="dev-err">⚠ {{ qrErr }}</div>
        <textarea v-if="qrDecoded" :value="qrDecoded" class="dev-io" readonly spellcheck="false" style="min-height: 60px"></textarea>
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
</style>