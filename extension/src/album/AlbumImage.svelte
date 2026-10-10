<script>
import { t } from '../shared/i18n.js';
  import { onMount, createEventDispatcher } from 'svelte';
  export let item;
  export let session;
  export let index;
  const dispatch = createEventDispatcher();
  let element;
  let src = '';
  let error = '';
  let pending = false;
  let animateReveal = false;
  function imageLoaded(event) {
    session.markDisplayed(item);
    dispatch('load', event);
  }
  onMount(() => {
    let mounted = true;
    let visible = false;
    function release() { if (src) URL.revokeObjectURL(src); src = ''; }
    async function load() {
      if (pending || src) return;
      pending = true;
      error = '';
      try {
        const result = await session.prepare(item);
        if (mounted && visible) {
          animateReveal = !session.hasDisplayed(item);
          src = URL.createObjectURL(result.blob);
        }
      } catch (failure) { if (mounted) error = String(failure?.message || failure); }
      finally { pending = false; }
    }
    const observer = new IntersectionObserver(entries => {
      visible = entries[0].isIntersecting;
      if (visible) load();
      else release();
    }, { rootMargin: '120px' });
    observer.observe(element);
    return () => { mounted = false; observer.disconnect(); release(); };
  });
</script>

<div class="album-image" bind:this={element}>
  {#if src}<img {src} class:album-image-first-reveal={animateReveal} alt={t('albumImage', { number: index + 1 })} onload={imageLoaded} />
  {:else if error}<span class="album-image-error" role="status">{error}</span>
  {:else}<span class="album-image-pending">{pending ? t('albumProcessingImage') : t('albumWaiting')}</span>{/if}
</div>
