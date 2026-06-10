<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri'
  import { onMount, onDestroy } from 'svelte'
  let jobs: any[] = []; let error = ''; let interval: any

  async function refresh() {
    try { jobs = await invoke('list_jobs') } catch (e: any) { error = e.toString() }
  }

  onMount(() => { refresh(); interval = setInterval(refresh, 2000) })
  onDestroy(() => clearInterval(interval))

  async function cancel(id: string) {
    try { await invoke('cancel_job', { jobId: id }); await refresh() }
    catch (e: any) { error = e.toString() }
  }

  function statusColor(s: string) {
    return s === 'Completed' ? '#3fb950' : s === 'Failed' ? '#f85149' : s === 'Running' ? '#58a6ff' : '#8b949e'
  }
</script>

<div class="panel">
  <h2 class="panel-title">⚙️ 작업 목록</h2>
  {#if error}<div class="alert">{error}</div>{/if}
  {#if jobs.length === 0}
    <div class="empty">실행 중인 작업이 없습니다</div>
  {:else}
    <div class="job-list">
      {#each jobs as job}
        <div class="job-card">
          <div class="job-header">
            <span class="job-type">{JSON.stringify(job.job_type).slice(0,40)}...</span>
            <span class="job-status" style="color: {statusColor(job.status)}">{job.status}</span>
          </div>
          {#if job.progress}
            <div class="prog-bg"><div class="prog-fill" style="width:{job.progress.percent?.toFixed(0) ?? 0}%"></div></div>
            <div class="prog-text">{job.progress.message} — {job.progress.percent?.toFixed(0) ?? 0}%</div>
          {/if}
          {#if job.status === 'Running' || job.status === 'Queued'}
            <button class="btn-cancel" on:click={() => cancel(job.id)}>취소</button>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .panel { max-width: 800px; }
  .panel-title { font-size: 20px; font-weight: 700; margin-bottom: 24px; }
  .empty { color: #8b949e; padding: 32px; text-align: center; }
  .job-list { display: flex; flex-direction: column; gap: 10px; }
  .job-card { background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 14px 16px; }
  .job-header { display: flex; justify-content: space-between; margin-bottom: 8px; font-size: 13px; }
  .job-type { color: #e6edf3; font-family: monospace; }
  .job-status { font-weight: 600; font-size: 12px; }
  .prog-bg { height: 6px; background: #21262d; border-radius: 3px; overflow: hidden; margin-bottom: 4px; }
  .prog-fill { height: 100%; background: #58a6ff; }
  .prog-text { font-size: 11px; color: #8b949e; }
  .btn-cancel { margin-top: 8px; padding: 4px 12px; background: #3d1a1a; border: 1px solid #f85149; border-radius: 4px; color: #f85149; cursor: pointer; font-size: 12px; }
  .alert { background: #3d1a1a; border: 1px solid #f85149; border-radius: 6px; padding: 10px 14px; color: #f85149; font-size: 13px; margin-bottom: 12px; }
</style>
