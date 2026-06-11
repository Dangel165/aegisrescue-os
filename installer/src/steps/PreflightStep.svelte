<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri'
  import { onMount } from 'svelte'
  import { t } from '../i18n'
  import { currentStep } from '../store'

  interface Check {
    name: string
    status: 'Pass' | 'Warn' | 'Fail' | 'Skip'
    detail: string
    required: boolean
  }

  interface PreflightResult {
    passed: boolean
    checks: Check[]
    warnings: string[]
    errors: string[]
  }

  let result: PreflightResult | null = null
  let loading = true
  let error = ''

  onMount(async () => {
    try {
      result = await invoke<PreflightResult>('preflight_check')
    } catch (e) {
      error = String(e)
      // 비-Linux 환경에서 mock 결과 사용
      result = mockResult()
    } finally {
      loading = false
    }
  })

  function mockResult(): PreflightResult {
    return {
      passed: true,
      warnings: [],
      errors: [],
      checks: [
        { name: 'RAM', status: 'Pass', detail: '4096 MiB RAM 감지', required: true },
        { name: '디스크 공간', status: 'Pass', detail: '32.0 GiB 여유', required: true },
        { name: 'CPU 아키텍처', status: 'Pass', detail: 'x86_64', required: true },
        { name: 'UEFI', status: 'Pass', detail: 'UEFI 모드', required: false },
        { name: 'Secure Boot', status: 'Warn', detail: 'MOK 등록 권장', required: false },
        { name: '관리자 권한', status: 'Pass', detail: 'root 권한', required: true },
      ]
    }
  }

  function statusBadge(s: string) {
    switch (s) {
      case 'Pass': return { cls: 'badge-pass', icon: '✓', label: $t.preflight_pass }
      case 'Warn': return { cls: 'badge-warn', icon: '⚠', label: $t.preflight_warn }
      case 'Fail': return { cls: 'badge-fail', icon: '✗', label: $t.preflight_fail }
      default:     return { cls: 'badge-skip', icon: '–', label: 'Skip' }
    }
  }

  function hasBlockingFailures(r: PreflightResult): boolean {
    return r.checks.some((c) => c.status === 'Fail' && c.required)
  }

  $: canProceed = result != null && !hasBlockingFailures(result)

  function next() {
    currentStep.set('license')
  }

  function back() {
    currentStep.set('welcome')
  }
</script>

<div class="step-container">
  <h2 class="step-title">{$t.preflight_title}</h2>

  {#if loading}
    <div class="loading-box">
      <div class="spinner"></div>
      <span>시스템 점검 중...</span>
    </div>
  {:else if error}
    <div class="warning-box">⚠ {error}</div>
    {#if result}
      <!-- mock 결과 표시 -->
    {/if}
  {/if}

  {#if result}
    <!-- 전체 결과 배너 -->
    {#if result.passed && result.warnings.length === 0}
      <div class="result-banner pass">✓ {$t.preflight_ok}</div>
    {:else if result.passed}
      <div class="result-banner warn">⚠ {$t.preflight_warn_msg}</div>
    {:else}
      <div class="result-banner fail">✗ {$t.preflight_fail_msg}</div>
    {/if}

    <!-- 점검 목록 -->
    <div class="checks-list">
      {#each result.checks as check}
        {@const badge = statusBadge(check.status)}
        <div class="check-row" class:check-fail={check.status === 'Fail' && check.required}>
          <div class="check-name">
            {check.name}
            {#if !check.required}
              <span class="check-optional">{$t.preflight_optional}</span>
            {/if}
          </div>
          <div class="check-detail">{check.detail}</div>
          <div class="check-badge {badge.cls}">
            <span>{badge.icon}</span>
            <span>{badge.label}</span>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <div class="step-actions">
    <button class="btn btn-secondary" on:click={back}>← {$t.back}</button>
    <button class="btn btn-primary" on:click={next} disabled={!canProceed}>
      {$t.next} →
    </button>
  </div>
</div>

<style>
  .loading-box {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 24px;
    color: #8b949e;
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid #30363d;
    border-top-color: #58a6ff;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  .result-banner {
    padding: 12px 16px;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 20px;
  }

  .result-banner.pass { background: rgba(126, 231, 135, 0.1); color: #7ee787; border: 1px solid rgba(126, 231, 135, 0.3); }
  .result-banner.warn { background: rgba(240, 136, 62, 0.1);  color: #f0883e; border: 1px solid rgba(240, 136, 62, 0.3); }
  .result-banner.fail { background: rgba(248, 81, 73, 0.1);   color: #f85149; border: 1px solid rgba(248, 81, 73, 0.3); }

  .checks-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .check-row {
    display: grid;
    grid-template-columns: 1fr 2fr auto;
    align-items: center;
    padding: 10px 14px;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 6px;
    font-size: 13px;
    gap: 12px;
  }

  .check-row.check-fail {
    border-color: rgba(248, 81, 73, 0.4);
    background: rgba(248, 81, 73, 0.05);
  }

  .check-name {
    font-weight: 600;
    color: #e6edf3;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .check-optional {
    font-size: 10px;
    background: #21262d;
    padding: 2px 6px;
    border-radius: 4px;
    color: #8b949e;
    font-weight: 400;
  }

  .check-detail { color: #8b949e; }

  .check-badge {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }
</style>
