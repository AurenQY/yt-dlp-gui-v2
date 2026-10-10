<script>
  import check from '../../../../assets/icons/Check.svg?raw';
  import close from '../../../../assets/icons/Close.svg?raw';
  export let name = 'next';
</script>

<span class="action-icon" aria-hidden="true">
  {#if name === 'external'}
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" d="M14 3h7v7M21 3L10 14M10 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-5" /></svg>
  {:else if name === 'check' || name === 'close'}
    {@html (name === 'check' ? check : close).replace(/<\?xml[^?]*\?>\s*/, '')}
  {:else}
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="currentColor" d="M4 11h12.17l-5.59-5.59L12 4l8 8-8 8-1.41-1.41L16.17 13H4z" /></svg>
  {/if}
</span>

<style>
  .action-icon { display: grid; place-items: center; width: var(--action-icon-size, 20px); height: var(--action-icon-size, 20px); flex: none; pointer-events: none; }
  .action-icon :global(svg) { display: block; width: 100%; height: 100%; }
</style>
