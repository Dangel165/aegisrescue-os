<script lang="ts">
  import { createEventDispatcher } from 'svelte'
  import { invoke } from '@tauri-apps/api/tauri'

  export let systemStatus: any = null
  const dispatch = createEventDispatcher()

  const quickActions = [
    { id: 'scan',     icon: '🛡', title: '빠른 검사',    desc: '악성코드 즉시 탐지', color: '#1f6feb' },
    { id: 'recover',  icon: '🔓', title: '랜섬웨어 복구', desc: '암호화된 파일 복원', color: '#388bfd' },
    { id: 'image',    icon: '💾', title: '파일 백업',     desc: '중요 데이터 안전 보관', color: '#3fb950' },
    { id: 'boot',     icon: '🧰', title: '부트 복구',     desc: 'MBR/GPT/EFI 수리', color: '#d29922' },
    { id: 'forensic', icon: '🔬', title: '포렌식 분석',   desc: '증거 수집 및 타임라인', color: '#8957e5' },
    { id: 'sandbox',  icon: '🧪', title: '샌드박스',      desc: '안전한 파일 분석', color: '#f85149' },
  ]

  function navigate(panel: string) {
    dispatch('navigate', panel)
  }
</script>

<div class="dashboard">
  <h1 class="page-title">AegisRescue 복구 센터</h1>
  <p class="page-desc">감염된 시스템을 안전하게 복구하세요</p>

  <!-- 상태 카드 -->
  <div class="status-section">
    <div class="status-card" class:danger={!systemStatus}>
      <div class="status-icon">{systemStatus ? '✅' : '⚠️'}</div>
      <div class="status-info">
        <div class="status-title">시스템 상태</div>
        <div class="status-value">
          {systemStatus ? '정상 작동 중' : '데몬 연결 안됨 (aegisd 실행 필요)'}
        </div>
      </div>
    </div>

    {#if systemStatus}
      <div class="status-card">
        <div class="status-icon">🌐</div>
        <div class="status-info">
          <div class="status-title">네트워크</div>
          <div class="status-value safe">{systemStatus.network_mode} 모드</div>
        </div>
      </div>
      <div class="status-card">
        <div class="status-icon">🔒</div>
        <div class="status-info">
          <div class="status-title">Secure Boot</div>
          <div class="status-value" class:safe={systemStatus.secure_boot_active}>
            {systemStatus.secure_boot_active ? '활성화됨' : '비활성'}
          </div>
        </div>
      </div>
      <div class="status-card">
        <div class="status-icon">⚙️</div>
        <div class="status-info">
          <div class="status-title">활성 작업</div>
          <div class="status-value">{systemStatus.active_jobs}개</div>
        </div>
      </div>
    {/if}
  </div>

  <!-- 빠른 실행 -->
  <h2 class="section-title">빠른 실행</h2>
  <div class="quick-grid">
    {#each quickActions as action}
      <button
        class="quick-card"
        style="--accent: {action.color}"
        on:click={() => navigate(action.id)}
      >
        <span class="quick-icon">{action.icon}</span>
        <span class="quick-title">{action.title}</span>
        <span class="quick-desc">{action.desc}</span>
      </button>
    {/each}
  </div>

  <!-- 주의사항 -->
  <div class="notice">
    <span class="notice-icon">ℹ️</span>
    <div>
      <strong>안전 원칙:</strong> 감염 디스크는 항상 읽기 전용으로 마운트됩니다.
      모든 복구 작업 전 데이터 백업을 먼저 수행하세요.
    </div>
  </div>
</div>

<style>
  .dashboard { max-width: 900px; }
  .page-title { font-size: 24px; font-weight: 700; color: #e6edf3; margin-bottom: 4px; }
  .page-desc { color: #8b949e; margin-bottom: 24px; }

  .status-section {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
    margin-bottom: 32px;
  }
  .status-card {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 8px;
    padding: 16px;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .status-card.danger { border-color: #f85149; }
  .status-icon { font-size: 24px; }
  .status-title { font-size: 12px; color: #8b949e; margin-bottom: 2px; }
  .status-value { font-size: 14px; font-weight: 600; }
  .status-value.safe { color: #3fb950; }

  .section-title { font-size: 16px; font-weight: 600; margin-bottom: 16px; color: #8b949e; }

  .quick-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 12px;
    margin-bottom: 32px;
  }
  .quick-card {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 10px;
    padding: 20px 16px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    transition: all 0.2s;
    color: #e6edf3;
    text-align: center;
  }
  .quick-card:hover {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 10%, #161b22);
    transform: translateY(-2px);
  }
  .quick-icon { font-size: 28px; }
  .quick-title { font-size: 14px; font-weight: 600; }
  .quick-desc { font-size: 11px; color: #8b949e; }

  .notice {
    background: #1a2d3d;
    border: 1px solid #58a6ff;
    border-radius: 8px;
    padding: 14px 16px;
    display: flex;
    gap: 10px;
    font-size: 13px;
    color: #a5d6ff;
  }
  .notice-icon { font-size: 18px; flex-shrink: 0; }
</style>
