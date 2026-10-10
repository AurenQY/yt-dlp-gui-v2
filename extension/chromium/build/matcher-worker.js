import { t as e } from "./chunks/i18n-SwgWtGUP.js";
import { t } from "./chunks/url-matcher-Dt5Hrtu5.js";
//#region src/matcher/matcher-worker.js
var n;
self.onmessage = async ({ data: r }) => {
	try {
		n ??= fetch(r.indexUrl).then((t) => {
			if (!t.ok) throw Error(e("matcherIndexLoadFailed"));
			return t.json();
		}).then(t);
		let i = await n;
		self.postMessage({
			id: r.id,
			result: i.classify(r.url),
			skipped: i.failures.length
		});
	} catch (e) {
		self.postMessage({
			id: r.id,
			error: e.message
		});
	}
};
//#endregion
