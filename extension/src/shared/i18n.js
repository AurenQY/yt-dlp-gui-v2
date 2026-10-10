import englishMessages from '../../chromium/_locales/en/messages.json' with { type: 'json' };

// One canonical English catalog for both Chrome and non-Chrome contexts.
const FALLBACK_MESSAGES = Object.fromEntries(
  Object.entries(englishMessages).map(([key, value]) => [key, value.message]),
);

export function t(key, values = {}) {
  const localized = globalThis.chrome?.i18n?.getMessage(key);
  const message = localized || FALLBACK_MESSAGES[key] || key;
  return Object.entries(values).reduce(
    (result, [name, value]) => result.replaceAll(`{${name}}`, String(value)),
    message,
  );
}

export function localizeDocument(root = document) {
  root.documentElement.lang = globalThis.chrome?.i18n?.getUILanguage?.() || 'en';
  root.querySelectorAll('[data-i18n]').forEach(element => {
    element.textContent = t(element.dataset.i18n);
  });
  for (const [attribute, dataAttribute] of [
    ['aria-label', 'i18nAriaLabel'],
    ['placeholder', 'i18nPlaceholder'],
    ['title', 'i18nTitle'],
  ]) {
    root.querySelectorAll(`[data-${dataAttribute.replace(/[A-Z]/g, letter => `-${letter.toLowerCase()}`)}]`)
      .forEach(element => element.setAttribute(attribute, t(element.dataset[dataAttribute])));
  }
}
