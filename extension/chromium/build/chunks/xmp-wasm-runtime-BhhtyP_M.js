//#region src/image/xmp-wasm-runtime.js
async function e(e) {
	let { instance: t } = await WebAssembly.instantiate(e, {}), n = t.exports, r = new TextEncoder(), i = new TextDecoder();
	return (e, t) => {
		let a = r.encode(JSON.stringify(t)), o = 0, s = 0, c = 0, l = 0;
		try {
			o = n.xmp_alloc(e.byteLength), s = n.xmp_alloc(a.byteLength), new Uint8Array(n.memory.buffer, o, e.byteLength).set(e), new Uint8Array(n.memory.buffer, s, a.byteLength).set(a), c = n.xmp_write(o, e.byteLength, s, a.byteLength);
			let t = new DataView(n.memory.buffer).getUint32(c, !0);
			l = t + 5;
			let r = new Uint8Array(n.memory.buffer, c + 4, 1)[0], u = new Uint8Array(n.memory.buffer, c + 5, t).slice();
			if (r) throw Error(i.decode(u));
			return u;
		} finally {
			c && n.xmp_free(c, l), s && n.xmp_free(s, a.byteLength), o && n.xmp_free(o, e.byteLength);
		}
	};
}
//#endregion
export { e as t };
