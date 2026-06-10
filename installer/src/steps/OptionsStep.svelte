<script lang="ts">
  import { t } from '../i18n'
  import { currentStep, config } from '../store'

  let hostname = $config.hostname
  let timezone = $config.timezone
  let keyboard = $config.keyboard
  let networkMode = $config.default_network_mode
  let secureboot = $config.security.enable_secure_boot

  const timezones = [
    'UTC', 'Asia/Seoul', 'Asia/Tokyo', 'Asia/Shanghai',
    'Europe/London', 'Europe/Berlin', 'America/New_York', 'America/Los_Angeles',
    'Australia/Sydney',
  ]

  const keyboards = [
    { code: 'us',    label: 'US English' },
    { code: 'ko',    label: '한국어 (Korean)' },
    { code: 'jp',    label: '日本語 (Japanese)' },
    { code: 'de',    label: 'Deutsch (German)' },
    { code: 'fr',    label: 'Français (French)' },
    { code: 'uk',    label: 'British English' },
  ]

  const networkModes = [
    { code: 'AirGap',     label: '' },
    { code: 'Whitelist',  label: '' },
    { code: 'Restricted', label: '' },
    { code: 'Full',       label: '' },
  ]

  $: networkModes[0].label = $t.network_airgap
  $: networkModes[1].label = $t.network_whitelist
  $: networkModes[2].label = $t.network_restricted
  $: networkModes[3].label = $t.network_full

  function saveAndNext() {
    config.update(c => ({
      ...c,
      hostname,
      timezone,
      keyboard,
      default_network_mode: networkMode,
      security: { ...c.security, enable_secure_boot: secureboot },
    }))
    currentStep.set('confirm')
  }

  $: canProceed = hostname.trim().length > 0

  function back() { currentStep.set('disk') }
</script>

<div class="step-container">
  <h2 class="step-title">{$t.options_title}</h2>

  <div class="form-group">
    <label for="hostname">{$t.hostname_label}</label>
    <input type="text" id="hostname" bind:value={hostname}
           placeholder="aegisrescue" autocomplete="off" />
  </div>

  <div class="form-group">
    <label for="tz">{$t.timezone_label}</label>
    <select id="tz" bind:value={timezone}>
      {#each timezones as tz}
        <option value={tz}>{tz}</option>
      {/each}
    </select>
  </div>

  <div class="form-group">
    <label for="kbd">{$t.keyboard_label}</label>
    <select id="kbd" bind:value={keyboard}>
      {#each keyboards as kb}
        <option value={kb.code}>{kb.label}</option>
      {/each}
    </select>
  </div>

  <div class="form-group">
    <label for="net">{$t.network_mode_label}</label>
    <div class="net-grid">
      {#each networkModes as mode}
        <label class="net-option card" class:selected={networkMode === mode.code}>
          <input type="radio" name="network_mode" value={mode.code} bind:group={networkMode} />
          <span class="net-label">{mode.label}</span>
        </label>
      {/each}
    </div>
  </div>

  <div class="card secureboot-row">
    <label class="sb-label">
      <input type="checkbox" bind:checked={secureboot} style="accent-color:#238636" />
      <span>🔐 {$t.secure_boot_label}</span>
    </label>
  </div>

  <div class="step-actions">
    <button class="btn btn-secondary" on:click={back}>← {$t.back}</button>
    <button class="btn btn-primary" on:click={saveAndNext} disabled={!canProceed}>
      {$t.next} →
    </button>
  </div>
</div>

<style>
  .net-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .net-option {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    padding: 10px 14px;
  }

  .net-option input { display: none; }
  .net-label { font-size: 13px; color: #c9d1d9; }

  .secureboot-row { padding: 12px 16px; }

  .sb-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    color: #e6edf3;
    cursor: pointer;
    margin-bottom: 0;
  }
</style>
