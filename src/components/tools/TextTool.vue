<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
type Sub = 'diff' | 'json2ts'
const active = ref<Sub>('diff')

// ---- Diff（行级 LCS）----
const diffA = ref('')
const diffB = ref('')
interface DiffRow {
  type: 'eq' | 'add' | 'del'
  text: string
  an: number
  bn: number
}
const diffRows = computed<DiffRow[]>(() => {
  if (!diffA.value && !diffB.value) return []
  const a = diffA.value.split('\n')
  const b = diffB.value.split('\n')
  const n = a.length
  const m = b.length
  const dp: Uint32Array[] = Array.from({ length: n + 1 }, () => new Uint32Array(m + 1))
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      dp[i]![j] = a[i] === b[j] ? dp[i + 1]![j + 1]! + 1 : Math.max(dp[i + 1]![j]!, dp[i]![j + 1]!)
    }
  }
  const rows: DiffRow[] = []
  let i = 0
  let j = 0
  while (i < n && j < m) {
    if (a[i] === b[j]) {
      rows.push({ type: 'eq', text: a[i]!, an: i + 1, bn: j + 1 })
      i++
      j++
    } else if (dp[i + 1]![j]! >= dp[i]![j + 1]!) {
      rows.push({ type: 'del', text: a[i]!, an: i + 1, bn: 0 })
      i++
    } else {
      rows.push({ type: 'add', text: b[j]!, an: 0, bn: j + 1 })
      j++
    }
  }
  while (i < n) rows.push({ type: 'del', text: a[i++]!, an: i, bn: 0 })
  while (j < m) rows.push({ type: 'add', text: b[j++]!, an: 0, bn: j })
  return rows
})
const diffStats = computed(() => {
  let add = 0
  let del = 0
  for (const r of diffRows.value) {
    if (r.type === 'add') add++
    else if (r.type === 'del') del++
  }
  return { add, del }
})
function swapDiff() {
  const tmp = diffA.value
  diffA.value = diffB.value
  diffB.value = tmp
}
function clearDiff() {
  diffA.value = ''
  diffB.value = ''
}

// ---- JSON → TypeScript ----
const tsIn = ref('')
const tsRoot = ref('Root')
const tsOptNull = ref(true)
const tsReadonly = ref(false)

function pascal(s: string): string {
  const parts = s
    .split(/[^a-zA-Z0-9]+/)
    .filter(Boolean)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
  return parts.join('') || 'Value'
}
function singular(name: string): string {
  return /s$/i.test(name) ? name.slice(0, -1) : name + 'Item'
}
function buildType(
  value: unknown,
  nameHint: string,
  out: string[],
  optNull: boolean,
  ro: boolean,
): string {
  if (value === null) return 'null'
  if (Array.isArray(value)) {
    if (value.length === 0) return 'unknown[]'
    const set = new Set<string>()
    for (const item of value) set.add(buildType(item, singular(nameHint), out, optNull, ro))
    const parts = Array.from(set)
    const itemT = parts.length === 1 ? parts[0]! : `(${parts.join(' | ')})`
    return `${itemT}[]`
  }
  if (typeof value === 'object') {
    const iface = pascal(nameHint)
    const obj = value as Record<string, unknown>
    const lines: string[] = []
    for (const [k, v] of Object.entries(obj)) {
      const type = buildType(v, k, out, optNull, ro)
      const key = /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(k) ? k : `"${k}"`
      const opt = optNull && v === null ? '?' : ''
      lines.push(`  ${ro ? 'readonly ' : ''}${key}${opt}: ${type};`)
    }
    const body = lines.length ? `\n${lines.join('\n')}\n` : '\n'
    out.push(`export interface ${iface} {${body}}`)
    return iface
  }
  return typeof value
}
const tsResult = computed<{ code: string; error: string }>(() => {
  const s = tsIn.value.trim()
  if (!s) return { code: '', error: '' }
  try {
    const data: unknown = JSON.parse(s)
    const out: string[] = []
    const rootIface = pascal(tsRoot.value) || 'Root'
    if (data !== null && typeof data === 'object' && !Array.isArray(data)) {
      buildType(data, tsRoot.value, out, tsOptNull.value, tsReadonly.value)
      return { code: out.join('\n\n'), error: '' }
    }
    const type = buildType(data, tsRoot.value, out, tsOptNull.value, tsReadonly.value)
    return {
      code: (out.length ? out.join('\n\n') + '\n\n' : '') + `export type ${rootIface} = ${type};`,
      error: '',
    }
  } catch (e) {
    return { code: '', error: String(e) }
  }
})
const tsOut = computed(() => tsResult.value.code)
const tsErr = computed(() => tsResult.value.error)

async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text)
  } catch (e) {
    console.warn(e)
  }
}
function download(name: string, text: string) {
  const blob = new Blob([text], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = name
  a.click()
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}
</script>

<template>
  <div class="text-tool">
    <div class="text-tabs">
      <button :class="{ active: active === 'diff' }" @click="active = 'diff'">
        {{ t('text.diff') }}
      </button>
      <button :class="{ active: active === 'json2ts' }" @click="active = 'json2ts'">
        {{ t('text.json2ts') }}
      </button>
    </div>

    <div class="text-body">
      <!-- Diff -->
      <div v-if="active === 'diff'" class="text-panel">
        <div class="diff-inputs">
          <textarea
            v-model="diffA"
            class="text-io"
            :placeholder="t('text.origPh')"
            spellcheck="false"
          ></textarea>
          <textarea
            v-model="diffB"
            class="text-io"
            :placeholder="t('text.newPh')"
            spellcheck="false"
          ></textarea>
        </div>
        <div class="text-actions">
          <button class="btn" @click="swapDiff">{{ t('text.swap') }}</button>
          <button class="btn" @click="clearDiff">
            {{ t('text.clear') }}
          </button>
          <span v-if="diffRows.length" class="diff-stats">
            <b class="add">+{{ diffStats.add }}</b>
            <b class="del">−{{ diffStats.del }}</b>
          </span>
        </div>
        <div v-if="diffRows.length" class="diff-result">
          <div v-for="(r, idx) in diffRows" :key="idx" class="diff-line" :class="r.type">
            <span class="diff-no">{{ r.an || '' }}</span>
            <span class="diff-no">{{ r.bn || '' }}</span>
            <span class="diff-sign">{{
              r.type === 'add' ? '+' : r.type === 'del' ? '−' : ' '
            }}</span>
            <span class="diff-text">{{ r.text || ' ' }}</span>
          </div>
        </div>
      </div>

      <!-- JSON → TS -->
      <div v-else class="text-panel">
        <textarea
          v-model="tsIn"
          class="text-io"
          :placeholder="t('text.jsonPh')"
          spellcheck="false"
          style="min-height: 140px; flex: none"
        ></textarea>
        <div class="text-actions">
          <label class="cfg-label">{{ t('text.rootName') }}</label>
          <input v-model="tsRoot" class="text-line" spellcheck="false" style="max-width: 160px" />
          <label class="cfg-check">
            <input v-model="tsOptNull" type="checkbox" />
            {{ t('text.optNull') }}
          </label>
          <label class="cfg-check">
            <input v-model="tsReadonly" type="checkbox" />
            readonly
          </label>
          <button class="btn" :disabled="!tsOut" @click="copy(tsOut)">{{ t('text.copy') }}</button>
          <button
            class="btn"
            :disabled="!tsOut"
            @click="download(`${tsRoot || 'types'}.ts`, tsOut)"
          >
            {{ t('text.download') }}
          </button>
        </div>
        <div v-if="tsErr" class="text-err">⚠ {{ tsErr }}</div>
        <textarea
          :value="tsOut"
          class="text-io"
          readonly
          :placeholder="t('text.output')"
          spellcheck="false"
        ></textarea>
      </div>
    </div>
  </div>
</template>

<style scoped>
.text-tool {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.text-tabs {
  display: flex;
  gap: 2px;
  padding: 8px 14px;
  border-bottom: 1px solid var(--border);
  background: var(--surface);
}

.text-tabs button {
  border: 0;
  background: transparent;
  color: var(--text-muted);
  padding: 6px 16px;
  font-size: 13px;
  border-radius: 6px;
  cursor: pointer;
}

.text-tabs button.active {
  background: var(--accent);
  color: var(--accent-fg);
}

.text-body {
  flex: 1;
  overflow-y: auto;
  padding: 18px 22px;
}

.text-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
}

.text-io {
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

.text-io:focus {
  outline: none;
  border-color: var(--accent);
}

.diff-inputs {
  display: flex;
  gap: 12px;
  flex: none;
}

.diff-inputs .text-io {
  min-height: 160px;
}

.text-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.cfg-label {
  font-size: 12px;
  color: var(--text-muted);
}

.cfg-check {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  color: var(--text-muted);
  cursor: pointer;
}

.text-line {
  padding: 7px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--text);
  font-size: 13px;
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

.text-err {
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

.diff-stats {
  font-size: 13px;
  font-family: var(--font-mono);
}

.diff-stats .add {
  color: #1a7f37;
  margin-right: 8px;
}

.diff-stats .del {
  color: #cf222e;
}

.diff-result {
  flex: 1;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg);
  font-family: var(--font-mono);
  font-size: 12.5px;
}

.diff-line {
  display: flex;
  gap: 8px;
  padding: 1px 10px;
  white-space: pre-wrap;
  word-break: break-word;
}

.diff-line.eq {
  color: var(--text-muted);
}

.diff-line.add {
  background: rgba(26, 127, 55, 0.12);
}

.diff-line.del {
  background: rgba(207, 34, 46, 0.12);
}

.diff-no {
  flex: 0 0 28px;
  text-align: right;
  color: var(--text-muted);
  user-select: none;
  opacity: 0.7;
}

.diff-sign {
  flex: 0 0 12px;
  user-select: none;
}

.diff-text {
  flex: 1;
  min-width: 0;
}
</style>
