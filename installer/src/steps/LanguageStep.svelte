<script lang="ts">
  import { t, currentLang, setLanguage, supportedLanguages } from '../i18n'
  import { currentStep, config } from '../store'
  import type { LangCode } from '../i18n'

  let selected: LangCode = 'en'

  function choose(code: LangCode) {
    selected = code
    setLanguage(code)
    config.update(c => ({ ...c, language: code }))
  }

  function next() {
    currentStep.set('welcome')
  }
</script>

<div class="lang-root">
  <div class="lang-center">
    <div class="lang-logo">
      <span class="logo-shield">🛡</span>
      <div>
        <div class="lang-logo-name">AegisRescue</div>
        <div class="lang-logo-sub">Recovery OS Installer</div>
      </div>
    </div>

    <h2 class="lang-title">{$t.choose_language}</h2>
    <p class="lang-note">{$t.language_note}</p>

    <div class="lang-grid">
      {#each supportedLanguages as lang}
        <button
          class="lang-btn"
          class:active={selected === lang.code}
          on:click={() => choose(lang.code)}
        >
          <span class="lang-native">{lang.native}</span>
          <span class="lang-english">{lang.label}</span>
        </button>
      {/each}
    </div>

    <button class="btn btn-primary lang-next" on:click={next}>
      {$t.next} →
    </button>
  </div>
</div>

<style>
  .lang-root {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    background: linear-gradient(135deg, #0d1117 0%, #161b22 100%);
  }

  .lang-center {
    width: 480px;
    text-align: center;
  }

  .lang-logo {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 14px;
    margin-bottom: 40px;
  }

  .logo-shield { font-size: 52px; }

  .lang-logo-name {
    font-size: 28px;
    font-weight: 800;
    color: #58a6ff;
    text-align: left;
  }

  .lang-logo-sub {
    font-size: 13px;
    color: #8b949e;
    text-align: left;
  }

  .lang-title {
    font-size: 20px;
    font-weight: 600;
    color: #e6edf3;
    margin-bottom: 8px;
  }

  .lang-note {
    font-size: 13px;
    color: #8b949e;
    margin-bottom: 28px;
  }

  .lang-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    margin-bottom: 28px;
  }

  .lang-btn {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 8px;
    padding: 16px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    transition: all 0.15s;
  }

  .lang-btn:hover {
    border-color: #58a6ff;
    background: rgba(88, 166, 255, 0.05);
  }

  .lang-btn.active {
    border-color: #58a6ff;
    background: rgba(88, 166, 255, 0.12);
    box-shadow: 0 0 0 3px rgba(88, 166, 255, 0.15);
  }

  .lang-native {
    font-size: 20px;
    font-weight: 700;
    color: #e6edf3;
  }

  .lang-english {
    font-size: 12px;
    color: #8b949e;
  }

  .lang-next {
    width: 100%;
    padding: 12px;
    font-size: 15px;
  }
</style>
