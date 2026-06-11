<script lang="ts">
  import { t } from '../i18n'
  import { currentStep } from '../store'

  let agreed = false

  const GPL3_TEXT = `AegisRescue Linux — Beta Release 0.1.0
Copyright (C) 2025 AegisRescue Project

BETA NOTICE: This is beta software. Features may be incomplete or unstable.
Use at your own risk. No warranty is expressed or implied.

This program is licensed under the GNU General Public License v3.0.
See LICENSE in the AegisRescue source tree or https://www.gnu.org/licenses/gpl-3.0.txt

────────────────────────────────────────────────────────────────────────

GNU GENERAL PUBLIC LICENSE
Version 3, 29 June 2007

Copyright (C) 2007 Free Software Foundation, Inc.
<https://fsf.org/>

Everyone is permitted to copy and distribute verbatim copies of this
license document, but changing it is not allowed.

PREAMBLE

The GNU General Public License is a free, copyleft license for
software and other kinds of works.

When we speak of free software, we are referring to freedom, not price.
Our General Public Licenses are designed to make sure that you have the
freedom to distribute copies of free software (and charge for them if
you wish), that you receive source code or can get it if you want it,
that you can change the software or use pieces of it in new free
programs, and that you know you can do these things.

To protect your rights, we need to prevent others from denying you
these rights or requiring you to surrender the rights. Therefore, you
have certain responsibilities if you distribute copies of the software,
or if you modify it: responsibilities to respect the freedom of others.

TERMS AND CONDITIONS

0. Definitions.

"This License" refers to version 3 of the GNU General Public License.

"The Program" refers to any copyrightable work licensed under this
License. Each licensee is addressed as "you".

1. Source Code.

The "source code" for a work means the preferred form of the work
for making modifications to it. "Object code" means any non-source
form of a work.

2. Basic Permissions.

All rights granted under this License are granted for the term of
copyright on the Program, and are irrevocable provided the stated
conditions are met. This License explicitly affirms your unlimited
permission to run the unmodified Program.

...

(Full license text: https://www.gnu.org/licenses/gpl-3.0.txt)`

  function next() {
    if (agreed) currentStep.set('components')
  }

  function back() {
    currentStep.set('preflight')
  }
</script>

<div class="step-container">
  <h2 class="step-title">{$t.license_title}</h2>
  <p class="step-subtitle">{$t.license_subtitle}</p>

  <div class="license-box">
    <pre class="license-text">{GPL3_TEXT}</pre>
  </div>

  <div class="agree-row">
    <label class="agree-label">
      <input type="checkbox" bind:checked={agreed} class="agree-check" />
      <span class="agree-text">{$t.license_agree}</span>
    </label>
  </div>

  {#if !agreed}
    <div class="info-box" style="margin-top: 12px;">
      {$t.license_must_agree}
    </div>
  {/if}

  <div class="step-actions">
    <button class="btn btn-secondary" on:click={back}>← {$t.back}</button>
    <button class="btn btn-primary" on:click={next} disabled={!agreed}>
      {$t.next} →
    </button>
  </div>
</div>

<style>
  .license-box {
    background: #0d1117;
    border: 1px solid #30363d;
    border-radius: 8px;
    padding: 16px;
    height: 320px;
    overflow-y: auto;
    margin-bottom: 20px;
  }

  .license-text {
    font-family: 'Courier New', monospace;
    font-size: 12px;
    color: #8b949e;
    white-space: pre-wrap;
    line-height: 1.6;
  }

  .agree-row {
    padding: 4px 0;
  }

  .agree-label {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    cursor: pointer;
    font-size: 14px;
    color: #e6edf3;
  }

  .agree-check {
    width: 16px;
    height: 16px;
    margin-top: 2px;
    accent-color: #238636;
    flex-shrink: 0;
  }

  .agree-text { line-height: 1.5; }
</style>
