<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri'
  let source = '/dev/sdb'; let dest = '/mnt/backup'; let compress = true; let running = false; let error = ''; let done = false
  async function run() {
    running = true; error = ''; done = false
    try { await invoke('start_image', { source, dest }); done = true }
    catch (e: any) { error = e.toString() }
    finally { running = false }
  }
</script>
<div class="panel">
  <h2 class="panel-title">💾 디스크 이미징 & 백업</h2>
  <div class="form-group"><label class="label">소스 장치/경로</label><input class="input" type="text" bind:value={source} /></div>
  <div class="form-group"><label class="label">저장 위치</label><input class="input" type="text" bind:value={dest} /></div>
  <label class="checkbox-item"><input type="checkbox" bind:checked={compress} /> zstd 압축 (권장)</label>
  <button class="btn btn-primary btn-large" on:click={run} disabled={running}>{running ? '이미징 중...' : '💾 이미지 생성'}</button>
  {#if done}<div class="success-box">✅ 완료</div>{/if}
  {#if error}<div class="alert alert-danger">{error}</div>{/if}
</div>
<style>
  .panel { max-width: 700px; }
  .panel-title { font-size: 20px; font-weight: 700; margin-bottom: 24px; }
  .form-group { margin-bottom: 14px; }
  .label { display: block; font-size: 12px; color: #8b949e; margin-bottom: 4px; }
  .input { width: 100%; background: #0d1117; border: 1px solid #30363d; border-radius: 6px; padding: 8px 12px; color: #e6edf3; font-size: 14px; margin-bottom: 4px; }
  .checkbox-item { display: flex; align-items: center; gap: 8px; cursor: pointer; margin-bottom: 16px; }
  .checkbox-item input { accent-color: #58a6ff; }
  .btn { padding: 8px 16px; border-radius: 6px; font-size: 14px; font-weight: 500; border: 1px solid transparent; cursor: pointer; }
  .btn-primary { background: #1f6feb; border-color: #388bfd; color: #fff; }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-large { width: 100%; padding: 12px; font-size: 15px; }
  .success-box { background: #1f3a1f; border: 1px solid #3fb950; border-radius: 6px; padding: 10px 14px; color: #3fb950; font-size: 13px; margin-top: 12px; }
  .alert { padding: 10px 14px; border-radius: 6px; font-size: 13px; margin-top: 10px; }
  .alert-danger { background: #3d1a1a; border: 1px solid #f85149; color: #f85149; }
</style>
