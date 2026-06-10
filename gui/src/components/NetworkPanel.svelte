<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri'
  export let mode = 'airgap'
  let saving = false; let error = ''

  const modes = [
    { id: 'airgap', label: '🔌 Air-Gap', desc: '완전 차단 (감염 분석 기본값)', color: '#3fb950' },
    { id: 'whitelist', label: '📋 화이트리스트', desc: '지정 호스트만 허용', color: '#d29922' },
    { id: 'restricted', label: '↑ 제한적 온라인', desc: '아웃바운드만 허용', color: '#58a6ff' },
    { id: 'full', label: '🌐 전체 온라인', desc: '모든 트래픽 허용 (위험)', color: '#f85149' },
  ]

  async function setMode(m: string) {
    saving = true; error = ''
    try { await invoke('set_network_mode', { mode: m }); mode = m }
    catch (e: any) { error = e.toString() }
    finally { saving = false }
  }
</script>

<div class="panel">
  <h2 class="panel-title">🌐 네트워크 격리</h2>
  <div class="current">현재 모드: <strong style="color:{modes.find(m2=>m2.id===mode)?.color ?? '#fff'}">{modes.find(m2=>m2.id===mode)?.label ?? mode}</strong></div>
  <div class="mode-list">
    {#each modes as m}
      <button class="mode-card" class:selected={mode===m.id} style="--c:{m.color}" on:click={() => setMode(m.id)} disabled={saving}>
        <span class="mode-label">{m.label}</span>
        <span class="mode-desc">{m.desc}</span>
      </button>
    {/each}
  </div>
  {#if mode === 'full'}<div class="alert alert-danger">⚠️ 전체 온라인 모드는 감염 시스템 분석 시 위험합니다!</div>{/if}
  {#if error}<div class="alert alert-danger">{error}</div>{/if}
</div>

<style>
  .panel { max-width: 600px; }
  .panel-title { font-size: 20px; font-weight: 700; margin-bottom: 16px; }
  .current { font-size: 14px; color: #8b949e; margin-bottom: 20px; }
  .mode-list { display: flex; flex-direction: column; gap: 10px; }
  .mode-card { background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 14px 16px; cursor: pointer; text-align: left; transition: all 0.15s; display: flex; flex-direction: column; gap: 4px; }
  .mode-card.selected { border-color: var(--c); background: color-mix(in srgb, var(--c) 10%, #161b22); }
  .mode-card:disabled { opacity: 0.5; cursor: not-allowed; }
  .mode-label { font-size: 15px; font-weight: 600; color: #e6edf3; }
  .mode-desc { font-size: 12px; color: #8b949e; }
  .alert { padding: 10px 14px; border-radius: 6px; font-size: 13px; margin-top: 12px; }
  .alert-danger { background: #3d1a1a; border: 1px solid #f85149; color: #f85149; }
</style>
