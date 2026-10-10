import { t as e } from "./i18n-SwgWtGUP.js";
//#region src/shared/page-cache-key.js
function t(e) {
	try {
		let t = new URL(e);
		t.hash = "";
		let n = t.hostname.toLowerCase().replace(/^www\./, "");
		return n === "instagram.com" && /^\/(?:p|reel|reels|tv)\/[^/?#]+\/?$/i.test(t.pathname) ? (t.hostname = "www.instagram.com", t.pathname = `${t.pathname.replace(/\/+$/, "")}/`, t.search = "") : (n === "x.com" || n === "twitter.com") && /^\/[^/?#]+\/status\/\d+\/?$/i.test(t.pathname) && (t.search = ""), t.href;
	} catch {
		return String(e || "");
	}
}
//#endregion
//#region src/shared/gallery-post-groups.js
function n(e = [], n = "") {
	let i = /* @__PURE__ */ new Map();
	for (let [a, o] of e.entries()) {
		if (!o?.url) continue;
		let e = String(n || ""), s = o.sourceScope === "timeline", c = /^https?:\/\/(?:www\.)?instagram\.com\/(?:p|reels?|tv)\/[^/?#]+/i.test(e), l = c || /^https?:\/\/(?:www\.)?(?:x\.com|twitter\.com)\/[^/?#]+\/(?:timeline|status\/\d+)(?:[/?#]|$)/i.test(e), u = String(o.postId || "").trim(), d = /^https?:\/\/(?:www\.)?(?:x\.com|twitter\.com)\/[^/?#]+(?:[/?#]|$)/i.test(e), f = l || d && /^\d+$/.test(u) ? e : String(o.canonicalMetadata?.source_url || o.sourcePageUrl || n || ""), p = c ? t(f) : f, m = s ? `timeline:${t(o.analysisSourceUrl || n)}` : Number.isInteger(o.analysisPostIndex) ? `probe-post:${o.analysisPostIndex}` : c ? `instagram-post:${p}` : u ? `post:${u}` : p || o.url, h = i.get(m);
		h || (h = {
			key: m,
			sourceScope: o.sourceScope || "unknown",
			postId: u,
			title: o.title || o.filename || f,
			creator: o.canonicalMetadata?.creator_handle || o.creator || "",
			date: o.date || "",
			canonicalTitle: s ? "" : o.canonicalMetadata?.title || "",
			description: o.description || "",
			thumbnail: o.thumbnail || o.url,
			sourcePageUrl: s ? o.analysisSourceUrl || n : f,
			downloadUrl: s ? o.analysisSourceUrl || n : r(f, u),
			sourceItemIndices: [],
			media: []
		}, i.set(m, h)), h.sourceItemIndices.push(a);
		let g = {
			url: o.url,
			thumbnail: o.thumbnail || o.url,
			fallbackUrls: Array.isArray(o.fallbackUrls) ? o.fallbackUrls : [],
			filename: o.filename || "",
			extension: o.extension || "",
			relativePath: o.relativePath ?? null,
			mediaType: o.mediaType || "",
			width: o.width,
			height: o.height,
			archiveKey: o.archiveKey || "",
			canonicalMetadata: o.canonicalMetadata || null
		};
		if (s) {
			h.probeComplete = (h.probeComplete ?? !0) && o.analysisProbeComplete !== !1, h.probePostCount = Math.max(h.probePostCount || 0, o.analysisProbePostCount || 0), h.previewPosts ||= [];
			let e = Number.isInteger(o.analysisPostIndex) ? o.analysisPostIndex : 0, t = h.previewPosts.find((t) => t.probeIndex === e);
			if (t || (t = {
				probeIndex: e,
				postId: u,
				title: o.canonicalMetadata?.title || o.title || "",
				creator: o.canonicalMetadata?.creator_handle || o.creator || "",
				date: o.date || "",
				description: o.description || "",
				sourcePageUrl: o.canonicalMetadata?.source_url || o.sourcePageUrl || "",
				media: []
			}, h.previewPosts.push(t)), t.media.push(g), e !== h.previewPosts[0].probeIndex) continue;
		}
		h.media.push(g);
	}
	return [...i.values()];
}
function r(e, t) {
	try {
		let n = new URL(e);
		if (!["http:", "https:"].includes(n.protocol)) return "";
		let r = n.hostname.toLowerCase().replace(/^www\./, "");
		if (r === "instagram.com" && !/^\/(?:p|reels?|tv)\/[^/?#]+(?:\/|$)/i.test(n.pathname)) return "";
		if ((r === "x.com" || r === "twitter.com") && t && /^\d+$/.test(t)) {
			let e = n.pathname.split("/").filter(Boolean)[0];
			return e && e !== "i" && !["status", "statuses"].includes(e.toLowerCase()) ? `${n.origin}/${e}/status/${t}` : "";
		}
		return n.href;
	} catch {
		return "";
	}
}
function i(t, n = "", r = "") {
	if (!Array.isArray(t) || !t.length) throw Error(e("albumEmpty"));
	let i = t.map((e) => ({
		postId: e.postId || "",
		title: e.title || "",
		creator: e.creator || "",
		date: e.date || "",
		description: e.description || "",
		sourcePageUrl: e.sourcePageUrl || "",
		siteName: n,
		media: e.media || []
	}));
	if (i.length === 1) {
		let e = t[0];
		if (e.sourceScope !== "timeline") return i[0];
		let r = e.previewPosts?.map(({ probeIndex: e, ...t }) => ({
			...t,
			siteName: n
		}));
		return {
			...i[0],
			...r?.length ? { posts: r } : {},
			timeline: {
				url: e.sourcePageUrl,
				postStart: 1,
				hasMore: !0,
				...e.probePostCount ? {
					seedNextStart: e.probePostCount + 1,
					seedHasMore: e.probeComplete === !1 || e.probePostCount >= 2
				} : {}
			}
		};
	}
	return {
		siteName: n,
		sourcePageUrl: r,
		posts: i
	};
}
function a({ sourceTabId: e, playlistScope: n, url: r }) {
	if (!Number.isInteger(e) || !r) return null;
	let i = `analysis:tab:${e}:${n === "playlist" ? "playlist" : "current"}:${t(r)}`;
	return {
		unified: i,
		y: `${i}:y`,
		g: `${i}:g`
	};
}
function o(e, t) {
	let n = (e) => e?.ok && e.data?.request?.schemaVersion === 22;
	if (!n(e)) return t || e || null;
	if (!n(t)) return e;
	let r = e.data.request, i = t.data.request;
	return r.startedAt === i.startedAt ? r.updatedAt === i.updatedAt ? r.status !== "analyzing" && i.status === "analyzing" ? e : t : Number(r.updatedAt || 0) > Number(i.updatedAt || 0) ? e : t : Number(r.startedAt || 0) > Number(i.startedAt || 0) ? e : t;
}
//#endregion
//#region src/shared/gallery-timeline-range.js
function s(t = {
	kind: "posts",
	count: 10
}) {
	if (t.kind === "all") return { kind: "all" };
	if (t.kind === "posts" && Number.isInteger(Number(t.count)) && Number(t.count) > 0 && Number(t.count) <= 4294967295) return {
		kind: "posts",
		count: Number(t.count)
	};
	if (t.kind === "recent" && Number.isInteger(Number(t.days)) && Number(t.days) > 0 && Number(t.days) <= 36500) return {
		kind: "recent",
		days: Number(t.days),
		...t.since != null && t.until != null ? {
			since: t.since,
			until: t.until
		} : {}
	};
	if (t.kind === "since" && /^\d{4}-\d{2}-\d{2}$/.test(t.date) && !t.date.startsWith("0000") && !Number.isNaN(Date.parse(`${t.date}T00:00:00Z`)) && (/* @__PURE__ */ new Date(`${t.date}T00:00:00Z`)).toISOString().slice(0, 10) === t.date) return {
		kind: "since",
		date: t.date
	};
	throw Error(e("rangeInvalid"));
}
function c(t) {
	let n = s(t);
	return n.kind === "posts" ? e("rangeLatest", { count: n.count }) : n.kind === "recent" ? n.days === 1 ? e("rangeRecent24h") : e("rangeRecentDays", { days: n.days }) : n.kind === "since" ? e("rangeSince", { date: n.date }) : e("rangeAll");
}
function l(e, t) {
	try {
		return s(t.galleryRanges?.[e.key]);
	} catch {
		return s();
	}
}
//#endregion
//#region src/shared/site-domain.js
var u;
function d(e) {
	return String(e || "").trim().toLowerCase().replace(/^\.+|\.+$/g, "");
}
function f(e) {
	return /^\d{1,3}(?:\.\d{1,3}){3}$/.test(e) || e.includes(":");
}
async function p() {
	return u ||= fetch(chrome.runtime.getURL("resources/public-suffix-list.dat")).then((e) => {
		if (!e.ok) throw Error(`Could not load Public Suffix List (${e.status})`);
		return e.text();
	}).then((e) => {
		let t = /* @__PURE__ */ new Set(), n = /* @__PURE__ */ new Set(), r = /* @__PURE__ */ new Set();
		for (let i of e.split(/\r?\n/)) {
			let e = i.trim().toLowerCase();
			e && !e.startsWith("//") && (e.startsWith("!") ? r.add(e.slice(1)) : e.startsWith("*.") ? n.add(e.slice(2)) : t.add(e));
		}
		return {
			exact: t,
			wildcard: n,
			exception: r
		};
	}), u;
}
async function m(e) {
	if (e = d(e), !e || f(e) || !e.includes(".")) return null;
	let t = e.split("."), { exact: n, wildcard: r, exception: i } = await p(), a = 0, o = 1;
	for (let e = 0; e < t.length; e += 1) {
		let s = t.slice(e).join("."), c = t.length - e;
		i.has(s) && c > a && (a = c), n.has(s) && c > o && (o = c), e > 0 && r.has(s) && c + 1 > o && (o = c + 1);
	}
	return a && (o = a - 1), t.length <= o ? null : t.slice(-(o + 1)).join(".");
}
async function h(e) {
	let t = d(new URL(e).hostname);
	return await m(t) || t;
}
async function g(e) {
	let t = new URL(e), n = await m(d(t.hostname));
	if (!n) return [`${t.origin}/*`];
	let r = t.protocol;
	return [.../* @__PURE__ */ new Set([`${r}//${n}/*`, `${r}//*.${n}/*`])];
}
//#endregion
//#region src/shared/native-protocol.js
var _ = Object.freeze({
	wireVersion: 1,
	capabilities: Object.freeze(["fields.ignore-unknown.v1", "events.ignore-unknown.v1"])
});
function v(e) {
	return {
		...e,
		client: {
			wireVersion: _.wireVersion,
			capabilities: [..._.capabilities]
		}
	};
}
function y(e) {
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
function b(e, t) {
	return Array.isArray(e?.capabilities) && e.capabilities.includes(t);
}
//#endregion
export { g as a, c, i as d, n as f, h as i, a as l, b as n, l as o, t as p, v as r, s, y as t, o as u };
