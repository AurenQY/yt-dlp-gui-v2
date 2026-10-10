<script>
  // A site's metadata image may be blocked from the extension even when it is
  // visible in the source tab. Try the loaded-post image and metadata URL in
  // order, then display a deliberate fallback instead of a broken image icon.
  export let sources = [];
  export let siteIcon = '';
  let index = 0;
  let iconFailed = false;
  let loaded = false;
  let previousSources = '';
  $: candidates = [...new Set(sources.filter(value => typeof value === 'string' && value))];
  $: if (candidates.join('\n') !== previousSources) {
    previousSources = candidates.join('\n');
    index = 0;
    loaded = false;
    iconFailed = false;
  }

  function nextCandidate() {
    loaded = false;
    index += 1;
  }
</script>

<div class="source-cover" class:loaded>
  {#if index < candidates.length}
    {#if loaded}<img class="blur-cover" src={candidates[index]} alt="" aria-hidden="true" />{/if}
    <img class="foreground" src={candidates[index]} alt="" onload={() => { loaded = true; }} onerror={nextCandidate} />
  {:else if siteIcon && !iconFailed}
    <img class="fallback-icon" src={siteIcon} alt="" onerror={() => { iconFailed = true; }} />
  {:else}
    <span class="fallback-glyph" aria-hidden="true">▧</span>
  {/if}
</div>

<style>
  /* Source and result previews use the desktop-sized semantic cover contract.
     Intrinsic image dimensions do not control the row; foreground stays contained. */
  .source-cover {
    display: grid;
    position: relative;
    width: var(--ui-cover-width);
    height: var(--ui-cover-height);
    flex: 0 0 var(--ui-cover-width);
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    place-items: center;
    background: var(--egui-extreme);
    border: 1px solid var(--egui-border);
  }
  .source-cover img {
    display: block;
    position: absolute;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    object-fit: contain;
    object-position: center;
  }
  .foreground { opacity: 0; transition: opacity 400ms ease-out; }
  .loaded .foreground { opacity: 1; }
  .source-cover .blur-cover { object-fit: cover; filter: blur(8px); opacity: .25; animation: cover-in 650ms ease-out; }
  @keyframes cover-in { from { opacity: 0; } to { opacity: .25; } }
  @media (prefers-reduced-motion: reduce) { .foreground { transition: none; } .source-cover .blur-cover { animation: none; } }
  .fallback-icon { width: 36px; height: 36px; }
  .fallback-glyph { font-size: 30px; color: var(--egui-text-muted); }
</style>
