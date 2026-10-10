import { t as e } from "./i18n-SwgWtGUP.js";
//#region src/shared/url-validation.js
function t(t) {
	if (typeof t != "string") throw Error(e("invalidUrlType"));
	let n = t.trim();
	if (n.length > 8192 || /[\u0000-\u0020\u007f]/u.test(n)) throw Error(e("invalidUrlLength"));
	let r;
	try {
		r = new URL(n);
	} catch {
		throw Error(e("invalidUrlComplete"));
	}
	if (!["http:", "https:"].includes(r.protocol) || !r.hostname) throw Error(e("invalidUrlProtocol"));
	if (r.username || r.password) throw Error(e("invalidUrlCredentials"));
	return n;
}
//#endregion
export { t };
