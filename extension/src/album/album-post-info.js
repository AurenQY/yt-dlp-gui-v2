/** One post can contain several media: show its post text in every Spotlight slide. */
export function albumPostInfo(gallery = {}) {
  const text = value => typeof value === 'string' ? value.trim() : '';
  const title = [text(gallery.creator), text(gallery.date)].filter(Boolean).join(' · ')
    || text(gallery.siteName);
  const description = text(gallery.description) || text(gallery.title);
  return { title: title === description ? '' : title, description };
}
