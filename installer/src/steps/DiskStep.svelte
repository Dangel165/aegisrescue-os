<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri'
  import { onMount } from 'svelte'
  import { t } from '../i18n'
  import { currentStep, config } from '../store'

  interface TargetDisk {
    device: string
    model: string
    size_bytes: number
    size_human: string
    removable: boolean
    partition_table: string
    has_os: boolean
    os_detected: string | null
  }

  let disks: TargetDisk[] = []
  let loading = true
  let selectedDevice = $config.target_device
  let installMode = $config.install_mode
  let encryptSystem = $config.security.encrypt_system
  let passphrase = $config.security.encrypt_passphrase ?? ''
  let passphraseConfirm = ''
  let passphraseError = ''

  onMount(async () => {
    try {
      disks = await invoke<TargetDisk[]>('list_target_disks')
    } catch {
      disks = [
        { device: '/dev/sda', model: 'Sample Disk', size_bytes: 32 * 1024**3,
          size_human: '32.0 GiB', removable: true, partition_table: 'GPT',
          has_os: false, os_detected: null }
      ]
    } finally {
      loading = false
    }
  })

  function selectDisk(dev: string) {
    selectedDevice = dev
    config.update(c => ({ ...c, target_device: dev }))
  }

  $: {
    if (encryptSystem) {
      if (passphrase.length > 0 && passphrase.length < 8) {
        passphraseError = $t.disk_passphrase_short
      } else if (passphraseConfirm && passphrase !== passphraseConfirm) {
        passphraseError = $t.disk_passphrase_mismatch
      } else {
        passphraseError = ''
      }
    } else {
      passphraseError = ''
    }
  }

  function saveAndNext() {
    config.update(c => ({
      ...c,
      target_device: selectedDevice,
      install_mode: installMode,
      security: {
        ...c.security,
        encrypt_system: encryptSystem,
        encrypt_passphrase: encryptSystem ? passphrase : null,
      }
    }))
    currentStep.set('options')
  }

  $: canProceed = installMode === 'LiveUsb' || (
    selectedDevice !== '' &&
    (!encryptSystem || (passphrase.length >= 8 && passphrase === passphraseConfirm))
  )

  function back() { currentStep.set('components') }

  function humanSize(bytes: number): string {
    const gib = bytes / (1024 ** 3)
    return `${gib.toFixed(1)} GiB`
  }
</script>

<div class="step-container">
  <h2 class="step-title">{$t.disk_title}</h2>
  <p class="step-subtitle">{$t.disk_subtitle}</p>

  <!-- 설치 모드 선택 -->
  <div class="mode-group">
    {#each [
      { value: 'WholeDisk',       icon: '💾', label: $t.disk_mode_whole },
      { value: 'LiveUsb',         icon: '💿', label: $t.disk_mode_live },
    ] as mode}
      <label class="mode-option card" class:selected={installMode === mode.value}>
        <input
          type="radio"
          name="install_mode"
          value={mode.value}
          bind:group={installMode}
          class="mode-radio"
        />
        <span class="mode-icon">{mode.icon}</span>
        <span class="mode-label">{mode.label}</span>
      </label>
    {/each}
  </div>

  {#if installMode !== 'LiveUsb'}
    <!-- 디스크 목록 -->
    {#if loading}
      <div class="loading-msg">디스크 탐색 중...</div>
    {:else if disks.length === 0}
      <div class="warning-box">{$t.no_disks}</div>
    {:else}
      <div class="disk-list">
        {#each disks as disk}
          <div
            class="disk-card card"
            class:selected={selectedDevice === disk.device}
            on:click={() => selectDisk(disk.device)}
            role="radio"
            aria-checked={selectedDevice === disk.device}
            tabindex="0"
            on:keydown={(e) => e.key === 'Enter' && selectDisk(disk.device)}
          >
            <div class="disk-icon">{disk.removable ? '🔌' : '🖴'}</div>
            <div class="disk-info">
              <div class="disk-device">{disk.device}</div>
              <div class="disk-model">{disk.model}</div>
              <div class="disk-meta">
                {disk.size_human} · {disk.partition_table}
                {#if disk.os_detected}
                  · <span class="os-tag">{disk.os_detected}</span>
                {/if}
              </div>
            </div>
            <div class="disk-select-dot" class:active={selectedDevice === disk.device}></div>
          </div>

          {#if selectedDevice === disk.device && disk.has_os}
            <div class="warning-box">{$t.disk_warning_os}</div>
          {/if}
        {/each}
      </div>
    {/if}

    <!-- 암호화 옵션 -->
    <div class="card encrypt-card">
      <label class="encrypt-label">
        <input type="checkbox" bind:checked={encryptSystem} style="accent-color:#238636" />
        <span>🔐 {$t.disk_encrypt}</span>
      </label>

      {#if encryptSystem}
        <div class="passphrase-group">
          <div class="form-group">
            <label for="pass">{$t.disk_passphrase}</label>
            <input type="password" id="pass" bind:value={passphrase}
                   placeholder="••••••••" autocomplete="new-password" />
          </div>
          <div class="form-group">
            <label for="pass2">{$t.disk_passphrase_confirm}</label>
            <input type="password" id="pass2" bind:value={passphraseConfirm}
                   placeholder="••••••••" autocomplete="new-password" />
          </div>
          {#if passphraseError}
            <div class="pass-error">{passphraseError}</div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  <div class="step-actions">
    <button class="btn btn-secondary" on:click={back}>← {$t.back}</button>
    <button class="btn btn-primary" on:click={saveAndNext} disabled={!canProceed}>
      {$t.next} →
    </button>
  </div>
</div>

<style>
  .mode-group {
    display: flex;
    gap: 12px;
    margin-bottom: 20px;
  }

  .mode-option {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    flex: 1;
    padding: 12px 16px;
  }

  .mode-radio { display: none; }
  .mode-icon  { font-size: 20px; }
  .mode-label { font-size: 13px; font-weight: 500; color: #e6edf3; }

  .loading-msg { color: #8b949e; padding: 16px; font-size: 14px; }

  .disk-list { display: flex; flex-direction: column; gap: 8px; margin-bottom: 16px; }

  .disk-card {
    display: grid;
    grid-template-columns: 32px 1fr 20px;
    align-items: center;
    gap: 14px;
    cursor: pointer;
  }

  .disk-icon  { font-size: 22px; text-align: center; }

  .disk-device { font-size: 14px; font-weight: 700; color: #e6edf3; }
  .disk-model  { font-size: 13px; color: #8b949e; }
  .disk-meta   { font-size: 12px; color: #484f58; margin-top: 2px; }

  .os-tag {
    color: #f0883e;
    font-weight: 600;
  }

  .disk-select-dot {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid #30363d;
    transition: all 0.12s;
  }

  .disk-select-dot.active {
    background: #58a6ff;
    border-color: #58a6ff;
  }

  .encrypt-card { margin-top: 4px; }

  .encrypt-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    color: #e6edf3;
    cursor: pointer;
    margin-bottom: 0;
  }

  .passphrase-group {
    margin-top: 14px;
    padding-top: 14px;
    border-top: 1px solid #21262d;
  }

  .pass-error {
    font-size: 12px;
    color: #f85149;
    margin-top: -8px;
    margin-bottom: 8px;
  }
</style>
