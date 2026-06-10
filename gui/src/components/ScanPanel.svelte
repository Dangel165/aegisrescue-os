<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri'

  let target = '/mnt/targets'
  let scanMode = 'standard'
  let scanning = false
  let progress = 0
  let results: any = null
  let error = ''
  let jobId = ''

  async function startScan() {
    scanning = true
    error = ''
    results = null
    progress = 0

    try {
      const quick = scanMode === 'quick'
      jobId = await invoke('start_scan', { target, quick })

      // 진행률 폴링
      const interval = setInterval(async () => {
        try {
          const job: any = await invoke('get_job_progress', { jobId })
          if (job.progress) {
            progress = job.progress.percent ?? progress
          }
          if (job.status === 'Completed' || job.status === 'Failed') {
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
      {#each [
        { id: 'quick',    label: '빠른 검사',  desc: '실행파일·자동시작 위주' },
        { id: 'standard', label: '표준 검사',  desc: '모든 실행 파일' },
        { id: 'full',     label: '정밀 검사',  desc: '전체 파일 심층 스캔' },
      ] as mode}
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

  {#if scanning}
    <div class="progress-section">
      <div class="progress-bar-bg">
        <div class="progress-bar-fill" style="width: {progress}%" />
      </div>
      <div class="progress-text">검사 중... {progress.toFixed(0)}%</div>
      <button class="btn btn-danger" on:click={cancelScan}>중단</button>
    </div>
  {:else}
    <button class="btn btn-primary btn-large" on:click={startScan}>
      🛡 검사 시작
    </button>
  {/if}

  {#if error}
    <div class="alert alert-danger">{error}</div>
  {/if}

  <div class="info-box">
    <strong>멀티엔진 탐지:</strong> ClamAV 시그니처 + 정적 휴리스틱 + YARA 룰 + AI 분류 + IOC 평판
  </div>
</div>

<style>
  .panel { max-width: 700px; }
  .panel-title { font-size: 20px; font-weight: 700; margin-bottom: 24px; }
  .form-group { margin-bottom: 20px; }
  .label { display: block; font-size: 13px; color: #8b949e; margin-bottom: 6px; }
  .input-row { display: flex; gap: 8px; }
  .input {
    flex: 1; background: #0d1117; border: 1px solid #30363d; border-radius: 6px;
    padding: 8px 12px; color: #e6edf3; font-size: 14px;
  }
  .input:focus { outline: none; border-color: #58a6ff; }

  .mode-group { display: flex; flex-direction: column; gap: 8px; }
  .mode-card {
    display: flex; align-items: center; gap: 12px;
    background: #161b22; border: 1px solid #30363d; border-radius: 8px;
    padding: 12px 16px; cursor: pointer; transition: all 0.15s;
  }
  .mode-card.selected { border-color: #58a6ff; background: #1a2d3d; }
  .mode-card input[type=radio] { accent-color: #58a6ff; }
  .mode-name { font-size: 14px; font-weight: 600; }
  .mode-desc { font-size: 12px; color: #8b949e; }

  .progress-section { display: flex; flex-direction: column; gap: 8px; margin-bottom: 16px; }
  .progress-bar-bg { height: 8px; background: #21262d; border-radius: 4px; overflow: hidden; }
  .progress-bar-fill { height: 100%; background: #58a6ff; transition: width 0.3s; }
  .progress-text { font-size: 13px; color: #8b949e; }

  .btn {
    padding: 8px 16px; border-radius: 6px; font-size: 14px; font-weight: 500;
    border: 1px solid transparent; cursor: pointer; transition: all 0.15s;
  }
  .btn-primary { background: #1f6feb; border-color: #388bfd; color: #fff; }
  .btn-primary:hover { background: #388bfd; }
  .btn-secondary { background: #21262d; border-color: #30363d; color: #e6edf3; }
  .btn-secondary:hover { background: #30363d; }
  .btn-danger { background: #3d1a1a; border-color: #f85149; color: #f85149; }
  .btn-large { width: 100%; padding: 12px; font-size: 15px; }

  .alert { padding: 12px; border-radius: 6px; font-size: 13px; margin-top: 12px; }
  .alert-danger { background: #3d1a1a; border: 1px solid #f85149; color: #f85149; }

  .info-box {
    margin-top: 20px; background: #161b22; border: 1px solid #30363d;
    border-radius: 8px; padding: 12px 16px; font-size: 13px; color: #8b949e;
  }
</style>
