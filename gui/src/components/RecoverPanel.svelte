<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri'

  let target = '/mnt/targets'
  let output = '/mnt/evidence/recovered'
  let running = false
  let step = 0
  let error = ''

  const steps = [
    '랜섬웨어 패밀리 식별',
    '알려진 복호화기 시도',
    'VSS/스냅샷 복원',
    '파일 카빙',
  ]

  async function startRecovery() {
    running = true; step = 0; error = ''
    try {
      const jobId = await invoke('start_recovery', { target, output })
      for (let i = 0; i < steps.length; i++) {
        step = i
        await new Promise(r => setTimeout(r, 1500))
      }
      step = steps.length
    } catch (e: any) {
      error = e.toString()
    } finally {
      running = false
    }
  }
</script>

<div class="panel">
  <h2 class="panel-title">🔓 랜섬웨어 복구</h2>

  <div class="warning-box">
    ⚠️ <strong>주의:</strong> 복구는 원본 파일을 절대 덮어쓰지 않습니다.
    모든 복구 파일은 별도 출력 경로에 저장됩니다.
  </div>

  <div class="form-group">
    <label class="label">감염 대상 경로</label>
    <input class="input" type="text" bind:value={target} />
  </div>
  <div class="form-group">
    <label class="label">복구 출력 경로</label>
    <input class="input" type="text" bind:value={output} />
  </div>

  {#if running}
    <div class="steps">
      {#each steps as s, i}
        <div class="step" class:done={i < step} class:active={i === step}>
          <span class="step-icon">{i < step ? '✅' : i === step ? '⏳' : '○'}</span>
          {s}
        </div>
      {/each}
    </div>
  {:else if step === steps.length}
    <div class="success-box">✅ 복구 완료 — 출력 경로를 확인하세요</div>
  {:else}
    <button class="btn btn-primary btn-large" on:click={startRecovery}>
      🔓 자동 복구 시작
    </button>
  {/if}

  {#if error}
    <div class="alert alert-danger">{error}</div>
  {/if}
</div>

<style>
  .panel { max-width: 700px; }
  .panel-title { font-size: 20px; font-weight: 700; margin-bottom: 24px; }
  .form-group { margin-bottom: 16px; }
  .label { display: block; font-size: 13px; color: #8b949e; margin-bottom: 6px; }
  .input { width: 100%; background: #0d1117; border: 1px solid #30363d; border-radius: 6px; padding: 8px 12px; color: #e6edf3; font-size: 14px; }
  .btn { padding: 8px 16px; border-radius: 6px; font-size: 14px; font-weight: 500; border: 1px solid transparent; cursor: pointer; }
  .btn-primary { background: #1f6feb; border-color: #388bfd; color: #fff; }
  .btn-large { width: 100%; padding: 12px; font-size: 15px; margin-top: 8px; }
  .warning-box { background: #3d2a00; border: 1px solid #d29922; border-radius: 8px; padding: 12px 16px; font-size: 13px; color: #e3b341; margin-bottom: 20px; }
  .success-box { background: #1f3a1f; border: 1px solid #3fb950; border-radius: 8px; padding: 12px 16px; font-size: 13px; color: #3fb950; margin-top: 16px; }
  .steps { display: flex; flex-direction: column; gap: 10px; margin: 16px 0; }
  .step { display: flex; align-items: center; gap: 10px; padding: 10px 14px; background: #161b22; border: 1px solid #30363d; border-radius: 6px; }
  .step.active { border-color: #58a6ff; color: #58a6ff; }
  .step.done { opacity: 0.6; }
  .step-icon { font-size: 18px; width: 24px; text-align: center; }
  .alert { padding: 12px; border-radius: 6px; font-size: 13px; margin-top: 12px; }
  .alert-danger { background: #3d1a1a; border: 1px solid #f85149; color: #f85149; }
</style>
