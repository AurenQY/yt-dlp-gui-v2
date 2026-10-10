import { t as e } from "./i18n-SwgWtGUP.js";
import { t } from "./site-permission-request-C1rASNQI.js";
//#region src/album/album-data.js
function n(e) {
	try {
		let t = new URL(e);
		return ["https:", "http:"].includes(t.protocol) && !!t.hostname && !t.username && !t.password;
	} catch {
		return !1;
	}
}
function r(e) {
	return Array.isArray(e) && e.length > 0 && e.every((e) => n(e?.url));
}
function i(e) {
	return [
		"mp4",
		"webm",
		"mov",
		"m3u8"
	].includes(String(e?.extension || "").toLowerCase());
}
function a(e, t) {
	let n = e?.relativePath?.replaceAll("\\", "/").split("/").at(-1);
	if (n) return n;
	let r = String(e?.extension || "").replace(/[^a-z0-9]/gi, "").slice(0, 10).toLowerCase(), i = String(e?.filename || `media-${t + 1}`).replace(/[\\/:*?"<>|\x00-\x1f]/g, "_").replace(/^\.+/, "").slice(0, 100) || `media-${t + 1}`;
	return r && !i.toLowerCase().endsWith(`.${r}`) ? `${i}.${r}` : i;
}
function o(e) {
	return String(e?.title || "gallery").replace(/[\\/:*?"<>|\x00-\x1f]/g, "_").slice(0, 64).trim() || "gallery";
}
function s(e) {
	return Array.isArray(e?.posts) && e.posts.length ? e.posts.flatMap((e) => Array.isArray(e?.media) ? e.media : []) : e?.media;
}
async function c(t, r) {
	let i = new URL(r).searchParams.get("id");
	if (!i || !/^[0-9a-f-]{36}$/i.test(i)) throw Error(e("albumNotFound"));
	let a = `galleryAlbum:${i}`, o = (await t.get(a))[a], c = s(o);
	if ((!Array.isArray(c) || !c.length) && !o?.timeline) throw Error(e("albumExpired"));
	let l = (c || []).filter((e) => n(e?.url));
	if (!l.length && !o?.timeline) throw Error(e("albumEmpty"));
	return {
		gallery: o,
		media: l,
		key: a
	};
}
//#endregion
//#region src/shared/operation-diagnostics.js
var l = "operationDiagnostics", u = Promise.resolve();
function d() {
	return crypto.randomUUID();
}
function f(e, t = {}) {
	if (typeof e != "string" || !/^[a-z.-]{1,80}$/.test(e)) return u;
	let n = {};
	for (let e of [
		"traceId",
		"parentTraceId",
		"backend",
		"phase",
		"status"
	]) {
		let r = t[e];
		typeof r == "string" && /^[A-Za-z0-9_-]{1,80}$/.test(r) && (n[e] = r);
	}
	for (let e of [
		"elapsedMs",
		"cookieCount",
		"itemCount",
		"postCount",
		"nextStart",
		"completedItems"
	]) Number.isFinite(t[e]) && t[e] >= 0 && (n[e] = t[e]);
	let r = {
		timestampMs: Date.now(),
		event: e,
		...n
	};
	return u = u.catch(() => {}).then(async () => {
		let e = await chrome.storage.local.get(l), t = Array.isArray(e[l]) ? e[l] : [];
		await chrome.storage.local.set({ [l]: [...t.slice(-999), r] });
	}).catch(() => console.warn("[operation-diagnostics] journal write unavailable")), u;
}
async function p() {
	return await u, (await chrome.storage.local.get(l))[l] || [];
}
//#endregion
//#region src/album/zip-store.js
var m = new TextEncoder(), h = /* @__PURE__ */ new Uint32Array(256);
for (let e = 0; e < 256; e += 1) {
	let t = e;
	for (let e = 0; e < 8; e += 1) t = t & 1 ? t >>> 1 ^ 3988292384 : t >>> 1;
	h[e] = t >>> 0;
}
function g(e) {
	let t = -1;
	for (let n of e) t = h[(t ^ n) & 255] ^ t >>> 8;
	return (t ^ -1) >>> 0;
}
function _(e) {
	return new Uint8Array(e);
}
function v(e, t, n) {
	e.setUint16(t, n, !0);
}
function y(e, t, n) {
	e.setUint32(t, n, !0);
}
function b(e) {
	let t = Math.max(1980, e.getFullYear());
	return {
		time: e.getHours() << 11 | e.getMinutes() << 5 | e.getSeconds() >> 1,
		date: t - 1980 << 9 | e.getMonth() + 1 << 5 | e.getDate()
	};
}
function x(t) {
	if (!t.length || t.length > 65535) throw Error(e("zipItemsLimit"));
	let n = [], r = [], i = 0, a = b(/* @__PURE__ */ new Date());
	for (let e of t) {
		let t = m.encode(e.name), o = e.bytes instanceof Uint8Array ? e.bytes : new Uint8Array(e.bytes);
		if (!t.length || t.length > 65535 || o.length > 4294967295) throw Error("Invalid ZIP item.");
		let s = g(o), c = _(30 + t.length), l = new DataView(c.buffer);
		y(l, 0, 67324752), v(l, 4, 20), v(l, 6, 2048), v(l, 8, 0), v(l, 10, a.time), v(l, 12, a.date), y(l, 14, s), y(l, 18, o.length), y(l, 22, o.length), v(l, 26, t.length), c.set(t, 30), n.push(c, o);
		let u = _(46 + t.length), d = new DataView(u.buffer);
		y(d, 0, 33639248), v(d, 4, 20), v(d, 6, 20), v(d, 8, 2048), v(d, 10, 0), v(d, 12, a.time), v(d, 14, a.date), y(d, 16, s), y(d, 20, o.length), y(d, 24, o.length), v(d, 28, t.length), y(d, 42, i), u.set(t, 46), r.push(u), i += c.length + o.length;
	}
	let o = r.reduce((e, t) => e + t.length, 0);
	if (i + o + 22 > 4294967295) throw Error(e("zipSizeLimit"));
	let s = _(22), c = new DataView(s.buffer);
	return y(c, 0, 101010256), v(c, 8, t.length), v(c, 10, t.length), y(c, 12, o), y(c, 16, i), new Blob([
		...n,
		...r,
		s
	], { type: "application/zip" });
}
//#endregion
//#region src/album/album-download-path.js
function S(t) {
	let n = t?.relativePath;
	if (typeof n != "string" || !n) throw Error(e("galleryPathMissing"));
	let r = n.replaceAll("\\", "/");
	if (r.split("/").some((e) => !e || e === "." || e === ".." || /[<>:"|?*\x00-\x1f\x7f]/.test(e) || /[. ]$/.test(e) || /^(?:con|prn|aux|nul|com[1-9¹²³]|lpt[1-9¹²³])(?:\.|$)/i.test(e))) throw Error(e("galleryPathInvalid"));
	return r;
}
function C(e) {
	let t = /* @__PURE__ */ new Set();
	return e.map((e) => {
		let n = e.lastIndexOf("/"), r = e.slice(0, n + 1), i = e.slice(n + 1), a = i.lastIndexOf("."), o = a > 0 ? i.slice(0, a) : i, s = a > 0 ? i.slice(a) : "", c = e, l = 2;
		for (; t.has(c.normalize("NFC").toLowerCase());) c = `${r}${o} (${l++})${s}`;
		return t.add(c.normalize("NFC").toLowerCase()), c;
	});
}
//#endregion
//#region src/album/album-download.js
async function w(n, r) {
	let i = [...new Set(n.map((e) => new URL(e.url).origin + "/*"))];
	if (!await chrome.permissions.contains({ origins: i }) && (r || !await t(i))) throw Error(e("mediaPermissionRequired"));
}
function T(t, n, r) {
	r?.throwIfAborted();
	let i = URL.createObjectURL(t);
	return new Promise((t, a) => {
		let o, s = !1, c = () => {
			s || (s = !0, chrome.downloads.onChanged.removeListener(u), r?.removeEventListener("abort", d), URL.revokeObjectURL(i));
		}, l = (n) => {
			n === "complete" ? (c(), t()) : n === "interrupted" && (c(), a(Error(e("browserDownloadInterrupted"))));
		}, u = (e) => {
			e.id === o && l(e.state?.current);
		}, d = () => {
			o != null && chrome.downloads.cancel(o).catch(() => {}), c(), a(r.reason);
		};
		chrome.downloads.onChanged.addListener(u), r?.addEventListener("abort", d, { once: !0 }), chrome.downloads.download({
			url: i,
			filename: n,
			conflictAction: "uniquify",
			saveAs: !1
		}).then(async (e) => {
			if (o = e, !Number.isInteger(o)) throw Error("Browser returned no download ID.");
			if (r?.aborted) {
				await chrome.downloads.cancel(o);
				return;
			}
			let [t] = await chrome.downloads.search({ id: o });
			l(t?.state);
		}).catch((e) => {
			c(), a(e);
		});
	});
}
async function E({ gallery: t, media: n, selected: r, asZip: i, progress: s, session: c, signal: l, permissionsAlreadyGranted: u = !1, save: d = T }) {
	let f = [...r].sort((e, t) => e - t);
	if (!f.length) return;
	let p = f.map((e) => S(n[e])), m = i ? C(p) : p;
	await w(f.map((e) => n[e]), u);
	let h = [], g = 0;
	for (let [t, r] of f.entries()) {
		l?.throwIfAborted(), await s(e("processingMedia", {
			current: t + 1,
			total: f.length,
			name: a(n[r], r)
		}));
		let o = await c.prepare(n[r], { signal: l });
		l?.throwIfAborted(), o.xmp || (g += 1);
		let u = m[t];
		i ? h.push({
			name: u,
			bytes: new Uint8Array(await o.blob.arrayBuffer())
		}) : (await s(e("savingMedia", {
			current: t + 1,
			total: f.length,
			name: u
		})), await d(o.blob, u, l));
	}
	i && (l?.throwIfAborted(), await s(e("creatingZip")), await d(x(h), `${o(t)}.zip`, l)), await s(g ? e("downloadCompletedOriginal", {
		count: f.length,
		unsupported: g
	}) : e("downloadCompletedXmp", { count: f.length }));
}
//#endregion
//#region src/image/gallery-image-processor.js
var D, O = 0, k = /* @__PURE__ */ new Map();
function A() {
	return D || (D = new Worker(chrome.runtime.getURL("build/xmp-worker.js"), { type: "module" }), D.onmessage = ({ data: e }) => {
		let t = k.get(e.id);
		t && (k.delete(e.id), e.error ? t.reject(Error(e.error)) : t.resolve(new Uint8Array(e.bytes)));
	}, D.onerror = (e) => {
		for (let t of k.values()) t.reject(Error(e.message || "XMP worker failed"));
		k.clear(), D?.terminate(), D = null;
	}, D);
}
function j(e) {
	return e[0] === 255 && e[1] === 216 ? ["image/jpeg", !0] : e.length >= 8 && [
		137,
		80,
		78,
		71,
		13,
		10,
		26,
		10
	].every((t, n) => e[n] === t) ? ["image/png", !0] : e.length >= 6 && new TextDecoder().decode(e.subarray(0, 6)).match(/^GIF8[79]a$/) ? ["image/gif", !0] : ["", !1];
}
async function M(t, { signal: n, writeXmp: r = N } = {}) {
	let i = await fetch(t.url, {
		credentials: "include",
		signal: n
	});
	if (!i.ok) throw Error(`HTTP ${i.status}`);
	let a = new Uint8Array(await i.arrayBuffer());
	n?.throwIfAborted();
	let [o, s] = j(a);
	if (!s) return {
		bytes: a,
		mime: i.headers.get("content-type") || "application/octet-stream",
		xmp: !1
	};
	if (!t.archiveKey || !t.canonicalMetadata) throw Error(e("missingXmpMetadata"));
	let c = await r(a, {
		archive_key: t.archiveKey,
		fields: t.canonicalMetadata
	});
	return n?.throwIfAborted(), {
		bytes: c,
		mime: o,
		xmp: !0
	};
}
function N(e, t) {
	return new Promise((n, r) => {
		let i = ++O, a = A();
		k.set(i, {
			resolve: n,
			reject: r
		});
		try {
			a.postMessage({
				id: i,
				bytes: e.buffer,
				metadata: t
			}, [e.buffer]);
		} catch (e) {
			k.delete(i), r(e);
		}
	});
}
//#endregion
//#region src/shared/gallery-timeline-session.js
function P(e) {
	return e.postId || e.media?.map((e) => e.archiveKey || e.relativePath).join("\n") || e.sourcePageUrl;
}
function F(e, t, { allowSameCursor: n = !1 } = {}) {
	if (!Array.isArray(t?.posts) || typeof t.hasMore != "boolean" || !t.timelineRange) throw Error("Incomplete timeline page response.");
	let r = e.timeline?.postStart > 1 && e.posts || [], i = new Set(r.map(P)), a = t.posts.filter((e) => {
		let t = P(e);
		return !t || i.has(t) ? !1 : (i.add(t), !0);
	}), o = e.timeline?.postStart || 1;
	if (!Number.isInteger(t.nextStart) || t.nextStart < o || t.hasMore && t.nextStart === o && !n) throw Error("Invalid timeline continuation cursor.");
	return {
		...e,
		media: void 0,
		posts: [...r, ...a],
		timeline: {
			...e.timeline,
			range: t.timelineRange,
			postStart: t.nextStart,
			hasMore: !!t.hasMore
		}
	};
}
function I(e) {
	let t = e.timeline;
	return {
		url: t.url,
		sourceTabId: t.sourceTabId,
		siteName: e.siteName,
		timelineRange: t.range,
		postStart: t.postStart || 1,
		...t.pageSize == null ? {} : { postCount: t.pageSize }
	};
}
//#endregion
export { S as a, f as c, r as d, i as f, E as i, s as l, a as m, I as n, p as o, c as p, M as r, d as s, F as t, n as u };
