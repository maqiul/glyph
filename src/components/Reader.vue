<script setup lang="ts">
import { ref, watch, onMounted, nextTick, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import MarkdownIt from 'markdown-it'
import anchor from 'markdown-it-anchor'
import taskLists from 'markdown-it-task-lists'
import hljs from 'highlight.js/lib/core'
import javascript from 'highlight.js/lib/languages/javascript'
import typescript from 'highlight.js/lib/languages/typescript'
import rust from 'highlight.js/lib/languages/rust'
import python from 'highlight.js/lib/languages/python'
import bash from 'highlight.js/lib/languages/bash'
import json from 'highlight.js/lib/languages/json'
import css from 'highlight.js/lib/languages/css'
import xml from 'highlight.js/lib/languages/xml'
import markdownLang from 'highlight.js/lib/languages/markdown'
import go2 from 'highlight.js/lib/languages/go'
import sql from 'highlight.js/lib/languages/sql'
import yaml from 'highlight.js/lib/languages/yaml'
import csharp from 'highlight.js/lib/languages/csharp'
import java from 'highlight.js/lib/languages/java'
import cpp from 'highlight.js/lib/languages/cpp'
import php from 'highlight.js/lib/languages/php'
import ruby from 'highlight.js/lib/languages/ruby'
import swift from 'highlight.js/lib/languages/swift'
import kotlin from 'highlight.js/lib/languages/kotlin'
import dockerfile from 'highlight.js/lib/languages/dockerfile'
import ini from 'highlight.js/lib/languages/ini'
import makefile from 'highlight.js/lib/languages/makefile'
import diff from 'highlight.js/lib/languages/diff'
import properties from 'highlight.js/lib/languages/properties'
import objectivec from 'highlight.js/lib/languages/objectivec'
import plaintext from 'highlight.js/lib/languages/plaintext'
import 'highlight.js/styles/github.css'
import Editor from './Editor.vue'
import { useSettingsStore } from '../stores/settings'
import { useI18n } from 'vue-i18n'

hljs.registerLanguage('javascript', javascript)
hljs.registerLanguage('js', javascript)
hljs.registerLanguage('typescript', typescript)
hljs.registerLanguage('ts', typescript)
hljs.registerLanguage('rust', rust)
hljs.registerLanguage('rs', rust)
hljs.registerLanguage('python', python)
hljs.registerLanguage('py', python)
hljs.registerLanguage('bash', bash)
hljs.registerLanguage('sh', bash)
hljs.registerLanguage('shell', bash)
hljs.registerLanguage('json', json)
hljs.registerLanguage('css', css)
hljs.registerLanguage('html', xml)
hljs.registerLanguage('xml', xml)
hljs.registerLanguage('vue', xml)
hljs.registerLanguage('markdown', markdownLang)
hljs.registerLanguage('md', markdownLang)
hljs.registerLanguage('go', go2)
hljs.registerLanguage('sql', sql)
hljs.registerLanguage('yaml', yaml)
hljs.registerLanguage('yml', yaml)
hljs.registerLanguage('csharp', csharp)
hljs.registerLanguage('cs', csharp)
hljs.registerLanguage('java', java)
hljs.registerLanguage('cpp', cpp)
hljs.registerLanguage('c', cpp)
hljs.registerLanguage('php', php)
hljs.registerLanguage('ruby', ruby)
hljs.registerLanguage('swift', swift)
hljs.registerLanguage('kotlin', kotlin)
hljs.registerLanguage('dockerfile', dockerfile)
hljs.registerLanguage('ini', ini)
hljs.registerLanguage('makefile', makefile)
hljs.registerLanguage('diff', diff)
hljs.registerLanguage('properties', properties)
hljs.registerLanguage('objectivec', objectivec)
hljs.registerLanguage('plaintext', plaintext)
hljs.registerLanguage('text', plaintext)

interface Heading {
  level: number
  text: string
  id: string
  line: number
}

interface ReadResult {
  content: string
  title: string
  bytes: number
  encoding: string
  headings: Heading[]
}

interface WriteResult {
  bytes: number
  encoding: string
}

const props = defineProps<{ path: string | null }>()
const settings = useSettingsStore()
const { t } = useI18n()

const md: MarkdownIt = new MarkdownIt({
  html: false,
  linkify: true,
  typographer: true,
  breaks: false,
  highlight(str: string, lang: string): string {
    if (lang && hljs.getLanguage(lang)) {
      try {
        const out = hljs.highlight(str, { language: lang, ignoreIllegals: true }).value
        return `<pre class="hljs"><code class="language-${lang}">${out}</code></pre>`
      } catch (_) {
        /* fall through */
      }
    }
    return `<pre class="hljs"><code>${md.utils.escapeHtml(str)}</code></pre>`
  },
})
md.use(anchor, {
  permalink: false,
  slugify: (s: string): string =>
    s
      .toLowerCase()
      .replace(/[^a-z0-9_-]+/g, '-')
      .replace(/-+/g, '-')
      .replace(/^-|-$/g, ''),
})
md.use(taskLists, { enabled: true, label: true, labelAfter: true })

// 状态
const source = ref<string>('')
const html = ref<string>('')
const title = ref<string>('')
const bytes = ref<number>(0)
const encoding = ref<string>('')
const headings = ref<Heading[]>([])
const loading = ref<boolean>(true)
const error = ref<string | null>(null)
const saving = ref<boolean>(false)
const saveToast = ref<string | null>(null)
const cursorLine = ref<number>(1)
const editorRef = ref<InstanceType<typeof Editor> | null>(null)
const previewRef = ref<HTMLDivElement | null>(null)
// 加载文件时抑制 watch(source) 把"打开"误标为"未保存"
let skipDirty = false

const previewContentClass = computed(() => ({
  content: true,
  [`width-${settings.lineWidth}`]: true,
  [`size-${settings.fontSize}`]: true,
}))

async function loadFile(p: string) {
  loading.value = true
  error.value = null
  try {
    const result = await invoke<ReadResult>('read_markdown_file', { path: p })
    skipDirty = true
    source.value = result.content
    html.value = md.render(result.content)
    title.value = result.title
    bytes.value = result.bytes
    encoding.value = result.encoding
    settings.setDirty(false)
    loading.value = false
    await nextTick()
    headings.value = applyHeadingIds(parseHeadings(result.content))
  } catch (e: unknown) {
    const msg = typeof e === 'string' ? e : (e as any)?.message ?? JSON.stringify(e)
    error.value = msg
    source.value = ''
    html.value = ''
    headings.value = []
    loading.value = false
  }
}

watch(
  () => props.path,
  (p) => {
    if (p) loadFile(p)
  },
  { immediate: true },
)

/**
 * 前端自己扫描 source 抽标题（level/text/line），编辑时实时更新，不依赖后端。
 * 规则与后端 extract_headings 一致：跳 fenced code，数开头 #，取 1–3 级。
 */
function parseHeadings(text: string): Heading[] {
  const lines = text.split('\n')
  const out: Heading[] = []
  let inCode = false
  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i] ?? ''
    if (raw.trimStart().startsWith('```')) {
      inCode = !inCode
      continue
    }
    if (inCode) continue
    const trimmed = raw.trimStart()
    let level = 0
    while (level < trimmed.length && trimmed[level] === '#') level++
    if (level >= 1 && level <= 3) {
      const rest = trimmed.slice(level).trim()
      if (rest) out.push({ level, text: rest, id: '', line: i + 1 })
    }
  }
  return out
}

/**
 * 渲染后给预览 DOM 的 h1–h3 按文档顺序打稳定 id（h-0/h-1…），并回填到 list。
 * TOC 与预览共用同一套 id，规避 markdown-it-anchor 对纯中文标题生成空 id 的问题。
 */
function applyHeadingIds(list: Heading[]): Heading[] {
  const root = document.querySelector<HTMLElement>('.reader .content')
  const els = root ? Array.from(root.querySelectorAll<HTMLElement>('h1, h2, h3')) : []
  return list.map((h, i) => {
    const id = `h-${i}`
    const el = els[i]
    if (el) el.id = id
    return { ...h, id }
  })
}

watch(source, (s) => {
  html.value = md.render(s)
  if (skipDirty) {
    skipDirty = false
  } else {
    settings.setDirty(true)
  }
  // 编辑后重新抽标题 + 给新 DOM 打稳定 id
  nextTick().then(() => {
    headings.value = applyHeadingIds(parseHeadings(s))
  })
})

// 保存
async function save() {
  if (!props.path) return
  if (saving.value) return
  saving.value = true
  try {
    const result = await invoke<WriteResult>('write_markdown_file', {
      path: props.path,
      content: source.value,
    })
    bytes.value = result.bytes
    encoding.value = result.encoding
    settings.setDirty(false)
    saveToast.value = t('reader.savedToast', { bytes: formatBytes(result.bytes) })
    setTimeout(() => {
      saveToast.value = null
    }, 1500)
  } catch (e: unknown) {
    const msg = typeof e === 'string' ? e : (e as any)?.message ?? JSON.stringify(e)
    error.value = t('reader.saveFailed', { msg })
  } finally {
    saving.value = false
  }
}

defineExpose({ save })

// 光标 → TOC 高亮
function onCursorChange(line: number) {
  cursorLine.value = line
}

const activeHeadingId = computed<string | null>(() => {
  if (!headings.value.length) return null
  let active: string | null = headings.value[0]?.id ?? null
  for (const h of headings.value) {
    if (cursorLine.value >= h.line) active = h.id
  }
  return active
})

// TOC 点击 → 编辑模式滚到源行；预览/分屏滚到对应标题
function jumpTo(id: string) {
  const h = headings.value.find((x) => x.id === id)
  if (!h) return
  if (settings.mode !== 'preview') {
    editorRef.value?.scrollToLine(h.line)
    editorRef.value?.focus()
  }
  if (settings.mode !== 'edit') {
    const container = previewRef.value
    const el = container?.querySelector<HTMLElement>(`[id="${id}"]`)
    if (container && el) {
      const cTop = container.getBoundingClientRect().top
      const eTop = el.getBoundingClientRect().top
      container.scrollTo({
        top: container.scrollTop + (eTop - cTop) - 8,
        behavior: 'smooth',
      })
    }
  }
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / 1024 / 1024).toFixed(2)} MB`
}
</script>

<template>
  <div class="reader">
    <div v-if="loading" class="status">
      <div class="spinner"></div>
      <span>{{ t('reader.loading') }}</span>
    </div>
    <div v-else-if="error" class="status status-error">
      <strong>⚠ {{ t('reader.error') }}</strong>
      <p>{{ error }}</p>
    </div>
    <div v-else class="reader-layout" :class="`mode-${settings.mode}`">
      <aside v-if="!settings.immersive && headings.length" class="toc">
        <div class="toc-title">{{ t('reader.outline') }}</div>
        <ul class="toc-list">
          <li
            v-for="(h, idx) in headings"
            :key="`${h.id}-${idx}`"
            :class="['toc-item', `toc-level-${h.level}`, { active: h.id === activeHeadingId }]"
          >
            <a :href="`#${h.id}`" @click.prevent="jumpTo(h.id)">{{ h.text }}</a>
          </li>
        </ul>
      </aside>
      <div v-if="settings.mode !== 'preview'" class="editor-pane">
        <Editor
          ref="editorRef"
          v-model="source"
          :readonly="false"
          @save="save"
          @cursor="onCursorChange"
        />
      </div>
      <div v-if="settings.mode !== 'edit'" ref="previewRef" :class="previewContentClass" v-html="html" />
      <aside v-if="!settings.immersive" class="meta-panel">
        <div class="meta-title">{{ title }}</div>
        <div class="meta-row"><span>{{ t('reader.size') }}</span><span>{{ formatBytes(bytes) }}</span></div>
        <div class="meta-row"><span>{{ t('reader.encoding') }}</span><span>{{ encoding }}</span></div>
        <div class="meta-row"><span>{{ t('reader.headings') }}</span><span>{{ headings.length }}</span></div>
        <div class="meta-row"><span>{{ t('reader.lines') }}</span><span>{{ source.split('\n').length }}</span></div>
        <div class="meta-row">
          <span>{{ t('reader.mode') }}</span>
          <span>{{ t('mode.' + settings.mode) }}</span>
        </div>
        <div class="meta-row">
          <span>{{ t('reader.cursor') }}</span>
          <span>L{{ cursorLine }}</span>
        </div>
      </aside>
    </div>
    <div v-if="saveToast" class="save-toast">{{ saveToast }}</div>
  </div>
</template>

<style scoped>
.reader {
  height: 100%;
  position: relative;
}

.status {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  height: 100%;
  color: var(--text-muted);
  font-size: 14px;
  padding: 40px;
  text-align: center;
}

.status-error strong {
  font-size: 15px;
  color: var(--error);
}

.status-error p {
  font-family: var(--font-mono);
  font-size: 12px;
  max-width: 480px;
  word-break: break-word;
}

.spinner {
  width: 16px;
  height: 16px;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.reader-layout {
  display: grid;
  gap: 0;
  height: 100%;
}

.reader-layout.mode-edit {
  grid-template-columns: 220px 1fr 240px;
}

.reader-layout.mode-preview {
  grid-template-columns: 220px 1fr 240px;
}

.reader-layout.mode-split {
  grid-template-columns: 220px 1fr 1fr 240px;
}

.editor-pane {
  border-right: 1px solid var(--border);
  overflow: hidden;
  min-width: 0;
}

.reader-layout.mode-edit .editor-pane {
  border-right: 1px solid var(--border);
}

.reader-layout.mode-split .content {
  border-left: 1px solid var(--border);
}

.toc {
  padding: 24px 16px;
  border-right: 1px solid var(--border);
  background: var(--surface);
  overflow-y: auto;
}

.toc-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-muted);
  margin-bottom: 12px;
}

.toc-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.toc-item {
  margin: 2px 0;
}

.toc-item a {
  display: block;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 13px;
  color: var(--text-muted);
  text-decoration: none;
  line-height: 1.4;
  border-left: 2px solid transparent;
  transition: all 0.15s ease;
}

.toc-item a:hover {
  background: var(--surface-hover);
  color: var(--text);
  border-left-color: var(--accent);
}

.toc-item.active a {
  background: var(--surface-hover);
  color: var(--accent);
  border-left-color: var(--accent);
  font-weight: 500;
}

.toc-level-1 a { padding-left: 8px; font-weight: 500; }
.toc-level-2 a { padding-left: 20px; }
.toc-level-3 a { padding-left: 32px; font-size: 12px; }

.content {
  padding: 32px 48px;
  overflow-y: auto;
  max-width: 100%;
  font-size: 15px;
  line-height: 1.7;
  color: var(--text);
}

/* 字号 */
.content.size-S { font-size: 13px; }
.content.size-M { font-size: 15px; }
.content.size-L { font-size: 17px; }
.content.size-XL { font-size: 19px; }

/* 行宽 */
.content.width-compact { max-width: 720px; margin: 0 auto; }
.content.width-standard { max-width: 860px; margin: 0 auto; }
.content.width-wide { max-width: 100%; }

.content :deep(h1),
.content :deep(h2),
.content :deep(h3) {
  margin-top: 1.5em;
  margin-bottom: 0.5em;
  font-weight: 600;
  line-height: 1.3;
}

.content :deep(h1) {
  font-size: 1.9em;
  border-bottom: 1px solid var(--border);
  padding-bottom: 0.3em;
}

.content :deep(h2) { font-size: 1.5em; }
.content :deep(h3) { font-size: 1.2em; }

.content :deep(p) { margin: 0.8em 0; }

.content :deep(code) {
  background: var(--code-bg);
  padding: 2px 6px;
  border-radius: 3px;
  font-family: var(--font-mono);
  font-size: 0.9em;
}

.content :deep(pre) {
  background: var(--code-bg);
  padding: 16px;
  border-radius: 6px;
  overflow-x: auto;
  line-height: 1.5;
  font-size: 13px;
  border: 1px solid var(--border);
}

.content :deep(pre code) {
  background: transparent;
  padding: 0;
}

.content :deep(a) {
  color: var(--accent);
  text-decoration: none;
}

.content :deep(a:hover) { text-decoration: underline; }

.content :deep(blockquote) {
  margin: 1em 0;
  padding: 0.5em 1em;
  border-left: 3px solid var(--accent);
  background: var(--surface);
  color: var(--text-muted);
}

.content :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 1em 0;
}

.content :deep(table th),
.content :deep(table td) {
  border: 1px solid var(--border);
  padding: 8px 12px;
  text-align: left;
}

.content :deep(table th) {
  background: var(--surface);
  font-weight: 600;
}

.content :deep(table tr:nth-child(even)) {
  background: var(--surface);
}

.content :deep(ul),
.content :deep(ol) { padding-left: 2em; }

.content :deep(img) {
  max-width: 100%;
  height: auto;
}

.content :deep(hr) {
  border: 0;
  border-top: 1px solid var(--border);
  margin: 2em 0;
}

.content :deep(input[type='checkbox']) {
  margin-right: 6px;
}

.content :deep(.task-list-item) {
  list-style: none;
  margin-left: -1.4em;
}

[data-theme='dark'] .content :deep(pre) {
  background: #0d1117;
}

.meta-panel {
  padding: 24px 16px;
  border-left: 1px solid var(--border);
  background: var(--surface);
  font-size: 12px;
  color: var(--text-muted);
  overflow-y: auto;
}

.meta-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 16px;
  word-break: break-all;
}

.meta-row {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  padding: 4px 0;
  border-bottom: 1px dashed var(--border);
}

.meta-row span:first-child {
  flex-shrink: 0;
}

.meta-row span:last-child {
  font-family: var(--font-mono);
  color: var(--text);
  text-align: right;
  word-break: break-all;
  overflow-wrap: anywhere;
}

.save-toast {
  position: absolute;
  bottom: 24px;
  right: 24px;
  background: var(--accent);
  color: var(--accent-fg);
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 13px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  animation: toast-in 0.2s ease;
}

@keyframes toast-in {
  from { opacity: 0; transform: translateY(8px); }
  to { opacity: 1; transform: translateY(0); }
}

/* 中等窄：隐藏侧栏，但编辑/预览仍左右并排 */
@media (max-width: 900px) {
  .reader-layout.mode-edit,
  .reader-layout.mode-preview {
    grid-template-columns: 1fr;
  }
  .reader-layout.mode-split {
    grid-template-columns: 1fr 1fr;
  }
  .toc,
  .meta-panel {
    display: none;
  }
}

/* 很窄：才真正上下堆叠 */
@media (max-width: 640px) {
  .reader-layout.mode-split {
    grid-template-columns: 1fr;
  }
}
</style>