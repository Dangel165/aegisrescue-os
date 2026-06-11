<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri'
  import { listen } from '@tauri-apps/api/event'
  import { onMount, onDestroy } from 'svelte'

  let events: any[] = []
  let error = ''
  let unlisten: (() => void) | undefined

  function prependEvent(ev: any) {
    events = [ev, ...events].slice(0, 200)
  }

  onMount(async () => {
    try {
      events = await invoke('get_events', { limit: 100 })
    } catch (e: any) {
      error = e.toString()
    }

    unlisten = await listen<any>('daemon-stream', (msg) => {
      const p = msg.payload
      if (p?.stream === 'event' && p.event) {
        prependEvent(p.event)
      } else if (p?.stream === 'threat_found' && p.threat) {
        prependEvent({
          timestamp: p.threat.detected_at,
          severity: 'Critical',
          event_type: 'ThreatDetected',
          message: `Threat: ${p.threat.path}`,
          context: p.threat,
        })
      } else if (p?.stream === 'job_progress') {
        prependEvent({
          timestamp: new Date().toISOString(),
          severity: 'Info',
          event_type: 'UserAction',
          message: p.progress?.message ?? 'Job progress',
          context: p,
        })
      }
    })
  })

  onDestroy(() => unlisten?.())

  function severityColor(s: string) {
    return s === 'Critical' || s === 'Error' ? '#f85149' : s === 'Warning' ? '#d29922' : '#3fb950'
  }
</script>

<div class="panel">
  <h2 class="panel-title">📜 이벤트 로그</h2>
  {#if error}<div class="alert">{error}</div>{/if}
  <div class="log-list">
    {#each events as ev}
      <div class="log-entry">
        <span class="log-ts">{ev.timestamp?.slice(0,19) ?? ''}</span>
        <span class="log-sev" style="color:{severityColor(ev.severity)}">[{ev.severity}]</span>
        <span class="log-type">{ev.event_type}</span>
        <span class="log-msg">{ev.message}</span>
      </div>
    {:else}
      <div class="empty">이벤트 없음 — aegisd 실행 후 스캔하면 실시간 표시됩니다</div>
    {/each}
  </div>
</div>

<style>
  .panel { max-width: 900px; }
  .panel-title { font-size: 20px; font-weight: 700; margin-bottom: 24px; }
  .log-list { font-family: monospace; font-size: 12px; max-height: 70vh; overflow-y: auto; }
  .log-entry { display: grid; grid-template-columns: 160px 80px 150px 1fr; gap: 8px; padding: 6px 4px; border-bottom: 1px solid #21262d; }
  .log-ts { color: #8b949e; }
  .log-sev { font-weight: 600; }
  .log-type { color: #58a6ff; }
  .log-msg { color: #e6edf3; }
  .empty { color: #8b949e; padding: 32px; text-align: center; }
  .alert { background: #3d1a1a; border: 1px solid #f85149; border-radius: 6px; padding: 10px 14px; color: #f85149; font-size: 13px; margin-bottom: 12px; }
</style>
