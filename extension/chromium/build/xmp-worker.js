import { t as e } from "./chunks/xmp-wasm-runtime-BhhtyP_M.js";
//#region src/image/xmp-worker.js
var t;
self.onmessage = async ({ data: n }) => {
	try {
		t ||= fetch(new URL("../resources/gallery-xmp.wasm", self.location.href)).then((e) => {
			if (!e.ok) throw Error("XMP WASM unavailable");
			return e.arrayBuffer();
		}).then(e).catch((e) => {
			throw t = null, e;
		});
		let r = (await t)(new Uint8Array(n.bytes), n.metadata);
		self.postMessage({
			id: n.id,
			bytes: r.buffer
		}, [r.buffer]);
	} catch (e) {
		self.postMessage({
			id: n.id,
			error: String(e?.message || e)
		});
	}
};
//#endregion
