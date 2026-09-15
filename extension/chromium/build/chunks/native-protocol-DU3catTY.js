//#region src/shared/native-protocol.js
var e = Object.freeze({
	wireVersion: 1,
	capabilities: Object.freeze(["fields.ignore-unknown.v1", "events.ignore-unknown.v1"])
});
function t(t) {
	return {
		...t,
		client: {
			wireVersion: e.wireVersion,
			capabilities: [...e.capabilities]
		}
	};
}
function n(e) {
	let t = e?.protocol;
	return !t || typeof t != "object" ? {
		advertised: !1,
		wireVersion: null,
		capabilities: [],
		limits: {}
	} : {
		advertised: !0,
		wireVersion: Number.isInteger(t.wireVersion) ? t.wireVersion : null,
		capabilities: Array.isArray(t.capabilities) ? t.capabilities.filter((e) => typeof e == "string") : [],
		limits: t.limits && typeof t.limits == "object" ? { ...t.limits } : {}
	};
}
function r(e, t) {
	return Array.isArray(e?.capabilities) && e.capabilities.includes(t);
}
//#endregion
export { r as n, t as r, n as t };
