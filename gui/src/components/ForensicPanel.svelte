<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri'
  let source = '/mnt/targets'
  let output = '/mnt/evidence'
  let mode = 'capture'
  let running = false; let error = ''; let done = false

  async function run() {
    running = true; error = ''; done = false
    try {
      if (mode === 'capture') await invoke('start_forensic_capture', { source, output })
      done = true
    } catch (e: any) { error = e.toString() }
    finally { running = false }
  }
</script>

<div class="panel">
  <h2 class="panel-title">🔬 디지털 포렌식</h2>
  <div class="mode-tabs">
    {#each [['capture','증거 수집'], ['timeline','타임라인'], ['artifacts','아티팩트'], ['verify','해시 검증']] as [id, label]}
      <button class="tab" class:active={mode===id} on:click={() => mode=id}>{label}</button>
    {/each}
  </div>
  <div class="form-group"><label class="label">소스</label><input class="input" type="text" bind:value={source} /></div>
  <div class="form-group"><label class="label">출력</label><input class="input" type="text" bind:value={output} /></div>
  <div class="warning-box">🔒 쓰기 차단 모드: 원본 파일 시스템에 일체 쓰지 않습니다</div>
  <button class="btn btn-primary btn-large" on:click={run} disabled={running}>
    {running ? '수집 중...' : '🔬 수집 시작'}
  </button>
  {#if done}<div class="success-box">✅ 완료 — {output} 확인</div>{/if}
  {#if error}<div class="alert alert-danger">{error}</div>{/if}
</div>

<style>
  .panel { max-width: 700px; }
  .panel-title { font-size: 20px; font-weight: 700; margin-bottom: 16px; }
  .mode-tabs { display: flex; gap: 4px; margin-bottom: 20px; }
  .tab { padding: 6px 14px; background: #21262d; border: 1px solid #30363d; border-radius: 6px; color: #8b949e; cursor: pointer; font-size: 13px; }
  .tab.active { background: #1f3a5f; border-color: #58a6ff; color: #58a6ff; }
  .form-group { margin-bottom: 14px; }
  .label { display: block; font-size: 12px; color: #8b949e; margin-bottom: 4px; }
  .input { width: 100%; background: #0d1117; border: 1px solid #30363d; border-radius: 6px; padding: 8px 12px; color: #e6edf3; font-size: 14px; }
  .warning-box { background: #1a2d3d; border: 1px solid #58a6ff; border-radius: 8px; padding: 10px 14px; font-size: 12px; color: #a5d6ff; margin-bottom: 16px; }
  .success-box { background: #1f3a1f; border: 1px solid #3fb950; border-radius: 6px; padding: 10px 14px; color: #3fb950; font-size: 13px; margin-top: 12px; }
  .btn { padding: 8px 16px; border-radius: 6px; font-size: 14px; font-weight: 500; border: 1px solid transparent; cursor: pointer; }
  .btn-primary { background: #1f6feb; border-color: #388bfd; color: #fff; }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-large { width: 100%; padding: 12px; font-size: 15px; }
  .alert { padding: 10px 14px; border-radius: 6px; font-size: 13px; margin-top: 10px; }
  .alert-danger { background: #3d1a1a; border: 1px solid #f85149; color: #f85149; }
</style>
