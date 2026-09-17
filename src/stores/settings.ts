import { defineStore } from 'pinia'

export type EditorMode = 'edit' | 'preview' | 'split'
export type Theme = 'light' | 'dark'
export type FontSize = 'S' | 'M' | 'L' | 'XL'
export type ActiveTool = 'markdown' | 'screenshot' | 'ocr' | 'dev'

interface SettingsState {
  activeTool: ActiveTool
  language: 'zh' | 'en'
  theme: Theme
  mode: EditorMode
  fontSize: FontSize
  lineWidth: 'compact' | 'standard' | 'wide'
  immersive: boolean
  editorFontSize: number
  recentFiles: string[]
  currentPath: string | null
  dirty: boolean
  screenshotDir: string
  ocrEngine: 'local' | 'cloud'
  ocrProvider: 'baidu' | 'ali' | 'tencent'
  ocrApiKey: string
  ocrSecretKey: string
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
    editorFontSize: 14,
    recentFiles: [],
    currentPath: null,
    dirty: false,
    screenshotDir: '',
    ocrEngine: 'local',
    ocrProvider: 'baidu',
    ocrApiKey: '',
    ocrSecretKey: '',
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
      this.currentPath = p
      if (p) this.addRecent(p)
    },
    setDirty(d: boolean) {
      this.dirty = d
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
  },
})