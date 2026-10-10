const POPUP_WIDTH = 480;
const POPUP_MIN_HEIGHT = 25;
const POPUP_MAX_HEIGHT = 600;

function px(value) {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function clampHeight(value) {
  return Math.max(POPUP_MIN_HEIGHT, Math.min(POPUP_MAX_HEIGHT, Math.ceil(value)));
}

function currentPage(root) {
  return root.querySelector('.page-current') || root.querySelector('.page');
}

function pageContent(page) {
  return page?.querySelector(':scope > .page-content') || null;
}

function pageName(page) {
  return page?.dataset.name || '';
}

function isNestedSelectionPage(page) {
  return [
    'format-grid',
    'smart-select-page',
    'time-range',
    'chapter-range',
    'gallery-format',
  ].includes(pageName(page));
}

function naturalPageHeight(pageContent) {
  const contentRect = pageContent.getBoundingClientRect();
  const contentStyle = getComputedStyle(pageContent);
  const paddingTop = px(contentStyle.paddingTop);
  const paddingBottom = px(contentStyle.paddingBottom);
  let bottom = paddingTop;

  for (const child of pageContent.children) {
    const rects = child.getClientRects();
    if (!rects.length) continue;

    const childRect = child.getBoundingClientRect();
    const childStyle = getComputedStyle(child);
    const relativeBottom = childRect.bottom - contentRect.top + pageContent.scrollTop;
    bottom = Math.max(bottom, relativeBottom + px(childStyle.marginBottom));
  }

  return clampHeight(bottom + paddingBottom);
}

function applyHostHeight(height, app) {
  const value = `${clampHeight(height)}px`;
  document.documentElement.style.height = value;
  document.body.style.height = value;
  app.style.height = value;
}

export function installPopupHostSizing(app) {
  if (!app) return () => {};

  document.documentElement.style.width = `${POPUP_WIDTH}px`;
  document.body.style.width = `${POPUP_WIDTH}px`;
  app.style.width = `${POPUP_WIDTH}px`;
  applyHostHeight(POPUP_MIN_HEIGHT, app);

  let frame = 0;
  let disposed = false;
  let detailPageHeight = null;

  const measure = () => {
    frame = 0;
    if (disposed) return;

    const page = currentPage(app);
    const content = pageContent(page);
    if (!content) return;

    const name = pageName(page);
    if (name === 'detail' || name === 'gallery-detail') {
      detailPageHeight = naturalPageHeight(content);
      applyHostHeight(detailPageHeight, app);
      return;
    }
    if (detailPageHeight != null && isNestedSelectionPage(page)) {
      applyHostHeight(detailPageHeight, app);
      return;
    }

    detailPageHeight = null;
    applyHostHeight(naturalPageHeight(content), app);
  };

  const schedule = () => {
    if (disposed || frame) return;
    frame = requestAnimationFrame(measure);
  };

  const mutationObserver = new MutationObserver(schedule);
  mutationObserver.observe(app, {
    childList: true,
    subtree: true,
    characterData: true,
    attributes: true,
    attributeFilter: ['class'],
  });

  app.addEventListener('load', schedule, true);
  app.addEventListener('page:afterin', schedule);
  app.addEventListener('transitionend', schedule);
  app.addEventListener('animationend', schedule);

  schedule();

  return () => {
    disposed = true;
    if (frame) cancelAnimationFrame(frame);
    mutationObserver.disconnect();
    app.removeEventListener('load', schedule, true);
    app.removeEventListener('page:afterin', schedule);
    app.removeEventListener('transitionend', schedule);
    app.removeEventListener('animationend', schedule);
  };
}
