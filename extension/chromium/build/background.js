import { d as e, f as t, i as n, l as r, p as i, r as a, s as o, t as s } from "./chunks/native-protocol-BGhhGYZS.js";
import { t as c } from "./chunks/i18n-SwgWtGUP.js";
import { a as l, c as u, d, i as f, l as p, n as m, o as h, r as g, s as _, t as v } from "./chunks/gallery-timeline-session-CJYptyAy.js";
import { t as ee } from "./chunks/url-matcher-Dt5Hrtu5.js";
import { t as te } from "./chunks/xmp-wasm-runtime-BhhtyP_M.js";
//#region src/shared/native-host-errors.js
var ne = "https://github.com/kannagi0303/yt-dlp-gui-v2/releases/latest";
function re(e) {
	let t = String(e || "").toLowerCase();
	return t.includes("native messaging host not found") || t.includes("specified native messaging host not found") || t.includes("failed to start native messaging host");
}
//#endregion
//#region src/background/gallery-timeline-port.js
function ie(e) {
	chrome.runtime.onConnect.addListener((t) => {
		if (t.name !== "yt-dlp-gui-timeline-browse") return;
		let n = new AbortController(), r = !1, i = (e) => {
			if (!n.signal.aborted) try {
				t.postMessage(e);
			} catch {
				n.abort(new DOMException("Album disconnected", "AbortError"));
			}
		};
		t.onDisconnect.addListener(() => n.abort(new DOMException("Album disconnected", "AbortError"))), t.onMessage.addListener((t) => {
			r || (r = !0, e(t.request, n.signal, (e) => i({
				event: "post",
				page: e
			})).then((e) => i({
				event: "complete",
				page: e
			})).catch((e) => {
				n.signal.aborted || i({
					event: "error",
					message: String(e.message || e)
				});
			}));
		});
	});
}
//#endregion
//#region src/background/browser-download-save.js
async function ae(e, t) {
	let n = new Uint8Array(await e.arrayBuffer()), r = [];
	for (let e = 0; e < n.length; e += 24576) t?.throwIfAborted(), r.push(btoa(String.fromCharCode(...n.subarray(e, e + 24576))));
	return `data:${/^[\w.+-]+\/[\w.+-]+$/.test(e.type) ? e.type : "application/octet-stream"};base64,${r.join("")}`;
}
async function oe(e, t, n, r) {
	n?.throwIfAborted();
	let i = await ae(e, n);
	n?.throwIfAborted();
	let a = await chrome.downloads.download({
		url: i,
		filename: t,
		conflictAction: "uniquify",
		saveAs: !1
	});
	if (!Number.isInteger(a)) throw Error("Browser returned no download ID.");
	try {
		await r(a), n?.throwIfAborted(), await new Promise((e, t) => {
			let r = () => {
				chrome.downloads.onChanged.removeListener(o), n?.removeEventListener("abort", s);
			}, i = (n) => {
				n === "complete" ? (r(), e()) : n === "interrupted" && (r(), t(Error(c("browserDownloadInterrupted"))));
			}, o = (e) => {
				e.id === a && i(e.state?.current);
			}, s = () => {
				r(), t(n.reason);
			};
			chrome.downloads.onChanged.addListener(o), n?.addEventListener("abort", s, { once: !0 }), chrome.downloads.search({ id: a }).then((e) => i(e[0]?.state)).catch((e) => {
				r(), t(e);
			}), n?.aborted && s();
		});
	} catch (e) {
		throw n?.aborted && await chrome.downloads.cancel(a).catch(() => {}), e;
	}
}
//#endregion
//#region src/background/browser-download-jobs.js
var y = "browserDownloadTask", se = "YT_DLP_GUI_BROWSER_DOWNLOAD_EVENT", b = (e) => ["running", "stopping"].includes(e?.state);
function ce({ readTimeline: e } = {}) {
	let t = { state: "idle" }, n, r = !1, i, a = Promise.resolve(), o = chrome.storage.session.get(y).then(async (e) => {
		t = e[y] || t, b(t) && await s({
			state: "failed",
			message: c("browserJobInterrupted")
		});
	});
	async function s(e) {
		e.state && e.state !== t.state && u("browser-download.state", {
			traceId: t.traceId,
			status: e.state,
			completedItems: t.completedItems || 0
		}), t = {
			...t,
			...e,
			revision: (t.revision || 0) + 1
		};
		let n = { ...t }, r = a.catch(() => {}).then(async () => {
			await chrome.storage.session.set({ [y]: n }), await chrome.runtime.sendMessage({
				type: se,
				task: n
			}).catch(() => {});
		});
		a = r, await r;
	}
	async function h() {
		return i ||= fetch(chrome.runtime.getURL("resources/gallery-xmp.wasm")).then((e) => {
			if (!e.ok) throw Error("XMP WASM unavailable");
			return e.arrayBuffer();
		}).then(te).catch((e) => {
			throw i = null, e;
		}), i;
	}
	async function ee(r, i) {
		let a = setInterval(() => chrome.runtime.getPlatformInfo().catch(() => {}), 25e3), o = 0;
		try {
			for (let n of r) {
				i.throwIfAborted(), await s({
					currentSourceItemKey: n.key,
					title: n.gallery.title || "gallery-dl"
				});
				let r = n.gallery;
				if (r.timeline) {
					if (!e) throw Error("Timeline reader unavailable.");
					for (; r.timeline.hasMore;) i.throwIfAborted(), await s({ message: c("timelineReadProgress", { number: r.timeline.postStart }) }), r = v(r, await e(m(r), i));
				}
				let a = p(r);
				if (!a?.length) throw Error(c("browserDownloadNoMedia"));
				await f({
					gallery: r,
					media: a,
					selected: new Set(a.map((e, t) => t)),
					asZip: n.mode === "browser-zip",
					permissionsAlreadyGranted: !0,
					signal: i,
					session: { prepare: async (e, t) => {
						let n = await g(e, {
							...t,
							writeXmp: async (e, t) => (await h())(e, t)
						});
						return {
							blob: new Blob([n.bytes], { type: n.mime }),
							xmp: n.xmp
						};
					} },
					save: (e, n, r) => oe(e, n, r, (e) => s({ downloadIds: [...t.downloadIds, e] })),
					progress: (e) => s({ message: e })
				}), o++, await s({
					completedItems: o,
					completedSourceItemKeys: [...t.completedSourceItemKeys, n.key]
				});
			}
			await s({ state: "completed" });
		} catch (e) {
			await s({
				state: i.aborted ? "stopped" : "failed",
				message: i.aborted ? c("downloadStopped") : String(e?.message || e)
			});
		} finally {
			clearInterval(a), n = null;
		}
	}
	async function ne(e) {
		if (await o, r || n || b(t)) throw Error(c("downloadAlreadyRunning"));
		r = !0;
		try {
			let r = e.jobs;
			if (!Array.isArray(r) || !r.length || r.length > 100) throw Error(c("downloadListInvalid"));
			for (let e of r) {
				let t = p(e.gallery);
				if (!["browser-files", "browser-zip"].includes(e.mode) || !d(t)) throw Error("Invalid browser download item.");
				t.forEach(l);
				let n = [...new Set(t.map((e) => new URL(e.url).origin + "/*"))];
				if (!await chrome.permissions.contains({ origins: n })) throw Error(c("mediaPermissionRequired"));
			}
			n = new AbortController(), t = {
				state: "running",
				revision: t.revision || 0,
				sourceTabId: e.sourceTabId,
				sourcePageUrl: e.sourcePageUrl,
				traceId: _(),
				sourceAnalysisStartedAt: e.sourceAnalysisStartedAt,
				itemCount: r.length,
				completedItems: 0,
				completedSourceItemKeys: [],
				downloadIds: [],
				message: c("downloadPreparing")
			}, u("browser-download.start", {
				traceId: t.traceId,
				itemCount: r.length
			});
			try {
				await s({});
			} catch (e) {
				throw n = null, t = {
					...t,
					state: "failed",
					message: c("browserJobStateFailed")
				}, e;
			}
			let i = { ...t };
			return ee(r, n.signal).catch((e) => console.error("Browser download state persistence failed:", e.name)), {
				ok: !0,
				task: i
			};
		} finally {
			r = !1;
		}
	}
	async function re() {
		return await o, n && (await s({ state: "stopping" }), n.abort(new DOMException("Download stopped", "AbortError"))), {
			ok: !0,
			task: { ...t }
		};
	}
	return {
		start: ne,
		stop: re,
		status: async () => (await o, {
			ok: !0,
			task: { ...t }
		})
	};
}
//#endregion
//#region src/background/gallery-timeline-reader.js
function x(n, r) {
	let i = t(n, r.url);
	if (!i.length) return [];
	let a = e(i, r.siteName, r.url);
	return a.posts || [a];
}
function le(e) {
	return async function(t, n, r = () => {}) {
		n?.throwIfAborted();
		let i = await e({
			action: "galleryRead",
			backend: "g",
			url: t.url,
			sourceTabId: t.sourceTabId,
			timelineRange: o(t.timelineRange),
			postStart: t.postStart || 1,
			...t.postCount == null ? {} : { postCount: t.postCount }
		});
		return n?.throwIfAborted(), new Promise((e, a) => {
			let s = Date.now();
			u("timeline.start", {
				traceId: i.traceId,
				nextStart: t.postStart || 1,
				postCount: t.postCount
			});
			let l = chrome.runtime.connectNative("kannagi0303.yt_dlp_gui"), d = [], f = !1, p = !1, m = Promise.resolve(), h = t.postStart || 1, g = (t, r) => {
				f || (f = !0, p = !0, u("timeline.finish", {
					traceId: i.traceId,
					elapsedMs: Date.now() - s,
					status: n?.aborted ? "cancelled" : t ? "failed" : "completed",
					nextStart: h,
					itemCount: d.length
				}), n?.removeEventListener("abort", v), l.disconnect(), t ? a(t) : e(r));
			}, _ = (e, t) => {
				p || (p = !0, m.then(() => g(e, t), (e) => g(e)));
			}, v = () => g(n.reason);
			n?.addEventListener("abort", v, { once: !0 }), l.onMessage.addListener((e) => {
				if (p) return;
				if (!e?.ok) {
					_(Error(e?.message || c("timelineReadFailed")));
					return;
				}
				let n = e.data;
				try {
					if (n?.event === "item" && d.push(n.item), n?.event === "post") {
						if (!Array.isArray(n.items) || n.nextStart !== h + 1) throw Error("Non-contiguous timeline cursor.");
						h = n.nextStart, d.push(...n.items), u("timeline.post", {
							traceId: i.traceId,
							nextStart: h,
							itemCount: n.items.length
						});
						let e = {
							posts: x(n.items, t),
							timelineRange: o(n.timelineRange),
							nextStart: h,
							hasMore: !0
						};
						m = m.then(() => {
							if (!f) return r(e);
						}), m.catch((e) => g(e));
					}
					if (n?.event === "complete") {
						if (!Number.isInteger(n.nextStart) || n.nextStart < h || typeof n.hasMore != "boolean") throw Error("Incomplete timeline completion response.");
						_(null, {
							posts: x(d, t),
							timelineRange: o(n.timelineRange),
							nextStart: n.nextStart,
							hasMore: n.hasMore
						});
					}
					n?.event === "stopped" && _(Error(c("analysisStoppedStatus")));
				} catch (e) {
					_(e);
				}
			}), l.onDisconnect.addListener(() => {
				p || _(Error(chrome.runtime.lastError?.message || c("nativeDisconnected")));
			}), l.postMessage(i), n?.aborted && v();
		});
	};
}
//#endregion
//#region src/background/analysis-retry.js
function S(e) {
	return e.request.retryBackend === "y" ? ["yt-dlp"] : e.request.retryBackend === "g" ? ["gallery-dl"] : ["yt-dlp", "gallery-dl"];
}
function ue(e, t, n) {
	if (e.retryBackend == null) return null;
	let r = e.retryBackend, i = r === "y" ? "yt-dlp" : r === "g" ? "gallery-dl" : null;
	if (!i || e.analysisTools?.length !== 1 || e.analysisTools[0] !== r || t?.data?.request?.schemaVersion !== 22 || !["failed", "interrupted"].includes(t.data[i]?.status)) throw Error("Only a failed analyzer in the current source document can be retried.");
	let a = structuredClone(t.data);
	return a.request = {
		...a.request,
		status: "analyzing",
		startedAt: n,
		updatedAt: n,
		retryBackend: r,
		analysisTools: e.analysisTools,
		routed: !1
	}, delete a.request.message, delete a.request.cachedAt, a[i] = {
		status: "pending",
		info: {},
		items: []
	}, a;
}
function C(e, t) {
	if (!e.request.retryBackend || t === "stopped") return t;
	let n = [e["yt-dlp"]?.status, e["gallery-dl"]?.status], r = n.some((e) => ["failed", "interrupted"].includes(e));
	return n.some((e) => ["found", "collection"].includes(e)) ? r ? "partial" : "completed" : r ? "failed" : n.every((e) => ["disabled", "notMatched"].includes(e)) ? "notMatched" : "notFound";
}
//#endregion
//#region src/background/tab-document-reset.js
function de() {
	let e = /* @__PURE__ */ new Map(), t = /* @__PURE__ */ new Map(), n = (t) => e.get(t) || 0;
	return {
		revision: n,
		isCurrent: (e, t) => n(e) === t,
		reset(r, i) {
			e.set(r, n(r) + 1);
			let a = (t.get(r) || Promise.resolve()).catch(() => {}).then(i);
			t.set(r, a);
			let o = () => {
				t.get(r) === a && t.delete(r);
			};
			return a.then(o, o), a;
		},
		async wait(e) {
			for (; t.has(e);) await t.get(e);
		}
	};
}
//#endregion
//#region src/background/background.js
var w = le(R);
ie(w);
var T = ce({ readTimeline: w }), E = "kannagi0303.yt_dlp_gui", fe = "YT_DLP_GUI_TASK_EVENT", pe = "YT_DLP_GUI_ANALYSIS_EVENT", me = {
	16: "/icons/app-enabled-16.png",
	32: "/icons/app-enabled-32.png"
}, he = {
	16: "/icons/app-disabled-16.png",
	32: "/icons/app-disabled-32.png"
}, ge = {
	analyzing: {
		badge: "",
		color: "#1f5fbf",
		label: c("analyzingFormats")
	},
	analyzed: {
		badge: "",
		color: "#1f5fbf",
		label: c("formatsAnalyzed")
	},
	downloading: {
		badge: "",
		color: "#287a47",
		label: c("downloading")
	},
	completed: {
		badge: "",
		color: "#287a47",
		label: c("downloadCompleted")
	},
	failed: {
		badge: "!",
		color: "#a83a3a",
		label: c("operationFailed")
	}
};
function _e(e) {
	let t = Math.max(0, Math.floor(Number(e) || 0));
	return t > 99 ? "99+" : t ? String(t) : "";
}
var ve = fetch(chrome.runtime.getURL("resources/extractor-index.json")).then((e) => {
	if (!e.ok) throw Error(c("matcherIndexLoadFailed"));
	return e.json();
}).then(ee);
ve.catch(() => {});
function D(e, t) {
	let n = String(e || "");
	return re(n) ? {
		code: "nativeHostUnavailable",
		message: c("nativeHostUnavailable"),
		helpUrl: ne
	} : { message: n || c(t) };
}
function O(e) {
	String(e?.message || e).includes("No tab with id") || console.error("[toolbar-state]", e);
}
async function k(e, t) {
	let n = !1, r = !1;
	try {
		let e = new URL(t);
		["http:", "https:"].includes(e.protocol) && e.hostname && (n = !0, r = (await ve).classify(t).kind === "known");
	} catch {}
	let i = `actionState:tab:${e}`, a = (n ? await chrome.storage.session.get(i) : {})[i], o = typeof a == "string" ? a : a?.operation, s = ge[o], l = o === "analyzed" || o === "completed" ? _e(a?.count) : s?.badge || "";
	await chrome.action.setIcon({
		tabId: e,
		path: r ? me : he
	}), await chrome.action.setBadgeText({
		tabId: e,
		text: n ? l : ""
	}), s && await chrome.action.setBadgeBackgroundColor({
		tabId: e,
		color: s.color
	}), await chrome.action.setTitle({
		tabId: e,
		title: n ? s?.label || c(r ? "supportedTitle" : "genericTitle") : c("unsupportedTitle")
	}), await (n ? chrome.action.enable(e) : chrome.action.disable(e));
}
async function A(e, t, n = 0) {
	if (!Number.isInteger(e)) return;
	let r = `actionState:tab:${e}`;
	t ? await chrome.storage.session.set({ [r]: {
		operation: t,
		count: n
	} }) : await chrome.storage.session.remove(r);
	try {
		await k(e, (await chrome.tabs.get(e)).url);
	} catch (e) {
		O(e);
	}
}
function ye(e) {
	return [
		"analysis",
		"choices",
		"selection",
		"galleryModes",
		"galleryRanges",
		"popupRoute"
	].map((t) => `${t}:tab:${e}:`);
}
async function be() {
	let e = (e) => e === "analysisCacheIndex" || e.startsWith("analysisResult:v1:") || /^(analysis|choices|selection|galleryModes):(current|playlist):/.test(e);
	for (let t of [chrome.storage.session, chrome.storage.local]) {
		let n = await t.get(null), r = Object.keys(n).filter(e);
		r.length && await t.remove(r);
	}
}
function j(e, t) {
	return e.startsWith(`analysis:tab:${t}:`);
}
function M(e, t) {
	return e.endsWith(`:${t}`) || e.endsWith(`:${t}:y`) || e.endsWith(`:${t}:g`);
}
function N(e, t = null) {
	let n = /* @__PURE__ */ new Set();
	for (let n of /* @__PURE__ */ new Set([...U.keys(), ...K.keys()])) !j(n, e) || t !== null && M(n, t) || (q.add(n), U.delete(n));
	for (let [r, i] of W) !j(r, e) || t !== null && M(r, t) || (q.add(r), U.delete(r), W.delete(r), n.add(i));
	for (let e of n) try {
		e.postMessage({ action: "cancelAnalysis" }), e.disconnect();
	} catch {}
}
async function P(e, t = null) {
	await Promise.all([...K].filter(([n]) => j(n, e) && (t === null || !M(n, t))).map(([, e]) => e.catch(() => {})));
}
async function F(e) {
	N(e), await P(e);
	let t = ye(e), n = await chrome.storage.session.get(null), r = Object.keys(n).filter((n) => n === `actionState:tab:${e}` || t.some((e) => n.startsWith(e)));
	r.length && await chrome.storage.session.remove(r);
}
async function xe(e, t) {
	N(e, t), await P(e, t);
	let n = ye(e), r = await chrome.storage.session.get(null), i = Object.keys(r).filter((e) => n.some((t) => e.startsWith(t)) && !M(e, t));
	i.length && await chrome.storage.session.remove([...i, `actionState:tab:${e}`]);
}
async function I() {
	let [e] = await chrome.tabs.query({
		active: !0,
		currentWindow: !0
	});
	e?.id != null && await k(e.id, e.url);
}
var L = de();
chrome.tabs.onUpdated.addListener((e, t, n) => {
	let r = !!t.url, a = t.status === "loading", o = null;
	if (a || r) {
		let n = a ? null : i(t.url);
		N(e, n), o = L.reset(e, () => a ? F(e) : xe(e, n)), a && u("analysis.page-reset", { phase: "reload" });
	}
	(async () => {
		o && await o, (a || t.url || t.status === "complete") && await k(e, t.url || n.url);
	})().catch(O);
}), chrome.tabs.onRemoved.addListener((e) => {
	N(e), L.reset(e, () => F(e)).catch(O);
}), chrome.tabs.onActivated.addListener(({ tabId: e }) => {
	chrome.tabs.get(e).then((t) => k(e, t.url)).catch(O);
}), chrome.windows?.onFocusChanged?.addListener(() => I().catch(O)), chrome.runtime.onInstalled.addListener(() => {
	be().catch(O), I().catch(O);
}), chrome.runtime.onStartup.addListener(() => I().catch(O)), I().catch(O);
async function R(e) {
	let t = e.traceId || _();
	u("cookies.start", { traceId: t });
	try {
		let n = await Se({
			...e,
			traceId: t
		});
		return u("cookies.resolved", {
			traceId: t,
			cookieCount: n.browserCookies?.length || 0
		}), n;
	} catch (e) {
		throw u("cookies.failed", { traceId: t }), e;
	}
}
async function Se(e) {
	if (e = a(e), ![
		"analyze",
		"analyzeSelected",
		"analyzeSources",
		"analyzeIndexedSources",
		"galleryRead",
		"download",
		"saveBrowserCookies"
	].includes(e?.action)) return e;
	let t = Number.isInteger(e.sourceTabId) ? e.sourceTabId : null, [r] = t == null ? await chrome.tabs.query({
		active: !0,
		currentWindow: !0
	}) : [null], i = t ?? r?.id;
	if (!Number.isInteger(i)) throw Error(c("sourceTabMissing"));
	let o = r || await chrome.tabs.get(i), s = Date.now(), l = (await chrome.cookies.getAllCookieStores()).find((e) => e.tabIds.includes(i));
	if (!l) throw Error(c("cookieReadFailed"));
	let u = await n(e.url), d = [await chrome.cookies.getAll({
		domain: u,
		storeId: l.id
	})], f = [...new Map(d.flat().map((e) => [`${e.domain}\n${e.path}\n${e.name}`, e])).values()];
	return {
		...e,
		browserCookieContext: {
			incognito: typeof o.incognito == "boolean" ? o.incognito : null,
			snapshotUnixMs: s
		},
		browserCookies: f.map(({ domain: e, hostOnly: t, path: n, secure: r, expirationDate: i, name: a, value: o }) => ({
			domain: e,
			hostOnly: t,
			path: n,
			secure: r,
			expirationDate: i,
			name: a,
			value: o
		}))
	};
}
var z = null, B = { state: "idle" }, V = [], H = [], U = /* @__PURE__ */ new Map(), W = /* @__PURE__ */ new Map();
function G(e) {
	let t = r(e);
	if (!t) throw Error(c("sourceTabMissing"));
	return t.unified;
}
var K = /* @__PURE__ */ new Map(), q = /* @__PURE__ */ new Set();
async function J(e, t) {
	let n = structuredClone(t), r = (K.get(e) || Promise.resolve()).catch(() => {}).then(async () => {
		q.has(e) || (await chrome.storage.session.set({ [e]: n }), q.has(e) || chrome.runtime.sendMessage({
			type: pe,
			key: e,
			response: n
		}).catch(() => {}));
	});
	K.set(e, r);
	try {
		await r;
	} finally {
		K.get(e) === r && K.delete(e);
	}
}
function Ce(e = "pending") {
	return {
		status: e,
		info: {},
		items: []
	};
}
function we(e, t) {
	return {
		request: {
			url: e.url,
			status: "analyzing",
			startedAt: t,
			updatedAt: t,
			playlistScope: e.playlistScope === "playlist" ? "playlist" : "current",
			analysisTools: [
				"analyzeSelected",
				"analyzeSources",
				"analyzeIndexedSources"
			].includes(e.action) ? e.analysisTools : ["y", "g"],
			schemaVersion: 22
		},
		"yt-dlp": Ce([
			"analyzeSelected",
			"analyzeSources",
			"analyzeIndexedSources"
		].includes(e.action) && !e.analysisTools?.includes("y") ? "disabled" : "pending"),
		"gallery-dl": Ce([
			"analyzeSelected",
			"analyzeSources",
			"analyzeIndexedSources"
		].includes(e.action) && !e.analysisTools?.includes("g") ? "disabled" : "pending")
	};
}
function Te(e) {
	return e === "y" ? "yt-dlp" : e === "g" ? "gallery-dl" : null;
}
function Y(e) {
	return (e["yt-dlp"]?.items?.length || 0) + t(e["gallery-dl"]?.items, e.request.url).length;
}
function X(e) {
	return e.request.status === "analyzing" ? "analyzing" : Y(e) ? "analyzed" : ["failed", "partial"].includes(e.request.status) ? "failed" : null;
}
async function Ee(e) {
	await L.wait(e.sourceTabId);
	let t = L.revision(e.sourceTabId), n = G(e), r = (await chrome.storage.session.get(n))[n];
	if (!L.isCurrent(e.sourceTabId, t)) return {
		ok: !0,
		data: null
	};
	let i = U.get(n), a = i || r || null;
	if (u("analysis.cache", {
		traceId: a?.data?.request?.traceId,
		status: i ? "running" : r ? "hit" : "miss"
	}), !i && a?.data?.request?.schemaVersion !== 22) return {
		ok: !0,
		data: null
	};
	if (!i && a?.ok && a.data?.request?.status === "analyzing") {
		let r = structuredClone(a);
		r.data.request.status = "interrupted", r.data.request.updatedAt = Math.max(Date.now(), Number(r.data.request.updatedAt || 0) + 1);
		for (let e of ["yt-dlp", "gallery-dl"]) ["pending", "analyzing"].includes(r.data[e]?.status) && (r.data[e].status = "interrupted");
		return await J(n, r), L.isCurrent(e.sourceTabId, t) ? {
			ok: !0,
			data: r
		} : {
			ok: !0,
			data: null
		};
	}
	return {
		ok: !0,
		data: a
	};
}
function De(e) {
	if (["found", "notFound"].includes(e.status)) for (let t of e.items) t?.analysisProbeComplete === !1 && (t.analysisProbeComplete = !0);
}
function Oe(e, t) {
	let n = t?.data || {};
	if (t?.ok && !e.request.routed && n.event !== "plan") {
		e.request.status = "failed", e.request.message = c("nativeAnalysisUpgradeRequired");
		for (let t of S(e)) e[t].status !== "disabled" && (e[t].status = "failed", e[t].info.message = e.request.message);
		return e.request.status = C(e, e.request.status), !0;
	}
	if (!t?.ok) {
		e.request.status = "failed", e.request.message = !e.request.routed && /action is not supported/i.test(t?.message || "") ? c("nativeAnalysisUpgradeRequired") : t?.message || c("nativeAnalysisFailed");
		for (let t of S(e)) ["pending", "analyzing"].includes(e[t].status) && (e[t].status = "failed", e[t].info.message = e.request.message);
		return e.request.status = C(e, e.request.status), !0;
	}
	if (n.event === "plan" && n.result) {
		e.request = {
			...e.request,
			...n.result.request,
			routed: !0
		};
		for (let t of S(e)) e[t].status = n.result[t]?.status || "notMatched", e[t].info = { ...n.result[t]?.info || {} };
		return n.protocol && (e.request.host = s({ protocol: n.protocol })), !1;
	}
	let r = Te(n.backend);
	if (r && e[r] && S(e).includes(r)) {
		let t = e[r];
		n.event === "item" && n.item ? (t.items[n.index ?? t.items.length] = n.item, t.status = "analyzing") : n.event === "backendResult" && n.result ? (t.status = n.result.status || "failed", t.info = {
			...t.info,
			...n.result.info || {}
		}, r === "gallery-dl" && De(t)) : (n.event === "phase" || n.event === "started") && (t.status = "analyzing", t.info = {
			...t.info,
			...n
		}, n.protocol && (e.request.host = s(n))), n.outputDirectory && chrome.storage.local.set({ v2DownloadDirectory: n.outputDirectory }).catch(O);
	}
	if (n.event === "result" && n.result) {
		e.request = {
			...e.request,
			...n.result.request
		};
		for (let t of S(e)) {
			let r = n.result[t] || {};
			e[t] = {
				...e[t],
				status: r.status || "failed",
				info: {
					...e[t].info,
					...r.info || {}
				},
				items: e[t].items
			}, t === "gallery-dl" && De(e[t]), e[t].info.outputDirectory && chrome.storage.local.set({ v2DownloadDirectory: e[t].info.outputDirectory }).catch(O);
		}
		return e.request.status = C(e, e.request.status), !0;
	}
	return !1;
}
async function ke(e) {
	await L.wait(e.sourceTabId);
	let t = L.revision(e.sourceTabId), n = () => {
		if (!L.isCurrent(e.sourceTabId, t)) throw Error(c("analysisStoppedStatus"));
	}, r = G(e), i = U.get(r);
	if (i) return {
		ok: !0,
		message: c("analysisAlreadyRunning"),
		data: structuredClone(i)
	};
	let a = (await chrome.storage.session.get(r))[r];
	if (n(), e.retryBackend && a?.ok && a.data?.request?.schemaVersion === 22) {
		let t = Te(e.retryBackend);
		if (t && (!["failed", "interrupted"].includes(a.data[t]?.status) || e.retryStartedAt != null && e.retryStartedAt !== a.data.request.startedAt)) return {
			ok: !0,
			data: structuredClone(a)
		};
	}
	let o = Math.max(Date.now(), Number(a?.data?.request?.startedAt || 0) + 1), s = {
		ok: !0,
		message: c("analysisStarted"),
		data: ue(e, a, o) || we(e, o)
	}, { backend: l, backends: d, retryBackend: f, retryStartedAt: p, ...m } = e, h = await R(m);
	n();
	let g = U.get(r);
	if (g) return {
		ok: !0,
		message: c("analysisAlreadyRunning"),
		data: structuredClone(g)
	};
	u("analysis.start", {
		traceId: h.traceId,
		backend: e.retryBackend
	}), s.data.request.traceId = h.traceId, q.delete(r), U.set(r, s), await J(r, s), n(), await A(e.sourceTabId, "analyzing"), n();
	let _ = chrome.runtime.connectNative(E);
	W.set(r, _);
	let v = Promise.resolve();
	return _.onMessage.addListener((t) => (v = v.then(async () => {
		if (U.get(r) !== s) return;
		let n = t?.data;
		([
			"phase",
			"started",
			"backendResult",
			"result",
			"complete",
			"error"
		].includes(n?.event) || !t?.ok) && u("analysis.frame", {
			traceId: h.traceId,
			backend: n?.backend,
			phase: n?.phase,
			status: t?.ok ? n?.event : "failed",
			itemCount: n?.itemCount
		});
		let i = Oe(s.data, t);
		s.data.request.updatedAt = Math.max(Date.now(), s.data.request.updatedAt + 1), i && (u("analysis.finish", {
			traceId: h.traceId,
			elapsedMs: Date.now() - o,
			status: X(s.data),
			itemCount: Y(s.data)
		}), s.data.request.cachedAt = Date.now(), U.delete(r), W.delete(r)), await J(r, s), await A(e.sourceTabId, X(s.data), Y(s.data)), i && !t?.ok && _.disconnect();
	}).catch((e) => console.error("[analysis-frame]", e)), v)), _.onDisconnect.addListener(async () => {
		if (await v, U.get(r) !== s) return;
		let t = D(chrome.runtime.lastError?.message, "analysisDisconnected");
		u("analysis.disconnect", {
			traceId: h.traceId,
			elapsedMs: Math.max(0, Date.now() - o)
		}), s.data.request.status = "interrupted", s.data.request.updatedAt = Math.max(Date.now(), s.data.request.updatedAt + 1), s.data.request.message = t.message, s.helpUrl = t.helpUrl;
		for (let e of ["yt-dlp", "gallery-dl"]) ["pending", "analyzing"].includes(s.data[e].status) && (s.data[e].status = "interrupted", s.data[e].info.message = t.message);
		U.delete(r), W.delete(r), await J(r, s), await A(e.sourceTabId, X(s.data), Y(s.data));
	}), _.postMessage(h), {
		ok: !0,
		message: c("analysisStarted"),
		data: structuredClone(s)
	};
}
async function Ae(e) {
	let t = G(e), n = W.get(t), r = U.get(t);
	if (!n || !r) return {
		ok: !1,
		message: c("analysisNotRunning")
	};
	r.data.request.status = "stopped", r.data.request.updatedAt = Math.max(Date.now(), r.data.request.updatedAt + 1);
	for (let e of ["yt-dlp", "gallery-dl"]) ["pending", "analyzing"].includes(r.data[e].status) && (r.data[e].status = "stopped");
	return U.delete(t), W.delete(t), await J(t, r), await A(e.sourceTabId, X(r.data), Y(r.data)), n.postMessage({ action: "cancelAnalysis" }), {
		ok: !0,
		message: c("analysisStoppedStatus"),
		data: structuredClone(r)
	};
}
async function Z(e) {
	e.state && e.state !== B.state && u("download.state", {
		traceId: B.traceId,
		status: e.state
	}), B = {
		...B,
		...e
	}, await chrome.storage.session.set({ activeTask: B }), chrome.runtime.sendMessage({
		type: fe,
		task: B
	}).catch(() => {});
}
async function Q(e) {
	await Z({
		state: "failed",
		...typeof e == "string" ? { message: e } : e,
		finishedAt: Date.now()
	}), await A(B.sourceTabId, "failed"), z = null, V = [], H = [];
}
async function $(e) {
	if (B.id !== e || !["running", "stopping"].includes(B.state)) return;
	z = null, V = [];
	let t = [...H];
	H = [], await Z({
		state: "stopped",
		outputPath: t.at(-1) || "",
		outputPaths: t,
		message: c("downloadStopped"),
		finishedAt: Date.now()
	}), await A(B.sourceTabId, "analyzed", B.itemCount);
}
async function je() {
	if (!["running", "stopping"].includes(B.state)) return {
		ok: !1,
		message: c("downloadNotRunning")
	};
	if (B.state === "running") {
		if (V = [], await Z({
			state: "stopping",
			message: c("downloadStopping")
		}), z) {
			let e = z;
			try {
				e.postMessage({ action: "cancelDownload" });
			} catch {
				try {
					e.disconnect();
				} catch {}
				await $(B.id);
			}
		} else await $(B.id);
	}
	return {
		ok: !0,
		message: c("downloadStopping"),
		data: { task: B }
	};
}
async function Me(e) {
	if (B.id !== e || B.state !== "running") return;
	let t = V.shift();
	if (!t) return;
	let n = B.itemCount - V.length;
	await Z({
		title: t.title || t.url,
		traceId: t.traceId,
		url: t.url,
		itemIndex: n,
		currentSourceItemIndex: t.sourceItemIndex,
		currentSourceItemKey: t.sourceItemKey,
		downloaded: "",
		total: "",
		speed: "",
		eta: "",
		itemPercent: 0,
		message: c("downloadPreparing")
	});
	let r = chrome.runtime.connectNative(E);
	z = r, r.onMessage.addListener(async (n) => {
		if (B.id !== e || z !== r) return;
		if (!n?.ok) {
			B.state === "stopping" ? await $(e) : await Q(n?.message || c("nativeDownloadFailed")), r.disconnect();
			return;
		}
		let i = n.data || {}, a = s(i);
		if (a.advertised && await Z({ host: a }), i.event === "stopped") {
			await $(e);
			return;
		}
		if (B.state === "stopping") {
			i.event === "complete" && i.outputPath && H.push(i.outputPath), await $(e);
			return;
		}
		if (i.event === "progress") {
			let e = Number.isFinite(i.percent) ? i.percent : 0;
			await Z({
				state: "running",
				percent: (B.completedItems + e / 100) / B.itemCount * 100,
				itemPercent: e,
				downloaded: i.downloaded || "",
				total: i.total || "",
				speed: i.speed || "",
				eta: i.eta || "",
				message: c("downloading")
			});
		} else if (i.event === "complete") {
			i.outputPath && H.push(i.outputPath), z = null;
			let n = B.completedItems + 1, r = t.sourceItemKey ? [...B.completedSourceItemKeys, t.sourceItemKey] : B.completedSourceItemKeys;
			V.length ? (await Z({
				completedItems: n,
				percent: n / B.itemCount * 100,
				outputPaths: [...H],
				completedSourceItemKeys: r
			}), await Me(e)) : (await Z({
				state: "completed",
				completedItems: n,
				percent: 100,
				outputPath: H.at(-1) || "",
				outputPaths: [...H],
				completedSourceItemKeys: r,
				itemPercent: 100,
				message: c("downloadCompleted"),
				finishedAt: Date.now()
			}), await A(B.sourceTabId, "completed", n), H = []);
		}
	}), r.onDisconnect.addListener(async () => {
		let t = chrome.runtime.lastError?.message;
		B.id === e && z === r && (B.state === "stopping" ? await $(e) : B.state === "running" && await Q(D(t, "nativeDisconnected")));
	}), r.postMessage(t);
}
async function Ne(e) {
	if (["running", "stopping"].includes(B.state)) return {
		ok: !1,
		message: c("downloadAlreadyRunning")
	};
	if (!Array.isArray(e) || !e.length || e.length > 100 || e.some((e) => e?.action !== "download" || !e.url)) return {
		ok: !1,
		message: c("downloadListInvalid")
	};
	let t = [];
	for (let n of e) t.push(await R(n));
	let n = crypto.randomUUID(), r = t[0];
	return B = {
		id: n,
		state: "running",
		title: r.title || r.url,
		url: r.url,
		sourcePageUrl: r.sourcePageUrl || r.url,
		sourceTabId: r.sourceTabId,
		sourceAnalysisStartedAt: r.sourceAnalysisStartedAt,
		itemCount: t.length,
		itemIndex: 1,
		completedItems: 0,
		completedSourceItemKeys: [],
		currentSourceItemIndex: r.sourceItemIndex,
		currentSourceItemKey: r.sourceItemKey,
		itemPercent: 0,
		percent: 0,
		downloaded: "",
		total: "",
		speed: "",
		eta: "",
		message: c("downloadPreparing"),
		startedAt: Date.now()
	}, V = t, H = [], await Z(B), await A(B.sourceTabId, "downloading"), await Me(n), {
		ok: !0,
		message: c("downloadStarted"),
		data: { task: B }
	};
}
async function Pe(e) {
	return Ne([e]);
}
function Fe(e, t) {
	chrome.runtime.sendNativeMessage(E, e, async (e) => {
		if (chrome.runtime.lastError) {
			t({
				ok: !1,
				message: chrome.runtime.lastError.message
			});
			return;
		}
		try {
			let n = e?.data?.outputDirectory || e?.data?.directory;
			n && await chrome.storage.local.set({ v2DownloadDirectory: n }), t(e || {
				ok: !1,
				message: c("nativeNoData")
			});
		} catch (e) {
			t({
				ok: !1,
				message: e.message
			});
		}
	});
}
async function Ie(e, t = null) {
	let n = p(e);
	if (!d(n)) return {
		ok: !1,
		message: "Invalid gallery media."
	};
	if (t != null && !["files", "zip"].includes(t)) return {
		ok: !1,
		message: "Invalid gallery download mode."
	};
	let r = crypto.randomUUID();
	return await chrome.storage.session.set({
		[`galleryAlbum:${r}`]: e,
		...t ? { [`galleryAutoDownload:${r}`]: t } : {}
	}), await chrome.tabs.create({ url: chrome.runtime.getURL(`album.html?id=${r}${t ? `&download=${t}` : ""}`) }), { ok: !0 };
}
chrome.runtime.onMessage.addListener((e, t, n) => {
	if (e?.type === "GET_OPERATION_DIAGNOSTICS") return h().then((e) => n({
		ok: !0,
		data: e
	})).catch(() => n({
		ok: !1,
		message: "Diagnostic journal unavailable."
	})), !0;
	if (e?.type === "YT_DLP_GUI_TIMELINE_READ") return w(e.request).then((e) => n({
		ok: !0,
		data: e
	})).catch((e) => n({
		ok: !1,
		message: e.message
	})), !0;
	let r = {
		YT_DLP_GUI_BROWSER_DOWNLOAD_START: () => T.start(e),
		YT_DLP_GUI_BROWSER_DOWNLOAD_STATUS: () => T.status(),
		YT_DLP_GUI_BROWSER_DOWNLOAD_STOP: () => T.stop()
	}[e?.type];
	return r ? (r().then(n).catch((e) => n({
		ok: !1,
		message: String(e?.message || e)
	})), !0) : e?.type === "YT_DLP_GUI_OPEN_GALLERY" ? (Ie(e.gallery, e.autoDownload).then(n).catch((e) => n({
		ok: !1,
		message: e.message
	})), !0) : e?.type === "YT_DLP_GUI_ANALYSIS_SNAPSHOT" ? (Ee(e.request).then(n).catch((e) => n({
		ok: !1,
		message: String(e?.message || e)
	})), !0) : e?.type === "YT_DLP_GUI_TASK_STATUS" ? (chrome.storage.session.get("activeTask").then((e) => {
		n({
			ok: !0,
			data: { task: e.activeTask || B }
		});
	}), !0) : e?.type === "YT_DLP_GUI_NATIVE_REQUEST" ? e.request?.action === "cancelDownload" ? (je().then(n).catch((e) => n({
		ok: !1,
		message: e.message || c("downloadStopFailed")
	})), !0) : e.request?.action === "cancelAnalysis" ? (Ae(e.request).then(n).catch((e) => n({
		ok: !1,
		message: e.message || c("analysisStopFailed")
	})), !0) : [
		"analyze",
		"analyzeSelected",
		"analyzeSources",
		"analyzeIndexedSources"
	].includes(e.request?.action) ? (ke(e.request).then(n).catch((e) => n({
		ok: !1,
		message: e.message || c("analysisStartFailed")
	})), !0) : e.request?.action === "download" ? (Pe(e.request).then(n).catch(async (e) => {
		let t = z, r = D(e.message, "nativeConnectFailed");
		await Q(r), t && t.disconnect(), n({
			ok: !1,
			...r
		});
	}), !0) : e.request?.action === "downloadBatch" ? (Ne(e.request.requests).then(n).catch(async (e) => {
		let t = z, r = D(e.message, "nativeConnectFailed");
		await Q(r), t && t.disconnect(), n({
			ok: !1,
			...r
		});
	}), !0) : (R(e.request).then((e) => Fe(e, n)).catch(() => n({
		ok: !1,
		message: c("cookieReadFailed")
	})), !0) : !1;
});
//#endregion
