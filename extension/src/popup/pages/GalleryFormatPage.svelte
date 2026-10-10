<script>
  import { Block, Link, Navbar, Page } from 'framework7-svelte';
  import { popupController as controller } from '../popup-runtime.js';
  import { galleryPostGroups } from '../../shared/gallery-post-groups.js';
  import { GALLERY_DOWNLOAD_MODES, galleryDownloadModeLabelKey } from '../../shared/gallery-download-mode.js';
  import { t } from '../../shared/i18n.js';

  export let galleryIndex;
  $: index = Number(galleryIndex);
  $: state = $controller;
  $: item = galleryPostGroups(state.analysisByBackend.g?.items || [], state.sourceUrl)[index];
  $: selected = item ? controller.galleryModeForGroup(item, state) : '';
</script>

<Page name="gallery-format">
  <Navbar title={t('galleryDownloadMode')} backLink={true} />
  {#if item}
    <div class="gallery-format-choices" role="radiogroup" aria-label={t('galleryDownloadMode')}>
      {#each GALLERY_DOWNLOAD_MODES as mode}
        <Link href="#" back noLinkClass
          class={`gallery-format-row${selected === mode ? ' selected' : ''}`}
          role="radio" aria-checked={selected === mode}
          onClick={() => controller.setGalleryDownloadMode(mode, item.key)}>
          <span class="radio-cell">
            <input type="radio" name={`gallery-format-${index}`} value={mode}
              checked={selected === mode} tabindex="-1" aria-hidden="true" />
          </span>
          <span>{t(galleryDownloadModeLabelKey(mode))}</span>
        </Link>
      {/each}
    </div>
  {:else}
    <Block strong inset>{t('noFormats')}</Block>
  {/if}
</Page>
