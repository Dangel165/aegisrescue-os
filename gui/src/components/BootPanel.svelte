<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri'
  let device = '/dev/sda'
  let repairWindows = false
  let repairLinux = false
  let repairEfi = false
  let running = false
  let log = ''
  let error = ''

  async function startRepair() {
    running = true; log = ''; error = ''
    try {
      const jobId = await invoke('start_boot_repair', {
        device,
        repairWindows,
        repairLinux,
        repairEfi,
      })
      log = `✓ 작업 제출됨 (ID: ${jobId})\n복구 진행 중...`
    } catch (e: any) {
      error = e.toString()
    } finally {
      running = false
    }
  }
</script>

<div class="panel">
  <h2 class="panel-title">🧰 부트 레코드 복구</h2>
  <div class="form-group">
    <label class="label">대상 장치</label>
    <input class="input" type="text" bind:value={device} placeholder="/dev/sda" />
  </div>
  <div class="checkbox-group">
    <label class="checkbox-item">
      <input type="checkbox" bind:checked={repairWindows} /> Windows BCD 복구
    </label>
    <label class="checkbox-item">
      <input type="checkbox" bind:checked={repairLinux} /> Linux GRUB 복구
    </label>
    <label class="checkbox-item">
      <input type="checkbox" bind:checked={repairEfi} /> EFI 부트로더 복구
    </label>
  </div>
  <div class="info-box">ℹ️ 작업 전 부트 레코드가 자동으로 백업됩니다.</div>
  <button class="btn btn-primary btn-large" on:click={startRepair} disabled={running}>
    {running ? '복구 중...' : '🧰 부트 복구 시작'}
  </button>
  {#if log}<pre class="log">{log}</pre>{/if}
  {#if error}<div class="alert alert-danger">{error}</div>{/if}
</div>

<style>
  .panel { max-width: 700px; }
  .panel-title { font-size: 20px; font-weight: 700; margin-bottom: 24px; }
  .form-group { margin-bottom: 16px; }
  .label { display: block; font-size: 13px; color: #8b949e; margin-bottom: 6px; }
  .input { width: 100%; background: #0d1117; border: 1px solid #30363d; border-radius: 6px; padding: 8px 12px; color: #e6edf3; font-size: 14px; }
  .checkbox-group { display: flex; flex-direction: column; gap: 8px; margin-bottom: 16px; }
  .checkbox-item { display: flex; align-items: center; gap: 8px; cursor: pointer; }
  .checkbox-item input { accent-color: #58a6ff; }
  .info-box { background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 10px 14px; font-size: 12px; color: #8b949e; margin-bottom: 16px; }
  .btn { padding: 8px 16px; border-radius: 6px; font-size: 14px; font-weight: 500; border: 1px solid transparent; cursor: pointer; }
  .btn-primary { background: #1f6feb; border-color: #388bfd; color: #fff; }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-large { width: 100%; padding: 12px; font-size: 15px; }
  .log { background: #0d1117; border: 1px solid #30363d; border-radius: 6px; padding: 12px; font-size: 12px; color: #3fb950; margin-top: 12px; white-space: pre-wrap; }
  .alert { padding: 12px; border-radius: 6px; font-size: 13px; margin-top: 12px; }
  .alert-danger { background: #3d1a1a; border: 1px solid #f85149; color: #f85149; }
</style>
