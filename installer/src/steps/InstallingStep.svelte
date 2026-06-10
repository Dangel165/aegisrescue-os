<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri'
  import { listen } from '@tauri-apps/api/event'
  import { onMount, onDestroy } from 'svelte'
  import { t } from '../i18n'
  import { currentStep, config, installError } from '../store'

  interface InstallProgress {
    phase: string
    percent: number
    message: string
    error: string | null
  }

  let progress: InstallProgress = {
    phase: 'Preparing',
    percent: 0,
    message: '설치 준비 중...',
    error: null,
  }

  let logs: string[] = []
  let unlisten: (() => void) | null = null
  let failed = false

  const phaseLabels: Record<string, string> = {
    Preparing:          '',
    Partitioning:       '',
    Formatting:         '',
    CopyingFiles:       '',
    InstallingComponents: '',
    InstallingBootloader: '',
    ConfiguringSystem:  '',
    Finalizing:         '',
    Complete:           '',
    Failed:             '',
  }

  $: {
    phaseLabels.Preparing           = $t.phase_preparing
    phaseLabels.Partitioning        = $t.phase_partitioning
    phaseLabels.Formatting          = $t.phase_formatting
    phaseLabels.CopyingFiles        = $t.phase_copying
    phaseLabels.InstallingComponents = $t.phase_components
    phaseLabels.InstallingBootloader = $t.phase_bootloader
    phaseLabels.ConfiguringSystem   = $t.phase_configuring
    phaseLabels.Finalizing          = $t.phase_finalizing
    phaseLabels.Complete            = $t.phase_complete
    phaseLabels.Failed              = $t.phase_failed
  }

  const phases = [
    'Preparing', 'Partitioning', 'Formatting', 'CopyingFiles',
    'InstallingComponents', 'InstallingBootloader', 'ConfiguringSystem',
    'Finalizing', 'Complete'
  ]

  onMount(async () => {
    // Tauri 이벤트 리스너
    unlisten = await listen<InstallProgress>('install-progress', (event) => {
      progress = event.payload
      logs = [...logs, `[${progress.phase}] ${progress.message}`].slice(-80)

      if (progress.phase === 'Complete') {
        setTimeout(() => currentStep.set('done'), 1500)
      }
      if (progress.phase === 'Failed') {
        failed = true
        installError.set(progress.error ?? '알 수 없는 오류')
      }
    })

    // 설치 시작
    try {
      await invoke('start_installation', { config: $config })
    } catch (e) {
      const msg = String(e)
      installError.set(msg)
      failed = true
      progress = { ...progress, phase: 'Failed', error: msg }
    }
  })

  onDestroy(() => {
    if (unlisten) unlisten()
  })

  $: currentPhaseIdx = phases.indexOf(progress.phase)

  function retry() {
    failed = false
    logs = []
    progress = { phase: 'Preparing', percent: 0, message: '재시도 중...', error: null }
    invoke('start_installation', { config: $config }).catch(e => {
      installError.set(String(e))
      failed = true
    })
  }
</script>

<div class="installing-root">
  <div class="installing-center">
    <div class="inst-logo">
      {#if progress.phase === 'Complete'}
        <span class="inst-done-icon">✓</span>
      {:else if failed}
        <span class="inst-fail-icon">✗</span>
      {:else}
        <div class="inst-spinner"></div>
      {/if}
    </div>

    <h2 class="inst-title">
      {#if progress.phase === 'Complete'}
        {$t.phase_complete}
      {:else if failed}
        {$t.phase_failed}
      {:else}
        {$t.installing_title}
      {/if}
    </h2>

    <!-- 진행률 바 -->
    <div class="progress-bar-wrap">
      <div class="progress-bar" style="width: {progress.percent}%"
           class:done={progress.phase === 'Complete'}
           class:fail={failed}></div>
    </div>
    <div class="progress-pct">{progress.percent}%</div>

    <!-- 현재 메시지 -->
    <div class="progress-msg">{progress.message}</div>

    <!-- 단계 파이프라인 -->
    <div class="phase-pipeline">
      {#each phases as ph, i}
        {@const done = currentPhaseIdx > i}
        {@const active = currentPhaseIdx === i}
        <div class="phase-dot-wrap">
          <div class="phase-dot"
               class:done
               class:active
               class:fail={failed && active}
               title={phaseLabels[ph] ?? ph}
          ></div>
          {#if i < phases.length - 1}
            <div class="phase-line" class:done></div>
          {/if}
        </div>
      {/each}
    </div>

    <!-- 로그 -->
    <div class="log-box">
      {#each logs as line}
        <div class="log-line">{line}</div>
      {/each}
    </div>

    {#if failed}
      <div class="warning-box" style="margin-top: 16px; text-align: left;">
        {progress.error ?? '설치 실패'}
      </div>
      <button class="btn btn-secondary" on:click={retry} style="margin-top: 12px;">
        재시도
      </button>
    {/if}
  </div>
</div>

<style>
  .installing-root {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    background: #0d1117;
  }

  .installing-center {
    width: 560px;
    text-align: center;
  }

  .inst-logo {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 60px;
    margin-bottom: 16px;
  }

  .inst-spinner {
    width: 48px;
    height: 48px;
    border: 4px solid #21262d;
    border-top-color: #58a6ff;
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  .inst-done-icon { font-size: 48px; color: #7ee787; }
  .inst-fail-icon { font-size: 48px; color: #f85149; }

  .inst-title {
    font-size: 22px;
    font-weight: 700;
    color: #e6edf3;
    margin-bottom: 24px;
  }

  .progress-bar-wrap {
    background: #21262d;
    border-radius: 8px;
    height: 10px;
    overflow: hidden;
    margin-bottom: 8px;
  }

  .progress-bar {
    height: 100%;
    background: linear-gradient(90deg, #58a6ff, #1f6feb);
    border-radius: 8px;
    transition: width 0.4s ease;
  }

  .progress-bar.done { background: #238636; }
  .progress-bar.fail { background: #da3633; }

  .progress-pct {
    font-size: 13px;
    color: #8b949e;
    margin-bottom: 8px;
  }

  .progress-msg {
    font-size: 14px;
    color: #c9d1d9;
    margin-bottom: 24px;
  }

  .phase-pipeline {
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 24px;
  }

  .phase-dot-wrap {
    display: flex;
    align-items: center;
  }

  .phase-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #21262d;
    border: 2px solid #30363d;
    transition: all 0.2s;
  }

  .phase-dot.done   { background: #238636; border-color: #238636; }
  .phase-dot.active { background: #58a6ff; border-color: #58a6ff;
                      box-shadow: 0 0 8px rgba(88,166,255,0.5); }
  .phase-dot.fail   { background: #da3633; border-color: #da3633; }

  .phase-line {
    width: 28px;
    height: 2px;
    background: #21262d;
    transition: all 0.2s;
  }

  .phase-line.done { background: #238636; }

  .log-box {
    background: #0d1117;
    border: 1px solid #21262d;
    border-radius: 6px;
    padding: 10px;
    height: 160px;
    overflow-y: auto;
    text-align: left;
    font-family: monospace;
    font-size: 11px;
    color: #484f58;
  }

  .log-line { padding: 2px 0; border-bottom: 1px solid #161b22; }
  .log-line:last-child { color: #8b949e; border-bottom: none; }
</style>
