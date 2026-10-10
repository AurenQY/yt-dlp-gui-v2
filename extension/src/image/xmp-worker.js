import { instantiateXmp } from './xmp-wasm-runtime.js';
let runtime;
self.onmessage = async ({ data }) => {
  try {
    runtime ||= fetch(new URL('../resources/gallery-xmp.wasm', self.location.href))
      .then(response => { if (!response.ok) throw Error('XMP WASM unavailable'); return response.arrayBuffer(); })
      .then(instantiateXmp)
      .catch(error => { runtime = null; throw error; });
    const write = await runtime;
    const bytes = write(new Uint8Array(data.bytes), data.metadata);
    self.postMessage({ id: data.id, bytes: bytes.buffer }, [bytes.buffer]);
  } catch (error) {
    self.postMessage({ id: data.id, error: String(error?.message || error) });
  }
};
