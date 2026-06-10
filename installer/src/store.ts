import { writable } from 'svelte/store'
import type { LangCode } from './i18n'

export type InstallMode = 'WholeDisk' | 'ExistingPartition' | 'LiveUsb'

export interface InstallConfig {
  language: LangCode
  target_device: string
  install_mode: InstallMode
  components: string[]
  hostname: string
  timezone: string
  keyboard: string
  security: {
    enable_secure_boot: boolean
    encrypt_system: boolean
    encrypt_passphrase: string | null
    enable_tpm: boolean
  }
  default_network_mode: string
}

export type Step =
  | 'language'
  | 'welcome'
  | 'preflight'
  | 'license'
  | 'components'
  | 'disk'
  | 'options'
  | 'confirm'
  | 'installing'
  | 'done'

export const STEPS: Step[] = [
  'language',
  'welcome',
  'preflight',
  'license',
  'components',
  'disk',
  'options',
  'confirm',
  'installing',
  'done',
]

export const DEFAULT_CONFIG: InstallConfig = {
  language: 'en',
  target_device: '',
  install_mode: 'WholeDisk',
  components: ['core', 'scanner', 'recover', 'boot', 'gui'],
  hostname: 'aegisrescue',
  timezone: 'UTC',
  keyboard: 'us',
  security: {
    enable_secure_boot: false,
    encrypt_system: false,
    encrypt_passphrase: null,
    enable_tpm: false,
  },
  default_network_mode: 'AirGap',
}

export const currentStep = writable<Step>('language')
export const config = writable<InstallConfig>({ ...DEFAULT_CONFIG })
export const installError = writable<string | null>(null)

export const COMPONENT_SIZES: Record<string, number> = {
  core:     180,
  scanner:  320,
  ai:       240,
  recover:  160,
  boot:     80,
  forensic: 120,
  image:    100,
  sandbox:  200,
  gui:      280,
}

export function totalSize(components: string[]): number {
  return components.reduce((sum, c) => sum + (COMPONENT_SIZES[c] ?? 0), 0)
}
