<script lang="ts">
  import { t } from '../i18n'
  import { currentStep, config } from '../store'

  function goLive() {
    config.update(c => ({ ...c, install_mode: 'LiveUsb' }))
    currentStep.set('preflight')
  }

  function goInstall() {
    config.update(c => ({ ...c, install_mode: 'WholeDisk' }))
    currentStep.set('preflight')
  }
</script>

<div class="step-container">
  <div class="welcome-hero">
    <span class="hero-icon">🛡</span>
    <h1 class="step-title">{$t.welcome_title}</h1>
    <p class="step-subtitle">{$t.welcome_desc}</p>
  </div>

  <div class="features-grid">
    {#each $t.welcome_features as feat}
      <div class="feature-item">
        <span class="feat-check">✓</span>
        <span>{feat}</span>
      </div>
    {/each}
  </div>

  <div class="min-req info-box">
    ℹ {$t.min_requirements}
  </div>

  <div class="welcome-actions">
    <button class="btn btn-secondary mode-btn" on:click={goLive}>
      <span class="mode-icon">💿</span>
      <div>
        <div class="mode-title">{$t.welcome_live}</div>
        <div class="mode-desc">No installation required</div>
      </div>
    </button>

    <button class="btn btn-primary mode-btn" on:click={goInstall}>
      <span class="mode-icon">💾</span>
      <div>
        <div class="mode-title">{$t.welcome_install}</div>
        <div class="mode-desc">Permanent installation</div>
      </div>
    </button>
  </div>

  <div class="step-actions">
    <button class="btn btn-secondary" on:click={() => currentStep.set('language')}>
      ← {$t.back}
    </button>
  </div>
</div>

<style>
  .welcome-hero {
    text-align: center;
    margin-bottom: 32px;
  }

  .hero-icon {
    font-size: 56px;
    display: block;
    margin-bottom: 16px;
  }

  .features-grid {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 24px;
  }

  .feature-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 6px;
    font-size: 14px;
    color: #c9d1d9;
  }

  .feat-check {
    color: #7ee787;
    font-weight: 700;
    font-size: 16px;
    flex-shrink: 0;
  }

  .min-req { margin-bottom: 28px; }

  .welcome-actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .mode-btn {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px 20px;
    text-align: left;
    border-radius: 10px;
  }

  .mode-icon { font-size: 28px; flex-shrink: 0; }

  .mode-title {
    font-size: 15px;
    font-weight: 600;
  }

  .mode-desc {
    font-size: 12px;
    opacity: 0.7;
    margin-top: 2px;
  }
</style>
