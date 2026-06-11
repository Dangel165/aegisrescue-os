<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri'
  import { listen } from '@tauri-apps/api/event'
  import { onMount, onDestroy } from 'svelte'
  let entries: any[] = []
  let error = ''
  let unlisten: (() => void) | undefined

  async function reload() {
    try {
      entries = await invoke('list_quarantine')
      error = ''
    } catch (e: any) {
      error = e.toString()
    }
  }

  onMount(async () => {
    await reload()
    unlisten = await listen<any>('daemon-stream', async (msg) => {
      const p = msg.payload
      if (p?.stream === 'event') {
        const t = p.event?.event_type
        if (t === 'QuarantineAdded' || t === 'QuarantineRestored') {
          await reload()
        }
      }
    })
  })

  onDestroy(() => unlisten?.())

  async function restore(id: string) {
    const path = prompt('복원 경로 입력:')
    if (!path) return
    try { await invoke('restore_quarantine', { entryId: id, restorePath: path }); await reload() }
    catch (e: any) { alert(e.toString()) }
  }

  async function del(id: string) {
    if (!confirm('영구 삭제하시겠습니까? (복구 불가)')) return
    try { await invoke('delete_quarantine', { entryId: id }); entries = entries.filter(e => e.id !== id) }
    catch (e: any) { alert(e.toString()) }
  }
</script>

<div class="panel">
  <h2 class="panel-title">🧪 격리 보관소</h2>
  {#if error}<div class="alert">{error}</div>{/if}
  {#if entries.length === 0}
    <div class="empty">격리된 항목이 없습니다</div>
  {:else}
    {#each entries as e}
      <div class="entry">
        <div class="entry-path">{e.original_path}</div>
        <div class="entry-meta">
          <span class="badge-danger">{e.threat?.verdict ?? '?'}</span>
          <span class="meta-score">Score: {e.threat?.risk_score?.toFixed(2) ?? '-'}</span>
        </div>
        <div class="entry-actions">
          <button class="btn-restore" on:click={() => restore(e.id)}>복원</button>
          <button class="btn-delete" on:click={() => del(e.id)}>삭제</button>
        </div>
      </div>
    {/each}
  {/if}
</div>

<style>
  .panel { max-width: 800px; }
  .panel-title { font-size: 20px; font-weight: 700; margin-bottom: 24px; }
  .empty { color: #8b949e; padding: 32px; text-align: center; }
  .entry { background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 12px 16px; margin-bottom: 10px; }
  .entry-path { font-family: monospace; font-size: 13px; color: #e6edf3; margin-bottom: 6px; }
  .entry-meta { display: flex; gap: 10px; align-items: center; margin-bottom: 8px; }
  .badge-danger { background: #3d1a1a; color: #f85149; border: 1px solid #f85149; padding: 2px 8px; border-radius: 10px; font-size: 11px; }
  .meta-score { font-size: 12px; color: #8b949e; }
  .entry-actions { display: flex; gap: 8px; }
  .btn-restore { padding: 4px 12px; background: #1f3a5f; border: 1px solid #58a6ff; color: #58a6ff; border-radius: 4px; cursor: pointer; font-size: 12px; }
  .btn-delete { padding: 4px 12px; background: #3d1a1a; border: 1px solid #f85149; color: #f85149; border-radius: 4px; cursor: pointer; font-size: 12px; }
  .alert { background: #3d1a1a; border: 1px solid #f85149; border-radius: 6px; padding: 10px 14px; color: #f85149; font-size: 13px; margin-bottom: 12px; }
</style>
