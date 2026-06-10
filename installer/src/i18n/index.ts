import { writable, derived } from 'svelte/store'
import { en } from './en'
import { ko } from './ko'
import { ja } from './ja'

export type LangCode = 'en' | 'ko' | 'ja' | 'zh'

const translations: Record<LangCode, typeof en> = { en, ko, ja, zh: en }

export const supportedLanguages: Array<{ code: LangCode; label: string; native: string }> = [
  { code: 'ko', label: 'Korean',   native: '한국어' },
  { code: 'en', label: 'English',  native: 'English' },
  { code: 'ja', label: 'Japanese', native: '日本語' },
  { code: 'zh', label: 'Chinese',  native: '中文 (简体)' },
]

export const currentLang = writable<LangCode>('en')

export const t = derived(currentLang, ($lang) => translations[$lang] ?? translations.en)

export function setLanguage(code: LangCode) {
  currentLang.set(code)
}
