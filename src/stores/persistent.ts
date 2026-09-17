import { LazyStore } from '@tauri-apps/plugin-store'
import { useSettingsStore, type ActiveTool } from './settings'
import { setLocale, type Lang } from '../i18n'

const STORE_FILE = 'glyph-settings.json'

interface PersistedState {
  activeTool?: ActiveTool
  language?: Lang
  theme: 'light' | 'dark'
  mode: 'edit' | 'preview' | 'split'
  fontSize: 'S' | 'M' | 'L' | 'XL'
  lineWidth: 'compact' | 'standard' | 'wide'
  editorFontSize: number
  recentFiles: string[]
  screenshotDir?: string
  ocrEngine?: 'local' | 'cloud'
  ocrProvider?: 'baidu' | 'ali' | 'tencent'
  ocrApiKey?: string
  ocrSecretKey?: string
}

let store: LazyStore | null = null

function getStore(): LazyStore {
  if (!store) store = new LazyStore(STORE_FILE)
  return store
}

export async function loadPersisted(): Promise<void> {
  try {
    const s = getStore()
    const settings = useSettingsStore()
    const saved = await s.get<PersistedState>('settings')
    if (saved) {
      if (saved.activeTool) settings.setActiveTool(saved.activeTool)
      if (saved.language) settings.setLanguage(saved.language)
      if (saved.theme) settings.setTheme(saved.theme)
      if (saved.mode) settings.setMode(saved.mode)
      if (saved.fontSize) settings.setFontSize(saved.fontSize)
      if (saved.lineWidth) settings.setLineWidth(saved.lineWidth)
      if (saved.editorFontSize) settings.setEditorFontSize(saved.editorFontSize)
      if (saved.recentFiles) settings.recentFiles = saved.recentFiles
      if (saved.screenshotDir !== undefined) settings.setScreenshotDir(saved.screenshotDir)
      if (saved.ocrEngine) settings.setOcrEngine(saved.ocrEngine)
      if (saved.ocrProvider) settings.setOcrProvider(saved.ocrProvider)
      if (saved.ocrApiKey !== undefined) settings.ocrApiKey = saved.ocrApiKey
      if (saved.ocrSecretKey !== undefined) settings.ocrSecretKey = saved.ocrSecretKey
    }
    setLocale(settings.language)
  } catch (e) {
    console.warn('loadPersisted failed:', e)
  }
}

export async function savePersisted(): Promise<void> {
  try {
    const s = getStore()
    const settings = useSettingsStore()
    const payload: PersistedState = {
      activeTool: settings.activeTool,
      language: settings.language,
      theme: settings.theme,
      mode: settings.mode,
      fontSize: settings.fontSize,
      lineWidth: settings.lineWidth,
      editorFontSize: settings.editorFontSize,
      recentFiles: settings.recentFiles,
      screenshotDir: settings.screenshotDir,
      ocrEngine: settings.ocrEngine,
      ocrProvider: settings.ocrProvider,
      ocrApiKey: settings.ocrApiKey,
      ocrSecretKey: settings.ocrSecretKey,
    }
    await s.set('settings', payload)
    await s.save()
  } catch (e) {
    console.warn('savePersisted failed:', e)
  }
}