<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri'
  import { listen } from '@tauri-apps/api/event'
  import { onMount, onDestroy } from 'svelte'

  let target = '/mnt/targets'
  let scanMode = 'standard'
  let scanning = false
  let progress = 0
  let statusMsg = ''
  let threats: any[] = []
  let error = ''
  let jobId = ''
  let unlisten: (() => void) | undefined

  onMount(async () => {
    unlisten = await listen<any>('daemon-stream', (ev) => {
      const payload = ev.payload
      if (!payload || !jobId) return

      if (payload.stream === 'job_progress' && payload.job_id === jobId) {
        const p = payload.progress
        if (p?.total) {
          progress = Math.min(100, (p.current / p.total) * 100)
        }
        statusMsg = p?.message ?? statusMsg
      }

      if (payload.stream === 'threat_found') {
        threats = [...threats, payload.threat]
      }

      if (payload.stream === 'event') {
        const et = payload.event?.event_type
        if (et === 'ScanCompleted' && payload.event?.context?.job_id === jobId) {
          scanning = false
        }
      }
    })
  })

  onDestroy(() => unlisten?.())

  async function startScan() {
    scanning = true
    error = ''
    threats = []
    progress = 0
    statusMsg = '제출 중…'

    try {
      jobId = await invoke('start_scan', { target, mode: scanMode })
      statusMsg = '스캔 실행 중…'

      const interval = setInterval(async () => {
        if (!scanning) {
          clearInterval(interval)
          return
        }
        try {
          const job: any = await invoke('get_job_progress', { jobId })
          if (job.progress) {
            const p = job.progress
            if (p.total) progress = Math.min(100, (p.current / p.total) * 100)
            statusMsg = p.message ?? statusMsg
          }
          if (job.status === 'Completed' || job.status === 'Failed' || job.status === 'Cancelled') {
            clearInterval(interval)
            scanning = false
            if (job.status === 'Failed') {
              error = job.error ?? '스캔 실패'
            }
          }
        } catch {}
      }, 1000)
    } catch (e: any) {
      error = e.toString()
      scanning = false
    }
  }

  async function cancelScan() {
    if (jobId) {
      await invoke('cancel_job', { jobId })
    }
    scanning = false
  }

  const scanModes = [
    { id: 'quick', label: '빠른 검사', desc: '실행파일·자동시작 위주' },
    { id: 'standard', label: '표준 검사', desc: '모든 실행 파일' },
    { id: 'full', label: '정밀 검사', desc: '전체 파일 심층 스캔' },
  ]
</script>

<div class="panel">
  <h2 class="panel-title">🛡 악성코드 검사</h2>

  <div class="form-group">
    <label class="label">검사 대상</label>
    <div class="input-row">
      <input class="input" type="text" bind:value={target} placeholder="/mnt/targets" />
      <button class="btn btn-secondary">찾아보기</button>
    </div>
  </div>

  <div class="form-group">
    <label class="label">검사 모드</label>
    <div class="mode-group">
      {#each scanModes as mode}
        <label class="mode-card" class:selected={scanMode === mode.id}>
          <input type="radio" bind:group={scanMode} value={mode.id} />
          <div>
            <div class="mode-name">{mode.label}</div>
            <div class="mode-desc">{mode.desc}</div>
          </div>
        </label>
      {/each}
    </div>
  </div>

  <div class="actions">
    {#if scanning}
      <button class="btn btn-danger" on:click={cancelScan}>중지</button>
      <div class="progress-wrap">
        <div class="progress-bar" style="width:{progress}%"></div>
      </div>
      <span class="progress-text">{progress.toFixed(0)}% — {statusMsg}</span>
    {:else}
      <button class="btn btn-primary" on:click={startScan}>검사 시작</button>
    {/if}
  </div>

  {#if error}
    <div class="alert">{error}</div>
  {/if}

  {#if threats.length > 0}
    <div class="results">
      <h3>탐지 ({threats.length})</h3>
      {#each threats as t}
        <div class="threat-row">
          <span class="path">{t.path}</span>
          <span class="score">{(t.risk_score * 100).toFixed(0)}%</span>
          <span class="verdict">{t.verdict}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .panel { max-width: 720px; }
  .panel-title { font-size: 20px; font-weight: 700; margin-bottom: 24px; }
  .form-group { margin-bottom: 20px; }
  .label { display: block; font-size: 13px; color: #8b949e; margin-bottom: 8px; }
  .input-row { display: flex; gap: 8px; }
  .input { flex: 1; background: #0d1117; border: 1px solid #30363d; border-radius: 6px; padding: 8px 12px; color: #e6edf3; }
  .mode-group { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
  .mode-card { display: flex; gap: 8px; padding: 12px; border: 1px solid #30363d; border-radius: 8px; cursor: pointer; }
  .mode-card.selected { border-color: #58a6ff; background: #161b22; }
  .mode-card input { display: none; }
  .mode-name { font-weight: 600; font-size: 13px; }
  .mode-desc { font-size: 11px; color: #8b949e; margin-top: 2px; }
  .actions { display: flex; align-items: center; gap: 12px; margin-top: 8px; flex-wrap: wrap; }
  .progress-wrap { flex: 1; min-width: 120px; height: 8px; background: #21262d; border-radius: 4px; overflow: hidden; }
  .progress-bar { height: 100%; background: #238636; transition: width 0.3s; }
  .progress-text { font-size: 12px; color: #8b949e; }
  .btn { padding: 8px 16px; border-radius: 6px; border: none; cursor: pointer; font-weight: 600; }
  .btn-primary { background: #238636; color: #fff; }
  .btn-danger { background: #da3633; color: #fff; }
  .btn-secondary { background: #21262d; color: #e6edf3; border: 1px solid #30363d; }
  .alert { background: #3d1a1a; border: 1px solid #f85149; border-radius: 6px; padding: 10px 14px; color: #f85149; font-size: 13px; margin-top: 16px; }
  .results { margin-top: 20px; }
  .results h3 { font-size: 14px; margin-bottom: 8px; }
  .threat-row { display: grid; grid-template-columns: 1fr 48px 80px; gap: 8px; font-size: 12px; padding: 6px 0; border-bottom: 1px solid #21262d; font-family: monospace; }
  .path { color: #e6edf3; overflow: hidden; text-overflow: ellipsis; }
  .score { color: #f85149; }
  .verdict { color: #d29922; }
</style>
