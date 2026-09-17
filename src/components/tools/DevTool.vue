<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
type Sub = 'json' | 'codec' | 'hash' | 'time' | 'uuid' | 'regex'
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
  const bytes = new TextEncoder().encode(s)
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('')
}
function codec(mode: 'b64e' | 'b64d' | 'urle' | 'urld' | 'hexe') {
  try {
    const i = codecIn.value
    if (mode === 'b64e') codecOut.value = b64Encode(i)
    else if (mode === 'b64d') codecOut.value = b64Decode(i)
    else if (mode === 'urle') codecOut.value = encodeURIComponent(i)
    else if (mode === 'urld') codecOut.value = decodeURIComponent(i)
    else if (mode === 'hexe') codecOut.value = hexEncode(i)
  } catch (e) {
    codecOut.value = String(e)
  }
}

// ---- Hash ----
const hashIn = ref('')
const hashOut = ref('')
async function doHash(algo: string) {
  const bytes = new TextEncoder().encode(hashIn.value)
  const buf = await crypto.subtle.digest(algo, bytes)
  hashOut.value = Array.from(new Uint8Array(buf), (b) => b.toString(16).padStart(2, '0')).join('')
}

// ---- Time ----
const timeIn = ref('')
const timeOut = ref('')
function nowStamp() {
  const n = Date.now()
  timeIn.value = String(n)
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
    timeOut.value = Number.isNaN(parsed) ? t('dev.invalid') : `${parsed}\n(ms) ${new Date(parsed).toISOString()}`
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
const reMatches = computed(() => {
  if (!rePattern.value) return []
  try {
    const re = new RegExp(rePattern.value, reFlags.value.includes('g') ? reFlags.value : reFlags.value + 'g')
    return Array.from(reText.value.matchAll(re)).map((m) => m[0])
  } catch {
    return null as unknown as string[]
  }
})
const reError = computed(() => reMatches.value === null)

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
      <button :class="{ active: active === 'json' }" @click="active = 'json'">{{ t('dev.json') }}</button>
      <button :class="{ active: active === 'codec' }" @click="active = 'codec'">{{ t('dev.codec') }}</button>
      <button :class="{ active: active === 'hash' }" @click="active = 'hash'">{{ t('dev.hash') }}</button>
      <button :class="{ active: active === 'time' }" @click="active = 'time'">{{ t('dev.time') }}</button>
      <button :class="{ active: active === 'uuid' }" @click="active = 'uuid'">{{ t('dev.uuid') }}</button>
      <button :class="{ active: active === 'regex' }" @click="active = 'regex'">{{ t('dev.regex') }}</button>
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
        <textarea v-model="jsonOut" class="dev-io" readonly :placeholder="t('dev.output')" spellcheck="false"></textarea>
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
        <textarea v-model="codecOut" class="dev-io" readonly :placeholder="t('dev.output')" spellcheck="false"></textarea>
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
        <input v-model="hashOut" class="dev-line" readonly spellcheck="false" />
      </div>

      <!-- Time -->
      <div v-else-if="active === 'time'" class="dev-panel">
        <div class="dev-actions">
          <input v-model="timeIn" class="dev-line" :placeholder="t('dev.timePh')" @input="renderTime" />
          <button class="btn" @click="nowStamp">{{ t('dev.now') }}</button>
        </div>
        <textarea v-model="timeOut" class="dev-io" readonly :placeholder="t('dev.output')"></textarea>
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
          <div class="dev-matches-head">{{ t('dev.matches') }}: {{ reMatches.length }}</div>
          <ul class="dev-match-list">
            <li v-for="(m, i) in reMatches" :key="i">{{ m }}</li>
          </ul>
        </div>
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
</style>