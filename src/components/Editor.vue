<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, watch, computed } from 'vue'
import { EditorView, keymap, lineNumbers, highlightActiveLineGutter, drawSelection, highlightActiveLine } from '@codemirror/view'
import { EditorState } from '@codemirror/state'
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands'
import { markdown, markdownLanguage } from '@codemirror/lang-markdown'
import { languages } from '@codemirror/language-data'
import { syntaxHighlighting, HighlightStyle, defaultHighlightStyle, bracketMatching, indentOnInput } from '@codemirror/language'
import { tags as t } from '@lezer/highlight'
import { useSettingsStore } from '../stores/settings'

const props = defineProps<{ modelValue: string; readonly?: boolean }>()
const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'save'): void
  (e: 'cursor', line: number): void
}>()

const settings = useSettingsStore()
const host = ref<HTMLDivElement | null>(null)
let view: EditorView | null = null
let suppressNext = false
let pendingCursor: number | null = null

const fontSizePx = computed(() => `${settings.editorFontSize}px`)

const mdHighlight = HighlightStyle.define([
  { tag: t.heading1, fontSize: '1.4em', fontWeight: '700' },
  { tag: t.heading2, fontSize: '1.2em', fontWeight: '700' },
  { tag: t.heading3, fontSize: '1.05em', fontWeight: '600' },
  { tag: t.heading, color: 'var(--cm-heading)', fontWeight: '600' },
  { tag: t.link, color: 'var(--cm-link)', textDecoration: 'underline' },
  { tag: t.url, color: 'var(--cm-link)' },
  { tag: t.monospace, color: 'var(--cm-code)' },
  { tag: t.emphasis, fontStyle: 'italic' },
  { tag: t.strong, fontWeight: '700' },
  { tag: t.strikethrough, textDecoration: 'line-through' },
])

function makeState(doc: string): EditorState {
  return EditorState.create({
    doc,
    extensions: [
      history(),
      drawSelection(),
      indentOnInput(),
      bracketMatching(),
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightActiveLine(),
      syntaxHighlighting(mdHighlight),
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
      markdown({ base: markdownLanguage, codeLanguages: languages }),
      keymap.of([
        ...defaultKeymap,
        ...historyKeymap,
        indentWithTab,
        {
          key: 'Mod-s',
          preventDefault: true,
          run: () => {
            emit('save')
            return true
          },
        },
      ]),
      EditorView.editable.of(!props.readonly),
      EditorView.updateListener.of((u) => {
        if (u.docChanged && !suppressNext) {
          emit('update:modelValue', u.state.doc.toString())
        }
        if (u.selectionSet || u.docChanged) {
          const head = u.state.selection.main.head
          const line = u.state.doc.lineAt(head).number
          emit('cursor', line)
        }
      }),
    ],
  })
}

function rebuild() {
  if (!host.value) return
  const doc = view ? view.state.doc.toString() : props.modelValue
  view?.destroy()
  view = new EditorView({
    state: makeState(doc),
    parent: host.value,
  })
  if (pendingCursor != null) scrollToLine(pendingCursor)
}

onMounted(() => {
  rebuild()
})

onBeforeUnmount(() => {
  view?.destroy()
  view = null
})

watch(
  () => props.modelValue,
  (v) => {
    if (!view) return
    if (v === view.state.doc.toString()) return
    suppressNext = true
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: v },
    })
    queueMicrotask(() => {
      suppressNext = false
    })
  },
)

watch(
  () => props.readonly,
  () => rebuild(),
)

function scrollToLine(line: number) {
  if (!view) return
  const target = Math.min(Math.max(1, line), view.state.doc.lines)
  const pos = view.state.doc.line(target).from
  view.dispatch({
    selection: { anchor: pos, head: pos },
    effects: EditorView.scrollIntoView(pos, { y: 'start' }),
  })
}

defineExpose({
  scrollToLine(line: number) {
    pendingCursor = line
    scrollToLine(line)
  },
  focus() {
    view?.focus()
  },
})
</script>

<template>
  <div ref="host" class="editor-host" :style="{ '--editor-font-size': fontSizePx }" />
</template>

<style scoped>
.editor-host {
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.editor-host :deep(.cm-editor) {
  height: 100%;
  font-family: var(--font-mono);
  font-size: var(--editor-font-size, 14px);
  line-height: 1.6;
  background: var(--bg);
  color: var(--text);
}

.editor-host :deep(.cm-scroller) {
  overflow: auto;
}

.editor-host :deep(.cm-content) {
  padding: 12px 0;
}

.editor-host :deep(.cm-gutters) {
  background: var(--bg);
  color: var(--text-muted);
  border-right: 1px solid var(--border);
}

.editor-host :deep(.cm-activeLineGutter),
.editor-host :deep(.cm-activeLine) {
  background: var(--surface);
}

.editor-host :deep(.cm-cursor) {
  border-left-color: var(--accent);
}

.editor-host :deep(.cm-selectionBackground),
.editor-host :deep(.cm-content ::selection) {
  background: var(--accent);
  color: var(--accent-fg);
}
</style>