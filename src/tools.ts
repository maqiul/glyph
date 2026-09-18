import type { Component } from 'vue'
import type { ActiveTool } from './stores/settings'
import MarkdownTool from './components/tools/MarkdownTool.vue'
import ScreenshotTool from './components/tools/ScreenshotTool.vue'
import OcrTool from './components/tools/OcrTool.vue'
import DevTool from './components/tools/DevTool.vue'
import PdfTool from './components/tools/PdfTool.vue'
import TextTool from './components/tools/TextTool.vue'
import FileTool from './components/tools/FileTool.vue'
import ImageTool from './components/tools/ImageTool.vue'

export interface ToolDef {
  id: ActiveTool
  labelKey: string
  /** SVG path d（24×24 viewBox，线性图标，stroke 渲染） */
  icon: string
  component: Component
}

/**
 * 工具箱注册表。新增工具 = 在这里加一项 + 建对应组件。
 * 顺序即活动栏从上到下的顺序。
 */
export const TOOLS: ToolDef[] = [
  {
    id: 'markdown',
    labelKey: 'tools.markdown',
    icon: 'M6 3h8l5 5v12a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z M13 3v6h6 M8 13h6M8 16h4',
    component: MarkdownTool,
  },
  {
    id: 'screenshot',
    labelKey: 'tools.screenshot',
    icon: 'M3 8V5a2 2 0 0 1 2-2h3 M16 3h3a2 2 0 0 1 2 2v3 M21 16v3a2 2 0 0 1-2 2h-3 M8 21H5a2 2 0 0 1-2-2v-3 M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6z',
    component: ScreenshotTool,
  },
  {
    id: 'ocr',
    labelKey: 'tools.ocr',
    icon: 'M4 8V5a1 1 0 0 1 1-1h3 M16 4h3a1 1 0 0 1 1 1v3 M20 16v3a1 1 0 0 1-1 1h-3 M8 20H5a1 1 0 0 1-1-1v-3 M8 10h8M8 14h5',
    component: OcrTool,
  },
  {
    id: 'dev',
    labelKey: 'tools.dev',
    icon: 'M8 9l-4 3 4 3 M16 9l4 3-4 3 M13 7l-2 10',
    component: DevTool,
  },
  {
    id: 'pdf',
    labelKey: 'tools.pdf',
    icon: 'M6 3h8l5 5v12a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z M13 3v6h6 M8 16h6',
    component: PdfTool,
  },
  {
    id: 'text',
    labelKey: 'tools.text',
    icon: 'M4 6h16 M4 12h10 M4 18h7 M15 15l3 3 4-5',
    component: TextTool,
  },
  {
    id: 'file',
    labelKey: 'tools.file',
    icon: 'M6 3h8l5 5v12a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z M13 3v6h6 M9 17h4 M9 13h6',
    component: FileTool,
  },
  {
    id: 'image',
    labelKey: 'tools.image',
    icon: 'M4 5h16a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1z M3 16l5-5 4 4 3-3 6 6 M8 9a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3z',
    component: ImageTool,
  },
]
