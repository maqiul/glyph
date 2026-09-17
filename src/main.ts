import './assets/main.css'

import { createApp } from 'vue'
import { createPinia } from 'pinia'
import i18n from './i18n'

import App from './App.vue'
import CaptureOverlay from './components/CaptureOverlay.vue'

// overlay 截图窗口用 ?mode=capture&monitor=N 加载，渲染框选界面而非主界面
const params = new URLSearchParams(window.location.search)
const isOverlay = params.get('mode') === 'capture'

const app = createApp(isOverlay ? CaptureOverlay : App)
app.use(createPinia())
app.use(i18n)
app.mount('#app')