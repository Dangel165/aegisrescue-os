<script lang="ts">
  import { t } from '../i18n'
  import { currentStep, config } from '../store'

  function startInstall() {
    currentStep.set('installing')
  }

  function back() { currentStep.set('options') }
</script>

<div class="step-container">
  <h2 class="step-title">{$t.confirm_title}</h2>

  {#if $config.install_mode !== 'LiveUsb'}
    <div class="warning-box">
      {$t.confirm_warning}
    </div>
  {/if}

  <div class="summary-table">
    <div class="sum-row">
      <span class="sum-key">{$t.confirm_language}</span>
      <span class="sum-val">{$config.language.toUpperCase()}</span>
    </div>
    <div class="sum-row">
      <span class="sum-key">{$t.confirm_target}</span>
      <span class="sum-val">
        {$config.install_mode === 'LiveUsb' ? 'Live USB (no install)' : $config.target_device}
      </span>
    </div>
    <div class="sum-row">
      <span class="sum-key">{$t.confirm_hostname}</span>
      <span class="sum-val">{$config.hostname}</span>
    </div>
    <div class="sum-row">
      <span class="sum-key">{$t.confirm_timezone}</span>
      <span class="sum-val">{$config.timezone}</span>
    </div>
    <div class="sum-row">
      <span class="sum-key">{$t.confirm_keyboard}</span>
      <span class="sum-val">{$config.keyboard}</span>
    </div>
    <div class="sum-row">
      <span class="sum-key">{$t.confirm_network}</span>
      <span class="sum-val">{$config.default_network_mode}</span>
    </div>
    <div class="sum-row">
      <span class="sum-key">{$t.confirm_encrypt}</span>
      <span class="sum-val" class:encrypt-yes={$config.security.encrypt_system}>
        {$config.security.encrypt_system ? $t.confirm_yes : $t.confirm_no}
      </span>
    </div>
    <div class="sum-row components-row">
      <span class="sum-key">{$t.confirm_components}</span>
      <div class="comp-tags">
        {#each $config.components as comp}
          <span class="comp-tag">{comp}</span>
        {/each}
      </div>
    </div>
  </div>

  <div class="step-actions">
    <button class="btn btn-secondary" on:click={back}>← {$t.back}</button>
    <button class="btn btn-danger" on:click={startInstall}>
      {$t.install} ▶
    </button>
  </div>
</div>

<style>
  .summary-table {
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 8px;
    overflow: hidden;
    margin-bottom: 8px;
  }

  .sum-row {
    display: flex;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid #21262d;
    font-size: 13px;
  }

  .sum-row:last-child { border-bottom: none; }

  .components-row { align-items: flex-start; }

  .sum-key {
    color: #8b949e;
    width: 140px;
    flex-shrink: 0;
  }

  .sum-val {
    color: #e6edf3;
    font-weight: 500;
  }

  .encrypt-yes { color: #7ee787; }

  .comp-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .comp-tag {
    background: #21262d;
    color: #58a6ff;
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 12px;
    font-weight: 600;
  }
</style>
