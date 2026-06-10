<script lang="ts">
  import { t } from '../i18n'
  import { currentStep, config, totalSize, COMPONENT_SIZES } from '../store'

  interface Component {
    id: string
    required: boolean
    nameKey: string
    descKey: string
    icon: string
  }

  const COMPONENTS: Component[] = [
    { id: 'core',     required: true,  nameKey: 'component_core',     descKey: 'component_core_desc',     icon: '⚙' },
    { id: 'scanner',  required: false, nameKey: 'component_scanner',  descKey: 'component_scanner_desc',  icon: '🔍' },
    { id: 'ai',       required: false, nameKey: 'component_ai',       descKey: 'component_ai_desc',       icon: '🤖' },
    { id: 'recover',  required: false, nameKey: 'component_recover',  descKey: 'component_recover_desc',  icon: '🔓' },
    { id: 'boot',     required: false, nameKey: 'component_boot',     descKey: 'component_boot_desc',     icon: '🖥' },
    { id: 'forensic', required: false, nameKey: 'component_forensic', descKey: 'component_forensic_desc', icon: '🔬' },
    { id: 'image',    required: false, nameKey: 'component_image',    descKey: 'component_image_desc',    icon: '💾' },
    { id: 'sandbox',  required: false, nameKey: 'component_sandbox',  descKey: 'component_sandbox_desc',  icon: '📦' },
    { id: 'gui',      required: false, nameKey: 'component_gui',      descKey: 'component_gui_desc',      icon: '🖱' },
  ]

  let selected = new Set<string>($config.components)
  selected.add('core')

  function toggle(id: string, required: boolean) {
    if (required) return
    if (selected.has(id)) {
      selected.delete(id)
    } else {
      selected.add(id)
    }
    selected = new Set(selected)
    config.update(c => ({ ...c, components: Array.from(selected) }))
  }

  $: sizeMb = totalSize(Array.from(selected))
  $: sizeText = sizeMb >= 1024
    ? `${(sizeMb / 1024).toFixed(1)} GiB`
    : `${sizeMb} MiB`

  function next() { currentStep.set('disk') }
  function back() { currentStep.set('license') }
</script>

<div class="step-container">
  <h2 class="step-title">{$t.components_title}</h2>
  <p class="step-subtitle">{$t.components_subtitle}</p>

  <div class="components-list">
    {#each COMPONENTS as comp}
      {@const isSelected = selected.has(comp.id)}
      <div
        class="comp-card card"
        class:selected={isSelected}
        class:required={comp.required}
        on:click={() => toggle(comp.id, comp.required)}
        role="checkbox"
        aria-checked={isSelected}
        tabindex="0"
        on:keydown={(e) => e.key === ' ' && toggle(comp.id, comp.required)}
      >
        <div class="comp-check">
          {#if comp.required}
            <div class="check-locked">🔒</div>
          {:else}
            <div class="check-box" class:checked={isSelected}>
              {#if isSelected}✓{/if}
            </div>
          {/if}
        </div>
        <div class="comp-icon">{comp.icon}</div>
        <div class="comp-info">
          <div class="comp-name">{$t[comp.nameKey]}</div>
          <div class="comp-desc">{$t[comp.descKey]}</div>
        </div>
        <div class="comp-size">
          {$t.component_size}: {COMPONENT_SIZES[comp.id]} MiB
        </div>
      </div>
    {/each}
  </div>

  <div class="size-summary">
    <span class="size-label">예상 총 크기:</span>
    <span class="size-value">{sizeText}</span>
  </div>

  <div class="step-actions">
    <button class="btn btn-secondary" on:click={back}>← {$t.back}</button>
    <button class="btn btn-primary" on:click={next}>{$t.next} →</button>
  </div>
</div>

<style>
  .components-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .comp-card {
    display: grid;
    grid-template-columns: 28px 32px 1fr auto;
    align-items: center;
    gap: 12px;
    cursor: pointer;
    transition: all 0.12s;
  }

  .comp-card.required { cursor: default; opacity: 0.8; }
  .comp-card.required.selected { border-color: #238636; background: rgba(35, 134, 54, 0.08); }

  .check-box {
    width: 18px;
    height: 18px;
    border: 2px solid #30363d;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    color: white;
    transition: all 0.12s;
  }

  .check-box.checked {
    background: #238636;
    border-color: #238636;
    font-weight: 700;
  }

  .check-locked { font-size: 14px; }

  .comp-icon { font-size: 20px; text-align: center; }

  .comp-name {
    font-size: 14px;
    font-weight: 600;
    color: #e6edf3;
    margin-bottom: 2px;
  }

  .comp-desc {
    font-size: 12px;
    color: #8b949e;
    line-height: 1.4;
  }

  .comp-size {
    font-size: 11px;
    color: #484f58;
    white-space: nowrap;
  }

  .size-summary {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 16px;
    padding: 10px 14px;
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 6px;
    font-size: 13px;
  }

  .size-label { color: #8b949e; }
  .size-value { color: #58a6ff; font-weight: 700; font-size: 15px; }
</style>
