<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri'
  import { onMount } from 'svelte'
  import Dashboard from './components/Dashboard.svelte'
  import ScanPanel from './components/ScanPanel.svelte'
  import RecoverPanel from './components/RecoverPanel.svelte'
  import BootPanel from './components/BootPanel.svelte'
  import ForensicPanel from './components/ForensicPanel.svelte'
  import ImagePanel from './components/ImagePanel.svelte'
  import JobsPanel from './components/JobsPanel.svelte'
  import QuarantinePanel from './components/QuarantinePanel.svelte'
  import NetworkPanel from './components/NetworkPanel.svelte'
  import AnalysisPanel from './components/AnalysisPanel.svelte'

  let activePanel = 'dashboard'
  let systemStatus: any = null
  let networkMode = 'airgap'

  const navItems = [
    { id: 'dashboard',   icon: '🏠', label: '홈' },
    { id: 'scan',        icon: '🛡', label: '검사' },
    { id: 'recover',     icon: '🔓', label: '복구' },
    { id: 'image',       icon: '💾', label: '백업' },
    { id: 'boot',        icon: '🧰', label: '부트복구' },
    { id: 'forensic',    icon: '🔬', label: '포렌식' },
    { id: 'quarantine',  icon: '🧪', label: '격리' },
    { id: 'jobs',        icon: '⚙️', label: '작업' },
    { id: 'network',     icon: '🌐', label: '네트워크' },
    { id: 'analysis',   icon: '🧠', label: 'AI/샌드박스' },
    { id: 'logs',        icon: '📜', label: '로그' },
  ]

  onMount(async () => {
    try {
      systemStatus = await invoke('get_status')
      networkMode = systemStatus?.network_mode?.toLowerCase() ?? 'airgap'
    } catch (e) {
      console.warn('aegisd not connected:', e)
    }
  })
</script>

<div class="app">
  <!-- 헤더 -->
  <header class="header">
    <div class="header-left">
      <span class="logo">🛡 AegisRescue</span>
      <span class="version">v0.1</span>
    </div>
    <div class="header-right">
      <span class="badge" class:badge-danger={networkMode !== 'airgap'}
            class:badge-safe={networkMode === 'airgap'}>
        🌐 {networkMode === 'airgap' ? '격리됨' : networkMode.toUpperCase()}
      </span>
      {#if systemStatus}
        <span class="badge badge-info">
          {systemStatus.active_jobs > 0 ? `⚙️ ${systemStatus.active_jobs} 작업 중` : '✅ 정상'}
        </span>
      {/if}
    </div>
  </header>

  <div class="layout">
    <!-- 사이드바 -->
    <nav class="sidebar">
      {#each navItems as item}
        <button
          class="nav-item"
          class:active={activePanel === item.id}
          on:click={() => activePanel = item.id}
        >
          <span class="nav-icon">{item.icon}</span>
          <span class="nav-label">{item.label}</span>
        </button>
      {/each}

      <div class="sidebar-footer">
        <button class="nav-item nav-advanced" on:click={() => activePanel = 'analysis'}>
          고급 분석
        </button>
      </div>
    </nav>

    <!-- 메인 콘텐츠 -->
    <main class="content">
      {#if activePanel === 'dashboard'}
        <Dashboard {systemStatus} on:navigate={(e) => activePanel = e.detail} />
      {:else if activePanel === 'scan'}
        <ScanPanel />
      {:else if activePanel === 'recover'}
        <RecoverPanel />
      {:else if activePanel === 'boot'}
        <BootPanel />
      {:else if activePanel === 'forensic'}
        <ForensicPanel />
      {:else if activePanel === 'image'}
        <ImagePanel />
      {:else if activePanel === 'jobs'}
        <JobsPanel />
      {:else if activePanel === 'quarantine'}
        <QuarantinePanel />
      {:else if activePanel === 'network'}
        <NetworkPanel bind:mode={networkMode} />
      {:else if activePanel === 'analysis'}
        <AnalysisPanel />
      {:else if activePanel === 'logs'}
        <LogsPanel />
      {/if}
    </main>
  </div>
</div>

<style>
  :global(*) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }
  :global(body) {
    background: #0d1117;
    color: #e6edf3;
    font-family: 'Segoe UI', -apple-system, sans-serif;
    font-size: 14px;
  }

  .app { display: flex; flex-direction: column; height: 100vh; }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 16px;
    height: 48px;
    background: #161b22;
    border-bottom: 1px solid #30363d;
    flex-shrink: 0;
  }
  .header-left { display: flex; align-items: center; gap: 8px; }
  .logo { font-size: 18px; font-weight: 700; color: #58a6ff; }
  .version { font-size: 12px; color: #8b949e; }
  .header-right { display: flex; gap: 8px; }

  .badge {
    padding: 2px 10px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 500;
  }
  .badge-safe   { background: #1f3a1f; color: #3fb950; border: 1px solid #3fb950; }
  .badge-danger { background: #3d1a1a; color: #f85149; border: 1px solid #f85149; }
  .badge-info   { background: #1a2d3d; color: #58a6ff; border: 1px solid #58a6ff; }

  .layout { display: flex; flex: 1; overflow: hidden; }

  .sidebar {
    width: 160px;
    background: #161b22;
    border-right: 1px solid #30363d;
    display: flex;
    flex-direction: column;
    padding: 8px 0;
    flex-shrink: 0;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    background: none;
    border: none;
    color: #8b949e;
    cursor: pointer;
    text-align: left;
    width: 100%;
    border-radius: 6px;
    margin: 0 4px;
    width: calc(100% - 8px);
    transition: all 0.15s;
  }
  .nav-item:hover { background: #21262d; color: #e6edf3; }
  .nav-item.active { background: #1f3a5f; color: #58a6ff; }
  .nav-icon { font-size: 16px; width: 20px; text-align: center; }
  .nav-label { font-size: 13px; }

  .sidebar-footer { margin-top: auto; padding: 8px 4px; }
  .nav-advanced { font-size: 11px; color: #6e7681; }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
  }
</style>
