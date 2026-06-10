<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri'
  import { t } from '../i18n'
  import { config } from '../store'

  async function reboot() {
    await invoke('reboot_system')
  }

  async function openCenter() {
    await invoke('open_aegiscenter')
  }

  function continueLive() {
    window.close()
  }
</script>

<div class="done-root">
  <div class="done-center">
    <div class="done-icon">🛡</div>
    <h1 class="done-title">{$t.done_title}</h1>
    <p class="done-desc">{$t.done_desc}</p>

    <div class="done-meta">
      <div class="meta-item">
        <span class="meta-icon">🌐</span>
        <span>Language: {$config.language.toUpperCase()}</span>
      </div>
      <div class="meta-item">
        <span class="meta-icon">🔒</span>
        <span>Network: {$config.default_network_mode}</span>
      </div>
      {#if $config.security.encrypt_system}
        <div class="meta-item">
          <span class="meta-icon">🔐</span>
          <span>LUKS encryption enabled</span>
        </div>
      {/if}
    </div>

    <div class="done-actions">
      {#if $config.install_mode !== 'LiveUsb'}
        <button class="btn btn-primary done-btn" on:click={reboot}>
          🔄 {$t.done_reboot}
        </button>
      {/if}
      <button class="btn btn-secondary done-btn" on:click={openCenter}>
        🛡 {$t.done_open_center}
      </button>
      <button class="btn btn-secondary done-btn" on:click={continueLive}>
        💿 {$t.done_live}
      </button>
    </div>
  </div>
</div>

<style>
  .done-root {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    background: linear-gradient(135deg, #0d1117 0%, #0d2818 100%);
  }

  .done-center {
    text-align: center;
    max-width: 480px;
  }

  .done-icon {
    font-size: 72px;
    margin-bottom: 20px;
    animation: pop 0.4s ease-out;
  }

  @keyframes pop {
    0%   { transform: scale(0.5); opacity: 0; }
    70%  { transform: scale(1.1); }
    100% { transform: scale(1);   opacity: 1; }
  }

  .done-title {
    font-size: 28px;
    font-weight: 800;
    color: #7ee787;
    margin-bottom: 12px;
  }

  .done-desc {
    font-size: 14px;
    color: #8b949e;
    line-height: 1.6;
    margin-bottom: 28px;
  }

  .done-meta {
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 28px;
  }

  .meta-item {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    color: #c9d1d9;
  }

  .meta-icon { font-size: 16px; }

  .done-actions {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .done-btn {
    width: 100%;
    padding: 12px;
    font-size: 14px;
  }
</style>
