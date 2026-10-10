/** One WASM instance per dedicated worker; caller retains original bytes. */
export async function instantiateXmp(bytes) {
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const api = instance.exports;
  const encoder = new TextEncoder();
  const decoder = new TextDecoder();
  return (image, metadata) => {
    const request = encoder.encode(JSON.stringify(metadata));
    let inputPointer = 0, metadataPointer = 0, outputPointer = 0, outputLength = 0;
    try {
      inputPointer = api.xmp_alloc(image.byteLength);
      metadataPointer = api.xmp_alloc(request.byteLength);
      new Uint8Array(api.memory.buffer, inputPointer, image.byteLength).set(image);
      new Uint8Array(api.memory.buffer, metadataPointer, request.byteLength).set(request);
      outputPointer = api.xmp_write(inputPointer, image.byteLength, metadataPointer, request.byteLength);
      const payloadLength = new DataView(api.memory.buffer).getUint32(outputPointer, true);
      outputLength = payloadLength + 5;
      const status = new Uint8Array(api.memory.buffer, outputPointer + 4, 1)[0];
      const output = new Uint8Array(api.memory.buffer, outputPointer + 5, payloadLength).slice();
      if (status) throw Error(decoder.decode(output));
      return output;
    } finally {
      if (outputPointer) api.xmp_free(outputPointer, outputLength);
      if (metadataPointer) api.xmp_free(metadataPointer, request.byteLength);
      if (inputPointer) api.xmp_free(inputPointer, image.byteLength);
    }
  };
}
