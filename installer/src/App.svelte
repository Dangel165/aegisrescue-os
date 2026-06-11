<script lang="ts">
  import { get } from 'svelte/store'
  import { onMount } from 'svelte'
  import { listen } from '@tauri-apps/api/event'
  import { invoke } from '@tauri-apps/api/tauri'
  import { currentStep } from './store'
  import { t, currentLang } from './i18n'
  import LanguageStep   from './steps/LanguageStep.svelte'
  import WelcomeStep    from './steps/WelcomeStep.svelte'
  import PreflightStep  from './steps/PreflightStep.svelte'
  import LicenseStep    from './steps/LicenseStep.svelte'
  import ComponentsStep from './steps/ComponentsStep.svelte'
  import DiskStep       from './steps/DiskStep.svelte'
  import OptionsStep    from './steps/OptionsStep.svelte'
  import ConfirmStep    from './steps/ConfirmStep.svelte'
  import InstallingStep from './steps/InstallingStep.svelte'
  import DoneStep       from './steps/DoneStep.svelte'

  import { STEPS } from './store'

  let showCloseModal = false
  let showBlockedModal = false

  $: stepIndex = STEPS.indexOf($currentStep)
  $: showSidebar = !['language', 'installing', 'done'].includes($currentStep)
  $: showSteps = ['preflight', 'license', 'components', 'disk', 'options', 'confirm']
  $: sidebarSteps = showSteps

  function stepLabel(step: string): string {
    const labels = $t as Record<string, string>
    return labels[`step_${step}`] ?? step
  }

  function dismissCloseModal() {
    showCloseModal = false
  }

  function dismissBlockedModal() {
    showBlockedModal = false
  }

  async function confirmReboot() {
    showCloseModal = false
    await invoke('reboot_system')
  }

  onMount(() => {
    let unlisten: (() => void) | undefined
    listen('close-requested', () => {
      if (get(currentStep) === 'installing') {
        showBlockedModal = true
        return
      }
      showCloseModal = true
    }).then((fn) => {
      unlisten = fn
    })
    return () => unlisten?.()
  })
</script>

<main class="installer-root" data-lang={$currentLang}>
  {#if showSidebar}
    <aside class="sidebar">
      <div class="sidebar-logo">
        <div class="logo-icon">🛡</div>
        <div class="logo-text">
          <span class="logo-name">AegisRescue</span>
          <span class="logo-sub">Installer</span>
        </div>
      </div>

      <nav class="step-nav">
        {#each sidebarSteps as step}
          {@const idx = STEPS.indexOf(step)}
          {@const current = STEPS.indexOf($currentStep)}
          <div class="step-item"
               class:active={step === $currentStep}
               class:done={current > idx}
               class:future={current < idx}>
            <div class="step-dot">
              {#if current > idx}✓{:else}{idx - 1}{/if}
            </div>
            <span class="step-label">{stepLabel(step)}</span>
          </div>
        {/each}
      </nav>

      <div class="sidebar-footer">
        <div class="version">v0.1.0 · GPL-3.0</div>
      </div>
    </aside>
  {/if}

  <div class="content" class:full-width={!showSidebar}>
    {#if $currentStep === 'language'}
      <LanguageStep />
    {:else if $currentStep === 'welcome'}
      <WelcomeStep />
    {:else if $currentStep === 'preflight'}
      <PreflightStep />
    {:else if $currentStep === 'license'}
      <LicenseStep />
    {:else if $currentStep === 'components'}
      <ComponentsStep />
    {:else if $currentStep === 'disk'}
      <DiskStep />
    {:else if $currentStep === 'options'}
      <OptionsStep />
    {:else if $currentStep === 'confirm'}
      <ConfirmStep />
    {:else if $currentStep === 'installing'}
      <InstallingStep />
    {:else if $currentStep === 'done'}
      <DoneStep />
    {/if}
  </div>
</main>

{#if showCloseModal}
  <div class="modal-backdrop" role="presentation" on:click={dismissCloseModal} on:keydown={() => {}}>
    <div class="modal-card" role="dialog" aria-modal="true" on:click|stopPropagation={() => {}}>
      <h2 class="modal-title">{$t.close_reboot_title}</h2>
      <p class="modal-message">{$t.close_reboot_message}</p>
      <div class="modal-actions">
        <button class="btn btn-secondary" on:click={dismissCloseModal}>
          {$t.close_reboot_no}
        </button>
        <button class="btn btn-primary" on:click={confirmReboot}>
          {$t.close_reboot_yes}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if showBlockedModal}
  <div class="modal-backdrop" role="presentation" on:click={dismissBlockedModal} on:keydown={() => {}}>
    <div class="modal-card" role="dialog" aria-modal="true" on:click|stopPropagation={() => {}}>
      <h2 class="modal-title">{$t.close_blocked_title}</h2>
      <p class="modal-message">{$t.close_blocked_message}</p>
      <div class="modal-actions">
        <button class="btn btn-primary" on:click={dismissBlockedModal}>
          {$t.close_reboot_no}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  :global(*) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(body) {
    font-family: 'Segoe UI', 'Noto Sans KR', 'Noto Sans JP', 'Noto Color Emoji', emoji, sans-serif;
    background: #0d1117;
    color: #e6edf3;
    height: 100vh;
    overflow: hidden;
  }

  .installer-root {
    display: flex;
    height: 100vh;
    background: #0d1117;
  }

  /* ── Sidebar ── */
  .sidebar {
    width: 220px;
    min-width: 220px;
    background: #161b22;
    border-right: 1px solid #21262d;
    display: flex;
    flex-direction: column;
    padding: 24px 0;
  }

  .sidebar-logo {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 20px 24px;
    border-bottom: 1px solid #21262d;
  }

  .logo-icon { font-size: 28px; }

  .logo-text {
    display: flex;
    flex-direction: column;
  }

  .logo-name {
    font-size: 14px;
    font-weight: 700;
    color: #58a6ff;
  }

  .logo-sub {
    font-size: 11px;
    color: #8b949e;
  }

  .step-nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 16px 0;
    flex: 1;
  }

  .step-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 20px;
    cursor: default;
    border-left: 3px solid transparent;
    transition: all 0.15s;
  }

  .step-item.active {
    background: rgba(88, 166, 255, 0.1);
    border-left-color: #58a6ff;
  }

  .step-item.done .step-dot {
    background: #238636;
    color: white;
  }

  .step-item.future .step-label {
    color: #484f58;
  }

  .step-dot {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: #21262d;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 600;
    flex-shrink: 0;
  }

  .step-item.active .step-dot {
    background: #58a6ff;
    color: #0d1117;
  }

  .step-label {
    font-size: 13px;
    color: #8b949e;
  }

  .step-item.active .step-label {
    color: #e6edf3;
    font-weight: 600;
  }

  .step-item.done .step-label {
    color: #7ee787;
  }

  .sidebar-footer {
    padding: 12px 20px 0;
    border-top: 1px solid #21262d;
  }

  .version {
    font-size: 11px;
    color: #484f58;
  }

  /* ── Content ── */
  .content {
    flex: 1;
    overflow-y: auto;
    background: #0d1117;
  }

  .content.full-width {
    width: 100%;
  }

  /* ── Global Step styles (used by children) ── */
  :global(.step-container) {
    padding: 40px 48px;
    max-width: 720px;
    margin: 0 auto;
  }

  :global(.step-title) {
    font-size: 24px;
    font-weight: 700;
    color: #e6edf3;
    margin-bottom: 8px;
  }

  :global(.step-subtitle) {
    font-size: 14px;
    color: #8b949e;
    margin-bottom: 32px;
    line-height: 1.6;
  }

  :global(.step-actions) {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 40px;
    padding-top: 24px;
    border-top: 1px solid #21262d;
  }

  :global(.btn) {
    padding: 8px 20px;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid transparent;
    transition: all 0.15s;
  }

  :global(.btn-primary) {
    background: #238636;
    color: white;
    border-color: #238636;
  }

  :global(.btn-primary:hover:not(:disabled)) {
    background: #2ea043;
  }

  :global(.btn-primary:disabled) {
    opacity: 0.4;
    cursor: not-allowed;
  }

  :global(.btn-secondary) {
    background: transparent;
    color: #e6edf3;
    border-color: #30363d;
  }

  :global(.btn-secondary:hover) {
    background: #21262d;
  }

  :global(.btn-danger) {
    background: #da3633;
    color: white;
    border-color: #da3633;
  }

  :global(.btn-danger:hover) {
    background: #f85149;
  }

  :global(.card) {
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 12px;
  }

  :global(.card.selected) {
    border-color: #58a6ff;
    background: rgba(88, 166, 255, 0.05);
  }

  :global(.badge-pass)   { color: #7ee787; }
  :global(.badge-warn)   { color: #f0883e; }
  :global(.badge-fail)   { color: #f85149; }
  :global(.badge-skip)   { color: #8b949e; }

  :global(.warning-box) {
    background: rgba(248, 81, 73, 0.1);
    border: 1px solid rgba(248, 81, 73, 0.3);
    border-radius: 6px;
    padding: 12px 16px;
    font-size: 13px;
    color: #f85149;
    margin-bottom: 16px;
  }

  :global(.info-box) {
    background: rgba(88, 166, 255, 0.1);
    border: 1px solid rgba(88, 166, 255, 0.3);
    border-radius: 6px;
    padding: 12px 16px;
    font-size: 13px;
    color: #79c0ff;
    margin-bottom: 16px;
  }

  :global(input[type=text], input[type=password]) {
    background: #0d1117;
    border: 1px solid #30363d;
    border-radius: 6px;
    color: #e6edf3;
    padding: 8px 12px;
    font-size: 14px;
    width: 100%;
    outline: none;
  }

  :global(input[type=text]:focus, input[type=password]:focus) {
    border-color: #58a6ff;
  }

  :global(select) {
    background: #0d1117;
    border: 1px solid #30363d;
    border-radius: 6px;
    color: #e6edf3;
    padding: 8px 12px;
    font-size: 14px;
    width: 100%;
    outline: none;
  }

  :global(label) {
    display: block;
    font-size: 13px;
    color: #8b949e;
    margin-bottom: 6px;
  }

  :global(.form-group) {
    margin-bottom: 20px;
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgba(1, 4, 9, 0.72);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }

  .modal-card {
    width: min(420px, 100%);
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 10px;
    padding: 24px;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.45);
  }

  .modal-title {
    font-size: 18px;
    font-weight: 700;
    color: #e6edf3;
    margin-bottom: 10px;
  }

  .modal-message {
    font-size: 14px;
    line-height: 1.6;
    color: #8b949e;
    margin-bottom: 22px;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
</style>
