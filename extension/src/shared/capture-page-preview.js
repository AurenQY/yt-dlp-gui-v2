import { collectPageSnapshot } from './page-snapshot.js';
import { collectYouTubePlayerThumbnail, youtubePageVideoId,
  mergeYouTubePlayerThumbnail, withYouTubeShortsThumbnailFallback } from './youtube-page-thumbnail.js';
import { collectTwitterPostThumbnail, twitterPagePostId, mergeTwitterPostThumbnail,
  collectTwitterProfileThumbnail, twitterPageProfileHandle, mergeTwitterProfileThumbnail } from './twitter-page-thumbnail.js';
import { selectPageThumbnail } from './page-thumbnail-policy.js';

/** DOM capture -> optional page-kind adapter -> one common thumbnail policy. */
export async function capturePagePreview(tabId) {
  if (!Number.isInteger(tabId) || !chrome.scripting?.executeScript) return null;
  const inject = async (func, world = 'ISOLATED') => {
    const [execution] = await chrome.scripting.executeScript({ target: { tabId }, func, world });
    return execution?.result;
  };
  try {
    const snapshot = await inject(collectPageSnapshot);
    if (!(snapshot?.schemaVersion >= 1 && snapshot.schemaVersion <= 3)) return null;
    let resolved = snapshot;
    if (twitterPagePostId(snapshot.url)) {
      let preview = null;
      try { preview = await inject(collectTwitterPostThumbnail); } catch {}
      resolved = mergeTwitterPostThumbnail(snapshot, preview);
    } else if (twitterPageProfileHandle(snapshot.url)) {
      let preview = null;
      try { preview = await inject(collectTwitterProfileThumbnail); } catch {}
      resolved = mergeTwitterProfileThumbnail(snapshot, preview);
    } else if (youtubePageVideoId(snapshot.url)) {
      let preview = null;
      try { preview = await inject(collectYouTubePlayerThumbnail, 'MAIN'); } catch {}
      resolved = withYouTubeShortsThumbnailFallback(mergeYouTubePlayerThumbnail(snapshot, preview));
    }
    return selectPageThumbnail(resolved);
  } catch { return null; }
}
