<script setup lang="ts">
import { ref, computed, reactive, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { LazyStore } from '@tauri-apps/plugin-store'
import JsonViewer from './JsonViewer.vue'

// Postman 式 HTTP 客户端：请求（Params/Headers/Body 分页 + 键值表格）+ 响应（Pretty/Raw/Headers）
// + 历史记录 + 已保存请求（Tauri plugin-store 本地持久化）。实际请求走 Rust 后端 ureq 代理绕开 CORS。

const { t } = useI18n()

type Method = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE' | 'HEAD' | 'OPTIONS'
const METHODS: Method[] = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS']
const METHOD_COLOR: Record<Method, string> = {
  GET: '#1a7f37',
  POST: '#d97706',
  PUT: '#0969da',
  PATCH: '#8250df',
  DELETE: '#cf222e',
  HEAD: '#6e7781',
  OPTIONS: '#6e7781',
}

interface Kv {
  enabled: boolean
  key: string
  value: string
}
type BodyMode = 'none' | 'form-data' | 'urlencoded' | 'raw'
type RawLang = 'json' | 'text' | 'xml'

interface HttpRequest {
  id: string
  name: string
  method: Method
  url: string
  params: Kv[]
  headers: Kv[]
  bodyMode: BodyMode
  rawLang: RawLang
  rawBody: string
  formData: Kv[]
  urlencoded: Kv[]
  timeout: number
}

interface HttpResp {
  status: number
  statusText: string
  ok: boolean
  headers: [string, string][]
  body: string
  truncated: boolean
  durationMs: number
  sizeBytes: number
}

interface HistoryItem {
  id: string
  method: Method
  url: string
  status: number
  ok: boolean
  at: number
  req: HttpRequest
}

function uid(): string {
  return Math.random().toString(36).slice(2) + Date.now().toString(36)
}
function kv(enabled = true, key = '', value = ''): Kv {
  return { enabled, key, value }
}
function blankReq(): HttpRequest {
  return {
    id: uid(),
    name: '',
    method: 'GET',
    url: '',
    params: [],
    headers: [kv(true, 'User-Agent', 'Glyph/0.2')],
    bodyMode: 'none',
    rawLang: 'json',
    rawBody: '',
    formData: [],
    urlencoded: [],
    timeout: 30,
  }
}

const req = reactive<HttpRequest>(blankReq())
const sending = ref(false)
const error = ref('')
const resp = ref<HttpResp | null>(null)

const reqTab = ref<'params' | 'headers' | 'body'>('params')
const resTab = ref<'body' | 'headers'>('body')
const viewMode = ref<'pretty' | 'raw'>('pretty')
const listTab = ref<'saved' | 'history'>('saved')
const listSearch = ref('')
const sidebarOpen = ref(true)

// 计数徽标：启用的行数
const paramCount = computed(() => req.params.filter((p) => p.enabled && p.key).length)
const headerCount = computed(() => req.headers.filter((h) => h.enabled && h.key).length)
const rawPlaceholder = computed(() =>
  req.rawLang === 'json' ? '{ "key": "value" }' : t('dev.httpBodyPh'),
)

// ---- 构造实际请求 ----
function enabled(rows: Kv[]): Kv[] {
  return rows.filter((r) => r.enabled && r.key.trim() !== '')
}

// 把启用的 Params 追加到 URL 查询串（URL 里已存在的同名键不重复追加）。
function effectiveUrl(): string {
  const base = req.url.trim()
  const rows = enabled(req.params)
  if (!rows.length) return base
  const [head, existingQuery = ''] = base.split('?')
  const existingKeys = new Set(
    existingQuery
      .split('&')
      .filter(Boolean)
      .map((kvStr) => decodeURIComponent(kvStr.split('=')[0] ?? '')),
  )
  const extra = rows
    .filter((r) => !existingKeys.has(r.key.trim()))
    .map((r) => `${encodeURIComponent(r.key.trim())}=${encodeURIComponent(r.value)}`)
  if (!extra.length) return base
  const q = (existingQuery ? existingQuery + '&' : '') + extra.join('&')
  return `${head}?${q}`
}

function buildBody(): { body: string; contentType: string } {
  if (req.bodyMode === 'raw') {
    const ct =
      req.rawLang === 'json'
        ? 'application/json'
        : req.rawLang === 'xml'
          ? 'application/xml'
          : 'text/plain'
    return { body: req.rawBody, contentType: ct }
  }
  if (req.bodyMode === 'urlencoded') {
    const usp = new URLSearchParams()
    for (const r of enabled(req.urlencoded)) usp.append(r.key.trim(), r.value)
    return { body: usp.toString(), contentType: 'application/x-www-form-urlencoded' }
  }
  if (req.bodyMode === 'form-data') {
    const boundary = '----GlyphBoundary' + Math.random().toString(36).slice(2)
    let out = ''
    for (const r of enabled(req.formData)) {
      out += `--${boundary}\r\nContent-Disposition: form-data; name="${r.key.trim()}"\r\n\r\n${r.value}\r\n`
    }
    out += `--${boundary}--\r\n`
    return { body: out, contentType: `multipart/form-data; boundary=${boundary}` }
  }
  return { body: '', contentType: '' }
}

function buildHeaders(contentType: string): [string, string][] {
  const rows = enabled(req.headers)
  const out: [string, string][] = rows.map((r) => [r.key.trim(), r.value])
  const hasCT = rows.some((r) => r.key.trim().toLowerCase() === 'content-type')
  if (contentType && !hasCT) out.push(['Content-Type', contentType])
  return out
}

async function send() {
  error.value = ''
  resp.value = null
  const url = effectiveUrl()
  if (!url) {
    error.value = t('dev.httpUrlRequired')
    return
  }
  const { body, contentType } = buildBody()
  const headers = buildHeaders(contentType)
  sending.value = true
  try {
    resp.value = await invoke<HttpResp>('http_request', {
      method: req.method,
      url,
      headers,
      body,
      timeoutSecs: req.timeout,
    })
    resTab.value = 'body'
    pushHistory()
  } catch (e) {
    error.value = String(e)
  } finally {
    sending.value = false
  }
}

// ---- 响应展示 ----
const isJson = computed(() => {
  const ct = resp.value?.headers.find((h) => h[0].toLowerCase() === 'content-type')?.[1] ?? ''
  if (ct.includes('json')) return true
  try {
    JSON.parse(resp.value?.body ?? '')
    return true
  } catch {
    return false
  }
})
const prettyBody = computed(() => {
  const b = resp.value?.body ?? ''
  if (!isJson.value) return b
  try {
    return JSON.stringify(JSON.parse(b), null, 2)
  } catch {
    return b
  }
})
const statusColor = computed(() => {
  const s = resp.value?.status ?? 0
  if (s >= 500) return '#cf222e'
  if (s >= 400) return '#d97706'
  if (s >= 300) return '#0969da'
  if (s >= 200) return '#1a7f37'
  return '#6e7781'
})
function fmtSize(n: number): string {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / 1024 / 1024).toFixed(2)} MB`
}

// ---- 保存 / 历史 / 持久化 ----
const STORE_FILE = 'http-tool.json'
let store: LazyStore | null = null
function getStore(): LazyStore {
  if (!store) store = new LazyStore(STORE_FILE)
  return store
}

const saved = ref<HttpRequest[]>([])
const history = ref<HistoryItem[]>([])

function snapshot(): HttpRequest {
  return JSON.parse(
    JSON.stringify({
      ...req,
      params: req.params,
      headers: req.headers,
      formData: req.formData,
      urlencoded: req.urlencoded,
    }),
  ) as HttpRequest
}

function saveReq() {
  if (!req.url.trim()) {
    error.value = t('dev.httpUrlRequired')
    return
  }
  if (!req.name.trim()) req.name = shortUrl(req.url) || req.method
  const idx = saved.value.findIndex((s) => s.id === req.id)
  const snap = snapshot()
  if (idx >= 0) saved.value[idx] = snap
  else {
    saved.value.unshift(snap)
    req.id = snap.id
  }
  listTab.value = 'saved'
  persist()
}

function newReq() {
  Object.assign(req, blankReq())
  resp.value = null
  error.value = ''
}

function loadReq(r: HttpRequest) {
  Object.assign(req, JSON.parse(JSON.stringify(r)))
  resp.value = null
  error.value = ''
}

function delReq(id: string) {
  saved.value = saved.value.filter((s) => s.id !== id)
  persist()
}

function pushHistory() {
  const item: HistoryItem = {
    id: uid(),
    method: req.method,
    url: req.url.trim(),
    status: resp.value?.status ?? 0,
    ok: resp.value?.ok ?? false,
    at: Date.now(),
    req: snapshot(),
  }
  history.value.unshift(item)
  if (history.value.length > 100) history.value.length = 100
  persist()
}

function clearHistory() {
  history.value = []
  persist()
}

function shortUrl(url: string): string {
  try {
    const u = new URL(url)
    return u.hostname + u.pathname.replace(/\/$/, '')
  } catch {
    return url.replace(/^https?:\/\//, '').slice(0, 40)
  }
}

async function persist() {
  try {
    const s = getStore()
    await s.set('saved', saved.value)
    await s.set('history', history.value.slice(0, 100))
    await s.set('last', { ...req })
    await s.save()
  } catch (e) {
    console.warn('persist http-tool failed:', e)
  }
}

let saveTimer: ReturnType<typeof setTimeout> | null = null
watch(
  req,
  () => {
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = setTimeout(persist, 500)
  },
  { deep: true },
)

onMounted(async () => {
  try {
    const s = getStore()
    const [sv, hi, last] = await Promise.all([
      s.get<HttpRequest[]>('saved'),
      s.get<HistoryItem[]>('history'),
      s.get<HttpRequest>('last'),
    ])
    if (sv) saved.value = sv
    if (hi) history.value = hi
    if (last) Object.assign(req, last)
  } catch (e) {
    console.warn('load http-tool failed:', e)
  }
})

// ---- 列表过滤 ----
const savedFiltered = computed(() => {
  const q = listSearch.value.trim().toLowerCase()
  if (!q) return saved.value
  return saved.value.filter((s) => (s.name + ' ' + s.url).toLowerCase().includes(q))
})
const historyFiltered = computed(() => {
  const q = listSearch.value.trim().toLowerCase()
  if (!q) return history.value
  return history.value.filter((h) => (h.method + ' ' + h.url).toLowerCase().includes(q))
})

function timeAgo(ts: number): string {
  const d = new Date(ts)
  return d.toLocaleString()
}

// ---- 复制 / 下载 ----
async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text)
  } catch (e) {
    console.warn(e)
  }
}
function downloadResp() {
  const b = resp.value?.body
  if (!b) return
  const ext = isJson.value ? 'json' : 'txt'
  const blob = new Blob([viewMode.value === 'pretty' ? prettyBody.value : b], {
    type: 'text/plain',
  })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `response.${ext}`
  a.click()
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}

// ---- 键值表格行操作 ----
function addRow(list: Kv[]) {
  list.push(kv())
}
function removeRow(list: Kv[], i: number) {
  list.splice(i, 1)
}
</script>

<template>
  <div class="http-tool">
    <!-- 左侧：已保存 + 历史 -->
    <aside class="http-side" :class="{ collapsed: !sidebarOpen }">
      <div class="side-head">
        <button :class="{ on: listTab === 'saved' }" @click="listTab = 'saved'">
          {{ t('dev.httpSavedTab') }}
        </button>
        <button :class="{ on: listTab === 'history' }" @click="listTab = 'history'">
          {{ t('dev.httpHistoryTab') }}
        </button>
      </div>
      <div class="side-search">
        <input v-model="listSearch" :placeholder="t('dev.httpSearch')" spellcheck="false" />
        <button class="icon-btn" :title="t('dev.httpNew')" @click="newReq">＋</button>
      </div>
      <div class="side-list">
        <template v-if="listTab === 'saved'">
          <div v-for="s in savedFiltered" :key="s.id" class="item" @click="loadReq(s)">
            <span class="m" :style="{ color: METHOD_COLOR[s.method] }">{{ s.method }}</span>
            <span class="label" :title="s.name || s.url">{{ s.name || shortUrl(s.url) }}</span>
            <button class="del" :title="t('dev.httpDelete')" @click.stop="delReq(s.id)">✕</button>
          </div>
          <div v-if="!savedFiltered.length" class="side-empty">{{ t('dev.httpNoSaved') }}</div>
        </template>
        <template v-else>
          <div v-for="h in historyFiltered" :key="h.id" class="item" @click="loadReq(h.req)">
            <span class="m" :style="{ color: METHOD_COLOR[h.method] }">{{ h.method }}</span>
            <span class="label" :title="h.url">{{ shortUrl(h.url) }}</span>
            <span class="code" :style="{ color: h.ok ? '#1a7f37' : '#cf222e' }">{{
              h.status || '—'
            }}</span>
          </div>
          <div v-if="!historyFiltered.length" class="side-empty">{{ t('dev.httpEmpty') }}</div>
          <button v-if="history.length" class="side-clear" @click="clearHistory">
            {{ t('dev.httpClearHistory') }}
          </button>
        </template>
      </div>
    </aside>
    <button
      class="side-toggle"
      :title="t('dev.httpToggleList')"
      @click="sidebarOpen = !sidebarOpen"
    >
      {{ sidebarOpen ? '‹' : '›' }}
    </button>

    <!-- 右侧：请求 / 响应 -->
    <section class="http-main">
      <!-- 请求栏 -->
      <div class="req-bar">
        <select
          v-model="req.method"
          class="method"
          :style="{ color: METHOD_COLOR[req.method as Method] }"
        >
          <option v-for="m in METHODS" :key="m" :value="m">{{ m }}</option>
        </select>
        <input
          v-model="req.url"
          class="url"
          :placeholder="t('dev.httpUrlPh')"
          spellcheck="false"
          @keyup.enter="send"
        />
        <button class="send" :disabled="sending" @click="send">
          {{ sending ? t('dev.httpSending') : t('dev.httpSend') }}
        </button>
        <button class="save" :title="t('dev.httpSave')" @click="saveReq">
          {{ t('dev.httpSave') }}
        </button>
      </div>
      <div class="name-row">
        <input
          v-model="req.name"
          class="name"
          :placeholder="t('dev.httpNamePh')"
          spellcheck="false"
        />
        <label class="timeout">
          {{ t('dev.httpTimeout') }}
          <input v-model.number="req.timeout" type="number" min="1" max="600" />
          {{ t('dev.httpTimeoutUnit') }}
        </label>
      </div>

      <!-- 请求分页 -->
      <div class="req-body">
        <div class="tabs">
          <button :class="{ on: reqTab === 'params' }" @click="reqTab = 'params'">
            {{ t('dev.httpParams') }}<span class="badge">{{ paramCount }}</span>
          </button>
          <button :class="{ on: reqTab === 'headers' }" @click="reqTab = 'headers'">
            {{ t('dev.httpHeaders') }}<span class="badge">{{ headerCount }}</span>
          </button>
          <button :class="{ on: reqTab === 'body' }" @click="reqTab = 'body'">
            {{ t('dev.httpBodyTab') }}
          </button>
        </div>

        <div v-show="reqTab === 'params'" class="pane">
          <table class="kv">
            <thead>
              <tr>
                <th class="c-en"></th>
                <th>{{ t('dev.httpKey') }}</th>
                <th>{{ t('dev.httpValue') }}</th>
                <th class="c-op"></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(r, i) in req.params" :key="i">
                <td class="c-en"><input v-model="r.enabled" type="checkbox" /></td>
                <td><input v-model="r.key" spellcheck="false" placeholder="key" /></td>
                <td><input v-model="r.value" spellcheck="false" placeholder="value" /></td>
                <td class="c-op">
                  <button class="del" @click="removeRow(req.params, i)">✕</button>
                </td>
              </tr>
            </tbody>
          </table>
          <button class="add-row" @click="addRow(req.params)">＋ {{ t('dev.httpAddRow') }}</button>
        </div>

        <div v-show="reqTab === 'headers'" class="pane">
          <table class="kv">
            <thead>
              <tr>
                <th class="c-en"></th>
                <th>{{ t('dev.httpKey') }}</th>
                <th>{{ t('dev.httpValue') }}</th>
                <th class="c-op"></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(r, i) in req.headers" :key="i">
                <td class="c-en"><input v-model="r.enabled" type="checkbox" /></td>
                <td><input v-model="r.key" spellcheck="false" placeholder="Header" /></td>
                <td><input v-model="r.value" spellcheck="false" placeholder="Value" /></td>
                <td class="c-op">
                  <button class="del" @click="removeRow(req.headers, i)">✕</button>
                </td>
              </tr>
            </tbody>
          </table>
          <button class="add-row" @click="addRow(req.headers)">＋ {{ t('dev.httpAddRow') }}</button>
        </div>

        <div v-show="reqTab === 'body'" class="pane">
          <div class="body-modes">
            <label><input v-model="req.bodyMode" type="radio" value="none" /> none</label>
            <label><input v-model="req.bodyMode" type="radio" value="form-data" /> form-data</label>
            <label
              ><input v-model="req.bodyMode" type="radio" value="urlencoded" />
              x-www-form-urlencoded</label
            >
            <label><input v-model="req.bodyMode" type="radio" value="raw" /> raw</label>
            <select v-if="req.bodyMode === 'raw'" v-model="req.rawLang" class="raw-lang">
              <option value="json">JSON</option>
              <option value="text">Text</option>
              <option value="xml">XML</option>
            </select>
          </div>

          <div v-if="req.bodyMode === 'none'" class="body-hint">
            {{ t('dev.httpBodyNoneHint') }}
          </div>

          <template v-else-if="req.bodyMode === 'form-data'">
            <table class="kv">
              <tbody>
                <tr v-for="(r, i) in req.formData" :key="i">
                  <td class="c-en"><input v-model="r.enabled" type="checkbox" /></td>
                  <td><input v-model="r.key" spellcheck="false" placeholder="field" /></td>
                  <td><input v-model="r.value" spellcheck="false" placeholder="value" /></td>
                  <td class="c-op">
                    <button class="del" @click="removeRow(req.formData, i)">✕</button>
                  </td>
                </tr>
              </tbody>
            </table>
            <button class="add-row" @click="addRow(req.formData)">
              ＋ {{ t('dev.httpAddRow') }}
            </button>
          </template>

          <template v-else-if="req.bodyMode === 'urlencoded'">
            <table class="kv">
              <tbody>
                <tr v-for="(r, i) in req.urlencoded" :key="i">
                  <td class="c-en"><input v-model="r.enabled" type="checkbox" /></td>
                  <td><input v-model="r.key" spellcheck="false" placeholder="field" /></td>
                  <td><input v-model="r.value" spellcheck="false" placeholder="value" /></td>
                  <td class="c-op">
                    <button class="del" @click="removeRow(req.urlencoded, i)">✕</button>
                  </td>
                </tr>
              </tbody>
            </table>
            <button class="add-row" @click="addRow(req.urlencoded)">
              ＋ {{ t('dev.httpAddRow') }}
            </button>
          </template>

          <textarea
            v-else
            v-model="req.rawBody"
            class="raw-editor"
            spellcheck="false"
            :placeholder="rawPlaceholder"
          ></textarea>
        </div>
      </div>

      <div v-if="error" class="err">⚠ {{ error }}</div>

      <!-- 响应区 -->
      <div class="resp">
        <div class="resp-head">
          <div class="resp-title">{{ t('dev.httpResponse') }}</div>
          <template v-if="resp">
            <span class="pill" :style="{ background: statusColor }"
              >{{ resp.status }} {{ resp.statusText }}</span
            >
            <span class="meta">{{ fmtSize(resp.sizeBytes) }}</span>
            <span class="meta">{{ resp.durationMs }} ms</span>
            <div class="spacer"></div>
            <button
              class="btn2"
              :disabled="!resp.body"
              @click="copyText(viewMode === 'pretty' ? prettyBody : resp.body)"
            >
              {{ t('dev.copy') }}
            </button>
            <button class="btn2" :disabled="!resp.body" @click="downloadResp">
              {{ t('dev.download') }}
            </button>
          </template>
          <span v-else class="meta placeholder">{{ t('dev.httpNoResp') }}</span>
        </div>

        <div v-if="resp" class="resp-tabs">
          <button :class="{ on: resTab === 'body' }" @click="resTab = 'body'">
            {{ t('dev.httpRespBody') }}
          </button>
          <button :class="{ on: resTab === 'headers' }" @click="resTab = 'headers'">
            {{ t('dev.httpRespHeaders') }}<span class="badge">{{ resp.headers.length }}</span>
          </button>
          <template v-if="resTab === 'body'">
            <div class="spacer"></div>
            <div class="view-toggle">
              <button
                :class="{ on: viewMode === 'pretty' }"
                :disabled="!isJson"
                @click="viewMode = 'pretty'"
              >
                {{ t('dev.httpPretty') }}
              </button>
              <button :class="{ on: viewMode === 'raw' }" @click="viewMode = 'raw'">
                {{ t('dev.httpRaw') }}
              </button>
            </div>
          </template>
        </div>

        <div v-if="resp && resTab === 'body'" class="resp-body">
          <div v-if="resp.truncated" class="trunc">⚠ {{ t('dev.httpTruncated') }}</div>
          <JsonViewer
            v-if="viewMode === 'pretty' && isJson"
            :model-value="prettyBody"
            class="resp-view"
          />
          <textarea
            v-else
            :value="resp.body"
            class="resp-raw"
            readonly
            spellcheck="false"
          ></textarea>
        </div>
        <div v-else-if="resp && resTab === 'headers'" class="resp-headers">
          <div v-for="(h, i) in resp.headers" :key="i" class="hrow">
            <code class="hk">{{ h[0] }}</code
            ><span class="hv">{{ h[1] }}</span>
          </div>
          <div v-if="!resp.headers.length" class="side-empty">{{ t('dev.httpNoResp') }}</div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.http-tool {
  height: 100%;
  display: flex;
  min-height: 0;
  gap: 0;
  position: relative;
}

/* 侧栏 */
.http-side {
  width: 240px;
  flex: 0 0 240px;
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  min-height: 0;
  background: var(--surface);
}
.http-side.collapsed {
  display: none;
}
.side-head {
  display: flex;
  gap: 2px;
  padding: 8px 8px 0;
}
.side-head button {
  flex: 1;
  border: 0;
  background: transparent;
  color: var(--text-muted);
  padding: 6px;
  font-size: 12.5px;
  border-radius: 6px 6px 0 0;
  cursor: pointer;
}
.side-head button.on {
  background: var(--bg);
  color: var(--text);
  font-weight: 600;
}
.side-search {
  display: flex;
  gap: 6px;
  padding: 8px;
  border-bottom: 1px solid var(--border);
}
.side-search input {
  flex: 1;
  min-width: 0;
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--text);
  font-size: 12.5px;
}
.icon-btn {
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text);
  border-radius: 6px;
  width: 30px;
  cursor: pointer;
  font-size: 15px;
}
.side-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px;
}
.item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 8px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12.5px;
}
.item:hover {
  background: var(--surface-hover);
}
.item .m {
  flex: 0 0 auto;
  font-weight: 700;
  font-size: 11px;
  font-family: var(--font-mono);
  width: 46px;
}
.item .label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text);
}
.item .code {
  flex: 0 0 auto;
  font-family: var(--font-mono);
  font-size: 11px;
}
.item .del {
  flex: 0 0 auto;
  border: 0;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  opacity: 0;
  font-size: 12px;
}
.item:hover .del {
  opacity: 1;
}
.side-empty {
  color: var(--text-muted);
  font-size: 12.5px;
  padding: 16px 10px;
  text-align: center;
}
.side-clear {
  margin: 8px;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text-muted);
  border-radius: 6px;
  padding: 6px;
  font-size: 12px;
  cursor: pointer;
}
.side-toggle {
  position: absolute;
  left: 240px;
  top: 50%;
  transform: translateY(-50%);
  z-index: 2;
  width: 14px;
  height: 44px;
  border: 1px solid var(--border);
  border-left: 0;
  border-radius: 0 6px 6px 0;
  background: var(--surface);
  color: var(--text-muted);
  cursor: pointer;
  font-size: 12px;
  padding: 0;
}
.http-side.collapsed + .side-toggle {
  left: 0;
}

/* 主区 */
.http-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: 12px 16px;
  gap: 10px;
}
.req-bar {
  display: flex;
  gap: 8px;
}
.method {
  flex: 0 0 auto;
  padding: 9px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  font-family: var(--font-mono);
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
}
.url {
  flex: 1;
  min-width: 0;
  padding: 9px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 13px;
}
.send {
  flex: 0 0 auto;
  padding: 9px 20px;
  border: 0;
  border-radius: 6px;
  background: var(--accent);
  color: var(--accent-fg);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}
.send:disabled {
  opacity: 0.55;
  cursor: default;
}
.save {
  flex: 0 0 auto;
  padding: 9px 14px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--surface);
  color: var(--text);
  font-size: 13px;
  cursor: pointer;
}
.name-row {
  display: flex;
  align-items: center;
  gap: 12px;
}
.name {
  flex: 1;
  min-width: 0;
  padding: 6px 10px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--text);
  font-size: 13px;
}
.name:focus {
  border-color: var(--accent);
  background: var(--bg);
  outline: none;
}
.timeout {
  font-size: 12px;
  color: var(--text-muted);
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.timeout input {
  width: 56px;
  padding: 4px 6px;
  border: 1px solid var(--border);
  border-radius: 5px;
  background: var(--bg);
  color: var(--text);
  font-size: 12px;
}

/* 请求分页 */
.req-body {
  display: flex;
  flex-direction: column;
  min-height: 150px;
  max-height: 40%;
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
}
.tabs {
  display: flex;
  gap: 2px;
  padding: 0 8px;
  border-bottom: 1px solid var(--border);
  background: var(--surface);
}
.tabs button {
  border: 0;
  background: transparent;
  color: var(--text-muted);
  padding: 8px 14px;
  font-size: 12.5px;
  cursor: pointer;
  border-bottom: 2px solid transparent;
}
.tabs button.on {
  color: var(--text);
  font-weight: 600;
  border-bottom-color: var(--accent);
}
.badge {
  margin-left: 6px;
  font-size: 11px;
  color: var(--text-muted);
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 0 6px;
}
.pane {
  flex: 1;
  overflow: auto;
  padding: 8px;
}

/* 键值表格 */
.kv {
  width: 100%;
  border-collapse: collapse;
  font-size: 12.5px;
}
.kv th {
  text-align: left;
  color: var(--text-muted);
  font-weight: 500;
  font-size: 11.5px;
  padding: 4px 6px;
  border-bottom: 1px solid var(--border);
}
.kv td {
  padding: 3px;
}
.kv input[type='text'],
.kv td input:not([type='checkbox']) {
  width: 100%;
  padding: 6px 8px;
  border: 1px solid var(--border);
  border-radius: 5px;
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 12.5px;
}
.kv .c-en {
  width: 28px;
  text-align: center;
}
.kv .c-op {
  width: 28px;
}
.kv .del {
  border: 0;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
}
.add-row {
  margin-top: 6px;
  border: 0;
  background: transparent;
  color: var(--accent);
  font-size: 12.5px;
  cursor: pointer;
  padding: 4px 6px;
}

/* Body */
.body-modes {
  display: flex;
  align-items: center;
  gap: 14px;
  flex-wrap: wrap;
  margin-bottom: 8px;
  font-size: 12.5px;
  color: var(--text);
}
.body-modes label {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  cursor: pointer;
}
.raw-lang {
  padding: 3px 6px;
  border: 1px solid var(--border);
  border-radius: 5px;
  background: var(--bg);
  color: var(--text);
  font-size: 12px;
}
.body-hint {
  color: var(--text-muted);
  font-size: 12.5px;
  padding: 12px 6px;
}
.raw-editor {
  width: 100%;
  min-height: 100px;
  resize: vertical;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 12.5px;
  line-height: 1.5;
}

.err {
  color: var(--error);
  font-size: 12.5px;
  font-family: var(--font-mono);
  padding: 8px 12px;
  background: var(--surface);
  border: 1px solid var(--error);
  border-radius: 6px;
  white-space: pre-wrap;
  word-break: break-word;
}

/* 响应 */
.resp {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  border-top: 1px solid var(--border);
  padding-top: 8px;
  gap: 8px;
}
.resp-head {
  display: flex;
  align-items: center;
  gap: 10px;
}
.resp-title {
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-muted);
}
.pill {
  color: #fff;
  font-size: 12px;
  font-weight: 700;
  padding: 2px 10px;
  border-radius: 12px;
  font-family: var(--font-mono);
}
.meta {
  font-size: 12.5px;
  color: var(--text-muted);
  font-family: var(--font-mono);
}
.meta.placeholder {
  text-transform: none;
  letter-spacing: 0;
}
.spacer {
  flex: 1;
}
.btn2 {
  padding: 4px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--surface);
  color: var(--text);
  font-size: 12px;
  cursor: pointer;
}
.btn2:disabled {
  opacity: 0.45;
  cursor: default;
}
.resp-tabs {
  display: flex;
  align-items: center;
  gap: 2px;
}
.resp-tabs > button {
  border: 0;
  background: transparent;
  color: var(--text-muted);
  padding: 6px 12px;
  font-size: 12.5px;
  cursor: pointer;
  border-bottom: 2px solid transparent;
}
.resp-tabs > button.on {
  color: var(--text);
  font-weight: 600;
  border-bottom-color: var(--accent);
}
.view-toggle {
  display: flex;
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
}
.view-toggle button {
  border: 0;
  background: var(--bg);
  color: var(--text-muted);
  padding: 3px 12px;
  font-size: 12px;
  cursor: pointer;
}
.view-toggle button.on {
  background: var(--accent);
  color: var(--accent-fg);
}
.view-toggle button:disabled {
  opacity: 0.4;
  cursor: default;
}
.resp-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.trunc {
  color: var(--error);
  font-size: 12px;
  margin-bottom: 6px;
}
.resp-view {
  flex: 1;
  min-height: 0;
}
.resp-raw {
  flex: 1;
  min-height: 0;
  resize: none;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 12.5px;
  line-height: 1.5;
}
.resp-headers {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 6px 12px;
  font-size: 12.5px;
  font-family: var(--font-mono);
}
.hrow {
  display: flex;
  gap: 10px;
  padding: 4px 0;
  border-bottom: 1px dashed var(--border);
  word-break: break-word;
}
.hrow:last-child {
  border-bottom: 0;
}
.hk {
  color: var(--accent);
  font-weight: 600;
  flex: 0 0 auto;
}
.hv {
  color: var(--text);
}
</style>
