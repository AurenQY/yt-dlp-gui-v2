import { t as e } from "./i18n-5PmJGYvv.js";
//#region \0rolldown/runtime.js
var t = Object.create, n = Object.defineProperty, r = Object.getOwnPropertyDescriptor, i = Object.getOwnPropertyNames, a = Object.getPrototypeOf, o = Object.prototype.hasOwnProperty, s = (e, t) => () => (t || (e((t = { exports: {} }).exports, t), e = null), t.exports), c = (e, t) => {
	let r = {};
	for (var i in e) n(r, i, {
		get: e[i],
		enumerable: !0
	});
	return t || n(r, Symbol.toStringTag, { value: "Module" }), r;
}, l = (e, t, a, s) => {
	if (t && typeof t == "object" || typeof t == "function") for (var c = i(t), l = 0, u = c.length, d; l < u; l++) d = c[l], !o.call(e, d) && d !== a && n(e, d, {
		get: ((e) => t[e]).bind(null, d),
		enumerable: !(s = r(t, d)) || s.enumerable
	});
	return e;
}, u = (e, r, i) => (i = e == null ? {} : t(a(e)), l(r || !e || !e.__esModule || !o.call(e, "default") ? n(i, "default", {
	value: e,
	enumerable: !0
}) : i, e));
//#endregion
//#region src/shared/url-validation.js
function d(t) {
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
export { u as i, s as n, c as r, d as t };
