import { $ as e, A as t, B as n, C as r, Ct as i, D as a, E as o, F as s, G as c, H as l, J as u, K as d, L as f, M as p, N as m, O as h, P as g, Q as _, S as v, St as y, T as b, U as x, V as S, W as C, X as w, Y as T, Z as E, _ as D, _t as O, a as k, at as A, b as j, bt as ee, c as M, ct as N, d as P, dt as F, et as te, f as ne, ft as I, g as re, gt as ie, h as L, ht as ae, it as R, j as z, k as oe, l as se, lt as B, m as ce, mt as V, n as le, nt as H, o as ue, ot as de, p as U, pt as W, q as fe, r as pe, s as me, st as G, t as he, tt as K, u as ge, ut as q, v as _e, vt as ve, wt as J, x as ye, xt as Y, y as X, yt as Z } from "./chunks/framework7-interaction-policy-C1jqd_Km.js";
import { a as be, c as xe, d as Se, f as Ce, n as we, o as Te, p as Ee, s as De, u as Oe } from "./chunks/native-protocol-BGhhGYZS.js";
import { t as Q } from "./chunks/i18n-SwgWtGUP.js";
import { t as ke } from "./chunks/url-validation-BNS-HJWZ.js";
import { t as Ae } from "./chunks/site-permission-request-C1rASNQI.js";
//#region src/popup/popup-host.js
var je = 480, Me = 25, Ne = 600;
function Pe(e) {
	let t = Number.parseFloat(e);
	return Number.isFinite(t) ? t : 0;
}
function Fe(e) {
	return Math.max(Me, Math.min(Ne, Math.ceil(e)));
}
function Ie(e) {
	return e.querySelector(".page-current") || e.querySelector(".page");
}
function Le(e) {
	return e?.querySelector(":scope > .page-content") || null;
}
function Re(e) {
	return e?.dataset.name || "";
}
function ze(e) {
	return [
		"format-grid",
		"smart-select-page",
		"time-range",
		"chapter-range",
		"gallery-format"
	].includes(Re(e));
}
function Be(e) {
	let t = e.getBoundingClientRect(), n = getComputedStyle(e), r = Pe(n.paddingTop), i = Pe(n.paddingBottom), a = r;
	for (let n of e.children) {
		if (!n.getClientRects().length) continue;
		let r = n.getBoundingClientRect(), i = getComputedStyle(n), o = r.bottom - t.top + e.scrollTop;
		a = Math.max(a, o + Pe(i.marginBottom));
	}
	return Fe(a + i);
}
function Ve(e, t) {
	let n = `${Fe(e)}px`;
	document.documentElement.style.height = n, document.body.style.height = n, t.style.height = n;
}
function He(e) {
	if (!e) return () => {};
	document.documentElement.style.width = `${je}px`, document.body.style.width = `${je}px`, e.style.width = `${je}px`, Ve(Me, e);
	let t = 0, n = !1, r = null, i = () => {
		if (t = 0, n) return;
		let i = Ie(e), a = Le(i);
		if (!a) return;
		let o = Re(i);
		if (o === "detail" || o === "gallery-detail") {
			r = Be(a), Ve(r, e);
			return;
		}
		if (r != null && ze(i)) {
			Ve(r, e);
			return;
		}
		r = null, Ve(Be(a), e);
	}, a = () => {
		n || t || (t = requestAnimationFrame(i));
	}, o = new MutationObserver(a);
	return o.observe(e, {
		childList: !0,
		subtree: !0,
		characterData: !0,
		attributes: !0,
		attributeFilter: ["class"]
	}), e.addEventListener("load", a, !0), e.addEventListener("page:afterin", a), e.addEventListener("transitionend", a), e.addEventListener("animationend", a), a(), () => {
		n = !0, t && cancelAnimationFrame(t), o.disconnect(), e.removeEventListener("load", a, !0), e.removeEventListener("page:afterin", a), e.removeEventListener("transitionend", a), e.removeEventListener("animationend", a);
	};
}
//#endregion
//#region src/popup/popup-navigation.js
var Ue = "popupRoute", We = /^\/gallery\/(0|[1-9]\d*)\/(?:format\/)?$/, Ge = /^\/item\/(0|[1-9]\d*)\/(?:format\/(video|audio)\/|time-range\/|chapter-range\/)?$/;
function Ke(e) {
	return e === "/" || e === "/settings/" || typeof e == "string" && (Ge.test(e) || We.test(e)) ? e : "/";
}
function qe(e, t) {
	return `${Ue}:tab:${e}:${Ee(t)}`;
}
async function Je(e, t, n = async () => {}) {
	let [r] = await e.query({
		active: !0,
		currentWindow: !0
	}), i = Number.isInteger(r?.id) ? r.id : null, a = Ee(r?.url || "");
	if (i === null || !a) return {
		key: null,
		route: "/"
	};
	await n(i, a);
	let o = qe(i, a);
	if (r.status === "loading") return {
		key: o,
		route: "/"
	};
	let s = (await t.get(o))[o];
	return {
		key: o,
		route: Ke(s?.route)
	};
}
function Ye(e, t, n) {
	return !t?.key || Ke(n) !== n ? Promise.resolve() : e.set({ [t.key]: { route: Ke(n) } });
}
//#endregion
//#region ../assets/site-icons/catalog.json
var Xe = [
	{
		id: "youtube",
		rustVariant: "YouTube",
		file: "youtube.ico",
		hosts: [
			"youtube.com",
			"youtu.be",
			"youtube-nocookie.com"
		],
		extractors: ["youtube"]
	},
	{
		id: "x",
		rustVariant: "X",
		file: "x.png",
		hosts: ["x.com", "twitter.com"],
		extractors: ["twitter", "x:"]
	},
	{
		id: "instagram",
		rustVariant: "Instagram",
		file: "instagram.png",
		hosts: ["instagram.com"],
		extractors: ["instagram"]
	},
	{
		id: "pixiv",
		rustVariant: "Pixiv",
		file: "pixiv.ico",
		hosts: ["pixiv.net"],
		extractors: ["pixiv"]
	},
	{
		id: "reddit",
		rustVariant: "Reddit",
		file: "reddit.svg",
		hosts: ["reddit.com", "redd.it"],
		extractors: ["reddit"]
	},
	{
		id: "twitch",
		rustVariant: "Twitch",
		file: "twitch.svg",
		hosts: ["twitch.tv"],
		extractors: ["twitch"]
	},
	{
		id: "bilibili",
		rustVariant: "Bilibili",
		file: "bilibili.svg",
		hosts: ["bilibili.com", "b23.tv"],
		extractors: ["bilibili"]
	},
	{
		id: "vimeo",
		rustVariant: "Vimeo",
		file: "vimeo.svg",
		hosts: ["vimeo.com"],
		extractors: ["vimeo"]
	},
	{
		id: "tiktok",
		rustVariant: "TikTok",
		file: "tiktok.svg",
		hosts: ["tiktok.com"],
		extractors: ["tiktok"]
	},
	{
		id: "facebook",
		rustVariant: "Facebook",
		file: "facebook.svg",
		hosts: ["facebook.com", "fb.watch"],
		extractors: ["facebook"]
	},
	{
		id: "pinterest",
		rustVariant: "Pinterest",
		file: "pinterest.svg",
		hosts: ["pinterest.com", "pin.it"],
		extractors: ["pinterest"]
	},
	{
		id: "soundcloud",
		rustVariant: "SoundCloud",
		file: "soundcloud.svg",
		hosts: ["soundcloud.com"],
		extractors: ["soundcloud"]
	},
	{
		id: "dailymotion",
		rustVariant: "Dailymotion",
		file: "dailymotion.svg",
		hosts: ["dailymotion.com", "dai.ly"],
		extractors: ["dailymotion"]
	},
	{
		id: "tumblr",
		rustVariant: "Tumblr",
		file: "tumblr.svg",
		hosts: ["tumblr.com"],
		extractors: ["tumblr"]
	},
	{
		id: "bluesky",
		rustVariant: "Bluesky",
		file: "bluesky.svg",
		hosts: ["bsky.app"],
		extractors: ["bluesky"]
	},
	{
		id: "mastodon",
		rustVariant: "Mastodon",
		file: "mastodon.svg",
		hosts: ["mastodon.social", "mastodon.online"],
		extractors: ["mastodon"]
	},
	{
		id: "spotify",
		rustVariant: "Spotify",
		file: "spotify.svg",
		hosts: ["spotify.com"],
		extractors: ["spotify"]
	}
];
//#endregion
//#region src/shared/builtin-site-icons.js
function Ze(e) {
	let t;
	try {
		let n = new URL(e);
		if (!["http:", "https:"].includes(n.protocol)) return "";
		t = n.hostname.toLowerCase();
	} catch {
		return "";
	}
	let n = Xe.find((e) => e.hosts.some((e) => t === e || t.endsWith(`.${e}`)));
	return n ? `resources/site-icons/${n.file}` : "";
}
//#endregion
//#region src/shared/gallery-download-mode.js
var Qe = "galleryDownloadMode", $e = Object.freeze(["browser-files", "v2-direct"]);
function et(e) {
	return e === "v2-zip" ? "v2-direct" : $e.includes(e) ? e : "browser-files";
}
function tt(e) {
	return et(e) === "v2-direct" ? "galleryModeV2Direct" : "galleryModeBrowserFiles";
}
function nt(e) {
	return et(e) === "browser-files";
}
function rt(e) {
	return [...new Set(e.flatMap((e) => e.media || []).map((e) => {
		let t = new URL(e.url);
		if (!["http:", "https:"].includes(t.protocol) || t.username || t.password) throw Error("Invalid media download URL.");
		return `${t.origin}/*`;
	}))];
}
function it(e, t) {
	return e.filter((e) => e.downloadUrl).map((e) => ({
		action: "download",
		backend: "g",
		url: e.downloadUrl,
		archiveMode: "files",
		title: e.title,
		sourcePageUrl: t.sourceUrl,
		...e.sourceScope === "timeline" ? { timelineRange: Te(e, t) } : {},
		sourceTabId: t.sourceTabId,
		sourceItemIndex: e.index
	}));
}
//#endregion
//#region src/shared/analysis-backend-routing.js
function at(e) {
	if (!e) return e;
	let t = e.y || null, n = e.g || null;
	return {
		...e,
		y: t,
		g: n,
		tools: {
			y: !!t,
			g: !!n
		},
		approximate: !!(e.approximate || n?.fallback)
	};
}
//#endregion
//#region src/shared/analysis-state.js
function ot(e, t) {
	let n = {
		y: null,
		g: null
	};
	for (let r of e) (r === "y" || r === "g") && (n[r] = {
		items: [],
		analyzing: !0,
		phase: "preparing",
		...r === "y" ? { itemLimit: t } : {}
	});
	return n;
}
function st(e, t, n) {
	if (!["y", "g"].includes(t) || !n || !Array.isArray(n.items)) return !1;
	let r = e.analysisByBackend?.[t];
	if (!r) return !0;
	let i = Number(r.startedAt || 0), a = Number(n.startedAt || 0);
	return i && a && i !== a ? a > i : !(!r.analyzing && n.analyzing && i === a);
}
function ct(e, t, n) {
	let r = e.analysisByBackend?.[t], i = r ? {
		...r,
		analyzing: !1
	} : null, a = {
		...e.analysisByBackend,
		[t]: i
	}, o = {
		...e.analysisErrors,
		[t]: String(n || "")
	}, s = t === "y" ? "g" : "y", c = a[s];
	return {
		analysisByBackend: a,
		analysisErrors: o,
		analysisRunning: Object.values(a).some((e) => !!e?.analyzing),
		switchTo: c?.items?.length || c?.analyzing ? s : null
	};
}
//#endregion
//#region src/shared/download-time-range.js
function lt(e) {
	let t = Number(e);
	return Number.isSafeInteger(t) && t >= 0 ? t : null;
}
function ut(e = {}) {
	let t = lt(e.rangeStartSeconds), n = lt(e.rangeEndSeconds);
	return t !== null && n !== null && n > t ? {
		startSeconds: t,
		endSeconds: n
	} : null;
}
function dt(e) {
	let t = String(e || "").trim();
	if (!t) return null;
	let n = t.split(":");
	if (n.length < 1 || n.length > 3 || n.some((e) => !/^\d+$/.test(e))) return null;
	let r = n.map(Number);
	return r.some((e) => !Number.isSafeInteger(e)) || n.length >= 2 && r.at(-1) >= 60 || n.length === 3 && r[1] >= 60 ? null : n.length === 1 ? r[0] : n.length === 2 ? r[0] * 60 + r[1] : r[0] * 3600 + r[1] * 60 + r[2];
}
function ft(e) {
	let t = lt(e) || 0;
	return [
		Math.floor(t / 3600),
		Math.floor(t % 3600 / 60),
		t % 60
	].map((e) => String(e).padStart(2, "0")).join(":");
}
//#endregion
//#region src/shared/download-chapter-range.js
function pt(e, t = {}) {
	let n = Array.isArray(e?.chapters) ? e.chapters.length : 0;
	return !Array.isArray(t.chapterIndices) || n === 0 ? [] : [...new Set(t.chapterIndices)].filter((e) => Number.isSafeInteger(e) && e >= 0 && e < n).sort((e, t) => e - t);
}
function mt(e, t = {}) {
	let n = Array.isArray(e?.chapters) ? e.chapters : [], r = pt(e, t), i = [];
	for (let e of r) {
		let t = i.at(-1);
		t && e === t.endIndex + 1 ? t.endIndex = e : i.push({
			startIndex: e,
			endIndex: e
		});
	}
	return i.map((e) => ({
		startMillis: Number(n[e.startIndex]?.startMillis),
		endMillis: Number(n[e.endIndex]?.endMillis)
	})).filter((e) => Number.isSafeInteger(e.startMillis) && Number.isSafeInteger(e.endMillis) && e.startMillis >= 0 && e.endMillis > e.startMillis);
}
//#endregion
//#region src/shared/media-download-options.js
function ht(e) {
	let t = String(e.resolution || "").match(/(\d+)\s*x\s*(\d+)/i);
	return t ? Number(t[1]) * Number(t[2]) : 0;
}
function gt(e, t) {
	let n = Number(e[t]);
	return Number.isFinite(n) ? n : 0;
}
function _t(e, t) {
	return ht(t) - ht(e) || gt(t, "fps") - gt(e, "fps") || gt(t, "bitrate") - gt(e, "bitrate") || gt(t, "filesize") - gt(e, "filesize");
}
function vt(e, t) {
	return gt(t, "bitrate") - gt(e, "bitrate") || gt(t, "sampleRate") - gt(e, "sampleRate") || gt(t, "filesize") - gt(e, "filesize");
}
function yt(e, t) {
	let n = e.formats || [];
	if (t === "audio") {
		let e = n.filter((e) => e.kind === "audio" && Ct(e)), t = n.filter((e) => e.kind === "muxed" && Ct(e));
		return (e.length ? e : t).slice().sort(vt);
	}
	return n.filter((e) => e.kind === "video" || e.kind === "muxed").slice().sort(_t);
}
function bt(e, t) {
	let n = yt(e, t), r = t === "audio" ? e.defaultAudio : e.defaultVideo;
	return n.some((e) => e.id === r) ? r : n.at(0)?.id || "";
}
function xt(e) {
	return ["video", "audio"].filter((t) => yt(e, t).length > 0);
}
function St(e) {
	return (e.subtitles || []).filter((e) => e.language?.toLowerCase() !== "live_chat");
}
function Ct(e) {
	return wt(e) === "present";
}
function wt(e) {
	if ([
		"present",
		"absent",
		"unknown"
	].includes(e.audioPresence)) return e.audioPresence;
	let t = String(e.audioCodec || (e.kind === "audio" ? e.codec : "")).trim().toLowerCase();
	return t && t !== "none" || Number(e.sampleRate) > 0 ? "present" : e.kind === "video" ? "unknown" : "absent";
}
function Tt(e) {
	let t = (e.formats || []).map(wt);
	return t.includes("present") ? "present" : t.includes("unknown") ? "unknown" : "absent";
}
function Et(e, t, n) {
	return e === "video" ? n?.kind !== "muxed" && t.length > 0 : t.length !== 1 || t[0].kind !== "muxed";
}
function Dt(e, t) {
	let n = String(e || "").trim().toLowerCase();
	return t.some((e) => n.startsWith(e));
}
function Ot(e) {
	return [...new Set(e.filter(Boolean))];
}
function kt(e) {
	let t = e?.audioCodec || e?.codec;
	return Dt(t, ["mp4a", "aac"]) ? "m4a" : Dt(t, ["opus"]) ? "opus" : Dt(t, ["vorbis"]) ? "vorbis" : Dt(t, ["mp3"]) ? "mp3" : [
		"m4a",
		"opus",
		"aac",
		"mp3",
		"flac",
		"vorbis",
		"alac"
	].includes(e?.audioExt || e?.ext) ? e.audioExt || e.ext : "m4a";
}
function At(e, t) {
	let n = e.formats || [];
	if (t.contentKind === "audio") return Ot([
		kt(n.find((e) => e.id === t.audioFormatId)),
		"mp3",
		"m4a",
		"flac",
		"opus",
		"aac",
		"vorbis",
		"alac"
	]);
	let r = n.find((e) => e.id === t.videoFormatId), i = r?.kind === "muxed" ? r : n.find((e) => e.id === t.audioFormatId), a = [];
	return Dt(r?.codec, [
		"h264",
		"h.264",
		"avc",
		"hevc",
		"h265",
		"h.265",
		"av1",
		"av01"
	]) && Dt(i?.audioCodec || i?.codec, [
		"aac",
		"mp4a",
		"mp3"
	]) && a.push("mp4"), Dt(r?.codec, [
		"vp8",
		"vp9",
		"vp09",
		"av1",
		"av01"
	]) && Dt(i?.audioCodec || i?.codec, ["opus", "vorbis"]) && a.push("webm"), r?.kind === "muxed" && [
		"mp4",
		"webm",
		"mkv"
	].includes(r.ext) && a.unshift(r.ext), !i && [
		"mp4",
		"webm",
		"mkv"
	].includes(r?.ext) && a.unshift(r.ext), Ot([...a, "mkv"]);
}
function jt(e, t) {
	return t.outputFormat ? t.outputFormat : At(e, t).at(0) || (t.contentKind === "audio" ? "m4a" : "mkv");
}
function Mt(e, t) {
	let n = {
		contentKind: t,
		videoFormatId: t === "video" ? bt(e, "video") : "",
		audioFormatId: bt(e, "audio"),
		subtitleLanguage: "",
		subtitleAutomatic: !1,
		subtitleMode: "off",
		chapterMode: "off",
		useAria2: !1,
		splitChapters: !1,
		rangeStartSeconds: null,
		rangeEndSeconds: null,
		chapterIndices: [],
		outputFormat: ""
	};
	return n.outputFormat = jt(e, n), n;
}
function Nt(e, t = "unknown") {
	let n = new URL(e), r = n.hostname.toLowerCase().replace(/^www\./, "");
	return !(r === "youtu.be" || r === "youtube.com" || r.endsWith(".youtube.com")) || !n.searchParams.get("list") ? {
		value: t === "playlist" ? "playlist" : "current",
		selectable: !1
	} : r === "youtu.be" || n.searchParams.has("v") || /^\/(shorts|live)\//.test(n.pathname) ? {
		value: "current",
		selectable: !0
	} : {
		value: t === "playlist" ? "playlist" : "current",
		selectable: !1
	};
}
function Pt(e, t) {
	let n = t.contentKind === "audio", r = e.formats || [], i = n ? null : r.find((e) => e.id === t.videoFormatId), a = r.find((e) => e.id === t.audioFormatId);
	if (n && !a) throw Error(Q("selectAudioFormat"));
	if (!n && !i) throw Error(Q("selectVideoFormat"));
	let o = r.some((e) => e.kind === "audio" && Ct(e));
	if (!n && i.kind === "video" && o && (!a || a.kind !== "audio")) throw Error(Q("selectMergeAudioFormat"));
	let s = !n && i.kind === "muxed", c = ut(t), l = mt(e, t), u = [
		"off",
		"download",
		"embed"
	].includes(t.chapterMode) ? t.chapterMode : t.splitChapters ? "download" : "off";
	return {
		action: "download",
		url: e.url,
		title: e.title,
		fileName: typeof t.fileName == "string" && t.fileName.trim() ? t.fileName.trim() : e.title,
		contentKind: t.contentKind,
		videoFormatId: n ? "" : i.id,
		videoExt: n ? "" : i.ext || "",
		videoIsMuxed: s,
		audioFormatId: n ? a.id : s ? "" : a?.id || "",
		audioExt: n ? a.audioExt || a.ext || "" : s ? "" : a?.ext || "",
		subtitleLanguage: n ? "" : t.subtitleLanguage || "",
		subtitleAutomatic: !n && !!t.subtitleAutomatic,
		subtitleMode: !n && t.subtitleLanguage && ["download", "embed"].includes(t.subtitleMode) ? t.subtitleMode : "off",
		thumbnailMode: [
			"off",
			"download",
			"embed"
		].includes(t.thumbnailMode) ? t.thumbnailMode : "off",
		chapterMode: u,
		splitChapters: u !== "off",
		useAria2: !!t.useAria2,
		downloadTimeRange: c,
		downloadChapterRanges: l,
		outputFormat: t.outputFormat || jt(e, t),
		playlistScope: "current"
	};
}
//#endregion
//#region src/shared/page-snapshot.js
function Ft(e = document, t = location.href) {
	let n = (e) => String(e || "").replace(/\s+/g, " ").trim().slice(0, 500), r = (t) => n(e.querySelector(t)?.content), i = (e) => {
		try {
			let n = new URL(e, t), r = n.hostname.replace(/^www\./, ""), i = n.pathname.replace(/\/+$/, "") || "/";
			if (r === "youtube.com" || r.endsWith(".youtube.com")) return `youtube.com${i}?v=${n.searchParams.get("v") || ""}&list=${n.searchParams.get("v") ? "" : n.searchParams.get("list") || ""}`;
			let a = (r === "x.com" || r === "twitter.com") && i.match(/\/(?:status|statuses)\/(\d+)(?:\/|$)/);
			if (a) return `x.com/status/${a[1]}`;
			if (r === "instagram.com" && /^\/(?:p|reel|reels|tv)\/[^/]+$/.test(i)) return `${r}${i}`;
			for (let e of [...n.searchParams.keys()]) (/^utm_/i.test(e) || ["fbclid", "gclid"].includes(e)) && n.searchParams.delete(e);
			return n.searchParams.sort(), `${r}${i}${n.search}`;
		} catch {
			return "";
		}
	}, a = e.querySelector("link[rel=\"canonical\"]")?.href || "", o = [r("meta[property=\"og:url\"]"), a].filter(Boolean).every((e) => i(e) === i(t)), s = (e) => o ? r(e) : "", c = (e) => {
		let t = Array.isArray(e) ? e[0] : e;
		return n(typeof t == "object" && t ? t.name : t);
	}, l = (e) => {
		let n = Array.isArray(e) ? e[0] : e, r = typeof n == "object" && n ? n.url || n.contentUrl : n;
		if (!r) return "";
		try {
			let e = new URL(String(r), t);
			return ["http:", "https:"].includes(e.protocol) ? e.href.slice(0, 4096) : "";
		} catch {
			return "";
		}
	}, u = (e) => {
		let t = Number(e);
		return Number.isFinite(t) && t > 0 ? Math.round(t) : null;
	}, d = (e) => {
		let t = String(e ?? "").trim();
		if (!t) return null;
		let n = Number(t);
		if (Number.isFinite(n) && n >= 0) return Math.round(n);
		let r = t.match(/^P(?:(\d+(?:\.\d+)?)D)?(?:T(?:(\d+(?:\.\d+)?)H)?(?:(\d+(?:\.\d+)?)M)?(?:(\d+(?:\.\d+)?)S)?)?$/i);
		if (!r || !r.slice(1).some(Boolean)) return null;
		let i = Number(r[1] || 0) * 86400 + Number(r[2] || 0) * 3600 + Number(r[3] || 0) * 60 + Number(r[4] || 0);
		return Number.isFinite(i) ? Math.round(i) : null;
	}, f = null, p = [], m = [], h = "", g = [...e.querySelectorAll("script[type=\"application/ld+json\"]")].slice(0, 20);
	for (let e of g) if (!(e.textContent.length > 1e6)) try {
		let n = [JSON.parse(e.textContent)];
		for (let e = 0; n.length && e < 500; e += 1) {
			let e = n.shift();
			if (!e || typeof e != "object") continue;
			if (Array.isArray(e)) {
				n.push(...e);
				continue;
			}
			let r = (Array.isArray(e["@type"]) ? e["@type"] : [e["@type"]]).map((e) => String(e || "").toLowerCase()), a = l(e.logo);
			a && m.length < 16 && m.push(a), h || (r.some((e) => [
				"website",
				"organization",
				"corporation",
				"onlinebusiness"
			].includes(e)) && (h = c(e.name)), h ||= c(e.publisher || e.provider || e.isPartOf));
			let s = e.mainEntityOfPage?.["@id"] || e.mainEntityOfPage;
			if (!s) {
				let n = e.url || e["@id"];
				try {
					let e = new URL(typeof n == "string" ? n : "", t);
					n && e.origin === new URL(t).origin && !/\.(?:mp4|webm|m4a|mp3|jpe?g|png|webp)$/i.test(e.pathname) && (s = n);
				} catch {}
			}
			let u = s ? typeof s == "string" && i(s) === i(t) : o;
			!f && u && r.some((e) => e === "videoobject" || e === "audioobject") && (f = e);
			let d = r.some((e) => [
				"article",
				"newsarticle",
				"blogposting",
				"socialmediaposting",
				"photograph",
				"profilepage",
				"person",
				"imagegallery"
			].includes(e)), g = s || e.url || e["@id"], _ = g ? typeof g == "string" && i(g) === i(t) : o, v = r.includes("person") && !g;
			if (d && _ && !v && p.length < 12) {
				let t = e.image || e.primaryImageOfPage || e.mainEntity?.image;
				for (let e of (Array.isArray(t) ? t : [t]).slice(0, 8)) {
					let t = l(e);
					t && p.push(t);
				}
			}
			n.push(...Object.values(e));
		}
	} catch {}
	let _ = e.querySelectorAll("video").length, v = e.querySelectorAll("audio").length, y = (() => {
		try {
			let e = new URL(t);
			return /(^|\.)youtube\.com$/i.test(e.hostname) && e.pathname === "/watch" ? e : null;
		} catch {
			return null;
		}
	})(), b = y ? e.querySelector("ytd-watch-flexy[video-id]") : null, x = b && b.getAttribute("video-id") === y.searchParams.get("v"), S = !(!b || x);
	S && (f = null);
	let C = x ? n(b.querySelector("ytd-watch-metadata h1, #title h1")?.textContent) : "", w = b ? x ? b.querySelector("video") : null : e.querySelector("video"), T = w || e.querySelector("audio"), E = s("meta[property=\"og:type\"]").toLowerCase(), D = (Array.isArray(f?.["@type"]) ? f["@type"] : [f?.["@type"]]).map((e) => String(e || "").toLowerCase()), O = _ > 0 || E.includes("video") || D.includes("videoobject"), k = v > 0 || E.includes("audio") || D.includes("audioobject"), A = O && k ? "mixed" : O ? "video" : k ? "audio" : "", j = [...e.querySelectorAll("main video, [role=\"main\"] video, article video")].filter((e) => !e.closest?.("aside, nav, footer, header, [aria-hidden=\"true\"]")), ee = x ? w : o && j.length === 1 ? j[0] : o && _ === 1 && (f || E.includes("video")) ? w : null, M = /^https?:\/\/(?:www\.)?instagram\.com\//i.test(t) && /^https?:\/\/[^/]+\/(?:p|reels?|tv)\/[^/?#]+/i.test(t), N = e.querySelector("[role=\"dialog\"]") || e, P = M && N.querySelectorAll("article").length <= 1 ? [...N.querySelectorAll("article img, main img, [role=\"main\"] img")].filter((e) => e.complete && e.naturalWidth >= 160 && e.naturalHeight >= 160).map((e) => l(e.currentSrc || e.src)).filter(Boolean).slice(0, 4) : [], F = r("meta[property=\"og:url\"]"), te = F ? i(F) === i(t) : o, ne = r("meta[name=\"twitter:url\"]"), I = ne ? i(ne) === i(t) : te, re = [
		...te ? [r("meta[property=\"og:image:secure_url\"]"), r("meta[property=\"og:image\"]")] : [],
		...I ? [r("meta[name=\"twitter:image\"]")] : [],
		...o ? [s("meta[itemprop=\"thumbnailUrl\"]"), e.querySelector("link[rel=\"image_src\"]")?.href] : []
	].map(l).filter(Boolean), ie = f?.thumbnailUrl || f?.image, L = [...new Set([
		...(Array.isArray(ie) ? ie : [ie]).slice(0, 8).map(l),
		...p,
		...re
	].filter(Boolean))], ae = [...new Set([
		...P,
		...L,
		l(ee?.poster)
	].filter(Boolean))];
	if (y) {
		let t = [
			w?.poster,
			r("meta[property=\"og:image:secure_url\"]"),
			r("meta[property=\"og:image\"]"),
			r("meta[name=\"twitter:image\"]"),
			r("meta[itemprop=\"thumbnailUrl\"]"),
			e.querySelector("link[itemprop=\"thumbnailUrl\"]")?.href,
			f?.thumbnailUrl,
			e.querySelector("link[rel=\"image_src\"]")?.href
		];
		ae = [...new Set(t.map(l).filter(Boolean))].filter((e) => {
			let t = new URL(e), n = t.pathname.match(/\/(?:vi|vi_webp)\/([^/]+)\//)?.[1];
			return n && /(^|\.)ytimg\.com$/i.test(t.hostname) ? n === y.searchParams.get("v") : o && !S;
		});
	}
	return {
		schemaVersion: 3,
		url: l(t),
		canonicalUrl: o ? l(a) : "",
		siteName: n(r("meta[property=\"og:site_name\"]") || r("meta[name=\"application-name\"]") || h),
		iconUrl: l(e.querySelector("link[rel~=\"icon\"]")?.href || e.querySelector("link[rel=\"apple-touch-icon\"]")?.href),
		title: S ? "" : n(C || (y ? e.title : "") || s("meta[property=\"og:title\"]") || s("meta[name=\"twitter:title\"]") || f?.name || f?.headline || e.title),
		thumbnailUrl: ae[0] || "",
		thumbnailCandidates: ae,
		thumbnailMetadataCandidates: L,
		brandingImageUrls: [...new Set(m)],
		creator: c(s("meta[name=\"author\"]") || f?.author || f?.creator || f?.publisher),
		uploadDate: n(f?.uploadDate || f?.datePublished),
		description: n(s("meta[property=\"og:description\"]") || s("meta[name=\"twitter:description\"]") || s("meta[name=\"description\"]") || f?.description),
		contentUrl: l(f?.contentUrl),
		embedUrl: l(f?.embedUrl),
		mediaDurationSeconds: d(ee?.duration ?? (_ === 0 ? T?.duration : null)) ?? d(f?.duration),
		mediaWidth: u(ee?.videoWidth) || u(s("meta[property=\"og:video:width\"]")) || u(f?.width),
		mediaHeight: u(ee?.videoHeight) || u(s("meta[property=\"og:video:height\"]")) || u(f?.height),
		mediaKind: A,
		mediaElementCount: _ + v
	};
}
//#endregion
//#region src/shared/youtube-page-thumbnail.js
function It() {
	let e = new URL(location.href);
	if (!/(^|\.)youtube\.com$/i.test(e.hostname)) return null;
	let t = e.pathname === "/watch" ? e.searchParams.get("v") : e.pathname.match(/^\/(?:shorts|live|embed)\/([^/]+)/)?.[1];
	if (!/^[\w-]{11}$/.test(t || "")) return null;
	let n = [], r = (e, r = "") => {
		if (!(typeof e?.url != "string" || e.url.length > 4096)) try {
			let i = new URL(e.url, location.href);
			if (i.protocol !== "https:" || !/(^|\.)ytimg\.com$/i.test(i.hostname)) return;
			let a = i.pathname.match(/\/(?:vi|vi_webp)\/([^/]+)\//)?.[1];
			if (a ? a !== t : r !== t) return;
			n.push({
				url: i.href,
				width: Number(e.width) || 0
			});
		} catch {}
	}, i = (e) => {
		let n = e?.videoDetails;
		if (n?.videoId === t && Array.isArray(n.thumbnail?.thumbnails)) for (let e of n.thumbnail.thumbnails.slice(0, 16)) r(e, t);
	};
	try {
		i(document.getElementById("movie_player")?.getPlayerResponse?.());
	} catch {}
	let a = [.../* @__PURE__ */ new Set([...document.querySelectorAll(`[is-active] .html5-video-player, [video-id="${t}"] .html5-video-player`), ...document.querySelectorAll(".html5-video-player")])].slice(0, 12);
	for (let e of a) try {
		i(e.getPlayerResponse?.());
	} catch {}
	if (e.pathname.startsWith("/shorts/")) {
		let e = [
			"data",
			"videoDetails",
			"playerResponse",
			"playerData",
			"reelWatchEndpoint",
			"watchEndpoint",
			"navigationEndpoint",
			"onTap",
			"innertubeCommand",
			"command",
			"endpoint",
			"thumbnail",
			"thumbnails",
			"thumbnailViewModel",
			"image",
			"sources",
			"reelItemRenderer",
			"shortsLockupViewModel"
		], n = [], i = [.../* @__PURE__ */ new Set([...document.querySelectorAll("ytd-reel-video-renderer[is-active], ytd-shorts-video-renderer[is-active], shorts-video[is-active]"), ...document.querySelectorAll("ytd-reel-video-renderer, ytd-shorts-video-renderer, ytm-reel-video-renderer, shorts-video")])].slice(0, 12);
		for (let e of i) try {
			let t = e.getAttribute("video-id") || "";
			n.push({
				value: e.data,
				ownerId: t
			}, {
				value: e.playerResponse,
				ownerId: t
			}, {
				value: e.playerData,
				ownerId: t
			});
		} catch {}
		let a = /* @__PURE__ */ new Set();
		for (let i = 0; n.length && i < 400; i += 1) {
			let { value: i, ownerId: o } = n.shift();
			if (!i || typeof i != "object" || a.has(i)) continue;
			a.add(i);
			let s = i.videoId || i.videoDetails?.videoId || o;
			if (!(s && s !== t)) {
				if (r(i, s), Array.isArray(i)) for (let e of i.slice(0, 16)) n.push({
					value: e,
					ownerId: s
				});
				else for (let t of e) try {
					i[t] && n.push({
						value: i[t],
						ownerId: s
					});
				} catch {}
			}
		}
	}
	for (let e of [...document.querySelectorAll("img[src*=\"ytimg.com\"], video[poster], .ytp-cued-thumbnail-overlay-image")].slice(0, 64)) {
		let t = e.style?.backgroundImage?.match(/url\(["']?([^"')]+)["']?\)/)?.[1];
		for (let n of [
			e.currentSrc,
			e.src,
			e.poster,
			t
		]) n && r({
			url: n,
			width: e.naturalWidth || 0
		});
	}
	for (let e of [...document.querySelectorAll("meta[property=\"og:image\"], meta[property=\"og:image:secure_url\"], meta[name=\"twitter:image\"], link[itemprop=\"thumbnailUrl\"]")].slice(0, 8)) r({ url: e.content || e.href });
	try {
		i(window.ytInitialPlayerResponse);
	} catch {}
	return n.sort((e, t) => t.width - e.width), {
		url: location.href,
		videoId: t,
		thumbnailCandidates: [...new Set(n.map((e) => e.url))].slice(0, 8)
	};
}
function Lt(e) {
	try {
		let t = new URL(e);
		if (!/(^|\.)youtube\.com$/i.test(t.hostname)) return "";
		let n = t.pathname === "/watch" ? t.searchParams.get("v") : t.pathname.match(/^\/(?:shorts|live|embed)\/([^/]+)/)?.[1];
		return /^[\w-]{11}$/.test(n || "") ? n : "";
	} catch {
		return "";
	}
}
function Rt(e) {
	let t = Lt(e?.url);
	if (!t || new URL(e.url).pathname.replace(/\/+$/, "") !== `/shorts/${t}`) return e;
	let n = [...e.thumbnailCandidates || [], ...e.thumbnailUrl ? [e.thumbnailUrl] : []].filter((e) => {
		if (typeof e != "string" || e.length > 4096) return !1;
		try {
			let n = new URL(e);
			return n.protocol === "https:" && /(^|\.)ytimg\.com$/i.test(n.hostname) && n.pathname.match(/\/(?:vi|vi_webp)\/([^/]+)\//)?.[1] === t;
		} catch {
			return !1;
		}
	}), r = [.../* @__PURE__ */ new Set([
		...n,
		`https://i.ytimg.com/vi/${t}/hqdefault.jpg`,
		`https://i.ytimg.com/vi/${t}/mqdefault.jpg`
	])];
	return {
		...e,
		thumbnailUrl: r[0],
		thumbnailCandidates: r
	};
}
function zt(e, t) {
	let n = Lt(e?.url);
	if (!n || n !== t?.videoId || n !== Lt(t?.url)) return e;
	let r = (Array.isArray(t.thumbnailCandidates) ? t.thumbnailCandidates : []).slice(0, 8).filter((e) => {
		if (typeof e != "string" || e.length > 4096) return !1;
		try {
			let t = new URL(e), r = t.pathname.match(/\/(?:vi|vi_webp)\/([^/]+)\//)?.[1];
			return t.protocol === "https:" && /(^|\.)ytimg\.com$/i.test(t.hostname) && (!r || r === n);
		} catch {
			return !1;
		}
	});
	if (!r.length) return e;
	let i = [.../* @__PURE__ */ new Set([...r, ...e.thumbnailCandidates || []])];
	return {
		...e,
		thumbnailUrl: i[0],
		thumbnailCandidates: i
	};
}
//#endregion
//#region src/shared/twitter-page-thumbnail.js
function Bt(e = document, t = location.href) {
	let n = (e) => {
		if (typeof e != "string" || !e) return "";
		try {
			let n = new URL(e, t);
			return /(^|\.)(?:x|twitter)\.com$/i.test(n.hostname) && n.pathname.match(/\/(?:status|statuses)\/(\d+)(?:\/|$)/)?.[1] || "";
		} catch {
			return "";
		}
	}, r = n(t);
	if (!r) return null;
	let i = (e) => {
		if (typeof e != "string" || !e || e.length > 4096) return "";
		try {
			let n = new URL(e, t);
			return n.protocol === "https:" && n.hostname === "pbs.twimg.com" && /^\/(?:media|ext_tw_video_thumb|amplify_video_thumb|tweet_video_thumb)\//.test(n.pathname) ? n.href : "";
		} catch {
			return "";
		}
	}, a = (e, t) => {
		if (e.closest("article") !== t || e.closest("[data-testid=\"quoteTweet\"], [data-testid=\"card.wrapper\"]")) return !1;
		for (let n = e.parentElement; n && n !== t; n = n.parentElement) if (n.tagName !== "A" && n.getAttribute("role") === "link" && n.querySelector("[data-testid=\"User-Name\"], [data-testid=\"Tweet-User-Avatar\"]")) return !1;
		return !0;
	}, o = [...e.querySelectorAll("article[data-testid=\"tweet\"], article[role=\"article\"]")].slice(0, 100).filter((e) => [...e.querySelectorAll("a[href]")].slice(0, 80).some((t) => a(t, e) && n(t.href) === r && (t.querySelector("time") || t.querySelector("[data-testid=\"tweetPhoto\"], [data-testid=\"videoPlayer\"], video[poster]")))), s = o.find((e) => [...e.querySelectorAll("time")].some((t) => a(t, e) && n(t.closest("a[href]")?.href) === r)) || o[0], c = [];
	if (s) {
		let e = [...s.querySelectorAll("video[poster], [data-testid=\"tweetPhoto\"] img, [data-testid=\"videoPlayer\"] img")].slice(0, 24).filter((e) => a(e, s)), o = new URL(t).pathname.match(/\/(photo|video)\/(\d+)\/?$/), l = o ? `/${o[1]}/${o[2]}` : "";
		e.sort((e, i) => {
			let a = (e) => {
				let i = e.closest("a[href]")?.href;
				return i && n(i) === r && new URL(i, t).pathname.endsWith(l);
			};
			return l ? Number(!!a(i)) - Number(!!a(e)) : 0;
		});
		for (let t of e) {
			let e = i(t.poster || t.currentSrc || t.src);
			e && c.push(e);
		}
	}
	let l = [e.querySelector("meta[property=\"og:url\"]")?.content, e.querySelector("link[rel=\"canonical\"]")?.href].filter(Boolean), u = l.length > 0 && l.every((e) => n(e) === r), d = s && [...s.querySelectorAll("video, [data-testid=\"tweetPhoto\"], [data-testid=\"videoPlayer\"]")].some((e) => a(e, s));
	if (!c.length && u && (!s || d)) for (let t of [
		"meta[property=\"og:image:secure_url\"]",
		"meta[property=\"og:image\"]",
		"meta[name=\"twitter:image\"]"
	]) {
		let n = i(e.querySelector(t)?.content);
		n && c.push(n);
	}
	return {
		url: t,
		postId: r,
		thumbnailCandidates: [...new Set(c)].slice(0, 8)
	};
}
function Vt(e) {
	try {
		let t = new URL(e);
		return /(^|\.)(?:x|twitter)\.com$/i.test(t.hostname) && t.pathname.match(/\/(?:status|statuses)\/(\d+)(?:\/|$)/)?.[1] || "";
	} catch {
		return "";
	}
}
function Ht(e, t) {
	let n = Vt(e?.url);
	if (!n) return e;
	let r = t?.postId === n && Vt(t?.url) === n && Array.isArray(t.thumbnailCandidates) ? [...new Set(t.thumbnailCandidates.slice(0, 8).filter((e) => {
		if (typeof e != "string" || e.length > 4096) return !1;
		try {
			let t = new URL(e);
			return t.protocol === "https:" && t.hostname === "pbs.twimg.com" && /^\/(?:media|ext_tw_video_thumb|amplify_video_thumb|tweet_video_thumb)\//.test(t.pathname);
		} catch {
			return !1;
		}
	}))] : [];
	return {
		...e,
		thumbnailUrl: r[0] || "",
		thumbnailCandidates: r
	};
}
function Ut(e) {
	try {
		let t = new URL(e);
		if (!/(^|\.)(?:x|twitter)\.com$/i.test(t.hostname)) return "";
		let n = t.pathname.match(/^\/([\w]{1,15})(?:\/(?:media|with_replies|highlights))?\/?$/)?.[1]?.toLowerCase();
		return n && ![
			"home",
			"explore",
			"notifications",
			"messages",
			"search",
			"settings",
			"compose",
			"i",
			"login",
			"signup",
			"tos",
			"privacy"
		].includes(n) ? n : "";
	} catch {
		return "";
	}
}
function Wt(e = document, t = location.href) {
	let n = (e) => {
		if (typeof e != "string" || !e) return "";
		try {
			let n = new URL(e, t);
			return /(^|\.)(?:x|twitter)\.com$/i.test(n.hostname) && n.pathname.match(/^\/([\w]{1,15})(?:\/(?:media|with_replies|highlights|photo|header_photo))?\/?$/)?.[1]?.toLowerCase() || "";
		} catch {
			return "";
		}
	}, r = n(t);
	if (!r) return null;
	let i = (e) => {
		if (typeof e != "string" || e.length > 4096) return "";
		try {
			let n = new URL(e, t);
			return n.protocol === "https:" && n.hostname === "pbs.twimg.com" && /^\/(?:profile_images|profile_banners)\//.test(n.pathname) ? n.href : "";
		} catch {
			return "";
		}
	}, a = e.querySelector("[data-testid=\"primaryColumn\"]") || e.querySelector("main") || e, o = [], s = [...a.querySelectorAll("a[href]")].slice(0, 200).filter((e) => !e.closest("article") && !e.closest("[data-testid=\"UserCell\"]") && n(e.href) === r);
	for (let e of ["/photo", "/header_photo"]) for (let n of s.filter((n) => new URL(n.href, t).pathname.replace(/\/+$/, "").endsWith(e))) for (let e of [...n.querySelectorAll("img")].slice(0, 4)) {
		let t = i(e.currentSrc || e.src);
		t && o.push(t);
	}
	for (let e of [...a.querySelectorAll(`[data-testid="UserAvatar-Container-${r}"] img`)].slice(0, 4)) {
		if (e.closest("article") || e.closest("[data-testid=\"UserCell\"]")) continue;
		let t = i(e.currentSrc || e.src);
		t && o.push(t);
	}
	let c = [e.querySelector("meta[property=\"og:url\"]")?.content, e.querySelector("link[rel=\"canonical\"]")?.href].filter(Boolean), l = [];
	if (c.length && c.every((e) => n(e) === r)) for (let t of ["meta[property=\"og:image\"]", "meta[name=\"twitter:image\"]"]) {
		let n = i(e.querySelector(t)?.content);
		n && l.push(n);
	}
	return {
		url: t,
		profileHandle: r,
		thumbnailCandidates: [.../* @__PURE__ */ new Set([...l, ...o])].slice(0, 8)
	};
}
function Gt(e, t) {
	let n = Ut(e?.url);
	if (!n) return e;
	let r = t?.profileHandle === n && Ut(t?.url) === n, i = [.../* @__PURE__ */ new Set([...e.thumbnailMetadataCandidates || [], ...r && Array.isArray(t.thumbnailCandidates) ? t.thumbnailCandidates : []])].slice(0, 20).filter((e) => {
		if (typeof e != "string" || e.length > 4096) return !1;
		try {
			let t = new URL(e);
			return t.protocol === "https:" && t.hostname === "pbs.twimg.com" && /^\/(?:profile_images|profile_banners)\//.test(t.pathname);
		} catch {
			return !1;
		}
	}).slice(0, 8);
	return {
		...e,
		thumbnailUrl: i[0] || "",
		thumbnailCandidates: i
	};
}
//#endregion
//#region src/shared/page-thumbnail-policy.js
function Kt(e) {
	if (!e) return e;
	let t = (e) => {
		try {
			let t = new URL(e);
			return `${t.origin}${t.pathname}`;
		} catch {
			return "";
		}
	}, n = new Set([e.iconUrl, ...e.brandingImageUrls || []].map(t).filter(Boolean)), r = [e.thumbnailUrl, ...e.thumbnailCandidates || []], i = [...new Set(r.filter((e) => {
		if (typeof e != "string" || !e || e.length > 4096) return !1;
		try {
			let r = new URL(e);
			return !(!["http:", "https:"].includes(r.protocol) || r.username || r.password || n.has(t(e)) || r.hostname === "abs.twimg.com" && /^\/(?:rweb\/|sticky\/default_profile_images\/)/.test(r.pathname));
		} catch {
			return !1;
		}
	}))].slice(0, 12);
	return {
		...e,
		thumbnailUrl: i[0] || "",
		thumbnailCandidates: i
	};
}
//#endregion
//#region src/shared/capture-page-preview.js
async function qt(e) {
	if (!Number.isInteger(e) || !chrome.scripting?.executeScript) return null;
	let t = async (t, n = "ISOLATED") => {
		let [r] = await chrome.scripting.executeScript({
			target: { tabId: e },
			func: t,
			world: n
		});
		return r?.result;
	};
	try {
		let e = await t(Ft);
		if (!(e?.schemaVersion >= 1 && e.schemaVersion <= 3)) return null;
		let n = e;
		if (Vt(e.url)) {
			let r = null;
			try {
				r = await t(Bt);
			} catch {}
			n = Ht(e, r);
		} else if (Ut(e.url)) {
			let r = null;
			try {
				r = await t(Wt);
			} catch {}
			n = Gt(e, r);
		} else if (Lt(e.url)) {
			let r = null;
			try {
				r = await t(It, "MAIN");
			} catch {}
			n = Rt(zt(e, r));
		}
		return Kt(n);
	} catch {
		return null;
	}
}
//#endregion
//#region src/popup/popup-page-source.js
function Jt({ captureSnapshot: e, onPage: t, onSnapshot: n, onError: r }) {
	let i = 0, a = !1, o = null, s, c, l = (e, t) => e?.id === t?.id && Ee(e?.url) === Ee(t?.url);
	async function u(t, r) {
		let o = await e(t.id);
		if (a || r !== i) return;
		let s = await chrome.tabs.get(t.id);
		if (!(a || r !== i)) {
			if (!l(t, s)) {
				await d(s);
				return;
			}
			o?.url && Ee(o.url) !== Ee(s.url) || n(s, o);
		}
	}
	async function d(e, n = !1) {
		if (a) return;
		let s = ++i;
		clearTimeout(c);
		let d = n || !l(o, e);
		o = e, t(e, d, n);
		try {
			if (e.status === "loading") return;
			await u(e, s), !a && s === i && (c = setTimeout(() => {
				u(e, s).catch((e) => {
					!a && s === i && r(e);
				});
			}, 600));
		} catch (e) {
			!a && s === i && r(e);
		}
	}
	async function f() {
		let e = ++i;
		clearTimeout(s), clearTimeout(c);
		try {
			let [t] = await chrome.tabs.query({
				active: !0,
				currentWindow: !0
			});
			!a && e === i && Number.isInteger(t?.id) && await d(t);
		} catch (t) {
			!a && e === i && r(t);
		}
	}
	function p(e, t, n) {
		a || e !== o?.id || (t.status === "loading" ? d(n, !0) : t.url ? d(n) : (t.title !== void 0 || t.status === "complete") && (clearTimeout(s), s = setTimeout(() => {
			f();
		}, 100)));
	}
	function m(e) {
		(!o || e.windowId === o.windowId) && f();
	}
	return {
		async start() {
			chrome.tabs.onUpdated.addListener(p), chrome.tabs.onActivated.addListener(m), await f();
		},
		refresh: f,
		destroy() {
			a = !0, i += 1, clearTimeout(s), clearTimeout(c), chrome.tabs.onUpdated.removeListener(p), chrome.tabs.onActivated.removeListener(m);
		}
	};
}
//#endregion
//#region src/popup/popup-result-list.js
function Yt(e) {
	let t = (e.analysisByBackend.y?.items || []).map((e, t) => ({
		backend: "y",
		index: t,
		key: `y:${t}`,
		item: e
	})), n = Ce(e.analysisByBackend.g?.items || [], e.sourceUrl).map((e, t) => ({
		backend: "g",
		index: t,
		key: `g:${e.key}`,
		item: e
	}));
	return [...t, ...n];
}
function Xt(e, t, n = Yt(e).length) {
	if (n === 1) return !0;
	let r = e.selectionsByBackend[t.backend];
	return t.backend === "y" ? r.has(t.index) : t.item.sourceItemIndices.every((e) => r.has(e));
}
function Zt(e) {
	let t = Yt(e);
	return t.filter((n) => Xt(e, n, t.length));
}
function Qt(e, t) {
	return et(e.galleryModes[t.key] ?? e.galleryDownloadMode);
}
function $t(e) {
	let t = Zt(e);
	return {
		selected: t,
		browserOnly: t.length > 0 && t.every((t) => t.backend === "g" && nt(Qt(e, t.item))),
		disabled: !t.length || e.busy || e.analysisRunning || e.downloadRunning || t.some((t) => t.backend === "g" && !nt(Qt(e, t.item)) && !t.item.downloadUrl)
	};
}
//#endregion
//#region src/shared/analysis-tool-preferences.js
var en = "analysisToolPreferences", tn = "analysisToolDetection";
function nn(e) {
	return {
		y: e?.y !== !1,
		g: e?.g !== !1
	};
}
function rn(e) {
	let t = nn(e);
	return ["y", "g"].filter((e) => t[e]);
}
function an(e, t = Date.now()) {
	return {
		y: typeof e?.ytDlpAvailable == "boolean" ? e.ytDlpAvailable : null,
		g: typeof e?.galleryDlAvailable == "boolean" ? e.galleryDlAvailable : null,
		checkedAt: t
	};
}
function on(e) {
	return {
		y: e.y !== !1,
		g: e.g !== !1
	};
}
//#endregion
//#region src/popup/popup-controller.js
var sn = "YT_DLP_GUI_ANALYSIS_EVENT";
function cn() {
	return {
		analysisView: "idle",
		recognized: null,
		recognitionPending: !0,
		analysis: null,
		unifiedAnalysis: null,
		analysisStorageScope: null,
		analysisByBackend: {
			y: null,
			g: null
		},
		analysisErrors: {
			y: "",
			g: ""
		},
		selectionsByBackend: {
			y: /* @__PURE__ */ new Set(),
			g: /* @__PURE__ */ new Set()
		},
		knownCountsByBackend: {
			y: 0,
			g: 0
		},
		busy: !1,
		analysisRunning: !1,
		downloadRunning: !1,
		currentTask: { state: "idle" },
		browserDownloadTask: { state: "idle" },
		sourceUrl: "",
		sourceSiteName: Q("recognizing"),
		sourceSiteIconUrl: "",
		sourcePageTitle: "",
		sourcePageSnapshot: null,
		pageResetRevision: 0,
		sourceTabId: null,
		choices: {},
		playlistScope: "current",
		playlistSelectable: !1,
		playlistItemLimit: 32,
		outputDirectory: "",
		galleryDownloadMode: "browser-files",
		galleryModes: {},
		galleryRanges: {},
		recognitionText: Q("recognizing"),
		recognitionVisible: !0,
		messageText: "",
		messageError: !1,
		helpUrl: "",
		analysisPhase: "preparing",
		cookieSaveRunning: !1,
		cookieSaveMessage: "",
		cookieSaveError: !1,
		preferencesLoaded: !1,
		albumWheelMode: "zoom",
		albumPreferenceSaving: !1,
		albumSettingsMessage: "",
		analysisTools: nn(),
		toolDetection: null,
		toolDetectionRunning: !1,
		toolPreferenceSaving: !1,
		toolSettingsMessage: "",
		toolSettingsError: !1
	};
}
function ln(e) {
	return Math.max(1, Math.min(100, Math.round(Number(e) || 32)));
}
function un(e, t = "off") {
	return [
		"off",
		"download",
		"embed"
	].includes(e) ? e : t;
}
function dn(e, t = "off") {
	return [
		"off",
		"download",
		"embed"
	].includes(e.chapterMode) ? e.chapterMode : e.chapters === "split" ? "download" : e.chapters === "full" ? "off" : un(t);
}
function fn(e) {
	return chrome.runtime.sendMessage({
		type: "YT_DLP_GUI_NATIVE_REQUEST",
		request: e
	}).then((e) => {
		if (!e?.ok) {
			let t = Error(e?.message || Q("hostRejected"));
			throw t.helpUrl = e?.helpUrl || "", t;
		}
		return e;
	});
}
function pn() {
	let e = ee(cn()), t = new Worker(chrome.runtime.getURL("build/matcher-worker.js"), { type: "module" }), n = chrome.runtime.getURL("resources/extractor-index.json"), r = 0, i = 0, a = "", o = !1, s = (t) => e.update((e) => ({
		...e,
		...t
	})), c = Jt({
		captureSnapshot: qt,
		onPage(t, n, r) {
			if (s({
				sourceTabId: t.id,
				sourceUrl: t.url || ""
			}), !n) return;
			let i = Z(e);
			s({
				pageResetRevision: i.pageResetRevision + +!!r,
				sourcePageSnapshot: null,
				sourcePageTitle: "",
				sourceSiteIconUrl: chrome.runtime.getURL(Ze(t.url || "") || "resources/site-icons/Web.svg"),
				analysisRunning: !1,
				busy: !i.analysisRunning && i.busy,
				messageText: "",
				messageError: !1,
				helpUrl: ""
			}), T(t.url || "").catch((n) => {
				!o && Z(e).sourceUrl === (t.url || "") && l(n.message, !0);
			});
		},
		onSnapshot(e, t) {
			s({
				sourcePageSnapshot: t,
				sourcePageTitle: String(t?.title || e.title || "").trim()
			});
		},
		onError(e) {
			l(e.message, !0);
		}
	});
	t.onerror = (e) => {
		a = e.message || Q("matcherIndexLoadFailed"), !o && s({
			recognized: null,
			recognitionPending: !1,
			recognitionText: a,
			recognitionVisible: !0
		});
	};
	let l = (e = "", t = !1, n = "") => s({
		messageText: e,
		messageError: t,
		helpUrl: n
	}), u = (t, n = "", r = Z(e)) => {
		let i = String(t || "").trim(), a = i.toLowerCase(), o = "";
		try {
			o = new URL(r.sourceUrl).hostname.toLowerCase().replace(/^www\./, "");
		} catch {}
		let s = r.sourcePageSnapshot, c = !!(s?.mediaKind || Number(s?.mediaElementCount)), u = a.includes("no media found"), d = (o === "reddit.com" || o.endsWith(".reddit.com")) && !!s && !c && a.includes("[generic]") && a.includes("http error 403") && a.includes("blocked"), f = u || d;
		l(f ? Q("noDownloadableMedia") : Q("analysisFailed", { message: i }), !f, f ? "" : n);
	}, d = (t, n = Z(e)) => `${t}:tab:${n.sourceTabId}:${n.analysisStorageScope ?? n.playlistScope}:${Ee(n.sourceUrl)}`;
	function f(e, t) {
		return !!(t && (e.subtitles || []).find((e) => e.language === t)?.automatic);
	}
	function p(t, n = {}) {
		let r = n.contentKind === "audio" && yt(t, "audio").length ? "audio" : "video", i = Mt(t, r), a = pt(t, n), o = a.length ? null : ut(n), s = un(n.thumbnailMode ?? Z(e).analysis?.thumbnailMode), c = r === "video" && n.subtitleLanguage || "", l = c ? un(n.subtitleMode, n.subtitleLanguage ? "download" : "off") : "off", u = !o && a.length === 0 && t.chapterCount > 0 ? dn(n, Z(e).analysis?.chapterMode) : "off", d = n.outputFormat || i.outputFormat;
		return {
			...i,
			fileName: typeof n.fileName == "string" ? n.fileName : t.title || "",
			rangeStartSeconds: o?.startSeconds ?? null,
			rangeEndSeconds: o?.endSeconds ?? null,
			chapterIndices: a,
			videoFormatId: r === "video" ? n.videoFormat || i.videoFormatId : "",
			audioFormatId: n.audioFormat || i.audioFormatId,
			subtitleLanguage: c,
			subtitleAutomatic: r === "video" && f(t, n.subtitleLanguage),
			subtitleMode: l,
			chapterMode: u,
			splitChapters: ["download", "embed"].includes(u),
			useAria2: !1,
			thumbnailMode: s,
			outputFormat: s === "embed" && d === "webm" ? "mkv" : d
		};
	}
	function m(t, n) {
		let r = Z(e), i = r.analysis?.items?.[t];
		if (!i) return null;
		let a = r.choices[t] || {}, o = pt(i, a), s = o.length ? null : ut(a), c = xt(i), l = n && c.includes(n) ? n : a.contentKind === "audio" && c.includes("audio") ? "audio" : "video", u = yt(i, "video"), d = l === "audio" ? yt(i, "audio") : yt(i, "audio").filter((e) => e.kind === "audio"), m = u.some((e) => e.id === a.videoFormat) ? a.videoFormat : bt(i, "video"), h = d.some((e) => e.id === a.audioFormat) ? a.audioFormat : bt(i, "audio"), g = {
			...p(i, a),
			contentKind: l,
			videoFormatId: l === "video" ? m : "",
			audioFormatId: h
		}, _ = At(i, g), v = jt(i, g), y = _.includes(a.outputFormat) ? a.outputFormat : _.includes(v) ? v : _[0] || "", b = un(a.thumbnailMode ?? r.analysis?.thumbnailMode), x = l === "video" && a.subtitleLanguage || "", S = x ? un(a.subtitleMode, a.subtitleLanguage ? "download" : "off") : "off", C = !s && o.length === 0 && i.chapterCount > 0 ? dn(a, r.analysis?.chapterMode) : "off";
		return b === "embed" && y === "webm" && (y = "mkv"), {
			...a,
			fileName: typeof a.fileName == "string" ? a.fileName : i.title || "",
			rangeStartSeconds: s?.startSeconds ?? null,
			rangeEndSeconds: s?.endSeconds ?? null,
			chapterIndices: o,
			contentKind: l,
			videoFormat: m,
			audioFormat: h,
			subtitleLanguage: x,
			subtitleAutomatic: l === "video" && f(i, a.subtitleLanguage),
			subtitleMode: S,
			chapterMode: C,
			chapters: ["download", "embed"].includes(C) ? "split" : "full",
			useAria2: !1,
			thumbnailMode: b,
			outputFormat: y
		};
	}
	function h(e) {
		s({ choices: e }), chrome.storage.session.set({ [d("choices")]: e });
	}
	function g(t, n, r = !0) {
		let i = Z(e), a = i.choices[t] || {}, o = { ...n };
		[
			"off",
			"download",
			"embed"
		].includes(o.chapterMode) && (o.chapters = o.chapterMode === "off" ? "full" : "split", o.chapterMode !== "off" && (o.rangeStartSeconds = null, o.rangeEndSeconds = null, o.chapterIndices = [])), o.thumbnailMode === "embed" && (o.outputFormat ?? a.outputFormat) === "webm" ? o.outputFormat = "mkv" : o.outputFormat === "webm" && (o.thumbnailMode ?? a.thumbnailMode) === "embed" && (o.thumbnailMode = "download");
		let c = {
			...i.choices,
			[t]: {
				...a,
				...o
			}
		};
		if (s({ choices: c }), r) {
			let e = m(t, c[t].contentKind);
			e && (c[t] = e);
		}
		h({ ...c });
	}
	function _(e, t) {
		if (!e) return !1;
		try {
			return Pt(e, p(e, t)), !0;
		} catch {
			return !1;
		}
	}
	function v(t, n) {
		let r = Z(e), i = Ce(r.analysisByBackend.g?.items, r.sourceUrl)[t];
		if (!i) return;
		let a = new Set(r.selectionsByBackend.g);
		for (let e of i.sourceItemIndices) n ? a.add(e) : a.delete(e);
		s({ selectionsByBackend: {
			...r.selectionsByBackend,
			g: new Set(a)
		} }), chrome.storage.session.set({ [`${d("selection", r)}:g`]: [...a].sort((e, t) => e - t) });
	}
	function y(t, n = Z(e)) {
		if (n.playlistScope !== "current" || t.items?.length !== 1 || !n.sourcePageSnapshot) return;
		let r = t.items[0];
		!String(r.title || "").trim() && n.sourcePageSnapshot.title && (r.title = n.sourcePageSnapshot.title), !String(r.thumbnail || "").trim() && n.sourcePageSnapshot.thumbnailUrl && (r.thumbnail = n.sourcePageSnapshot.thumbnailUrl), !String(r.creator || "").trim() && n.sourcePageSnapshot.creator && (r.creator = n.sourcePageSnapshot.creator), !String(r.duration || "").trim() && Number.isFinite(n.sourcePageSnapshot.mediaDurationSeconds) && n.sourcePageSnapshot.mediaDurationSeconds >= 0 && (r.duration = String(n.sourcePageSnapshot.mediaDurationSeconds));
	}
	function b(t) {
		if (!t?.ok || !t.data?.request) {
			t?.message && u(t.message, t.helpUrl);
			return;
		}
		let n = t.data, r = Z(e), i = r.unifiedAnalysis ? {
			ok: !0,
			data: r.unifiedAnalysis
		} : null;
		if (Oe(i, t)?.data !== n) return;
		s({
			unifiedAnalysis: n,
			analysisStorageScope: n.request.playlistScope ?? r.analysisStorageScope ?? r.playlistScope
		});
		for (let t of ["y", "g"]) {
			let r = n[t === "y" ? "yt-dlp" : "gallery-dl"], i = r?.status || "pending", a = r?.info || {}, o = ["pending", "analyzing"].includes(i);
			x(t, {
				...a,
				items: r?.items || [],
				backend: t,
				status: i,
				analyzing: o,
				phase: a.phase || (o ? "preparing" : "complete"),
				host: n.request.host,
				schemaVersion: 22,
				startedAt: n.request.startedAt,
				updatedAt: n.request.updatedAt
			}), s({ analysisErrors: {
				...Z(e).analysisErrors,
				[t]: ["failed", "interrupted"].includes(i) ? a.message || n.request.message || Q("nativeAnalysisFailed") : ""
			} });
		}
		let a = ["y", "g"].filter((e) => ![
			"notMatched",
			"disabled",
			"pending"
		].includes(n[e === "y" ? "yt-dlp" : "gallery-dl"]?.status)), o = n.request.status === "analyzing";
		if (s({ analysisRunning: o }), !o && n["yt-dlp"]?.info?.playlistRequired && s({
			playlistScope: "playlist",
			playlistSelectable: !1,
			scopeLabel: Q("fullPlaylist")
		}), !o && !a.length) s({
			analysis: null,
			analysisView: "idle",
			analysisPhase: "complete"
		}), l(Q("noMatchingExtractor"), !0);
		else if (!o && a.length && !a.some((t) => Z(e).analysisByBackend[t]?.items?.length)) {
			let r = a.map((e) => n[e === "y" ? "yt-dlp" : "gallery-dl"]?.info?.message || "").filter(Boolean).join(" / ");
			r && !Object.values(Z(e).analysisErrors).some(Boolean) && u(r, t.helpUrl);
		}
	}
	function x(t, n) {
		if (!["y", "g"].includes(t)) return;
		let r = Z(e);
		if (!st(r, t, n)) return;
		t === "y" && y(n, r);
		let i = {
			...r.analysisByBackend,
			[t]: n
		}, a = new Set(r.selectionsByBackend[t]), o = r.knownCountsByBackend[t] || 0;
		for (let e = o; e < n.items.length; e += 1) a.add(e);
		for (let e of a) e >= n.items.length && a.delete(e);
		let c = Object.values(i).some((e) => !!e?.analyzing);
		if (s({
			analysisByBackend: i,
			analysis: i.y,
			selectionsByBackend: {
				...r.selectionsByBackend,
				[t]: a
			},
			knownCountsByBackend: {
				...r.knownCountsByBackend,
				[t]: Math.max(o, n.items.length)
			},
			analysisPhase: Object.values(i).find((e) => e?.analyzing)?.phase || "complete",
			analysisView: Object.values(i).some((e) => e?.items?.length) ? "results" : c ? "loading" : "idle",
			analysisRunning: c
		}), !n.analyzing && !n.stopped) {
			let e = `${d("selection")}${t === "g" ? ":g" : ""}`;
			chrome.storage.session.set({ [e]: [...a].sort((e, t) => e - t) });
		}
	}
	async function S(t = r) {
		let n = ++i, a = Z(e);
		if (!Number.isInteger(a.sourceTabId) || !a.sourceUrl) return;
		let c = d("choices", a), l = d("selection", a), u = d("galleryModes", a), [f, p] = await Promise.all([chrome.storage.session.get([
			c,
			l,
			`${l}:g`,
			u,
			d("galleryRanges", a)
		]), chrome.runtime.sendMessage({
			type: "YT_DLP_GUI_ANALYSIS_SNAPSHOT",
			request: {
				sourceTabId: a.sourceTabId,
				playlistScope: a.analysisStorageScope ?? a.playlistScope,
				url: a.sourceUrl
			}
		}).catch(() => null)]);
		if (o || t !== r || n !== i) return;
		let m = Z(e);
		if (m.unifiedAnalysis !== a.unifiedAnalysis || m.sourceTabId !== a.sourceTabId || m.playlistScope !== a.playlistScope || Ee(m.sourceUrl) !== Ee(a.sourceUrl)) return;
		let h = {
			y: new Set(f[l] || []),
			g: new Set(f[`${l}:g`] || [])
		};
		s({
			choices: f[c] || {},
			galleryModes: f[u] || {},
			galleryRanges: f[d("galleryRanges", a)] || {},
			selectionsByBackend: h,
			knownCountsByBackend: {
				y: Array.isArray(f[l]) && p?.data?.data?.["yt-dlp"]?.items?.length || 0,
				g: Array.isArray(f[`${l}:g`]) && p?.data?.data?.["gallery-dl"]?.items?.length || 0
			},
			analysisStorageScope: a.analysisStorageScope ?? a.playlistScope,
			analysisByBackend: {
				y: null,
				g: null
			},
			unifiedAnalysis: null,
			analysisErrors: {
				y: "",
				g: ""
			}
		}), p?.ok && p.data && b(p.data);
	}
	function C(t = { state: "idle" }) {
		let n = ["running", "stopping"].includes(t.state) || ["running", "stopping"].includes(Z(e).browserDownloadTask.state);
		if (s({
			currentTask: t,
			downloadRunning: n
		}), t.state === "running") {
			let e = Math.max(0, Math.min(100, Number(t.percent) || 0)), n = [
				t.itemCount > 1 ? `${t.completedItems + 1} / ${t.itemCount}` : "",
				t.downloaded && t.total ? `${t.downloaded} / ${t.total}` : t.downloaded,
				t.speed,
				t.eta ? Q("remaining", { eta: t.eta }) : ""
			].filter(Boolean).join(" · ");
			l(`${t.title || Q("downloading")} · ${e.toFixed(0)}%${n ? ` · ${n}` : ""}`);
		} else t.state === "stopping" ? l(Q("downloadStopping")) : t.state === "stopped" ? l(Q("downloadStopped")) : t.state === "completed" ? l(t.itemCount > 1 ? Q("completedItems", { count: t.itemCount }) : Q("completedValue", { value: t.outputPath || t.title || Q("downloadComplete") })) : t.state === "failed" && l(t.message || Q("downloadFailed"), !0, t.helpUrl);
	}
	function w(e, t) {
		if (t.value === "playlist") return "fullPlaylist";
		try {
			let t = new URL(e).hostname.toLowerCase().replace(/^www\./, "");
			if (t === "youtube.com" || t.endsWith(".youtube.com") || t === "youtu.be") return "currentVideo";
		} catch {}
		return "currentPage";
	}
	async function T(i) {
		let o = ++r, c = i.trim();
		s({
			recognized: null,
			recognitionPending: !0,
			analysis: null,
			unifiedAnalysis: null,
			analysisStorageScope: null,
			analysisByBackend: {
				y: null,
				g: null
			},
			analysisErrors: {
				y: "",
				g: ""
			},
			selectionsByBackend: {
				y: /* @__PURE__ */ new Set(),
				g: /* @__PURE__ */ new Set()
			},
			knownCountsByBackend: {
				y: 0,
				g: 0
			},
			choices: {},
			galleryModes: {},
			galleryRanges: {},
			sourceUrl: c,
			sourceSiteName: Q("recognizing"),
			recognitionText: Q("recognizing"),
			recognitionVisible: !0,
			analysisView: "idle"
		});
		try {
			ke(c);
			let t = Nt(c), n = new URL(c).hostname.toLowerCase(), r = n.startsWith("www.") ? n.slice(4) : n, i = String(Z(e).sourcePageSnapshot?.siteName || "").trim();
			s({
				sourceSiteName: i || r || Q("genericDownload"),
				playlistScope: t.value,
				playlistSelectable: t.selectable,
				scopeLabel: Q(w(c, t))
			});
		} catch (e) {
			s({
				recognized: null,
				recognitionPending: !1,
				sourceSiteName: Q("unavailable"),
				recognitionText: e.message,
				recognitionVisible: !0
			});
			return;
		}
		a ? s({
			recognitionPending: !1,
			recognitionText: a,
			recognitionVisible: !0
		}) : t.postMessage({
			id: o,
			url: c,
			indexUrl: n
		}), await S(o);
	}
	t.onmessage = async ({ data: t }) => {
		if (o || t.id !== r) return;
		if (t.error) {
			s({
				recognized: null,
				recognitionPending: !1,
				recognitionText: t.error,
				recognitionVisible: !0
			});
			return;
		}
		let n = at(t.result, Z(e).sourceUrl), i = String(n.name || "").trim(), a = Z(e), c = String(a.sourcePageSnapshot?.siteName || "").trim(), l = a.unifiedAnalysis?.["yt-dlp"]?.info?.playlistRequired ? {
			value: "playlist",
			selectable: !1
		} : Nt(a.sourceUrl, n.y?.sourceScope), u = "";
		try {
			u = new URL(a.sourceUrl).hostname.toLowerCase().replace(/^www\./, "");
		} catch {}
		let d = n.kind === "known" ? {
			youtube: "YouTube",
			reddit: "Reddit",
			twitter: "X / Twitter"
		}[i.toLowerCase()] || i || Q("supportedSite") : c || u || Q("genericDownload");
		s({
			recognized: n,
			playlistScope: l.value,
			playlistSelectable: l.selectable,
			scopeLabel: Q(w(a.sourceUrl, l)),
			recognitionPending: !1,
			sourceSiteName: d,
			recognitionText: n.kind === "known" ? "" : Q("generic"),
			recognitionVisible: n.kind !== "known"
		}), l.value !== a.playlistScope && !a.unifiedAnalysis && (s({ analysisStorageScope: l.value }), await S(t.id));
	};
	function E(t = { state: "idle" }) {
		let n = Z(e);
		(t.revision || 0) < (n.browserDownloadTask.revision || 0) || (s({
			browserDownloadTask: t,
			downloadRunning: ["running", "stopping"].includes(t.state) || ["running", "stopping"].includes(n.currentTask.state)
		}), t.state === "failed" && l(t.message || Q("downloadFailed"), !0));
	}
	let D = (e) => {
		e?.type === "YT_DLP_GUI_TASK_EVENT" ? C(e.task) : e?.type === "YT_DLP_GUI_BROWSER_DOWNLOAD_EVENT" ? E(e.task) : e?.type === sn && e.key === d("analysis") && b(e.response);
	}, O = (e, t) => {
		t === "local" && (e.v2DownloadDirectory || e.galleryDownloadMode || e.analysisToolPreferences || e.analysisToolDetection || e.albumWheelMode) && k();
	};
	async function k() {
		let e = await chrome.storage.local.get([
			"v2DownloadDirectory",
			"playlistItemLimit",
			Qe,
			en,
			tn,
			le
		]);
		s({
			playlistItemLimit: ln(e.playlistItemLimit),
			outputDirectory: e.v2DownloadDirectory || "",
			galleryDownloadMode: et(e[Qe]),
			analysisTools: nn(e[en]),
			albumWheelMode: pe(e[le]),
			toolDetection: e.analysisToolDetection || null,
			preferencesLoaded: !0
		});
	}
	async function A(t) {
		let n = Z(e);
		if (!n.preferencesLoaded || n.albumPreferenceSaving) return;
		let r = pe(t);
		s({
			albumWheelMode: r,
			albumPreferenceSaving: !0,
			albumSettingsMessage: ""
		});
		try {
			await chrome.storage.local.set({ [le]: r });
		} catch (e) {
			s({
				albumWheelMode: n.albumWheelMode,
				albumSettingsMessage: Q("albumSettingsFailed", { message: e.message })
			});
		} finally {
			s({ albumPreferenceSaving: !1 });
		}
	}
	async function j(t, n) {
		let r = Z(e);
		if (!["y", "g"].includes(t) || !r.preferencesLoaded || r.busy || r.analysisRunning || r.downloadRunning || r.toolDetectionRunning || r.toolPreferenceSaving) return;
		let i = {
			...r.analysisTools,
			[t]: !!n
		};
		s({
			analysisTools: i,
			toolPreferenceSaving: !0
		});
		try {
			await chrome.storage.local.set({ [en]: i }), s({
				analysisTools: i,
				toolSettingsMessage: "",
				toolSettingsError: !1
			});
		} catch (e) {
			s({
				analysisTools: r.analysisTools,
				toolSettingsMessage: Q("toolSettingsFailed", { message: e.message }),
				toolSettingsError: !0
			});
		} finally {
			s({ toolPreferenceSaving: !1 });
		}
	}
	async function M() {
		let t = Z(e);
		if (!(!t.preferencesLoaded || t.busy || t.analysisRunning || t.downloadRunning || t.toolDetectionRunning || t.toolPreferenceSaving)) {
			s({
				toolDetectionRunning: !0,
				toolSettingsMessage: "",
				toolSettingsError: !1
			});
			try {
				let e = an((await fn({ action: "status" })).data), t = await chrome.storage.local.get(en);
				await chrome.storage.local.set({
					[tn]: e,
					...t.analysisToolPreferences == null ? { [en]: on(e) } : {}
				}), await k(), s({ toolSettingsMessage: e.g == null ? Q("toolDetectionLegacy") : Q("toolDetectionDone") });
			} catch (e) {
				s({
					toolSettingsMessage: Q("toolSettingsFailed", { message: e.message }),
					toolSettingsError: !0
				});
			} finally {
				s({ toolDetectionRunning: !1 });
			}
		}
	}
	function N(t, n = Z(e)) {
		return et(n.galleryModes[t.key] ?? n.galleryDownloadMode);
	}
	async function P(t, n) {
		let r = Z(e);
		if (r.busy || r.downloadRunning || r.analysisRunning) return;
		let i = {
			...r.galleryRanges,
			[t]: De(n)
		};
		s({ galleryRanges: i });
		try {
			await chrome.storage.session.set({ [d("galleryRanges", r)]: i });
		} catch (e) {
			s({ galleryRanges: r.galleryRanges }), l(e.message, !0);
		}
	}
	function F(e, t) {
		let n = Se([e], t.sourceSiteName, t.sourceUrl);
		return n.timeline &&= {
			...n.timeline,
			range: Te(e, t),
			sourceTabId: t.sourceTabId
		}, n;
	}
	async function te(t, n) {
		let r = Z(e);
		if (!Ce(r.analysisByBackend.g?.items, r.sourceUrl).find((e) => e.key === n) || r.downloadRunning || r.analysisRunning || r.busy) return;
		let i = et(t), a = { ...r.galleryModes };
		for (let e of Ce(r.analysisByBackend.g?.items, r.sourceUrl)) a[e.key] = N(e, r);
		a[n] = i, s({
			galleryDownloadMode: i,
			galleryModes: a
		});
		try {
			await Promise.all([chrome.storage.local.set({ [Qe]: i }), chrome.storage.session.set({ [d("galleryModes", r)]: a })]);
		} catch (e) {
			l(Q("galleryDownloadChoiceSaveFailed", { message: e.message }), !0);
		}
	}
	async function ne() {
		await Promise.all([chrome.runtime.sendMessage({ type: "YT_DLP_GUI_TASK_STATUS" }).then((e) => {
			e?.ok && C(e.data.task);
		}), chrome.runtime.sendMessage({ type: "YT_DLP_GUI_BROWSER_DOWNLOAD_STATUS" }).then((e) => {
			e?.ok && E(e.task);
		})]);
	}
	async function I(t = null) {
		let n = Z(e), a = r;
		if (!n.preferencesLoaded || n.toolDetectionRunning || n.toolPreferenceSaving || n.busy || n.analysisRunning || n.downloadRunning || n.recognitionPending || !n.recognized || t != null && (!["y", "g"].includes(t) || !n.analysisErrors[t])) return;
		let c = t ? [t] : rn(n.analysisTools);
		if (!c.length) {
			l(Q("selectAnalysisTool"), !0);
			return;
		}
		if (!await Ae([new URL(n.sourceUrl).origin + "/*"])) {
			l(Q("permissionRequired"), !0);
			return;
		}
		if (o || a !== r) return;
		let d = Z(e);
		if (d.busy || d.analysisRunning || d.downloadRunning || d.recognitionPending || d.recognized !== n.recognized || d.playlistScope !== n.playlistScope || t && d.unifiedAnalysis !== n.unifiedAnalysis) return;
		let f = t ? n.analysisStorageScope ?? n.playlistScope : n.playlistScope;
		i += 1;
		let p = ot(c, n.playlistItemLimit), m = t ? {
			...n.analysisByBackend,
			[t]: p[t]
		} : p;
		s({
			busy: !0,
			analysisStorageScope: f,
			unifiedAnalysis: t ? n.unifiedAnalysis : null,
			analysis: m.y,
			analysisByBackend: m,
			analysisRunning: !0,
			analysisErrors: t ? {
				...n.analysisErrors,
				[t]: ""
			} : {
				y: "",
				g: ""
			},
			selectionsByBackend: t ? {
				...n.selectionsByBackend,
				[t]: /* @__PURE__ */ new Set()
			} : {
				y: /* @__PURE__ */ new Set(),
				g: /* @__PURE__ */ new Set()
			},
			knownCountsByBackend: t ? {
				...n.knownCountsByBackend,
				[t]: 0
			} : {
				y: 0,
				g: 0
			},
			analysisView: "loading",
			analysisPhase: "preparing"
		}), l();
		try {
			let i = await fn({
				action: "analyzeIndexedSources",
				analysisTools: c,
				...t ? {
					retryBackend: t,
					retryStartedAt: n.unifiedAnalysis?.request?.startedAt
				} : {},
				url: n.sourceUrl,
				playlistScope: f,
				itemLimit: n.playlistItemLimit,
				sourceTabId: n.sourceTabId
			});
			!o && a === r && n.sourceUrl === Z(e).sourceUrl && b(i.data);
		} catch (i) {
			if (!o && a === r && n.sourceUrl === Z(e).sourceUrl) {
				if (t) {
					let { switchTo: n, ...r } = ct(Z(e), t, i.message);
					s(r);
				} else u(i.message, i.helpUrl);
			}
		} finally {
			!o && a === r && n.sourceUrl === Z(e).sourceUrl && s({ busy: !1 });
		}
	}
	async function re() {
		let t = Z(e);
		if (!t.busy && t.analysisRunning) {
			s({ busy: !0 });
			try {
				try {
					b((await fn({
						action: "cancelAnalysis",
						url: t.sourceUrl,
						playlistScope: t.analysisStorageScope ?? t.playlistScope,
						sourceTabId: t.sourceTabId
					})).data);
				} catch (e) {
					u(e.message, e.helpUrl);
				}
			} finally {
				s({ busy: !1 });
			}
		}
	}
	async function ie() {
		let t = Z(e);
		if (t.busy || t.downloadRunning || t.analysisRunning) return;
		let n = Zt(t);
		if (n.length) try {
			let e = n.filter((e) => e.backend === "g").map((e) => ({
				group: e.item,
				mode: N(e.item, t),
				key: e.key,
				index: e.index
			})), r = e.filter(({ mode: e }) => !nt(e)), i = e.filter(({ mode: e }) => nt(e)), a = t.analysisByBackend.g?.host?.capabilities || [];
			if (e.some(({ group: e }) => e.sourceScope === "timeline") && !a.includes("gallery.timeline.v1")) {
				l(Q("galleryTimelineUpgradeRequired"), !0);
				return;
			}
			if (r.length && !a.includes("download.gallery-post.v1")) {
				l(Q("nativeGalleryDownloadUpgradeRequired"), !0);
				return;
			}
			let o = r.flatMap(({ group: e, key: n, index: r }) => it([{
				...e,
				index: r
			}], t).map((e) => ({
				...e,
				sourceItemKey: n,
				sourceAnalysisStartedAt: t.unifiedAnalysis?.request?.startedAt
			})));
			if (o.length !== r.length) {
				l(Q("galleryDownloadUrlUnavailable"), !0);
				return;
			}
			let c = n.filter((e) => e.backend === "y").map((e) => ({
				...Pt(e.item, p(e.item, t.choices[e.index])),
				sourcePageUrl: t.sourceUrl,
				sourceTabId: t.sourceTabId,
				sourceItemIndex: e.index,
				sourceItemKey: e.key,
				sourceAnalysisStartedAt: t.unifiedAnalysis?.request?.startedAt
			})), u = [...c, ...o];
			if (u.length > 100) {
				l(Q("downloadListInvalid"), !0);
				return;
			}
			let d = i.map(({ group: e }) => e), f = [.../* @__PURE__ */ new Set([...rt(d), ...c.map((e) => `${new URL(e.url).origin}/*`)])];
			if (!await Ae(f)) {
				l(Q("permissionRequired"), !0);
				return;
			}
			let m = (e) => F(e, t);
			s({ busy: !0 });
			try {
				if (i.length) {
					let e = await chrome.runtime.sendMessage({
						type: "YT_DLP_GUI_BROWSER_DOWNLOAD_START",
						jobs: i.map(({ group: e, mode: t, key: n }) => ({
							gallery: m(e),
							mode: t,
							key: n
						})),
						sourceTabId: t.sourceTabId,
						sourcePageUrl: t.sourceUrl,
						sourceAnalysisStartedAt: t.unifiedAnalysis?.request?.startedAt
					});
					if (!e?.ok) throw Error(e?.message || Q("browserDownloadUnavailable"));
					E(e.task);
				}
				u.length && C((await fn({
					action: "downloadBatch",
					requests: u
				})).data.task);
			} catch (e) {
				l(e.message || Q("browserDownloadUnavailable"), !0, e.helpUrl);
			} finally {
				s({ busy: !1 });
			}
		} catch (e) {
			l(e.message, !0, e.helpUrl);
		}
	}
	async function L(t) {
		let n = Z(e), r = Ce(n.analysisByBackend.g?.items, n.sourceUrl)[t];
		if (r) try {
			if (r.sourceScope === "timeline" && !n.analysisByBackend.g?.host?.capabilities?.includes("gallery.timeline.v1")) throw Error(Q("galleryTimelineUpgradeRequired"));
			if (!await Ae(rt([r]))) {
				l(Q("permissionRequired"), !0);
				return;
			}
			let e = await chrome.runtime.sendMessage({
				type: "YT_DLP_GUI_OPEN_GALLERY",
				gallery: F(r, n)
			});
			if (!e?.ok) throw Error(e?.message || Q("galleryAlbumOpenFailed"));
		} catch (e) {
			l(e.message || Q("galleryAlbumOpenFailed"), !0);
		}
	}
	async function ae() {
		let t = Z(e), n = Ce(t.analysisByBackend.g?.items, t.sourceUrl);
		if (!(n.length < 2 || t.busy || t.analysisRunning)) try {
			if (!await Ae(rt(n))) {
				l(Q("permissionRequired"), !0);
				return;
			}
			let e = await chrome.runtime.sendMessage({
				type: "YT_DLP_GUI_OPEN_GALLERY",
				gallery: Se(n, t.sourceSiteName, t.sourceUrl)
			});
			if (!e?.ok) throw Error(e?.message || Q("galleryAlbumOpenFailed"));
		} catch (e) {
			l(e.message || Q("galleryAlbumOpenFailed"), !0);
		}
	}
	async function R() {
		let t = Z(e);
		if (!t.busy && t.downloadRunning && t.currentTask?.state !== "stopping") {
			s({ busy: !0 });
			try {
				if (["running", "stopping"].includes(t.browserDownloadTask.state)) {
					let e = await chrome.runtime.sendMessage({ type: "YT_DLP_GUI_BROWSER_DOWNLOAD_STOP" });
					if (!e?.ok) throw Error(e?.message || Q("downloadStopFailed"));
					E(e.task);
				}
				["running", "stopping"].includes(t.currentTask.state) && C((await fn({ action: "cancelDownload" })).data.task);
			} catch (e) {
				l(Q("downloadStopFailed", { message: e.message }), !0);
			} finally {
				s({ busy: !1 });
			}
		}
	}
	async function z(t) {
		let n = Z(e);
		n.busy || n.analysisRunning || n.downloadRunning || n.playlistScope === t || (s({
			playlistScope: t,
			analysisStorageScope: t,
			analysis: null,
			unifiedAnalysis: null,
			analysisByBackend: {
				y: null,
				g: null
			},
			analysisErrors: {
				y: "",
				g: ""
			},
			choices: {},
			galleryModes: {},
			selectionsByBackend: {
				y: /* @__PURE__ */ new Set(),
				g: /* @__PURE__ */ new Set()
			},
			galleryRanges: {},
			knownCountsByBackend: {
				y: 0,
				g: 0
			},
			analysisView: "idle"
		}), l(), await S());
	}
	async function oe(e) {
		let t = ln(e);
		s({ playlistItemLimit: t }), await chrome.storage.local.set({ playlistItemLimit: t });
	}
	function se(t, n) {
		let r = Z(e), i = new Set(r.selectionsByBackend.y);
		n ? i.add(t) : i.delete(t), s({ selectionsByBackend: {
			...r.selectionsByBackend,
			y: new Set(i)
		} }), chrome.storage.session.set({ [d("selection")]: [...i].sort((e, t) => e - t) });
	}
	function B(t) {
		let n = m(t);
		n && h({
			...Z(e).choices,
			[t]: n
		});
	}
	async function ce() {
		let t = Z(e);
		if (!(t.busy || t.analysisRunning || t.downloadRunning || t.cookieSaveRunning)) {
			if (!t.sourceUrl || !Number.isInteger(t.sourceTabId)) {
				s({
					cookieSaveMessage: Q("cookieSaveNoPage"),
					cookieSaveError: !0
				});
				return;
			}
			s({
				cookieSaveRunning: !0,
				cookieSaveMessage: "",
				cookieSaveError: !1
			});
			try {
				let e = await be(t.sourceUrl);
				if (!await Ae(e)) {
					s({
						cookieSaveMessage: Q("permissionRequired"),
						cookieSaveError: !0
					});
					return;
				}
				let n = await fn({
					action: "saveBrowserCookies",
					url: t.sourceUrl,
					sourceTabId: t.sourceTabId
				});
				s({
					cookieSaveMessage: Q(n.data?.autoUpdateEnabled === !0 ? "cookieSaveSuccess" : "cookieSaveAutoUpdateUnavailable"),
					cookieSaveError: n.data?.autoUpdateEnabled !== !0
				});
			} catch (e) {
				s({
					cookieSaveMessage: e.message || Q("cookieSaveFailed"),
					cookieSaveError: !0
				});
			} finally {
				s({ cookieSaveRunning: !1 });
			}
		}
	}
	async function V() {
		let t = Z(e);
		if (!(t.busy || t.analysisRunning || t.downloadRunning)) {
			s({ busy: !0 });
			try {
				await fn({ action: "chooseDirectory" }), await k();
			} catch (e) {
				l(e.message, !0);
			} finally {
				s({ busy: !1 });
			}
		}
	}
	async function H() {
		let t = Z(e);
		if (!(t.busy || t.analysisRunning || t.downloadRunning || !t.outputDirectory)) {
			s({ busy: !0 });
			try {
				await fn({ action: "openDirectory" });
			} catch (e) {
				l(Q("openFolderFailed", { message: e.message }), !0);
			} finally {
				s({ busy: !1 });
			}
		}
	}
	function ue() {
		o = !0, r += 1, c.destroy(), t.terminate(), chrome.runtime.onMessage.removeListener(D), chrome.storage.onChanged.removeListener(O);
	}
	async function de() {
		document.documentElement.lang = chrome.i18n.getUILanguage?.() || "en", chrome.runtime.onMessage.addListener(D), chrome.storage.onChanged.addListener(O), await Promise.all([
			k(),
			c.start(),
			ne()
		]);
	}
	return {
		subscribe: e.subscribe,
		init: de,
		destroy: ue,
		analyze: I,
		retryAnalysis: (e) => I(e),
		stopAnalysis: re,
		setGalleryDownloadMode: te,
		setGalleryRange: P,
		galleryModeForGroup: N,
		toggleGalleryGroup: v,
		downloadSelected: ie,
		openGalleryAlbum: L,
		openGalleryAlbumAll: ae,
		stopDownload: R,
		setPlaylistScope: z,
		setPlaylistItemLimit: oe,
		toggleSelection: se,
		prepareDetail: B,
		setChoice: g,
		chooseDirectory: V,
		openDirectory: H,
		saveSiteCookies: ce,
		setAnalysisTool: j,
		setAlbumWheelMode: A,
		detectAnalysisTools: M,
		storedDownloadChoice: p,
		downloadChoiceAvailable: _
	};
}
//#endregion
//#region src/popup/popup-runtime.js
var $ = pn();
//#endregion
//#region src/shared/format-display.js
function mn(e) {
	let t = String(e || "").trim().toLowerCase();
	return t ? /^(av01|av1)/.test(t) ? "AV1" : /^(avc|h\.264|h264)/.test(t) ? "H.264" : /^(hev|hvc|h\.265|h265)/.test(t) ? "H.265" : /^(vp09|vp9)/.test(t) ? "VP9" : /^(vp08|vp8)/.test(t) ? "VP8" : /^(mp4a|aac)/.test(t) ? "AAC" : t.startsWith("opus") ? "Opus" : t.startsWith("vorbis") ? "Vorbis" : t.startsWith("mp3") ? "MP3" : t.split(".")[0].toUpperCase() : "";
}
function hn(e) {
	let t = Number(e) / 1e3;
	return !Number.isFinite(t) || t <= 0 ? "" : `${Number.isInteger(t) ? t.toFixed(0) : t.toFixed(1)} kHz`;
}
function gn(e) {
	let t = String(e ?? "").trim();
	if (!t) return "";
	let n;
	if (t.includes(":")) {
		let e = t.split(":").map((e) => Number(e.trim()));
		if (e.length < 2 || e.length > 3 || e.some((e) => !Number.isInteger(e) || e < 0)) return t;
		n = e.reduce((e, t) => e * 60 + t, 0);
	} else {
		let e = Number(t);
		if (!Number.isFinite(e) || e < 0) return t;
		n = Math.round(e);
	}
	let r = Math.floor(n / 3600), i = Math.floor(n % 3600 / 60), a = n % 60;
	return r > 0 ? `${r}:${String(i).padStart(2, "0")}:${String(a).padStart(2, "0")}` : `${String(i).padStart(2, "0")}:${String(a).padStart(2, "0")}`;
}
function _n(e) {
	let t = Number(e);
	return Number.isFinite(t) && t > 0 ? `${Math.round(t)} kbps` : "";
}
function vn(e, t = !1) {
	let n = Number(e);
	if (!Number.isFinite(n) || n <= 0) return "";
	let r = n >= 1073741824 ? `${(n / 1073741824).toFixed(1)} GB` : `${(n / 1048576).toFixed(n >= 104857600 ? 0 : 1)} MB`;
	return t ? Q("approximate", { value: r }) : r;
}
function yn(e, t) {
	if (!e) return Q("noFormats");
	if (t) return [
		e.kind === "muxed" ? "V+A" : "",
		String(e.language || "").trim(),
		hn(e.sampleRate),
		mn(e.audioCodec || e.codec),
		_n(e.bitrate),
		vn(e.filesize, e.filesizeApproximate)
	].filter(Boolean).join(" · ") || e.id;
	let n = [mn(e.codec), e.kind === "muxed" ? mn(e.audioCodec) : ""].filter(Boolean).join(" + ");
	return [
		e.kind === "muxed" ? "V+A" : "",
		e.resolution,
		e.fps ? `${e.fps} fps` : "",
		n,
		vn(e.filesize, e.filesizeApproximate)
	].filter(Boolean).join(" · ") || e.id;
}
//#endregion
//#region src/popup/popup-view-model.js
function bn(e) {
	let t = {
		preparing: "analysisPrepareBrowser",
		connectingHost: "analysisConnectHost",
		validatingHost: "analysisValidateTools",
		startingYtDlp: "analysisLaunchYtDlp",
		startingGalleryDl: "analysisLaunchGalleryDl",
		readingMedia: "analysisReadMedia",
		readingPlaylist: "analysisReadPlaylist"
	}[e.analysisPhase] || "analysisPrepareBrowser";
	return e.analysisPhase === "readingPlaylist" ? `${Q(t)} · ${Q("analysisPlaylistLimitDetail", { count: e.playlistItemLimit })}` : Q(t);
}
function xn(e) {
	let t = Object.values(e.analysisByBackend).find((e) => e?.analyzing) || e.analysisByBackend.y || e.analysisByBackend.g;
	if (!t) return "";
	if (t.backend === "g") return t.stopped ? Q("stoppedCount", { count: t.items.length }) : t.items.length ? Q("galleryMediaCount", { count: t.items.length }) : "";
	let n = t.itemLimit || e.playlistItemLimit || 32;
	return t.analyzing ? Q("analyzedCount", {
		count: t.items.length,
		limit: n
	}) : t.stopped ? Q("stoppedCount", { count: t.items.length }) : t.truncated ? Q("showFirstItems", { count: n }) : t.items.length > 1 ? Q("mediaCountLimit", {
		count: t.items.length,
		limit: n
	}) : "";
}
function Sn(e) {
	let t = e.sourcePageSnapshot;
	if (!t) return "";
	let n = Number(t.mediaWidth), r = Number(t.mediaHeight), i = Number.isFinite(n) && n > 0 && Number.isFinite(r) && r > 0 ? `${Math.round(n)}×${Math.round(r)}` : "";
	return [t.creator, i].filter(Boolean).join(" · ");
}
function Cn(e, t, n) {
	let r = n(e, t), i = e.formats || [], a = i.find((e) => e.id === r.videoFormatId), o = i.find((e) => e.id === r.audioFormatId), s = jt(e, r).toUpperCase();
	if (r.contentKind === "audio") return {
		stream: [
			String(o?.language || "").trim(),
			hn(o?.sampleRate),
			mn(o?.audioCodec || o?.codec)
		].filter(Boolean).join(" · ") || Q("audioSummary"),
		container: s,
		size: vn(o?.filesize, o?.filesizeApproximate)
	};
	let c = a?.kind === "muxed" ? a : o, l = [a?.resolution, mn(a?.codec)].filter(Boolean).join(" · "), u = [
		String(c?.language || "").trim(),
		hn(c?.sampleRate),
		mn(c?.audioCodec || (c?.kind === "audio" ? c?.codec : ""))
	].filter(Boolean).join(" · "), d = u ? `${l} + ${u}` : `${l} · ${Tt(e) === "absent" ? Q("noAudio") : Q("audioUnknown")}`, f = a?.kind !== "muxed" && o;
	return {
		stream: d,
		container: s,
		size: vn(a?.filesize && (!f || o?.filesize) ? a.filesize + (f ? o.filesize : 0) : 0, !!(a?.filesizeApproximate || f && o?.filesizeApproximate))
	};
}
function wn(e, t) {
	let n = e.analysis?.items?.[t];
	if (!n) return null;
	let r = e.choices[t] || {}, i = r.contentKind || "video", a = yt(n, "video"), o = i === "audio" ? yt(n, "audio") : yt(n, "audio").filter((e) => e.kind === "audio"), s = a.find((e) => e.id === r.videoFormat), c = o.find((e) => e.id === r.audioFormat), l = {
		contentKind: i,
		videoFormatId: r.videoFormat || "",
		audioFormatId: r.audioFormat || "",
		outputFormat: r.outputFormat || ""
	}, u = ut(r), d = pt(n, r);
	return {
		item: n,
		choice: r,
		contentKinds: xt(n),
		videoFormats: a,
		audioFormats: o,
		selectedVideo: s,
		selectedAudio: c,
		showAudioFormat: Et(i, o, s),
		outputs: At(n, l),
		subtitles: St(n),
		timeRange: u,
		timeRangeSummary: u ? `${ft(u.startSeconds)}–${ft(u.endSeconds)}` : Q("fullVideo"),
		chapterIndices: d,
		chapterRangeSummary: d.length ? Q("selectedChapters", { count: d.length }) : Q("fullVideo"),
		videoSummary: yn(s, !1),
		audioSummary: yn(c, !0)
	};
}
function Tn(e, t, n) {
	let r = wn(e, t);
	if (!r) return [];
	let i = n(r.item, r.choice), a = e.analysis?.host, o = [], s = (e, t) => Q(`${e}${t === "embed" ? "Embed" : t === "download" ? "Download" : "Off"}`);
	if (["download", "embed"].includes(i.thumbnailMode) && we(a, "postprocess.thumbnail-mode.v1") && o.push({
		label: Q("thumbnailMode"),
		value: s("thumbnail", i.thumbnailMode)
	}), i.contentKind === "video" && r.subtitles.length && ["download", "embed"].includes(i.subtitleMode)) {
		let e = r.subtitles.find((e) => e.language === i.subtitleLanguage);
		o.push({
			label: Q("subtitles"),
			value: `${e?.label || i.subtitleLanguage} · ${s("subtitle", i.subtitleMode)}`
		});
	}
	return r.item.chapterCount > 0 && ["download", "embed"].includes(i.chapterMode) && we(a, "postprocess.chapter-mode.v1") && o.push({
		label: Q("chapterMode"),
		value: s("chapter", i.chapterMode)
	}), (i.chapterIndices?.length || r.timeRange) && o.push({
		label: Q("downloadRange"),
		value: i.chapterIndices?.length ? r.chapterRangeSummary : r.timeRangeSummary
	}), o;
}
function En(e, t, n = `y:${t}`) {
	let r = e.browserDownloadTask, i = n.startsWith("g:") && r && (n === r.currentSourceItemKey || r.completedSourceItemKeys?.includes(n)) ? r : e.currentTask;
	if (!(Number.isInteger(i.sourceTabId) ? i.sourceTabId === e.sourceTabId : i.sourcePageUrl === e.sourceUrl) || i.sourceAnalysisStartedAt && i.sourceAnalysisStartedAt !== e.unifiedAnalysis?.request?.startedAt) return null;
	let a = ["running", "stopping"].includes(i.state);
	if (new Set(i.completedSourceItemKeys || []).has(n)) return {
		state: "completed",
		percent: 100,
		label: Q("completed")
	};
	if (n === i.currentSourceItemKey && (a || ["failed", "stopped"].includes(i.state))) {
		let e = Math.max(0, Math.min(100, Number(i.itemPercent) || 0)), t = i.state === "failed" ? Q("downloadFailed") : i.state === "stopped" ? Q("downloadStopped") : i.state === "stopping" ? Q("stopping") : [
			`${e.toFixed(0)}%`,
			i.speed,
			i.eta ? Q("remaining", { eta: i.eta }) : ""
		].filter(Boolean).join(" · ");
		return {
			state: i.state,
			percent: e,
			label: t
		};
	}
	return null;
}
//#endregion
//#region src/popup/components/SourcePreview.svelte
var Dn = u("<img class=\"blur-cover svelte-147hc70\" alt=\"\" aria-hidden=\"true\"/>"), On = u("<!> <img class=\"foreground svelte-147hc70\" alt=\"\"/>", 1), kn = u("<img class=\"fallback-icon svelte-147hc70\" alt=\"\"/>"), An = u("<span class=\"fallback-glyph svelte-147hc70\" aria-hidden=\"true\">▧</span>"), jn = u("<div><!></div>");
function Mn(t, n) {
	y(n, !1);
	let r = I(), i = o(n, "sources", 24, () => []), a = o(n, "siteIcon", 8, ""), s = I(0), c = I(!1), l = I(!1), u = I("");
	function p() {
		W(l, !1), W(s, H(s) + 1);
	}
	A(() => K(i()), () => {
		W(r, [...new Set(i().filter((e) => typeof e == "string" && e))]);
	}), A(() => (H(r), H(u)), () => {
		H(r).join("\n") !== H(u) && (W(u, H(r).join("\n")), W(s, 0), W(l, !1), W(c, !1));
	}), de(), h();
	var m = jn();
	let g;
	var _ = N(m), v = (t) => {
		var n = On(), i = B(n), a = (e) => {
			var t = Dn();
			G(() => z(t, "src", (H(r), H(s), R(() => H(r)[H(s)])))), d(e, t);
		};
		x(i, (e) => {
			H(l) && e(a);
		});
		var o = F(i, 2);
		G(() => z(o, "src", (H(r), H(s), R(() => H(r)[H(s)])))), e("load", o, () => {
			W(l, !0);
		}), e("error", o, p), te(o), d(t, n);
	}, b = (t) => {
		var n = kn();
		G(() => z(n, "src", a())), e("error", n, () => {
			W(c, !0);
		}), te(n), d(t, n);
	}, S = (e) => {
		var t = An();
		d(e, t);
	};
	x(_, (e) => {
		H(s), H(r), R(() => H(s) < H(r).length) ? e(v) : a() && !H(c) ? e(b, 1) : e(S, -1);
	}), J(m), G(() => g = f(m, 1, "source-cover svelte-147hc70", null, g, { loaded: H(l) })), d(t, m), Y();
}
//#endregion
//#region ../assets/icons/Check.svg?raw
var Nn = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" width=\"24\" height=\"24\" role=\"img\" aria-label=\"Check\">\n  <path d=\"M9,16.17L4.83,12L3.41,13.41L9,19L21,7L19.59,5.59L9,16.17Z\" fill=\"currentColor\"/>\n</svg>\n", Pn = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" width=\"24\" height=\"24\" role=\"img\" aria-label=\"Close\">\n  <path d=\"M19,6.41L17.59,5L12,10.59L6.41,5L5,6.41L10.59,12L5,17.59L6.41,19L12,13.41L17.59,19L19,17.59L13.41,12L19,6.41Z\" fill=\"currentColor\"/>\n</svg>\n", Fn = T("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\"M14 3h7v7M21 3L10 14M10 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-5\"></path></svg>"), In = T("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path fill=\"currentColor\" d=\"M4 11h12.17l-5.59-5.59L12 4l8 8-8 8-1.41-1.41L16.17 13H4z\"></path></svg>"), Ln = u("<span class=\"action-icon svelte-jns3ii\" aria-hidden=\"true\"><!></span>");
function Rn(e, t) {
	y(t, !1);
	let r = o(t, "name", 8, "next");
	h();
	var i = Ln(), a = N(i), s = (e) => {
		var t = Fn();
		d(e, t);
	}, c = (e) => {
		var t = fe(), i = B(t);
		n(i, () => (K(r()), K(Nn), K(Pn), R(() => (r() === "check" ? Nn : Pn).replace(/<\?xml[^?]*\?>\s*/, "")))), d(e, t);
	}, l = (e) => {
		var t = In();
		d(e, t);
	};
	x(a, (e) => {
		r() === "external" ? e(s) : r() === "check" || r() === "close" ? e(c, 1) : e(l, -1);
	}), J(i), d(e, i), Y();
}
//#endregion
//#region src/popup/components/ResultCard.svelte
var zn = u("<label class=\"result-select-toggle\"><input type=\"checkbox\"/> <!></label>"), Bn = u("<span class=\"cover-status\">✓</span>"), Vn = u("<span class=\"cover-count\"> </span>"), Hn = u("<span class=\"result-title-icon\"><img class=\"site-icon\" alt=\"\"/></span>"), Un = u("<span class=\"result-format-summary\"> </span>"), Wn = u("<span class=\"result-date\"> </span><span class=\"result-format-summary\"> </span> <!>", 1), Gn = u("<div class=\"result-media-row\"><span class=\"result-media-label\"> </span><span> </span><span class=\"result-output\"> </span></div>"), Kn = u("<!> <!>", 1), qn = u("<span><!><span class=\"result-title-text\"> </span></span> <div><!></div>", 1), Jn = u("<div class=\"result-album-actions\"><button class=\"result-album-button\"><span> </span><!></button></div>"), Yn = u("<div><dt> </dt><dd> </dd></div>"), Xn = u("<dl class=\"result-options\"></dl>"), Zn = u("<progress max=\"100\"></progress>"), Qn = u("<div class=\"result-task\" role=\"status\"> </div> <!>", 1), $n = u("<article class=\"result-card\"><div class=\"result-cover\"><div class=\"result-preview\"><!> <!> <!> <!></div> <span> </span></div> <div class=\"result-content\"><!> <!> <!> <!></div></article>");
function er(e, n) {
	y(n, !1);
	let r = I(), i = I(), a = I(), s = I(), u = I(), g = I(), v = I(), b = I(), C = I(), w = I(), T = I(), E = I(), D = I(), O = I(), k = I(), j = I(), ee = I(), M = o(n, "result", 8), P = o(n, "state", 8), te = o(n, "count", 8);
	function ne(e) {
		H(i) ? $.toggleGalleryGroup(M().index, e.currentTarget.checked) : $.toggleSelection(M().index, e.currentTarget.checked);
	}
	function re() {
		H(i) || $.prepareDetail(M().index);
	}
	A(() => K(M()), () => {
		W(r, M().item);
	}), A(() => K(M()), () => {
		W(i, M().backend === "g");
	}), A(() => K(P()), () => {
		W(a, P().busy || P().analysisRunning || P().downloadRunning);
	}), A(() => (K(P()), K(M())), () => {
		W(s, En(P(), M().index, M().key));
	}), A(() => (H(i), H(r), K(P()), K(M()), $), () => {
		W(u, H(i) ? null : Cn(H(r), P().choices[M().index] || {}, $.storedDownloadChoice));
	}), A(() => (H(i), H(r), K(P()), K(M())), () => {
		W(g, H(i) ? null : $.storedDownloadChoice(H(r), P().choices[M().index] || {}));
	}), A(() => (H(i), K(P()), K(M()), $), () => {
		W(v, H(i) ? [] : Tn(P(), M().index, $.storedDownloadChoice));
	}), A(() => (K(P()), K(M()), K(te())), () => {
		W(b, Xt(P(), M(), te()));
	}), A(() => (H(r), H(g)), () => {
		W(C, H(r).formats?.find((e) => e.id === H(g)?.videoFormatId));
	}), A(() => (H(r), H(g)), () => {
		W(w, H(r).formats?.find((e) => e.id === H(g)?.audioFormatId));
	}), A(() => (H(i), H(g), H(r), H(C)), () => {
		W(T, !H(i) && Et(H(g).contentKind, yt(H(r), "audio").filter((e) => e.kind === "audio"), H(C)));
	}), A(() => (H(i), H(T), H(g)), () => {
		W(E, !H(i) && (H(T) || H(g).contentKind === "audio"));
	}), A(() => (H(i), K(M())), () => {
		W(D, H(i) ? `/gallery/${M().index}/` : `/item/${M().index}/`);
	}), A(() => (H(i), H(r), K(P())), () => {
		W(O, H(i) ? $.galleryModeForGroup(H(r), P()) : "");
	}), A(() => (H(i), H(O)), () => {
		W(k, H(i) ? Q(tt(H(O))) : "");
	}), A(() => (H(i), H(r), K(P())), () => {
		W(j, H(i) ? H(r).canonicalTitle || [P().sourceSiteName, H(r).creator && `@${H(r).creator.replace(/^@/, "")}`].filter(Boolean).join(" · ") || H(r).title : H(r).title || H(r).url);
	}), A(() => H(r), () => {
		W(ee, String(H(r).date || "").replace("T", " ").replace(/:\d\d(?:\.\d+)?(?:Z|[+-]\d\d:\d\d)?$/, ""));
	}), de(), h();
	var ie = $n(), ae = N(ie), oe = N(ae), se = N(oe);
	{
		let e = V(() => (H(r), R(() => [H(r).thumbnail].filter(Boolean))));
		Mn(se, {
			get sources() {
				return H(e);
			},
			get siteIcon() {
				return K(P()), R(() => P().sourceSiteIconUrl);
			}
		});
	}
	var ce = F(se, 2), le = (e) => {
		var n = zn(), r = N(n);
		t(r);
		var i = F(r, 2), o = (e) => {
			Rn(e, { name: "check" });
		};
		x(i, (e) => {
			H(b) && e(o);
		}), J(n), G(() => {
			z(r, "aria-label", H(j)), p(r, H(b)), r.disabled = H(a);
		}), _("change", r, ne), d(e, n);
	};
	x(ce, (e) => {
		te() > 1 && e(le);
	});
	var ue = F(ce, 2), U = (e) => {
		var t = Bn();
		G((e) => z(t, "aria-label", e), [() => (K(Q), R(() => Q("completed")))]), d(e, t);
	};
	x(ue, (e) => {
		H(s), R(() => H(s)?.state === "completed") && e(U);
	});
	var fe = F(ue, 2), pe = (e) => {
		var t = Vn(), n = q(t, !0);
		G((e) => c(n, e), [() => (H(i), K(Q), H(r), K(gn), R(() => H(i) ? Q("galleryMediaCount", { count: H(r).media.length }) : gn(H(r).duration)))]), d(e, t);
	};
	x(fe, (e) => {
		H(i), H(r), R(() => H(i) || H(r).duration) && e(pe);
	}), J(oe);
	var me = F(oe, 2);
	let he;
	var ge = q(me, !0);
	J(ae);
	var _e = F(ae, 2), ve = N(_e);
	{
		let e = V(() => H(a) ? !0 : void 0), t = V(() => H(a) ? -1 : void 0), n = V(() => (K(Q), H(j), R(() => `${Q("downloadContent")} · ${H(j)}`)));
		L(ve, {
			get href() {
				return H(D);
			},
			class: "result-summary",
			get disabled() {
				return H(e);
			},
			get tabindex() {
				return H(t);
			},
			onClick: re,
			get "aria-label"() {
				return H(n);
			},
			children: (e, t) => {
				var n = qn(), a = B(n);
				let o;
				var s = N(a), l = (e) => {
					var t = Hn(), n = q(t);
					G(() => z(n, "src", (K(P()), R(() => P().sourceSiteIconUrl)))), d(e, t);
				};
				x(s, (e) => {
					K(P()), R(() => P().sourceSiteIconUrl) && e(l);
				});
				var p = F(s), m = q(p, !0);
				J(a);
				var h = F(a, 2);
				let _;
				var v = N(h), y = (e) => {
					var t = Wn(), n = B(t), i = q(n, !0), a = F(n), o = q(a, !0), s = F(a, 2), l = (e) => {
						var t = Un(), n = q(t, !0);
						G((e) => c(n, e), [() => (K(xe), K(Te), H(r), K(P()), R(() => xe(Te(H(r), P()))))]), d(e, t);
					};
					x(s, (e) => {
						H(r), R(() => H(r).sourceScope === "timeline") && e(l);
					}), G(() => {
						c(i, H(ee)), c(o, H(k));
					}), d(e, t);
				}, b = (e) => {
					var t = Kn(), n = B(t), r = (e) => {
						var t = Gn(), n = N(t), r = q(n, !0), i = F(n), a = q(i, !0), o = F(i), s = q(o, !0);
						J(t), G((e, t) => {
							c(r, e), c(a, t), c(s, (H(E), H(u), R(() => H(E) ? "" : H(u).container)));
						}, [() => (K(Q), R(() => Q("summaryVideo"))), () => (K(yn), H(C), R(() => yn(H(C), !1)))]), d(e, t);
					};
					x(n, (e) => {
						H(g), R(() => H(g).contentKind !== "audio") && e(r);
					});
					var i = F(n, 2), a = (e) => {
						var t = Gn(), n = N(t), r = q(n, !0), i = F(n), a = q(i, !0), o = F(i), s = q(o, !0);
						J(t), G((e, t) => {
							c(r, e), c(a, t), c(s, (H(u), R(() => H(u).container)));
						}, [() => (K(Q), R(() => Q("summaryAudio"))), () => (K(yn), H(w), R(() => yn(H(w), !0)))]), d(e, t);
					};
					x(i, (e) => {
						H(E) && e(a);
					}), d(e, t);
				};
				x(v, (e) => {
					H(i) ? e(y) : e(b, -1);
				}), J(h), G((e) => {
					o = f(a, 1, "result-title", null, o, { "has-site-icon": e }), c(m, H(j)), _ = f(h, 1, "result-summary-details", null, _, { "result-media-summary": !H(i) });
				}, [() => !!P().sourceSiteIconUrl]), d(e, n);
			},
			$$slots: { default: !0 }
		});
	}
	var ye = F(ve, 2), X = (e) => {
		var t = Jn(), n = N(t), r = N(n), i = q(r, !0);
		Rn(F(r), { name: "external" }), J(n), J(t), G((e) => {
			n.disabled = H(a), c(i, e);
		}, [() => (K(Q), R(() => Q("album")))]), _("click", n, () => $.openGalleryAlbum(M().index)), d(e, t);
	};
	x(ye, (e) => {
		H(i) && e(X);
	});
	var Z = F(ye, 2), be = (e) => {
		var t = Xn();
		S(t, 5, () => H(v), l, (e, t) => {
			var n = Yn(), r = N(n), i = q(r, !0), a = F(r), o = q(a, !0);
			J(n), G(() => {
				c(i, (H(t), R(() => H(t).label))), c(o, (H(t), R(() => H(t).value)));
			}), d(e, n);
		}), J(t), d(e, t);
	};
	x(Z, (e) => {
		H(v), R(() => H(v).length) && e(be);
	});
	var Se = F(Z, 2), Ce = (e) => {
		var t = Qn(), n = B(t), r = q(n, !0), a = F(n, 2), o = (e) => {
			var t = Zn();
			G(() => {
				m(t, (H(s), R(() => H(s).percent))), z(t, "aria-label", (H(s), R(() => H(s).label)));
			}), d(e, t);
		};
		x(a, (e) => {
			H(i) || e(o);
		}), G((e) => c(r, e), [() => (H(i), H(s), K(Q), R(() => H(i) && H(s).state === "running" ? Q("downloading") : H(s).label))]), d(e, t);
	};
	x(Se, (e) => {
		H(s), R(() => H(s) && H(s).state !== "completed") && e(Ce);
	}), J(_e), J(ie), G(() => {
		z(ie, "aria-label", `${H(i) ? "gallery-dl" : "yt-dlp"} · ${H(j)}`), he = f(me, 1, "backend-label", null, he, {
			"backend-gallery": H(i),
			"backend-video": !H(i)
		}), c(ge, H(i) ? "gallery-dl" : "yt-dlp");
	}), d(e, ie), Y();
}
E(["change", "click"]);
//#endregion
//#region ../assets/icons/PlayBoxOutline.svg?raw
var tr = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" role=\"img\" aria-label=\"PlayBoxOutline\"><path fill=\"currentColor\" d=\"M19,19H5V5H19M19,3H5A2,2 0 0,0 3,5V19A2,2 0 0,0 5,21H19A2,2 0 0,0 21,19V5C21,3.89 20.1,3 19,3M10,8V16L15,12L10,8Z\"/></svg>\n", nr = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" role=\"img\" aria-label=\"PlaylistPlay\"><path fill=\"currentColor\" d=\"M3 10H14V12H3V10M3 6H14V8H3V6M3 14H10V16H3V14M16 13V21L22 17L16 13Z\"/></svg>\n", rr = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" width=\"24\" height=\"24\" aria-label=\"ImageMultipleOutline\">\n  <g fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\">\n    <rect x=\"6\" y=\"3\" width=\"15\" height=\"15\" rx=\"2\"/>\n    <path d=\"M3 7v12a2 2 0 0 0 2 2h12M7 16l4-5 3 3 2-2 4 4\"/>\n    <circle cx=\"16.5\" cy=\"7.5\" r=\"1\"/>\n  </g>\n</svg>\n", ir = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" width=\"24\" height=\"24\" role=\"img\" aria-label=\"Multimedia\">\n  <path d=\"M9 13V5C9 3.9 9.9 3 11 3H20C21.1 3 22 3.9 22 5V11H18.57L17.29 9.26C17.23 9.17 17.11 9.17 17.05 9.26L15.06 12C15 12.06 14.88 12.07 14.82 12L13.39 10.25C13.33 10.18 13.22 10.18 13.16 10.25L11.05 12.91C10.97 13 11.04 13.15 11.16 13.15H17.5V15H11C9.89 15 9 14.11 9 13M6 22V21H4V22H2V2H4V3H6V2H8.39C7.54 2.74 7 3.8 7 5V13C7 15.21 8.79 17 11 17H15.7C14.67 17.83 14 19.08 14 20.5C14 21.03 14.11 21.53 14.28 22H6M4 7H6V5H4V7M4 11H6V9H4V11M4 15H6V13H4V15M6 19V17H4V19H6M23 13V15H21V20.5C21 21.88 19.88 23 18.5 23S16 21.88 16 20.5 17.12 18 18.5 18C18.86 18 19.19 18.07 19.5 18.21V13H23Z\" fill=\"currentColor\"/>\n</svg>\n";
//#endregion
//#region src/shared/source-capabilities.js
function ar(e, t) {
	let n = [], r = e?.y;
	if (r && !r.approximate && r.working !== !1) {
		let e = {
			single: "video",
			playlist: "playlist",
			any: "media"
		}[r.sourceScope];
		e && n.push({
			kind: "video",
			name: e
		});
	}
	return e?.g && t?.g === !0 && n.push({
		kind: "gallery",
		name: "gallery"
	}), n;
}
//#endregion
//#region src/popup/components/SourceCapabilities.svelte
var or = u("<span role=\"img\"></span>"), sr = u("<span class=\"source-capabilities\"></span>");
function cr(e, t) {
	y(t, !1);
	let r = I(), i = o(t, "recognized", 8), a = o(t, "detection", 8), s = {
		video: tr,
		playlist: nr,
		gallery: rr,
		media: ir
	};
	A(() => (K(i()), K(a())), () => {
		W(r, ar(i(), a()));
	}), de(), h();
	var c = sr();
	S(c, 5, () => H(r), l, (e, t) => {
		var r = or();
		n(r, () => (H(t), R(() => s[H(t).name].replace(/<\?xml[^?]*\?>\s*/, ""))), !0), J(r), G(() => {
			f(r, 1, `source-capability capability-${H(t), R(() => H(t).kind) ?? ""}`), z(r, "aria-label", (H(t), R(() => H(t).kind === "gallery" ? "gallery-dl" : "yt-dlp")));
		}), d(e, r);
	}), J(c), d(e, c), Y();
}
//#endregion
//#region src/popup/pages/HomePage.svelte
var lr = u("<img class=\"site-icon\" alt=\"\"/>"), ur = u("<span class=\"site-identity\"><!> <span class=\"site-name\"> </span></span>"), dr = u("<!> ", 1), fr = u("<!> <!>", 1), pr = u("<div class=\"analysis-error-actions\"><span> </span> <!></div> <div> </div>", 1), mr = u("<article class=\"source-card\"><!> <div class=\"source-card-content\"><div class=\"source-card-heading\"><div class=\"source-card-title\"> </div> <!></div> <span class=\"result-date\"> </span></div></article>"), hr = u("<div class=\"analysis-status\" role=\"status\" aria-live=\"polite\"><!><span> <!></span></div>"), gr = u("<div class=\"result-card-stack\"><!> <!></div>"), _r = u(" <!>", 1), vr = u("<div class=\"download-toolbar\"><!> <button class=\"download-location result-field\"> </button> <!></div>"), yr = u("<!> <!> <!> <!> <!> <!> <!> <!> <!> <!>", 1);
function br(e, t) {
	y(t, !1);
	let n = () => ve($, "$controller", r), [r, a] = O(), o = I(), s = I(), u = I(), f = I(), p = I();
	A(() => n(), () => {
		W(o, n());
	}), A(() => H(o), () => {
		W(s, Yt(H(o)));
	}), A(() => H(o), () => {
		W(u, $t(H(o)));
	}), A(() => H(o), () => {
		W(f, H(o).sourcePageSnapshot);
	}), A(() => H(o), () => {
		W(p, Ce(H(o).analysisByBackend.g?.items || [], H(o).sourceUrl));
	}), de(), h(), M(e, {
		name: "home",
		class: "home-page",
		children: (e, t) => {
			var n = yr(), r = B(n);
			se(r, {
				class: "home-navbar",
				children: (e, t) => {
					var n = fr(), r = B(n);
					ge(r, {
						children: (e, t) => {
							var n = ur(), r = N(n), i = (e) => {
								var t = lr();
								G(() => z(t, "src", (H(o), R(() => H(o).sourceSiteIconUrl)))), d(e, t);
							};
							x(r, (e) => {
								H(o), R(() => H(o).sourceSiteIconUrl) && e(i);
							});
							var a = F(r, 2), s = q(a, !0);
							J(n), G(() => c(s, (H(o), R(() => H(o).sourceSiteName || "yt-dlp-gui")))), d(e, n);
						},
						$$slots: { default: !0 }
					});
					var a = F(r, 2);
					P(a, {
						children: (e, t) => {
							var n = fr(), r = B(n);
							{
								let e = V(() => (H(o), R(() => H(o).busy || H(o).analysisRunning || H(o).downloadRunning)));
								D(r, {
									small: !0,
									href: "/settings/",
									get disabled() {
										return H(e);
									},
									children: (e, t) => {
										i();
										var n = w();
										G((e) => c(n, e), [() => (K(Q), R(() => Q("settings")))]), d(e, n);
									},
									$$slots: { default: !0 }
								});
							}
							var a = F(r, 2), s = (e) => {
								D(e, {
									small: !0,
									fill: !0,
									get disabled() {
										return H(o), R(() => H(o).busy);
									},
									onClick: () => $.stopAnalysis(),
									children: (e, t) => {
										var n = dr(), r = B(n);
										_e(r, {});
										var i = F(r);
										G((e) => c(i, ` ${e ?? ""}`), [() => (K(Q), R(() => Q("stop")))]), d(e, n);
									},
									$$slots: { default: !0 }
								});
							}, l = (e) => {
								{
									let t = V(() => (H(o), R(() => !H(o).preferencesLoaded || H(o).toolDetectionRunning || H(o).toolPreferenceSaving || !H(o).analysisTools.y && !H(o).analysisTools.g || !H(o).recognized || H(o).recognitionPending || H(o).busy || H(o).downloadRunning)));
									D(e, {
										small: !0,
										fill: !0,
										get disabled() {
											return H(t);
										},
										onClick: () => $.analyze(),
										children: (e, t) => {
											i();
											var n = w();
											G((e) => c(n, e), [() => (K(Q), R(() => Q("analyze")))]), d(e, n);
										},
										$$slots: { default: !0 }
									});
								}
							};
							x(a, (e) => {
								H(o), R(() => H(o).analysisRunning) ? e(s) : e(l, -1);
							}), d(e, n);
						},
						$$slots: { default: !0 }
					}), d(e, n);
				},
				$$slots: { default: !0 }
			});
			var a = F(r, 2), m = (e) => {
				X(e, {
					inset: !0,
					children: (e, t) => {
						i();
						var n = w();
						G((e) => c(n, e), [() => (K(Q), R(() => Q("selectAnalysisTool")))]), d(e, n);
					},
					$$slots: { default: !0 }
				});
			};
			x(a, (e) => {
				H(o), R(() => H(o).preferencesLoaded && !H(o).analysisTools.y && !H(o).analysisTools.g) && e(m);
			});
			var h = F(a, 2);
			S(h, 0, () => ["y", "g"], l, (e, t) => {
				var n = fe(), r = B(n), a = (e) => {
					X(e, {
						strong: !0,
						inset: !0,
						children: (e, n) => {
							var r = pr(), a = B(r), s = N(a), l = q(s, !0), u = F(s, 2);
							{
								let e = V(() => (H(o), R(() => H(o).busy || H(o).analysisRunning || H(o).downloadRunning || H(o).recognitionPending)));
								D(u, {
									small: !0,
									outline: !0,
									get disabled() {
										return H(e);
									},
									onClick: () => $.retryAnalysis(t),
									children: (e, t) => {
										i();
										var n = w();
										G((e) => c(n, e), [() => (K(Q), R(() => Q("retryAnalysis")))]), d(e, n);
									},
									$$slots: { default: !0 }
								});
							}
							J(a);
							var f = F(a, 2), p = q(f, !0);
							G((e) => {
								c(l, t === "y" ? "yt-dlp" : "gallery-dl"), c(p, e);
							}, [() => (H(o), K(Q), R(() => t === "g" && H(o).analysisByBackend.g?.items?.length ? Q("galleryAnalysisWarning", { message: H(o).analysisErrors[t] }) : H(o).analysisErrors[t]))]), d(e, r);
						},
						$$slots: { default: !0 }
					});
				};
				x(r, (e) => {
					H(o), R(() => H(o).analysisErrors[t]) && e(a);
				}), d(e, n);
			});
			var g = F(h, 2), v = (e) => {
				X(e, {
					inset: !0,
					role: "alert",
					children: (e, t) => {
						i();
						var n = w();
						G(() => c(n, (H(o), R(() => H(o).recognitionText)))), d(e, n);
					},
					$$slots: { default: !0 }
				});
			};
			x(g, (e) => {
				H(o), R(() => !H(o).recognitionPending && !H(o).recognized) && e(v);
			});
			var y = F(g, 2), b = (e) => {
				var t = mr(), n = N(t);
				{
					let e = V(() => (H(f), R(() => H(f)?.thumbnailCandidates?.length ? H(f).thumbnailCandidates : [H(f)?.thumbnailUrl])));
					Mn(n, {
						get sources() {
							return H(e);
						},
						get siteIcon() {
							return H(o), R(() => H(o).sourceSiteIconUrl);
						}
					});
				}
				var r = F(n, 2), i = N(r), a = N(i), s = q(a, !0);
				cr(F(a, 2), {
					get recognized() {
						return H(o), R(() => H(o).recognized);
					},
					get detection() {
						return H(o), R(() => H(o).toolDetection);
					}
				}), J(i);
				var l = F(i, 2), u = q(l, !0);
				J(r), J(t), G((e) => {
					c(s, (H(f), H(o), R(() => H(f)?.title || H(o).sourceSiteName))), c(u, e);
				}, [() => (K(Sn), H(o), R(() => Sn(H(o))))]), d(e, t);
			};
			x(y, (e) => {
				H(s), H(o), H(f), R(() => !H(s).length && (H(o).recognized || H(f)?.title || H(f)?.thumbnailUrl || H(f)?.thumbnailCandidates?.length)) && e(b);
			});
			var C = F(y, 2), T = (e) => {
				X(e, {
					inset: !0,
					children: (e, t) => {
						me(e, {
							strong: !0,
							children: (e, t) => {
								var n = fr(), r = B(n);
								{
									let e = V(() => (H(o), R(() => H(o).playlistScope === "current")));
									D(r, {
										get active() {
											return H(e);
										},
										onClick: () => $.setPlaylistScope("current"),
										children: (e, t) => {
											i();
											var n = w();
											G((e) => c(n, e), [() => (K(Q), R(() => Q("currentVideo")))]), d(e, n);
										},
										$$slots: { default: !0 }
									});
								}
								var a = F(r, 2);
								{
									let e = V(() => (H(o), R(() => H(o).playlistScope === "playlist")));
									D(a, {
										get active() {
											return H(e);
										},
										onClick: () => $.setPlaylistScope("playlist"),
										children: (e, t) => {
											i();
											var n = w();
											G((e) => c(n, e), [() => (K(Q), R(() => Q("fullPlaylist")))]), d(e, n);
										},
										$$slots: { default: !0 }
									});
								}
								d(e, n);
							},
							$$slots: { default: !0 }
						});
					},
					$$slots: { default: !0 }
				});
			};
			x(C, (e) => {
				H(o), R(() => H(o).playlistSelectable) && e(T);
			});
			var E = F(C, 2), O = (e) => {
				X(e, {
					inset: !0,
					children: (e, t) => {
						{
							let t = V(() => (H(o), R(() => H(o).busy || H(o).analysisRunning)));
							D(e, {
								outline: !0,
								get disabled() {
									return H(t);
								},
								onClick: () => $.openGalleryAlbumAll(),
								children: (e, t) => {
									i();
									var n = w();
									G((e) => c(n, e), [() => (K(Q), R(() => Q("openAllPostsInNewTab")))]), d(e, n);
								},
								$$slots: { default: !0 }
							});
						}
					},
					$$slots: { default: !0 }
				});
			};
			x(E, (e) => {
				H(p), R(() => H(p).length > 1) && e(O);
			});
			var k = F(E, 2), A = (e) => {
				var t = gr(), n = N(t);
				S(n, 1, () => H(s), (e) => e.key, (e, t) => {
					er(e, {
						get result() {
							return H(t);
						},
						get state() {
							return H(o);
						},
						get count() {
							return H(s), R(() => H(s).length);
						}
					});
				});
				var r = F(n, 2), i = (e) => {
					var t = hr(), n = N(t);
					_e(n, {});
					var r = F(n), i = N(r, !0), a = F(i), s = (e) => {
						var t = w();
						G((e) => c(t, `· ${e ?? ""}`), [() => (K(xn), H(o), R(() => xn(H(o))))]), d(e, t);
					}, l = ae(() => (K(xn), H(o), R(() => xn(H(o)))));
					x(a, (e) => {
						H(l) && e(s);
					}), J(r), J(t), G((e) => c(i, e), [() => (K(bn), H(o), R(() => bn(H(o))))]), d(e, t);
				};
				x(r, (e) => {
					H(o), R(() => H(o).analysisRunning) && e(i);
				}), J(t), G(() => z(t, "aria-busy", (H(o), R(() => H(o).analysisRunning)))), d(e, t);
			};
			x(k, (e) => {
				H(s), H(o), R(() => H(s).length || H(o).analysisRunning) && e(A);
			});
			var j = F(k, 2), ee = (e) => {
				X(e, {
					strong: !0,
					inset: !0,
					role: "status",
					children: (e, t) => {
						i();
						var n = _r(), r = B(n), a = F(r), s = (e) => {
							D(e, {
								get href() {
									return H(o), R(() => H(o).helpUrl);
								},
								target: "_blank",
								external: !0,
								children: (e, t) => {
									i();
									var n = w();
									G((e) => c(n, e), [() => (K(Q), R(() => Q("nativeHostGitHub")))]), d(e, n);
								},
								$$slots: { default: !0 }
							});
						};
						x(a, (e) => {
							H(o), R(() => H(o).helpUrl) && e(s);
						}), G(() => c(r, `${H(o), R(() => H(o).messageText) ?? ""} `)), d(e, n);
					},
					$$slots: { default: !0 }
				});
			}, M = ae(() => (H(o), R(() => H(o).messageText && (H(o).messageError || ![
				"running",
				"stopping",
				"completed"
			].includes(H(o).currentTask?.state)))));
			x(j, (e) => {
				H(M) && e(ee);
			});
			var te = F(j, 2);
			ue(te, {
				bottom: !0,
				class: "download-toolbar-shell",
				children: (e, t) => {
					var n = vr(), r = N(n);
					{
						let e = V(() => (H(o), H(u), R(() => H(o).busy || H(o).analysisRunning || H(o).downloadRunning || H(u).browserOnly)));
						D(r, {
							small: !0,
							fill: !0,
							get disabled() {
								return H(e);
							},
							onClick: () => $.chooseDirectory(),
							children: (e, t) => {
								i();
								var n = w();
								G((e) => c(n, e), [() => (K(Q), R(() => Q("chooseFolder")))]), d(e, n);
							},
							$$slots: { default: !0 }
						});
					}
					var a = F(r, 2), s = q(a, !0), l = F(a, 2), f = (e) => {
						{
							let t = V(() => (H(o), R(() => H(o).busy || H(o).currentTask?.state === "stopping")));
							D(e, {
								small: !0,
								fill: !0,
								get disabled() {
									return H(t);
								},
								onClick: () => $.stopDownload(),
								children: (e, t) => {
									i();
									var n = w();
									G((e) => c(n, e), [() => (H(o), K(Q), R(() => H(o).currentTask?.state === "stopping" ? Q("stopping") : Q("stop")))]), d(e, n);
								},
								$$slots: { default: !0 }
							});
						}
					}, p = (e) => {
						D(e, {
							small: !0,
							fill: !0,
							class: "primary-action",
							get disabled() {
								return H(u), R(() => H(u).disabled);
							},
							onClick: () => $.downloadSelected(),
							children: (e, t) => {
								i();
								var n = w();
								G((e) => c(n, e), [() => (K(Q), R(() => Q("download")))]), d(e, n);
							},
							$$slots: { default: !0 }
						});
					};
					x(l, (e) => {
						H(o), R(() => H(o).downloadRunning) ? e(f) : e(p, -1);
					}), J(n), G((e, t) => {
						a.disabled = (H(o), H(u), R(() => H(o).busy || H(o).analysisRunning || H(o).downloadRunning || H(u).browserOnly || !H(o).outputDirectory)), z(a, "aria-label", e), c(s, t);
					}, [() => (K(Q), R(() => Q("openDownloadFolder"))), () => (H(u), K(Q), H(o), R(() => H(u).browserOnly ? Q("browserDownloadDestination") : H(o).outputDirectory || Q("downloadLocation")))]), _("click", a, () => $.openDirectory()), d(e, n);
				},
				$$slots: { default: !0 }
			}), d(e, n);
		},
		$$slots: { default: !0 }
	}), Y(), a();
}
E(["click"]);
//#endregion
//#region src/popup/pages/DetailPage.svelte
var xr = u("<!> <!>", 1), Sr = u("<div class=\"file-name-value\"><div class=\"file-name-editor\"><textarea rows=\"3\" wrap=\"soft\" maxlength=\"200\" spellcheck=\"false\"></textarea> <button type=\"button\" class=\"file-name-clear\"><!></button></div> <span class=\"file-name-hint\"> </span></div>"), Cr = u("<span class=\"format-choice-summary\"> </span>"), wr = u("<option> </option>"), Tr = u("<select></select>"), Er = u("<select><option> </option><option> </option><option> </option></select>"), Dr = u("<select><option> </option><!></select>"), Or = u("<!> <!> <!> <!> <!> <!> <!> <!> <!>", 1), kr = u("<article class=\"source-card\"><!> <div class=\"source-card-content\"><div class=\"source-card-title\"> </div><span class=\"result-date\"> </span></div></article> <!> <!>", 1);
function Ar(e, t) {
	y(t, !1);
	let n = () => ve($, "$controller", r), [r, a] = O(), u = I(), f = I(), p = I(), v = I(), b = I(), C = I(), T = o(t, "itemIndex", 8), E = I();
	function k(e) {
		$.setChoice(H(u), { contentKind: e });
	}
	function j(e) {
		let t = e.currentTarget.value.replace(/[\r\n]+/g, " ");
		t !== e.currentTarget.value && (e.currentTarget.value = t), $.setChoice(H(u), { fileName: t }, !1);
	}
	function ee(e) {
		e.key === "Enter" && !e.isComposing && e.preventDefault();
	}
	function P() {
		$.setChoice(H(u), { fileName: "" }, !1), H(E)?.focus();
	}
	function te(e) {
		$.setChoice(H(u), { outputFormat: e.currentTarget.value });
	}
	function L(e) {
		$.setChoice(H(u), { thumbnailMode: e.currentTarget.value });
	}
	function ae(e) {
		let t = e.currentTarget.value, n = H(p)?.subtitles.find((e) => e.language === t), r = H(b) && ["download", "embed"].includes(H(f).analysis?.subtitleMode) ? H(f).analysis.subtitleMode : "download", i = ["download", "embed"].includes(H(p)?.choice.subtitleMode) ? H(p).choice.subtitleMode : r;
		$.setChoice(H(u), {
			subtitleLanguage: t,
			subtitleAutomatic: !!n?.automatic,
			subtitleMode: t ? i : "off"
		}, !1);
	}
	function ce(e) {
		let t = e.currentTarget.value;
		$.setChoice(H(u), {
			subtitleMode: t,
			...t === "off" ? {
				subtitleLanguage: "",
				subtitleAutomatic: !1
			} : {}
		}, !1);
	}
	function le(e) {
		let t = e.currentTarget.checked;
		$.setChoice(H(u), { chapterMode: t ? "download" : "off" });
	}
	function ue(e) {
		$.setChoice(H(u), { chapterMode: e.currentTarget.value });
	}
	A(() => K(T()), () => {
		W(u, Number(T()));
	}), A(() => n(), () => {
		W(f, n());
	}), A(() => (H(f), H(u)), () => {
		W(p, wn(H(f), H(u)));
	}), A(() => H(f), () => {
		W(v, we(H(f).analysis?.host, "postprocess.thumbnail-mode.v1"));
	}), A(() => H(f), () => {
		W(b, we(H(f).analysis?.host, "postprocess.subtitle-mode.v1"));
	}), A(() => H(f), () => {
		W(C, we(H(f).analysis?.host, "postprocess.chapter-mode.v1"));
	}), de(), h(), M(e, {
		name: "detail",
		children: (e, t) => {
			var n = xr(), r = B(n);
			{
				let e = V(() => (K(Q), R(() => Q("downloadContent"))));
				se(r, {
					get title() {
						return H(e);
					},
					backLink: !0
				});
			}
			var a = F(r, 2), o = (e) => {
				var t = kr(), n = B(t), r = N(n);
				{
					let e = V(() => (H(p), R(() => [H(p).item.thumbnail].filter(Boolean))));
					Mn(r, {
						get sources() {
							return H(e);
						},
						get siteIcon() {
							return H(f), R(() => H(f).sourceSiteIconUrl);
						}
					});
				}
				var a = F(r, 2), o = N(a), h = q(o, !0), y = F(o), T = q(y, !0);
				J(a), J(n);
				var O = F(n, 2), A = (e) => {
					X(e, {
						inset: !0,
						children: (e, t) => {
							me(e, {
								strong: !0,
								children: (e, t) => {
									var n = xr(), r = B(n);
									{
										let e = V(() => (H(p), R(() => H(p).choice.contentKind === "video")));
										D(r, {
											get active() {
												return H(e);
											},
											onClick: () => k("video"),
											children: (e, t) => {
												i();
												var n = w();
												G((e) => c(n, e), [() => (K(Q), R(() => Q("video")))]), d(e, n);
											},
											$$slots: { default: !0 }
										});
									}
									var a = F(r, 2);
									{
										let e = V(() => (H(p), R(() => H(p).choice.contentKind === "audio")));
										D(a, {
											get active() {
												return H(e);
											},
											onClick: () => k("audio"),
											children: (e, t) => {
												i();
												var n = w();
												G((e) => c(n, e), [() => (K(Q), R(() => Q("audioOnly")))]), d(e, n);
											},
											$$slots: { default: !0 }
										});
									}
									d(e, n);
								},
								$$slots: { default: !0 }
							});
						},
						$$slots: { default: !0 }
					});
				};
				x(O, (e) => {
					H(p), R(() => H(p).contentKinds.length > 1) && e(A);
				});
				var M = F(O, 2);
				ne(M, {
					strong: !0,
					inset: !0,
					class: "option-list",
					children: (e, t) => {
						var n = Or(), r = B(n);
						{
							let e = (e) => {
								var t = Sr(), n = N(t), r = N(n);
								ie(r), oe(r, (e) => W(E, e), () => H(E));
								var i = F(r, 2);
								Rn(N(i), { name: "close" }), J(i), J(n);
								var a = F(n, 2), o = q(a, !0);
								J(t), G((e, t, n) => {
									z(r, "id", `file-name-${H(u)}`), z(r, "aria-label", e), z(r, "aria-describedby", `file-name-hint-${H(u)}`), m(r, (H(p), R(() => H(p).choice.fileName))), z(i, "aria-label", t), z(i, "aria-controls", `file-name-${H(u)}`), i.disabled = (H(p), R(() => !H(p).choice.fileName)), z(a, "id", `file-name-hint-${H(u)}`), c(o, n);
								}, [
									() => (K(Q), R(() => Q("fileName"))),
									() => (K(Q), R(() => Q("clearFileName"))),
									() => (K(Q), R(() => Q("fileNameBaseHint")))
								]), _("input", r, j), _("keydown", r, ee), _("click", i, P), d(e, t);
							}, t = V(() => (K(Q), R(() => Q("fileName"))));
							U(r, {
								class: "file-name-option",
								get title() {
									return H(t);
								},
								after: e,
								$$slots: { after: !0 }
							});
						}
						var i = F(r, 2), a = (e) => {
							{
								let t = (e) => {
									var t = Cr(), n = q(t, !0);
									G(() => c(n, (H(p), R(() => H(p).videoSummary)))), d(e, t);
								}, n = V(() => (K(Q), R(() => Q("videoFormat")))), r = V(() => `/item/${H(u)}/format/video/`);
								U(e, {
									get title() {
										return H(n);
									},
									get link() {
										return H(r);
									},
									after: t,
									$$slots: { after: !0 }
								});
							}
						};
						x(i, (e) => {
							H(p), R(() => H(p).choice.contentKind === "video") && e(a);
						});
						var o = F(i, 2), f = (e) => {
							{
								let t = (e) => {
									var t = Cr(), n = q(t, !0);
									G(() => c(n, (H(p), R(() => H(p).audioSummary)))), d(e, t);
								}, n = V(() => (K(Q), R(() => Q("audioFormat")))), r = V(() => `/item/${H(u)}/format/audio/`);
								U(e, {
									get title() {
										return H(n);
									},
									get link() {
										return H(r);
									},
									after: t,
									$$slots: { after: !0 }
								});
							}
						};
						x(o, (e) => {
							H(p), R(() => H(p).choice.contentKind === "audio" || H(p).showAudioFormat) && e(f);
						});
						var h = F(o, 2);
						{
							let e = V(() => (K(Q), R(() => Q("output")))), t = V(() => (K(Q), R(() => ({
								openIn: "page",
								pageTitle: Q("output"),
								searchbar: !1,
								scrollToSelectedItem: !0
							}))));
							U(h, {
								get title() {
									return H(e);
								},
								smartSelect: !0,
								get smartSelectParams() {
									return H(t);
								},
								children: (e, t) => {
									var n = Tr();
									S(n, 5, () => (H(p), R(() => H(p).outputs)), l, (e, t) => {
										var n = wr(), r = q(n, !0), i = {};
										G((e) => {
											c(r, e), i !== (i = H(t)) && (n.value = (n.__value = i) ?? "");
										}, [() => (H(t), R(() => H(t).toUpperCase()))]), d(e, n);
									}), J(n);
									var r;
									g(n), G(() => {
										z(n, "name", `output-format-${H(u)}`), r !== (r = (H(p), R(() => H(p).choice.outputFormat))) && (n.value = (n.__value = r) ?? "", s(n, r));
									}), _("change", n, te), d(e, n);
								},
								$$slots: { default: !0 }
							});
						}
						var y = F(h, 2), w = (e) => {
							{
								let t = V(() => (K(Q), R(() => Q("thumbnailMode")))), n = V(() => (K(Q), R(() => ({
									openIn: "page",
									pageTitle: Q("thumbnailMode"),
									searchbar: !1,
									scrollToSelectedItem: !0
								}))));
								U(e, {
									get title() {
										return H(t);
									},
									smartSelect: !0,
									get smartSelectParams() {
										return H(n);
									},
									children: (e, t) => {
										var n = Er(), r = N(n), i = q(r, !0);
										r.value = r.__value = "off";
										var a = F(r), o = q(a, !0);
										a.value = a.__value = "download";
										var l = F(a), f = q(l, !0);
										l.value = l.__value = "embed", J(n);
										var m;
										g(n), G((e, t, r) => {
											z(n, "name", `thumbnail-mode-${H(u)}`), c(i, e), c(o, t), c(f, r), m !== (m = (H(p), R(() => H(p).choice.thumbnailMode))) && (n.value = (n.__value = m) ?? "", s(n, m));
										}, [
											() => (K(Q), R(() => Q("thumbnailOff"))),
											() => (K(Q), R(() => Q("thumbnailDownload"))),
											() => (K(Q), R(() => Q("thumbnailEmbed")))
										]), _("change", n, L), d(e, n);
									},
									$$slots: { default: !0 }
								});
							}
						};
						x(y, (e) => {
							H(v) && e(w);
						});
						var T = F(y, 2), D = (e) => {
							var t = xr(), n = B(t);
							{
								let e = V(() => (K(Q), R(() => Q("subtitles")))), t = V(() => (K(Q), R(() => ({
									openIn: "page",
									pageTitle: Q("subtitles"),
									searchbar: !1,
									scrollToSelectedItem: !0
								}))));
								U(n, {
									get title() {
										return H(e);
									},
									smartSelect: !0,
									get smartSelectParams() {
										return H(t);
									},
									children: (e, t) => {
										var n = Dr(), r = N(n), i = q(r, !0);
										r.value = r.__value = "";
										var a = F(r);
										S(a, 1, () => (H(p), R(() => H(p).subtitles)), l, (e, t) => {
											var n = wr(), r = q(n), i = {};
											G((e) => {
												c(r, `${H(t), R(() => H(t).label || H(t).language) ?? ""}${e ?? ""}`), i !== (i = (H(t), R(() => H(t).language))) && (n.value = (n.__value = i) ?? "");
											}, [() => (H(t), K(Q), R(() => H(t).automatic ? ` · ${Q("automatic")}` : ""))]), d(e, n);
										}), J(n);
										var o;
										g(n), G((e) => {
											z(n, "name", `subtitle-${H(u)}`), c(i, e), o !== (o = (H(p), R(() => H(p).choice.subtitleLanguage || ""))) && (n.value = (n.__value = o) ?? "", s(n, o));
										}, [() => (K(Q), R(() => Q("noSubtitles")))]), _("change", n, ae), d(e, n);
									},
									$$slots: { default: !0 }
								});
							}
							var r = F(n, 2), i = (e) => {
								{
									let t = V(() => (K(Q), R(() => Q("subtitleMode")))), n = V(() => (K(Q), R(() => ({
										openIn: "page",
										pageTitle: Q("subtitleMode"),
										searchbar: !1,
										scrollToSelectedItem: !0
									}))));
									U(e, {
										get title() {
											return H(t);
										},
										smartSelect: !0,
										get smartSelectParams() {
											return H(n);
										},
										children: (e, t) => {
											var n = Er(), r = N(n), i = q(r, !0);
											r.value = r.__value = "off";
											var a = F(r), o = q(a, !0);
											a.value = a.__value = "download";
											var l = F(a), f = q(l, !0);
											l.value = l.__value = "embed", J(n);
											var m;
											g(n), G((e, t, r) => {
												z(n, "name", `subtitle-mode-${H(u)}`), c(i, e), c(o, t), c(f, r), m !== (m = (H(p), R(() => H(p).choice.subtitleMode))) && (n.value = (n.__value = m) ?? "", s(n, m));
											}, [
												() => (K(Q), R(() => Q("subtitleOff"))),
												() => (K(Q), R(() => Q("subtitleDownload"))),
												() => (K(Q), R(() => Q("subtitleEmbed")))
											]), _("change", n, ce), d(e, n);
										},
										$$slots: { default: !0 }
									});
								}
							};
							x(r, (e) => {
								H(b), H(p), R(() => H(b) && H(p).choice.subtitleLanguage) && e(i);
							}), d(e, t);
						};
						x(T, (e) => {
							H(p), R(() => H(p).choice.contentKind === "video" && H(p).subtitles.length > 0) && e(D);
						});
						var O = F(T, 2);
						{
							let e = (e) => {
								var t = Cr(), n = q(t, !0);
								G(() => c(n, (H(p), R(() => H(p).timeRangeSummary)))), d(e, t);
							}, t = V(() => (K(Q), R(() => Q("downloadRange")))), n = V(() => `/item/${H(u)}/time-range/`);
							U(O, {
								get title() {
									return H(t);
								},
								get link() {
									return H(n);
								},
								after: e,
								$$slots: { after: !0 }
							});
						}
						var k = F(O, 2), A = (e) => {
							{
								let t = (e) => {
									var t = Cr(), n = q(t, !0);
									G(() => c(n, (H(p), R(() => H(p).chapterRangeSummary)))), d(e, t);
								}, n = V(() => (K(Q), R(() => Q("chapterRange")))), r = V(() => `/item/${H(u)}/chapter-range/`);
								U(e, {
									get title() {
										return H(n);
									},
									get link() {
										return H(r);
									},
									after: t,
									$$slots: { after: !0 }
								});
							}
						};
						x(k, (e) => {
							H(p), R(() => H(p).item.chapters?.length) && e(A);
						});
						var M = F(k, 2), ne = (e) => {
							var t = fe(), n = B(t), r = (e) => {
								{
									let t = V(() => (K(Q), R(() => Q("chapterMode")))), n = V(() => (K(Q), R(() => ({
										openIn: "page",
										pageTitle: Q("chapterMode"),
										searchbar: !1,
										scrollToSelectedItem: !0
									}))));
									U(e, {
										get title() {
											return H(t);
										},
										smartSelect: !0,
										get smartSelectParams() {
											return H(n);
										},
										children: (e, t) => {
											var n = Er(), r = N(n), i = q(r, !0);
											r.value = r.__value = "off";
											var a = F(r), o = q(a, !0);
											a.value = a.__value = "download";
											var l = F(a), f = q(l, !0);
											l.value = l.__value = "embed", J(n);
											var m;
											g(n), G((e, t, r) => {
												z(n, "name", `chapter-mode-${H(u)}`), c(i, e), c(o, t), c(f, r), m !== (m = (H(p), R(() => H(p).choice.chapterMode))) && (n.value = (n.__value = m) ?? "", s(n, m));
											}, [
												() => (K(Q), R(() => Q("chapterOff"))),
												() => (K(Q), R(() => Q("chapterDownload"))),
												() => (K(Q), R(() => Q("chapterEmbed")))
											]), _("change", n, ue), d(e, n);
										},
										$$slots: { default: !0 }
									});
								}
							}, i = (e) => {
								{
									let t = (e) => {
										{
											let t = V(() => (H(p), R(() => H(p).choice.chapters === "split")));
											re(e, {
												get checked() {
													return H(t);
												},
												onChange: le
											});
										}
									}, n = V(() => (K(Q), R(() => Q("chapters"))));
									U(e, {
										get title() {
											return H(n);
										},
										after: t,
										$$slots: { after: !0 }
									});
								}
							};
							x(n, (e) => {
								H(C) ? e(r) : e(i, -1);
							}), d(e, t);
						};
						x(M, (e) => {
							H(p), R(() => H(p).item.chapterCount > 0) && e(ne);
						}), d(e, n);
					},
					$$slots: { default: !0 }
				}), G((e) => {
					c(h, (H(p), R(() => H(p).item.title || H(p).item.url))), c(T, e);
				}, [() => (K(gn), H(p), R(() => gn(H(p).item.duration)))]), d(e, t);
			}, h = (e) => {
				ne(e, {
					strong: !0,
					dividers: !0,
					outline: !0,
					inset: !0,
					children: (e, t) => {
						{
							let t = V(() => (K(Q), R(() => Q("noFormats"))));
							U(e, { get title() {
								return H(t);
							} });
						}
					},
					$$slots: { default: !0 }
				});
			};
			x(a, (e) => {
				H(p) ? e(o) : e(h, -1);
			}), d(e, n);
		},
		$$slots: { default: !0 }
	}), Y(), a();
}
E([
	"input",
	"keydown",
	"click",
	"change"
]);
//#endregion
//#region src/popup/pages/GalleryDetailPage.svelte
var jr = u("<span class=\"format-choice-summary\"> </span>"), Mr = u("<!> <!>", 1), Nr = u("<article class=\"source-card\"><!> <div class=\"source-card-content\"><div class=\"source-card-title\"> </div><span class=\"result-date\"> </span></div></article> <!>", 1);
function Pr(e, t) {
	y(t, !1);
	let n = () => ve($, "$controller", r), [r, i] = O(), a = I(), s = I(), l = I(), u = I(), f = I(), p = o(t, "galleryIndex", 8);
	A(() => K(p()), () => {
		W(a, Number(p()));
	}), A(() => n(), () => {
		W(s, n());
	}), A(() => H(s), () => {
		W(l, Ce(H(s).analysisByBackend.g?.items || [], H(s).sourceUrl));
	}), A(() => (H(l), H(a)), () => {
		W(u, H(l)[H(a)]);
	}), A(() => (H(u), H(s)), () => {
		W(f, H(u) ? $.galleryModeForGroup(H(u), H(s)) : "browser-files");
	}), de(), h(), M(e, {
		name: "gallery-detail",
		children: (e, t) => {
			var n = Mr(), r = B(n);
			{
				let e = V(() => (K(Q), R(() => Q("downloadContent"))));
				se(r, {
					get title() {
						return H(e);
					},
					backLink: !0
				});
			}
			var i = F(r, 2), o = (e) => {
				var t = Nr(), n = B(t), r = N(n);
				{
					let e = V(() => (H(u), R(() => [H(u).thumbnail].filter(Boolean))));
					Mn(r, {
						get sources() {
							return H(e);
						},
						get siteIcon() {
							return H(s), R(() => H(s).sourceSiteIconUrl);
						}
					});
				}
				var i = F(r, 2), o = N(i), l = q(o, !0), p = F(o), m = q(p, !0);
				J(i), J(n);
				var h = F(n, 2);
				ne(h, {
					strong: !0,
					inset: !0,
					class: "option-list",
					children: (e, t) => {
						var n = Mr(), r = B(n), i = (e) => {
							{
								let t = (e) => {
									var t = jr(), n = q(t, !0);
									G((e) => c(n, e), [() => (K(xe), K(Te), H(u), H(s), R(() => xe(Te(H(u), H(s)))))]), d(e, t);
								}, n = V(() => (K(Q), R(() => Q("downloadRange")))), r = V(() => `/gallery/${H(a)}/range/`), i = V(() => (H(s), R(() => H(s).busy || H(s).downloadRunning || H(s).analysisRunning)));
								U(e, {
									get title() {
										return H(n);
									},
									get link() {
										return H(r);
									},
									get disabled() {
										return H(i);
									},
									after: t,
									$$slots: { after: !0 }
								});
							}
						};
						x(r, (e) => {
							H(u), R(() => H(u).sourceScope === "timeline") && e(i);
						});
						var o = F(r, 2);
						{
							let e = (e) => {
								var t = jr(), n = q(t, !0);
								G((e) => c(n, e), [() => (K(Q), K(tt), H(f), R(() => Q(tt(H(f)))))]), d(e, t);
							}, t = V(() => (K(Q), R(() => Q("galleryDownloadMode")))), n = V(() => `/gallery/${H(a)}/format/`), r = V(() => (H(s), R(() => H(s).busy || H(s).downloadRunning || H(s).analysisRunning)));
							U(o, {
								get title() {
									return H(t);
								},
								get link() {
									return H(n);
								},
								get disabled() {
									return H(r);
								},
								after: e,
								$$slots: { after: !0 }
							});
						}
						d(e, n);
					},
					$$slots: { default: !0 }
				}), G((e) => {
					c(l, (H(u), R(() => H(u).title || H(u).sourcePageUrl))), c(m, e);
				}, [() => (H(u), K(Q), R(() => [H(u).date, Q("galleryMediaCount", { count: H(u).media.length })].filter(Boolean).join(" · ")))]), d(e, t);
			}, l = (e) => {
				ne(e, {
					strong: !0,
					dividers: !0,
					outline: !0,
					inset: !0,
					children: (e, t) => {
						{
							let t = V(() => (K(Q), R(() => Q("noFormats"))));
							U(e, { get title() {
								return H(t);
							} });
						}
					},
					$$slots: { default: !0 }
				});
			};
			x(i, (e) => {
				H(u) ? e(o) : e(l, -1);
			}), d(e, n);
		},
		$$slots: { default: !0 }
	}), Y(), i();
}
//#endregion
//#region src/popup/pages/GalleryFormatPage.svelte
var Fr = u("<span class=\"radio-cell\"><input type=\"radio\" tabindex=\"-1\" aria-hidden=\"true\"/></span> <span> </span>", 1), Ir = u("<div class=\"gallery-format-choices\" role=\"radiogroup\"></div>"), Lr = u("<!> <!>", 1);
function Rr(e, n) {
	y(n, !1);
	let r = () => ve($, "$controller", a), [a, s] = O(), u = I(), f = I(), g = I(), _ = I(), v = o(n, "galleryIndex", 8);
	A(() => K(v()), () => {
		W(u, Number(v()));
	}), A(() => r(), () => {
		W(f, r());
	}), A(() => (H(f), H(u)), () => {
		W(g, Ce(H(f).analysisByBackend.g?.items || [], H(f).sourceUrl)[H(u)]);
	}), A(() => (H(g), H(f)), () => {
		W(_, H(g) ? $.galleryModeForGroup(H(g), H(f)) : "");
	}), de(), h(), M(e, {
		name: "gallery-format",
		children: (e, n) => {
			var r = Lr(), a = B(r);
			{
				let e = V(() => (K(Q), R(() => Q("galleryDownloadMode"))));
				se(a, {
					get title() {
						return H(e);
					},
					backLink: !0
				});
			}
			var o = F(a, 2), s = (e) => {
				var n = Ir();
				S(n, 5, () => $e, l, (e, n) => {
					{
						let r = V(() => `gallery-format-row${H(_) === H(n) ? " selected" : ""}`), i = V(() => H(_) === H(n));
						L(e, {
							href: "#",
							back: !0,
							noLinkClass: !0,
							get class() {
								return H(r);
							},
							role: "radio",
							get "aria-checked"() {
								return H(i);
							},
							onClick: () => $.setGalleryDownloadMode(H(n), H(g).key),
							children: (e, r) => {
								var i = Fr(), a = B(i), o = N(a);
								t(o), J(a);
								var s = F(a, 2), l = q(s, !0);
								G((e) => {
									z(o, "name", `gallery-format-${H(u)}`), m(o, H(n)), p(o, H(_) === H(n)), c(l, e);
								}, [() => (K(Q), K(tt), H(n), R(() => Q(tt(H(n)))))]), d(e, i);
							},
							$$slots: { default: !0 }
						});
					}
				}), J(n), G((e) => z(n, "aria-label", e), [() => (K(Q), R(() => Q("galleryDownloadMode")))]), d(e, n);
			}, f = (e) => {
				X(e, {
					strong: !0,
					inset: !0,
					children: (e, t) => {
						i();
						var n = w();
						G((e) => c(n, e), [() => (K(Q), R(() => Q("noFormats")))]), d(e, n);
					},
					$$slots: { default: !0 }
				});
			};
			x(o, (e) => {
				H(g) ? e(s) : e(f, -1);
			}), d(e, r);
		},
		$$slots: { default: !0 }
	}), Y(), s();
}
//#endregion
//#region src/popup/pages/GalleryRangePage.svelte
var zr = u("<option> </option><option> </option><option> </option><option> </option>", 1), Br = u("<!> <!>", 1), Vr = u("<div class=\"range-actions\"><!></div>"), Hr = u("<!> <!> <!> <!>", 1);
function Ur(e, t) {
	y(t, !1);
	let n = () => ve($, "$controller", r), [r, a] = O(), s = I(), u = I(), f = I(), p = I(), m = o(t, "galleryIndex", 8), g = I(!1), _ = I("posts"), v = I(10), b = I(7), C = I("");
	function T(e) {
		try {
			return De(e), !0;
		} catch {
			return !1;
		}
	}
	A(() => n(), () => {
		W(s, n());
	}), A(() => (H(s), K(m())), () => {
		W(u, Ce(H(s).analysisByBackend.g?.items || [], H(s).sourceUrl)[Number(m())]);
	}), A(() => (H(u), H(g), H(s)), () => {
		if (H(u) && !H(g)) {
			let e = Te(H(u), H(s));
			W(_, e.kind), W(v, e.count || 10), W(b, e.days || 7), W(C, e.date || ""), W(g, !0);
		}
	}), A(() => (H(_), H(v), H(b), H(C)), () => {
		W(f, H(_) === "posts" ? {
			kind: H(_),
			count: Number(H(v))
		} : H(_) === "recent" ? {
			kind: H(_),
			days: Number(H(b))
		} : H(_) === "since" ? {
			kind: H(_),
			date: H(C)
		} : { kind: H(_) });
	}), A(() => H(f), () => {
		W(p, T(H(f)));
	}), de(), h(), M(e, {
		name: "gallery-range",
		children: (e, t) => {
			var n = Br(), r = B(n);
			{
				let e = V(() => (K(Q), R(() => Q("downloadRange"))));
				se(r, {
					get title() {
						return H(e);
					},
					backLink: !0
				});
			}
			var a = F(r, 2), o = (e) => {
				var t = Hr(), n = B(t);
				ne(n, {
					strong: !0,
					inset: !0,
					class: "option-list",
					children: (e, t) => {
						var n = Br(), r = B(n);
						{
							let e = V(() => (K(Q), R(() => Q("timelineRangeMethod"))));
							ce(r, {
								get label() {
									return H(e);
								},
								type: "select",
								get value() {
									return H(_);
								},
								onChange: (e) => W(_, e.currentTarget.value),
								children: (e, t) => {
									var n = zr(), r = B(n), i = q(r, !0);
									r.value = r.__value = "posts";
									var a = F(r), o = q(a, !0);
									a.value = a.__value = "recent";
									var s = F(a), l = q(s, !0);
									s.value = s.__value = "since";
									var u = F(s), f = q(u, !0);
									u.value = u.__value = "all", G((e, t, n, r) => {
										c(i, e), c(o, t), c(l, n), c(f, r);
									}, [
										() => (K(Q), R(() => Q("rangePosts"))),
										() => (K(Q), R(() => Q("rangeTime"))),
										() => (K(Q), R(() => Q("rangeSinceChoice"))),
										() => (K(Q), R(() => Q("rangeAll")))
									]), d(e, n);
								},
								$$slots: { default: !0 }
							});
						}
						var i = F(r, 2), a = (e) => {
							{
								let t = V(() => (K(Q), R(() => Q("rangeCount"))));
								ce(e, {
									get label() {
										return H(t);
									},
									type: "number",
									min: "1",
									step: "1",
									get value() {
										return H(v);
									},
									onInput: (e) => W(v, e.currentTarget.value)
								});
							}
						}, o = (e) => {
							{
								let t = V(() => (K(Q), R(() => Q("rangeDays"))));
								ce(e, {
									get label() {
										return H(t);
									},
									type: "number",
									min: "1",
									max: "36500",
									step: "1",
									get value() {
										return H(b);
									},
									onInput: (e) => W(b, e.currentTarget.value)
								});
							}
						}, s = (e) => {
							{
								let t = V(() => (K(Q), R(() => Q("rangeDate"))));
								ce(e, {
									get label() {
										return H(t);
									},
									type: "date",
									get value() {
										return H(C);
									},
									onInput: (e) => W(C, e.currentTarget.value)
								});
							}
						};
						x(i, (e) => {
							H(_) === "posts" ? e(a) : H(_) === "recent" ? e(o, 1) : H(_) === "since" && e(s, 2);
						}), d(e, n);
					},
					$$slots: { default: !0 }
				});
				var r = F(n, 2), a = (e) => {
					X(e, {
						inset: !0,
						class: "range-actions range-presets",
						children: (e, t) => {
							var n = fe(), r = B(n);
							S(r, 0, () => [
								10,
								50,
								100
							], l, (e, t) => {
								{
									let n = V(() => (H(v), R(() => Number(H(v)) === t ? "button button-fill" : "button button-outline")));
									L(e, {
										get class() {
											return H(n);
										},
										onClick: () => W(v, t),
										children: (e, n) => {
											i();
											var r = w();
											G((e) => c(r, e), [() => (K(Q), R(() => Q("rangeLatest", { count: t })))]), d(e, r);
										},
										$$slots: { default: !0 }
									});
								}
							}), d(e, n);
						},
						$$slots: { default: !0 }
					});
				}, o = (e) => {
					X(e, {
						inset: !0,
						class: "range-actions range-presets",
						children: (e, t) => {
							var n = fe(), r = B(n);
							S(r, 0, () => [
								1,
								7,
								30
							], l, (e, t) => {
								{
									let n = V(() => (H(b), R(() => Number(H(b)) === t ? "button button-fill" : "button button-outline")));
									L(e, {
										get class() {
											return H(n);
										},
										onClick: () => W(b, t),
										children: (e, n) => {
											i();
											var r = w();
											G((e) => c(r, e), [() => (K(Q), R(() => Q("rangeRecentDays", { days: t })))]), d(e, r);
										},
										$$slots: { default: !0 }
									});
								}
							}), d(e, n);
						},
						$$slots: { default: !0 }
					});
				};
				x(r, (e) => {
					H(_) === "posts" ? e(a) : H(_) === "recent" && e(o, 1);
				});
				var s = F(r, 2), m = (e) => {
					X(e, {
						class: "range-error",
						children: (e, t) => {
							i();
							var n = w();
							G((e) => c(n, e), [() => (K(Q), R(() => Q("rangeInvalid")))]), d(e, n);
						},
						$$slots: { default: !0 }
					});
				};
				x(s, (e) => {
					H(p) || e(m);
				});
				var h = F(s, 2);
				ue(h, {
					bottom: !0,
					class: "range-toolbar",
					children: (e, t) => {
						var n = Vr(), r = N(n);
						{
							let e = V(() => !H(p) || void 0);
							L(r, {
								back: !0,
								href: "#",
								class: "button button-fill",
								get disabled() {
									return H(e);
								},
								onClick: () => H(p) && $.setGalleryRange(H(u).key, H(f)),
								children: (e, t) => {
									i();
									var n = w();
									G((e) => c(n, e), [() => (K(Q), R(() => Q("confirm")))]), d(e, n);
								},
								$$slots: { default: !0 }
							});
						}
						J(n), d(e, n);
					},
					$$slots: { default: !0 }
				}), d(e, t);
			};
			x(a, (e) => {
				H(u), R(() => H(u)?.sourceScope === "timeline") && e(o);
			}), d(e, n);
		},
		$$slots: { default: !0 }
	}), Y(), a();
}
//#endregion
//#region src/popup/pages/FormatPage.svelte
var Wr = u("<span> </span> <span> </span> <span> </span> <span> </span> <span> </span>", 1), Gr = u("<span> </span> <span> </span> <span> </span> <span> </span>", 1), Kr = u("<span class=\"format-cell\"> </span>"), qr = u("<span class=\"radio-cell\"><input type=\"radio\" tabindex=\"-1\" aria-hidden=\"true\"/></span> <!>", 1), Jr = u("<div role=\"radiogroup\"><div class=\"format-header\" role=\"row\"><span aria-hidden=\"true\"></span> <!></div> <!></div>"), Yr = u("<!> <!>", 1);
function Xr(e, n) {
	y(n, !1);
	let r = () => ve($, "$controller", a), [a, s] = O(), u = I(), g = I(), _ = I(), v = I(), b = I(), C = I(), T = I(), E = o(n, "itemIndex", 8), D = o(n, "formatKind", 8);
	function k(e) {
		if (H(g)) return [
			String(e.language || "").trim() || "—",
			hn(e.sampleRate) || "—",
			mn(e.audioCodec || e.codec) || "—",
			_n(e.bitrate) || "—",
			vn(e.filesize, e.filesizeApproximate) || "—"
		];
		let t = [mn(e.codec), e.kind === "muxed" ? mn(e.audioCodec) : ""].filter(Boolean).join(" + ");
		return [
			e.resolution || "—",
			e.fps ? String(e.fps) : "—",
			t || "—",
			vn(e.filesize, e.filesizeApproximate) || "—"
		];
	}
	function j(e) {
		$.setChoice(H(u), { [H(g) ? "audioFormat" : "videoFormat"]: String(e) });
	}
	A(() => K(E()), () => {
		W(u, Number(E()));
	}), A(() => K(D()), () => {
		W(g, D() === "audio");
	}), A(() => r(), () => {
		W(_, r());
	}), A(() => (H(_), H(u)), () => {
		W(v, wn(H(_), H(u)));
	}), A(() => (H(v), H(g)), () => {
		W(b, H(v) ? H(g) ? H(v).audioFormats : H(v).videoFormats : []);
	}), A(() => (H(v), H(g)), () => {
		W(C, H(v) ? String(H(g) ? H(v).choice.audioFormat || "" : H(v).choice.videoFormat || "") : "");
	}), A(() => H(g), () => {
		W(T, Q(H(g) ? "audioFormat" : "videoFormat"));
	}), de(), h(), M(e, {
		name: "format-grid",
		children: (e, n) => {
			var r = Yr(), a = B(r);
			se(a, {
				get title() {
					return H(T);
				},
				backLink: !0
			});
			var o = F(a, 2), s = (e) => {
				var n = Jr();
				let r;
				var i = N(n), a = F(N(i), 2), o = (e) => {
					var t = Wr(), n = B(t), r = q(n, !0), i = F(n, 2), a = q(i, !0), o = F(i, 2), s = q(o, !0), l = F(o, 2), u = q(l, !0), f = F(l, 2), p = q(f, !0);
					G((e, t, n, i, o) => {
						c(r, e), c(a, t), c(s, n), c(u, i), c(p, o);
					}, [
						() => (K(Q), R(() => Q("language"))),
						() => (K(Q), R(() => Q("sampleRate"))),
						() => (K(Q), R(() => Q("codec"))),
						() => (K(Q), R(() => Q("bitrate"))),
						() => (K(Q), R(() => Q("size")))
					]), d(e, t);
				}, s = (e) => {
					var t = Gr(), n = B(t), r = q(n, !0), i = F(n, 2), a = q(i, !0), o = F(i, 2), s = q(o, !0), l = F(o, 2), u = q(l, !0);
					G((e, t, n, i) => {
						c(r, e), c(a, t), c(s, n), c(u, i);
					}, [
						() => (K(Q), R(() => Q("resolution"))),
						() => (K(Q), R(() => Q("fps"))),
						() => (K(Q), R(() => Q("codec"))),
						() => (K(Q), R(() => Q("size")))
					]), d(e, t);
				};
				x(a, (e) => {
					H(g) ? e(o) : e(s, -1);
				}), J(i);
				var h = F(i, 2);
				S(h, 1, () => H(b), l, (e, n) => {
					let r = V(() => (H(n), R(() => String(H(n).id)))), i = V(() => (H(n), R(() => k(H(n)))));
					{
						let n = V(() => `format-row${H(r) === H(C) ? " selected" : ""}`), a = V(() => H(r) === H(C));
						L(e, {
							href: "#",
							back: !0,
							noLinkClass: !0,
							get class() {
								return H(n);
							},
							role: "radio",
							get "aria-checked"() {
								return H(a);
							},
							onClick: () => j(H(r)),
							children: (e, n) => {
								var a = qr(), o = B(a), s = N(o);
								t(s), J(o);
								var f = F(o, 2);
								S(f, 1, () => H(i), l, (e, t) => {
									var n = Kr(), r = q(n, !0);
									G(() => c(r, H(t))), d(e, n);
								}), G(() => {
									z(s, "name", `${D()}-format-${H(u)}`), m(s, H(r)), p(s, H(r) === H(C));
								}), d(e, a);
							},
							$$slots: { default: !0 }
						});
					}
				}), J(n), G(() => {
					r = f(n, 1, "format-grid", null, r, { "audio-grid": H(g) }), z(n, "aria-label", H(T));
				}), d(e, n);
			}, h = (e) => {
				X(e, {
					strong: !0,
					inset: !0,
					children: (e, t) => {
						i();
						var n = w();
						G((e) => c(n, e), [() => (K(Q), R(() => Q("noFormats")))]), d(e, n);
					},
					$$slots: { default: !0 }
				});
			};
			x(o, (e) => {
				H(b), R(() => H(b).length) ? e(s) : e(h, -1);
			}), d(e, r);
		},
		$$slots: { default: !0 }
	}), Y(), s();
}
//#endregion
//#region src/popup/pages/TimeRangePage.svelte
var Zr = u("<!> <!>", 1), Qr = u("<span class=\"button button-fill disabled\" aria-disabled=\"true\"> </span>"), $r = u("<div class=\"range-actions\"><!> <!></div>"), ei = u("<!> <!> <!> <!> <!>", 1);
function ti(e, t) {
	y(t, !1);
	let n = () => ve($, "$controller", r), [r, a] = O(), s = I(), l = I(), u = I(), f = I(), p = I(), m = I(), g = I(), _ = I(), v = o(t, "itemIndex", 8), b = I(!1), S = I("00:00:00"), C = I("");
	function T(e) {
		W(S, e.currentTarget.value);
	}
	function E(e) {
		W(C, e.currentTarget.value);
	}
	function D() {
		H(_) && $.setChoice(H(s), {
			rangeStartSeconds: H(f),
			rangeEndSeconds: H(p),
			chapterIndices: [],
			chapterMode: "off",
			chapters: "full"
		}, !1);
	}
	function k() {
		$.setChoice(H(s), {
			rangeStartSeconds: null,
			rangeEndSeconds: null
		}, !1);
	}
	A(() => K(v()), () => {
		W(s, Number(v()));
	}), A(() => n(), () => {
		W(l, n());
	}), A(() => (H(l), H(s)), () => {
		W(u, wn(H(l), H(s)));
	}), A(() => (H(u), H(b), dt), () => {
		if (H(u) && !H(b)) {
			W(S, ft(H(u).timeRange?.startSeconds || 0));
			let e = dt(H(u).item.duration);
			W(C, H(u).timeRange ? ft(H(u).timeRange.endSeconds) : e === null ? "" : ft(e)), W(b, !0);
		}
	}), A(() => H(S), () => {
		W(f, dt(H(S)));
	}), A(() => H(C), () => {
		W(p, dt(H(C)));
	}), A(() => H(u), () => {
		W(m, dt(H(u)?.item.duration));
	}), A(() => (H(f), H(p), H(m)), () => {
		W(g, H(f) === null || H(p) === null ? "timeRangeInvalid" : H(p) <= H(f) ? "timeRangeOrderInvalid" : H(m) !== null && H(p) > H(m) ? "timeRangeBeyondDuration" : "");
	}), A(() => H(g), () => {
		W(_, !H(g));
	}), de(), h(), M(e, {
		name: "time-range",
		children: (e, t) => {
			var n = ei(), r = B(n);
			{
				let e = V(() => (K(Q), R(() => Q("downloadRange"))));
				se(r, {
					get title() {
						return H(e);
					},
					backLink: !0
				});
			}
			var a = F(r, 2);
			X(a, {
				strong: !0,
				inset: !0,
				children: (e, t) => {
					i();
					var n = w();
					G((e) => c(n, e), [() => (K(Q), R(() => Q("timeRangeHint")))]), d(e, n);
				},
				$$slots: { default: !0 }
			});
			var o = F(a, 2);
			ne(o, {
				strong: !0,
				inset: !0,
				class: "option-list",
				children: (e, t) => {
					var n = Zr(), r = B(n);
					{
						let e = V(() => (K(Q), R(() => Q("rangeStart"))));
						ce(r, {
							get label() {
								return H(e);
							},
							type: "text",
							inputmode: "numeric",
							get value() {
								return H(S);
							},
							placeholder: "00:00:00",
							onInput: T
						});
					}
					var i = F(r, 2);
					{
						let e = V(() => (K(Q), R(() => Q("rangeEnd"))));
						ce(i, {
							get label() {
								return H(e);
							},
							type: "text",
							inputmode: "numeric",
							get value() {
								return H(C);
							},
							placeholder: "00:00:00",
							onInput: E
						});
					}
					d(e, n);
				},
				$$slots: { default: !0 }
			});
			var s = F(o, 2), l = (e) => {
				X(e, {
					inset: !0,
					class: "range-error",
					children: (e, t) => {
						i();
						var n = w();
						G((e) => c(n, e), [() => (K(Q), H(g), R(() => Q(H(g))))]), d(e, n);
					},
					$$slots: { default: !0 }
				});
			};
			x(s, (e) => {
				H(g) && e(l);
			});
			var u = F(s, 2);
			ue(u, {
				bottom: !0,
				class: "range-toolbar",
				children: (e, t) => {
					var n = $r(), r = N(n);
					L(r, {
						href: "#",
						back: !0,
						class: "button button-outline",
						onClick: k,
						children: (e, t) => {
							i();
							var n = w();
							G((e) => c(n, e), [() => (K(Q), R(() => Q("fullVideo")))]), d(e, n);
						},
						$$slots: { default: !0 }
					});
					var a = F(r, 2), o = (e) => {
						L(e, {
							href: "#",
							back: !0,
							class: "button button-fill",
							onClick: D,
							children: (e, t) => {
								i();
								var n = w();
								G((e) => c(n, e), [() => (K(Q), R(() => Q("confirm")))]), d(e, n);
							},
							$$slots: { default: !0 }
						});
					}, s = (e) => {
						var t = Qr(), n = q(t, !0);
						G((e) => c(n, e), [() => (K(Q), R(() => Q("confirm")))]), d(e, t);
					};
					x(a, (e) => {
						H(_) ? e(o) : e(s, -1);
					}), J(n), d(e, n);
				},
				$$slots: { default: !0 }
			}), d(e, n);
		},
		$$slots: { default: !0 }
	}), Y(), a();
}
//#endregion
//#region src/popup/pages/ChapterRangePage.svelte
var ni = u("<!> <!>", 1), ri = u("<span class=\"button button-fill disabled\" aria-disabled=\"true\"> </span>"), ii = u("<div class=\"chapter-range-actions\"><!> <!></div>"), ai = u("<!> <!> <!> <!> <!>", 1);
function oi(e, t) {
	y(t, !1);
	let n = () => ve($, "$controller", r), [r, a] = O(), s = I(), u = I(), f = I(), p = I(), m = o(t, "itemIndex", 8), g = I(!1), _ = I(/* @__PURE__ */ new Set());
	function v(e) {
		return `${ft(Math.floor(e.startMillis / 1e3))}–${ft(Math.floor(e.endMillis / 1e3))}`;
	}
	function b(e, t) {
		let n = new Set(H(_));
		t.currentTarget.checked ? n.add(e) : n.delete(e), W(_, n);
	}
	function C() {
		W(_, new Set(H(p).map((e, t) => t)));
	}
	function T() {
		W(_, /* @__PURE__ */ new Set());
	}
	function E() {
		H(_).size && $.setChoice(H(s), {
			chapterIndices: [...H(_)].sort((e, t) => e - t),
			rangeStartSeconds: null,
			rangeEndSeconds: null,
			chapterMode: "off",
			chapters: "full"
		}, !1);
	}
	function k() {
		$.setChoice(H(s), { chapterIndices: [] }, !1);
	}
	A(() => K(m()), () => {
		W(s, Number(m()));
	}), A(() => n(), () => {
		W(u, n());
	}), A(() => (H(u), H(s)), () => {
		W(f, wn(H(u), H(s)));
	}), A(() => H(f), () => {
		W(p, H(f)?.item.chapters || []);
	}), A(() => (H(f), H(g)), () => {
		H(f) && !H(g) && (W(_, new Set(H(f).chapterIndices)), W(g, !0));
	}), de(), h(), M(e, {
		name: "chapter-range",
		children: (e, t) => {
			var n = ai(), r = B(n);
			{
				let e = V(() => (K(Q), R(() => Q("chapterRange"))));
				se(r, {
					get title() {
						return H(e);
					},
					backLink: !0
				});
			}
			var a = F(r, 2);
			X(a, {
				strong: !0,
				inset: !0,
				children: (e, t) => {
					i();
					var n = w();
					G((e) => c(n, e), [() => (K(Q), R(() => Q("chapterRangeHint")))]), d(e, n);
				},
				$$slots: { default: !0 }
			});
			var o = F(a, 2);
			X(o, {
				inset: !0,
				children: (e, t) => {
					me(e, {
						strong: !0,
						children: (e, t) => {
							var n = ni(), r = B(n);
							D(r, {
								onClick: C,
								children: (e, t) => {
									i();
									var n = w();
									G((e) => c(n, e), [() => (K(Q), R(() => Q("selectAll")))]), d(e, n);
								},
								$$slots: { default: !0 }
							});
							var a = F(r, 2);
							D(a, {
								onClick: T,
								children: (e, t) => {
									i();
									var n = w();
									G((e) => c(n, e), [() => (K(Q), R(() => Q("clearSelection")))]), d(e, n);
								},
								$$slots: { default: !0 }
							}), d(e, n);
						},
						$$slots: { default: !0 }
					});
				},
				$$slots: { default: !0 }
			});
			var s = F(o, 2);
			ne(s, {
				strong: !0,
				dividers: !0,
				outline: !0,
				inset: !0,
				children: (e, t) => {
					var n = fe(), r = B(n);
					S(r, 1, () => H(p), l, (e, t, n) => {
						{
							let r = V(() => (H(_), R(() => H(_).has(n)))), i = V(() => (H(t), K(Q), R(() => H(t).title || Q("chapterNumber", { number: n + 1 })))), a = V(() => (H(t), R(() => v(H(t)))));
							U(e, {
								checkbox: !0,
								get checked() {
									return H(r);
								},
								get title() {
									return H(i);
								},
								get after() {
									return H(a);
								},
								onChange: (e) => b(n, e)
							});
						}
					}), d(e, n);
				},
				$$slots: { default: !0 }
			});
			var u = F(s, 2);
			ue(u, {
				bottom: !0,
				class: "chapter-range-toolbar",
				children: (e, t) => {
					var n = ii(), r = N(n);
					L(r, {
						href: "#",
						back: !0,
						class: "button button-outline",
						onClick: k,
						children: (e, t) => {
							i();
							var n = w();
							G((e) => c(n, e), [() => (K(Q), R(() => Q("fullVideo")))]), d(e, n);
						},
						$$slots: { default: !0 }
					});
					var a = F(r, 2), o = (e) => {
						L(e, {
							href: "#",
							back: !0,
							class: "button button-fill",
							onClick: E,
							children: (e, t) => {
								i();
								var n = w();
								G((e) => c(n, e), [() => (K(Q), R(() => Q("confirm")))]), d(e, n);
							},
							$$slots: { default: !0 }
						});
					}, s = (e) => {
						var t = ri(), n = q(t, !0);
						G((e) => c(n, e), [() => (K(Q), R(() => Q("confirm")))]), d(e, t);
					};
					x(a, (e) => {
						H(_), R(() => H(_).size) ? e(o) : e(s, -1);
					}), J(n), d(e, n);
				},
				$$slots: { default: !0 }
			}), d(e, n);
		},
		$$slots: { default: !0 }
	}), Y(), a();
}
//#endregion
//#region src/popup/export-operation-diagnostics.js
async function si() {
	let e = await chrome.runtime.sendMessage({ type: "GET_OPERATION_DIAGNOSTICS" });
	if (!e?.ok || !Array.isArray(e.data)) throw Error("Diagnostic journal unavailable.");
	let t = "data:application/json;charset=utf-8," + encodeURIComponent(JSON.stringify(e.data, null, 2));
	await chrome.downloads.download({
		url: t,
		filename: "yt-dlp-gui-extension-diagnostics.json",
		saveAs: !0
	});
}
//#endregion
//#region src/popup/pages/SettingsPage.svelte
var ci = u("<label class=\"tool-settings-row\"><input type=\"checkbox\"/> <span> </span> <span class=\"tool-availability\"> </span></label>"), li = u("<!> ", 1), ui = u("<p class=\"tool-settings-hint\"> </p>"), di = u("<p role=\"status\"> </p>"), fi = u("<div class=\"tool-settings-heading\"> </div> <!> <p class=\"tool-settings-hint\"> </p> <!> <!> <!>", 1), pi = u("<div class=\"tool-settings-heading\">yt-dlp</div> <!>", 1), mi = u("<label class=\"tool-settings-row\"><input type=\"radio\" name=\"album-wheel\"/> <span> </span></label>"), hi = u("<p role=\"status\" class=\"error-message\"> </p>"), gi = u("<div class=\"tool-settings-heading\"> </div> <div> </div> <!> <p class=\"tool-settings-hint\"> </p> <!>", 1), _i = u("<div class=\"tool-settings-heading\">Diagnostics</div> <!> <!>", 1), vi = u("<span> </span>"), yi = u("<!> <!> <!> <!> <!> <!> <!> <!>", 1);
function bi(e, n) {
	y(n, !1);
	let r = () => ve($, "$controller", a), [a, o] = O(), s = I(), u = I(), g = I("");
	async function v() {
		try {
			await si(), W(g, "Diagnostic export requested.");
		} catch {
			W(g, "Could not export diagnostics.");
		}
	}
	let b = chrome.runtime.getManifest().version;
	A(() => r(), () => {
		W(s, r());
	}), A(() => H(s), () => {
		W(u, !H(s).preferencesLoaded || H(s).busy || H(s).analysisRunning || H(s).downloadRunning || H(s).toolDetectionRunning || H(s).toolPreferenceSaving);
	}), de(), h(), M(e, {
		name: "settings",
		children: (e, n) => {
			var r = yi(), a = B(r);
			{
				let e = V(() => (K(Q), R(() => Q("settings"))));
				se(a, {
					get title() {
						return H(e);
					},
					backLink: !0
				});
			}
			var o = F(a, 2);
			X(o, {
				inset: !0,
				class: "tool-settings",
				children: (e, n) => {
					var r = fi(), i = B(r), a = q(i, !0), o = F(i, 2);
					S(o, 0, () => ["y", "g"], l, (e, n) => {
						var r = ci(), i = N(r);
						t(i);
						var a = F(i, 2), o = q(a, !0), l = F(a, 2), f = q(l, !0);
						J(r), G((e) => {
							p(i, (H(s), R(() => H(s).analysisTools[n]))), i.disabled = H(u), c(o, n === "y" ? "yt-dlp" : "gallery-dl"), c(f, e);
						}, [() => (K(Q), H(s), R(() => Q(H(s).toolDetection?.[n] === !0 ? "toolAvailable" : H(s).toolDetection?.[n] === !1 ? "toolUnavailable" : "toolNotDetected")))]), _("change", i, (e) => $.setAnalysisTool(n, e.currentTarget.checked)), d(e, r);
					});
					var m = F(o, 2), h = q(m, !0), g = F(m, 2);
					D(g, {
						small: !0,
						fill: !0,
						get disabled() {
							return H(u);
						},
						onClick: () => $.detectAnalysisTools(),
						children: (e, t) => {
							var n = li(), r = B(n), i = (e) => {
								_e(e, {});
							};
							x(r, (e) => {
								H(s), R(() => H(s).toolDetectionRunning) && e(i);
							});
							var a = F(r, 1, !0);
							G((e) => c(a, e), [() => (K(Q), R(() => Q("detectTools")))]), d(e, n);
						},
						$$slots: { default: !0 }
					});
					var v = F(g, 2), y = (e) => {
						var t = ui(), n = q(t, !0);
						G((e) => c(n, e), [() => (K(Q), H(s), R(() => Q("lastToolDetection", { date: new Date(H(s).toolDetection.checkedAt).toLocaleString() })))]), d(e, t);
					};
					x(v, (e) => {
						H(s), R(() => H(s).toolDetection?.checkedAt) && e(y);
					});
					var b = F(v, 2), C = (e) => {
						var t = di();
						let n;
						var r = q(t, !0);
						G(() => {
							n = f(t, 1, "", null, n, { "error-message": H(s).toolSettingsError }), c(r, (H(s), R(() => H(s).toolSettingsMessage)));
						}), d(e, t);
					};
					x(b, (e) => {
						H(s), R(() => H(s).toolSettingsMessage) && e(C);
					}), G((e, t) => {
						c(a, e), c(h, t);
					}, [() => (K(Q), R(() => Q("analysisTools"))), () => (K(Q), R(() => Q("analysisToolsHint")))]), d(e, r);
				},
				$$slots: { default: !0 }
			});
			var h = F(o, 2);
			X(h, {
				inset: !0,
				class: "tool-settings",
				children: (e, t) => {
					var n = pi(), r = F(B(n), 2);
					ne(r, {
						strong: !0,
						dividers: !0,
						outline: !0,
						class: "option-list",
						children: (e, t) => {
							{
								let t = V(() => (K(Q), R(() => Q("ytDlpFetchLimit")))), n = V(() => (H(s), R(() => String(H(s).playlistItemLimit)))), r = V(() => (K(Q), R(() => Q("ytDlpFetchLimitHint"))));
								ce(e, {
									get label() {
										return H(t);
									},
									type: "number",
									min: "1",
									max: "100",
									step: "1",
									get disabled() {
										return H(u);
									},
									get value() {
										return H(n);
									},
									get info() {
										return H(r);
									},
									onChange: (e) => $.setPlaylistItemLimit(e.currentTarget.value)
								});
							}
						},
						$$slots: { default: !0 }
					}), d(e, n);
				},
				$$slots: { default: !0 }
			});
			var y = F(h, 2);
			X(y, {
				inset: !0,
				class: "tool-settings",
				children: (e, n) => {
					var r = gi(), i = B(r), a = q(i, !0), o = F(i, 2), u = q(o, !0), f = F(o, 2);
					S(f, 0, () => ["zoom", "navigate"], l, (e, n) => {
						var r = mi(), i = N(r);
						t(i);
						var a = F(i, 2), o = q(a, !0);
						J(r), G((e) => {
							m(i, n), p(i, (H(s), R(() => H(s).albumWheelMode === n))), i.disabled = (H(s), R(() => !H(s).preferencesLoaded || H(s).albumPreferenceSaving)), c(o, e);
						}, [() => (K(Q), R(() => Q(n === "zoom" ? "albumWheelZoom" : "albumWheelNavigate")))]), _("change", i, () => $.setAlbumWheelMode(n)), d(e, r);
					});
					var h = F(f, 2), g = q(h, !0), v = F(h, 2), y = (e) => {
						var t = hi(), n = q(t, !0);
						G(() => c(n, (H(s), R(() => H(s).albumSettingsMessage)))), d(e, t);
					};
					x(v, (e) => {
						H(s), R(() => H(s).albumSettingsMessage) && e(y);
					}), G((e, t, n) => {
						c(a, e), c(u, t), c(g, n);
					}, [
						() => (K(Q), R(() => Q("albumSettings"))),
						() => (K(Q), R(() => Q("albumWheelOperation"))),
						() => (K(Q), R(() => Q("albumWheelHint")))
					]), d(e, r);
				},
				$$slots: { default: !0 }
			});
			var C = F(y, 2);
			ne(C, {
				strong: !0,
				dividers: !0,
				outline: !0,
				inset: !0,
				children: (e, t) => {
					{
						let t = (e) => {
							var t = fe(), n = B(t), r = (e) => {
								_e(e, {});
							}, a = (e) => {
								{
									let t = V(() => (H(s), R(() => H(s).busy || H(s).analysisRunning || H(s).downloadRunning)));
									D(e, {
										small: !0,
										fill: !0,
										get disabled() {
											return H(t);
										},
										onClick: () => $.saveSiteCookies(),
										children: (e, t) => {
											i();
											var n = w();
											G((e) => c(n, e), [() => (K(Q), R(() => Q("save")))]), d(e, n);
										},
										$$slots: { default: !0 }
									});
								}
							};
							x(n, (e) => {
								H(s), R(() => H(s).cookieSaveRunning) ? e(r) : e(a, -1);
							}), d(e, t);
						}, n = V(() => (K(Q), R(() => Q("saveSiteCookies")))), r = V(() => (K(Q), R(() => Q("saveSiteCookiesHint"))));
						U(e, {
							get title() {
								return H(n);
							},
							get footer() {
								return H(r);
							},
							after: t,
							$$slots: { after: !0 }
						});
					}
				},
				$$slots: { default: !0 }
			});
			var T = F(C, 2);
			X(T, {
				inset: !0,
				class: "extension-version",
				children: (e, t) => {
					i();
					var n = w();
					G(() => c(n, `v${b ?? ""} · F7 + Spotlight album`)), d(e, n);
				},
				$$slots: { default: !0 }
			});
			var E = F(T, 2);
			X(E, {
				inset: !0,
				class: "tool-settings",
				children: (e, t) => {
					var n = _i(), r = F(B(n), 2);
					D(r, {
						small: !0,
						fill: !0,
						onClick: v,
						children: (e, t) => {
							i();
							var n = w("Export diagnostics");
							d(e, n);
						},
						$$slots: { default: !0 }
					});
					var a = F(r, 2), o = (e) => {
						var t = di(), n = q(t, !0);
						G(() => c(n, H(g))), d(e, t);
					};
					x(a, (e) => {
						H(g) && e(o);
					}), d(e, n);
				},
				$$slots: { default: !0 }
			});
			var O = F(E, 2), k = (e) => {
				X(e, {
					strong: !0,
					inset: !0,
					children: (e, t) => {
						var n = vi();
						let r;
						var i = q(n, !0);
						G(() => {
							r = f(n, 1, "", null, r, { "error-message": H(s).cookieSaveError }), c(i, (H(s), R(() => H(s).cookieSaveMessage)));
						}), d(e, n);
					},
					$$slots: { default: !0 }
				});
			};
			x(O, (e) => {
				H(s), R(() => H(s).cookieSaveMessage) && e(k);
			}), d(e, r);
		},
		$$slots: { default: !0 }
	}), Y(), o();
}
E(["change"]);
//#endregion
//#region src/popup/routes.js
var xi = [
	{
		path: "/",
		component: br
	},
	{
		path: "/item/:itemIndex/",
		component: Ar
	},
	{
		path: "/gallery/:galleryIndex/",
		component: Pr
	},
	{
		path: "/gallery/:galleryIndex/format/",
		component: Rr
	},
	{
		path: "/gallery/:galleryIndex/range/",
		component: Ur
	},
	{
		path: "/item/:itemIndex/format/:formatKind/",
		component: Xr
	},
	{
		path: "/item/:itemIndex/time-range/",
		component: ti
	},
	{
		path: "/item/:itemIndex/chapter-range/",
		component: oi
	},
	{
		path: "/settings/",
		component: bi
	}
];
//#endregion
//#region src/popup/App.svelte
function Si(e, t) {
	y(t, !1);
	let n = o(t, "navigation", 28, () => ({
		key: null,
		route: "/"
	})), r = matchMedia("(prefers-color-scheme: dark)"), i = I(r.matches), s = {
		name: "yt-dlp-gui",
		id: "me.kannagi.ytdlpgui.extension",
		theme: "md",
		touch: he,
		routes: xi,
		smartSelect: {
			openIn: "page",
			closeOnSelect: !0,
			scrollToSelectedItem: !0,
			searchbar: !1
		}
	};
	b(() => {
		let e = (e) => {
			W(i, e.matches);
		};
		r.addEventListener("change", e);
		let t = !1, a = null, o = 0, s = $.subscribe((e) => {
			e.pageResetRevision !== o && (o = e.pageResetRevision, n(n().route = "/", !0), a && a.currentRoute?.url !== "/" && a.back("/", {
				force: !0,
				animate: !1
			}));
		}), c = (e) => {
			t || Ye(chrome.storage.session, n(), e?.url).catch(() => {});
		};
		return v((e) => {
			t || (a = e.views.main?.router, a && (n().route !== "/" && a.navigate(n().route, { animate: !1 }), a.on("routeChange", c)));
		}), $.init(), () => {
			r.removeEventListener("change", e), t = !0, s(), a?.off("routeChange", c), $.destroy();
		};
	}), h(), j(e, a(() => s, {
		get dark() {
			return H(i);
		},
		children: (e, t) => {
			ye(e, {
				main: !0,
				url: "/",
				class: "popup-view"
			});
		},
		$$slots: { default: !0 }
	})), Y();
}
//#endregion
//#region src/popup/main.js
r.use(k);
var Ci = document.getElementById("app"), wi = He(Ci), Ti = await Je(chrome.tabs, chrome.storage.session, (e, t) => chrome.runtime.sendMessage({
	type: "YT_DLP_GUI_ANALYSIS_SNAPSHOT",
	request: {
		sourceTabId: e,
		url: t,
		playlistScope: "current"
	}
})).catch(() => ({
	key: null,
	route: "/"
}));
C(Si, {
	target: Ci,
	props: { navigation: Ti }
}), window.addEventListener("unload", wi, { once: !0 });
//#endregion
