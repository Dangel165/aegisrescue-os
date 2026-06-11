<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri'

  let sample = '/mnt/targets/suspicious.bin'
  let aiFile = '/mnt/targets/suspicious.exe'
  let sandboxLevel = 1
  let running = false
  let aiResult: any = null
  let sandboxResult: any = null
  let error = ''

  async function runAi() {
    running = true
    error = ''
    aiResult = null
    try {
      aiResult = await invoke('run_ai_analysis', { file: aiFile })
    } catch (e: any) {
      error = e.toString()
    } finally {
      running = false
    }
  }

  async function runSandbox() {
    running = true
    error = ''
    sandboxResult = null
    try {
      sandboxResult = await invoke('run_sandbox', { sample, level: sandboxLevel })
    } catch (e: any) {
      error = e.toString()
    } finally {
      running = false
    }
  }
</script>

<div class="panel">
  <h2 class="panel-title">🧠 AI / 샌드박스 분석</h2>

  <section class="section">
    <h3>AI 정적 분류 (aegis-ai)</h3>
    <div class="form-group">
      <label class="label">파일 경로</label>
      <input class="input" type="text" bind:value={aiFile} />
    </div>
    <button class="btn btn-primary" on:click={runAi} disabled={running}>AI 분석</button>
    {#if aiResult}
      <pre class="out">{JSON.stringify(aiResult, null, 2)}</pre>
    {/if}
  </section>

  <section class="section">
    <h3>동적 샌드박스 (aegis-sandbox)</h3>
    <div class="form-group">
      <label class="label">샘플 경로</label>
      <input class="input" type="text" bind:value={sample} />
    </div>
    <div class="form-group">
      <label class="label">격리 레벨 (0=정적, 1=네임스페이스, 2=VM)</label>
      <input class="input" type="number" min="0" max="2" bind:value={sandboxLevel} />
    </div>
    <button class="btn btn-primary" on:click={runSandbox} disabled={running}>샌드박스 실행</button>
    {#if sandboxResult}
      <pre class="out">{JSON.stringify(sandboxResult, null, 2)}</pre>
    {/if}
  </section>

  {#if error}<div class="alert">{error}</div>{/if}
</div>

<style>
  .panel { max-width: 800px; }
  .panel-title { font-size: 20px; font-weight: 700; margin-bottom: 20px; }
  .section { margin-bottom: 28px; padding-bottom: 20px; border-bottom: 1px solid #21262d; }
  .section h3 { font-size: 14px; margin-bottom: 12px; color: #58a6ff; }
  .form-group { margin-bottom: 12px; }
  .label { display: block; font-size: 12px; color: #8b949e; margin-bottom: 4px; }
  .input { width: 100%; background: #0d1117; border: 1px solid #30363d; border-radius: 6px; padding: 8px 12px; color: #e6edf3; }
  .btn { padding: 8px 16px; border-radius: 6px; border: none; cursor: pointer; background: #1f6feb; color: #fff; font-weight: 600; }
  .btn:disabled { opacity: 0.5; }
  .out { background: #0d1117; border: 1px solid #30363d; border-radius: 6px; padding: 12px; font-size: 11px; margin-top: 12px; overflow-x: auto; color: #3fb950; }
  .alert { background: #3d1a1a; border: 1px solid #f85149; color: #f85149; padding: 10px; border-radius: 6px; margin-top: 12px; font-size: 13px; }
</style>
