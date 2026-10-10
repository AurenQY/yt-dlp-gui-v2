/** Serialize document eviction; snapshots/commands must never race a reload. */
export function createTabDocumentResetState() {
  const revisions = new Map();
  const pending = new Map();
  const revision = tabId => revisions.get(tabId) || 0;
  return {
    revision,
    isCurrent: (tabId, expected) => revision(tabId) === expected,
    reset(tabId, clear) {
      revisions.set(tabId, revision(tabId) + 1);
      const previous = pending.get(tabId) || Promise.resolve();
      const operation = previous.catch(() => {}).then(clear);
      pending.set(tabId, operation);
      const release = () => {
        if (pending.get(tabId) === operation) pending.delete(tabId);
      };
      operation.then(release, release);
      return operation;
    },
    async wait(tabId) {
      while (pending.has(tabId)) await pending.get(tabId);
    },
  };
}
