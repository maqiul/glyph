import { createI18n } from 'vue-i18n'
import zh from '../locales/zh'
import en from '../locales/en'

export type Lang = 'zh' | 'en'

const i18n = createI18n({
  legacy: false,
  locale: 'zh',
  fallbackLocale: 'en',
  messages: { zh, en },
})

export function setLocale(lang: Lang) {
  i18n.global.locale.value = lang
  document.documentElement.setAttribute('lang', lang === 'zh' ? 'zh-CN' : 'en')
}

export default i18n