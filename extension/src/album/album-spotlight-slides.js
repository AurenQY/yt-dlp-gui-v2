import { isVideo } from './album-data.js';
import { albumPostInfo } from './album-post-info.js';

/** One Spotlight gallery; image captions follow posts, videos keep controls clear. */
export function albumSpotlightSlides(media, gallery = {}, postByMediaIndex = []) {
  return media.map((item, index) => {
    const post = albumPostInfo(postByMediaIndex[index] || gallery);
    return {
      src: item.url,
      title: post.title,
      description: post.description,
      ...(isVideo(item)
        // Spotlight hides its footer when title, description and button are false.
        // Use that native contract rather than covering or repositioning video controls.
        ? { media: 'video', fit: 'contain', controls: true, autoplay: false,
          poster: item.thumbnail || '', title: false, description: false, button: false,
          // Video fullscreen belongs to native playback controls, not the lightbox.
          // Per-slide whitelist restores image tools automatically on the next image.
          control: ['page', 'prev', 'next', 'close'] }
        : {}),
    };
  });
}
