import { defineStore } from 'pinia'

export type EditorMode = 'edit' | 'preview' | 'split'
export type Theme = 'light' | 'dark'
export type FontSize = 'S' | 'M' | 'L' | 'XL'
export type ActiveTool = 'markdown' | 'screenshot' | 'ocr' | 'dev' | 'pdf' | 'file' | 'image' | 'ftp'

interface SettingsState {
  activeTool: ActiveTool
  language: 'zh' | 'en'
  theme: Theme
  mode: EditorMode
  fontSize: FontSize
  lineWidth: 'compact' | 'standard' | 'wide'
  immersive: boolean
  /** 右侧文件信息面板是否显示 */
  showMeta: boolean
  editorFontSize: number
  recentFiles: string[]
  currentPath: string | null
  dirty: boolean
  tabs: { path: string; dirty: boolean }[]
  screenshotDir: string
  ocrEngine: 'local' | 'cloud'
  ocrProvider: 'baidu' | 'ali' | 'tencent'
  ocrApiKey: string
  ocrSecretKey: string
  /** 全局截图快捷键触发标记（瞬态，不持久化） */
  pendingCapture: boolean
  /** 跨工具待插入 Markdown 的内容（截图 OCR 结果等，瞬态） */
  pendingInsert: string
}

export const useSettingsStore = defineStore('settings', {
  state: (): SettingsState => ({
    activeTool: 'markdown',
    language: 'zh',
    theme: 'light',
    mode: 'split',
    fontSize: 'M',
    lineWidth: 'standard',
    immersive: false,
    showMeta: true,
    editorFontSize: 14,
    recentFiles: [],
    currentPath: null,
    dirty: false,
    tabs: [],
    screenshotDir: '',
    ocrEngine: 'local',
    ocrProvider: 'baidu',
    ocrApiKey: '',
    ocrSecretKey: '',
    pendingCapture: false,
    pendingInsert: '',
  }),
  actions: {
    setActiveTool(t: ActiveTool) {
      this.activeTool = t
    },
    setLanguage(l: 'zh' | 'en') {
      this.language = l
    },
    setTheme(t: Theme) {
      this.theme = t
      document.documentElement.setAttribute('data-theme', t)
    },
    toggleTheme() {
      this.setTheme(this.theme === 'light' ? 'dark' : 'light')
    },
    setMode(m: EditorMode) {
      this.mode = m
    },
    cycleMode() {
      const order: EditorMode[] = ['edit', 'split', 'preview']
      const idx = order.indexOf(this.mode)
      const next = order[(idx + 1) % order.length]
      if (next) this.mode = next
    },
    setFontSize(s: FontSize) {
      this.fontSize = s
    },
    setLineWidth(w: 'compact' | 'standard' | 'wide') {
      this.lineWidth = w
    },
    setImmersive(b: boolean) {
      this.immersive = b
    },
    setShowMeta(b: boolean) {
      this.showMeta = b
    },
    setEditorFontSize(n: number) {
      this.editorFontSize = Math.max(10, Math.min(28, n))
    },
    addRecent(path: string) {
      this.recentFiles = [path, ...this.recentFiles.filter((p) => p !== path)].slice(0, 10)
    },
    removeRecent(path: string) {
      this.recentFiles = this.recentFiles.filter((p) => p !== path)
    },
    clearRecent() {
      this.recentFiles = []
    },
    setCurrentPath(p: string | null) {
      const cur = this.tabs.find((t) => t.path === this.currentPath)
      if (cur) cur.dirty = this.dirty
      this.currentPath = p
      if (p) {
        this.addRecent(p)
        let tab = this.tabs.find((t) => t.path === p)
        if (!tab) {
          tab = { path: p, dirty: false }
          this.tabs.push(tab)
        }
        this.dirty = tab.dirty
      } else {
        this.dirty = false
      }
    },
    switchTab(path: string) {
      if (path !== this.currentPath) this.setCurrentPath(path)
    },
    closeTab(path: string) {
      const idx = this.tabs.findIndex((t) => t.path === path)
      if (idx === -1) return
      this.tabs.splice(idx, 1)
      if (this.currentPath === path) {
        const next = this.tabs[idx] || this.tabs[idx - 1]
        this.currentPath = null
        this.setCurrentPath(next ? next.path : null)
      }
    },
    setDirty(d: boolean) {
      this.dirty = d
      const tab = this.tabs.find((t) => t.path === this.currentPath)
      if (tab) tab.dirty = d
    },
    setScreenshotDir(d: string) {
      this.screenshotDir = d
    },
    setOcrEngine(e: 'local' | 'cloud') {
      this.ocrEngine = e
    },
    setOcrProvider(p: 'baidu' | 'ali' | 'tencent') {
      this.ocrProvider = p
    },
    setOcrKeys(key: string, secret: string) {
      this.ocrApiKey = key
      this.ocrSecretKey = secret
    },
    setPendingInsert(text: string) {
      this.pendingInsert = text
    },
    /** 全局截图快捷键：切到截图工具并置待捕获标记，由 ScreenshotTool 消费 */
    requestScreenshotCapture() {
      this.activeTool = 'screenshot'
      this.pendingCapture = true
    },
  },
})
