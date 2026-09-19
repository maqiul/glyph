<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, watch } from 'vue'
import { EditorView, lineNumbers, drawSelection } from '@codemirror/view'
import { EditorState } from '@codemirror/state'
import type { Extension } from '@codemirror/state'
import {
  syntaxHighlighting,
  defaultHighlightStyle,
  bracketMatching,
  foldGutter,
  codeFolding,
  foldAll as foldAllCmd,
  unfoldAll as unfoldAllCmd,
  ensureSyntaxTree,
  LanguageDescription,
  type LanguageSupport,
} from '@codemirror/language'
import { languages } from '@codemirror/language-data'

// 只读代码查看器：行号 + 语法高亮 + 折叠gutter（点击 ▾/▸ 折叠对象/数组/代码块）
// lang 支持 'json' | 'typescript'，默认 json。
const props = withDefaults(defineProps<{ modelValue: string; lang?: 'json' | 'typescript' }>(), {
  lang: 'json',
})

const host = ref<HTMLDivElement | null>(null)
let view: EditorView | null = null
let langSupport: LanguageSupport | null = null

function extensions(): Extension[] {
  const ext: Extension[] = [
    lineNumbers(),
    foldGutter(),
    codeFolding(),
    drawSelection(),
    bracketMatching(),
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    EditorView.editable.of(false),
    EditorState.readOnly.of(true),
  ]
  if (langSupport) ext.push(langSupport.language)
  return ext
}

function rebuild() {
  if (!host.value) return
  view?.destroy()
  view = new EditorView({
    state: EditorState.create({ doc: props.modelValue, extensions: extensions() }),
    parent: host.value,
  })
}

function setDoc(v: string) {
  if (!view) return
  if (v === view.state.doc.toString()) return
  view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: v } })
}

// 折叠/展开全部：等语法树就绪后再执行，避免大文档刚格式化时折叠无效
function runFold(cmd: (v: EditorView) => void) {
  if (!view) return
  ensureSyntaxTree(view.state, 500)
  cmd(view)
}

onMounted(async () => {
  rebuild()
  try {
    const fname = props.lang === 'typescript' ? 'snippet.ts' : 'data.json'
    const desc = LanguageDescription.matchFilename(languages, fname)
    if (desc) langSupport = await desc.load()
    rebuild()
  } catch (e) {
    console.warn('加载代码语言支持失败：', e)
  }
})

onBeforeUnmount(() => {
  view?.destroy()
  view = null
})

watch(
  () => props.modelValue,
  (v) => setDoc(v ?? ''),
)

defineExpose({
  foldAll() {
    runFold(foldAllCmd)
  },
  unfoldAll() {
    runFold(unfoldAllCmd)
  },
})
</script>

<template>
  <div ref="host" class="json-view" />
</template>

<style scoped>
.json-view {
  width: 100%;
  height: 100%;
  min-height: 160px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg);
}

.json-view :deep(.cm-editor) {
  height: 100%;
  font-family: var(--font-mono);
  font-size: 13px;
  line-height: 1.5;
  background: var(--bg);
  color: var(--text);
}

.json-view :deep(.cm-scroller) {
  overflow: auto;
}

.json-view :deep(.cm-content) {
  padding: 10px 0;
}

.json-view :deep(.cm-gutters) {
  background: var(--bg);
  color: var(--text-muted);
  border-right: 1px solid var(--border);
}

.json-view :deep(.cm-foldGutter span) {
  cursor: pointer;
}

.json-view :deep(.cm-selectionBackground),
.json-view :deep(.cm-content ::selection) {
  background: var(--accent);
  color: var(--accent-fg);
}
</style>
