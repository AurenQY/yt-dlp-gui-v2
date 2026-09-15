import { t as e } from "./chunks/i18n-5PmJGYvv.js";
import { r as t, t as n } from "./chunks/native-protocol-DU3catTY.js";
//#region src/shared/native-host-errors.js
var r = "https://github.com/kannagi0303/yt-dlp-gui-v2/releases/latest";
function i(e) {
	let t = String(e || "").toLowerCase();
	return t.includes("native messaging host not found") || t.includes("specified native messaging host not found") || t.includes("failed to start native messaging host");
}
//#endregion
//#region src/background/background.js
var a = "kannagi0303.yt_dlp_gui", o = "YT_DLP_GUI_TASK_EVENT", s = "YT_DLP_GUI_ANALYSIS_EVENT", c = 9, l = {
	16: "/icons/app-enabled-16.png",
	32: "/icons/app-enabled-32.png"
}, u = {
	16: "/icons/app-disabled-16.png",
	32: "/icons/app-disabled-32.png"
}, d = {
	analyzing: {
		badge: "",
		color: "#1f5fbf",
		label: e("analyzingFormats")
	},
	analyzed: {
		badge: "",
		color: "#1f5fbf",
		label: e("formatsAnalyzed")
	},
	downloading: {
		badge: "",
		color: "#287a47",
		label: e("downloading")
	},
	completed: {
		badge: "",
		color: "#287a47",
		label: e("downloadCompleted")
	},
	failed: {
		badge: "!",
		color: "#a83a3a",
		label: e("operationFailed")
	}
};
function f(e) {
	let t = Math.max(0, Math.floor(Number(e) || 0));
	return t > 99 ? "99+" : t ? String(t) : "";
}
var p = fetch(chrome.runtime.getURL("resources/supported-hosts.json")).then((e) => {
	if (!e.ok) throw Error("Could not load the yt-dlp host index.");
	return e.json();
}).then((e) => new Set(e.hosts));
function m(t, n) {
	let a = String(t || "");
	return i(a) ? {
		code: "nativeHostUnavailable",
		message: e("nativeHostUnavailable"),
		helpUrl: r
	} : { message: a || e(n) };
}
function h(e) {
	String(e?.message || e).includes("No tab with id") || console.error("[toolbar-state]", e);
}
async function g(t, n) {
	let r = !1, i = !1;
	try {
		let e = new URL(n);
		if (["http:", "https:"].includes(e.protocol) && e.hostname) {
			r = !0;
			let t = await p, n = e.hostname.toLowerCase();
			for (; n;) {
				if (t.has(n)) {
					i = !0;
					break;
				}
				let e = n.indexOf(".");
				if (e < 0) break;
				n = n.slice(e + 1);
			}
		}
	} catch {}
	let a = `actionState:tab:${t}`, o = (r ? await chrome.storage.session.get(a) : {})[a], s = typeof o == "string" ? o : o?.operation, c = d[s], m = s === "analyzed" || s === "completed" ? f(o?.count) : c?.badge || "";
	await chrome.action.setIcon({
		tabId: t,
		path: i ? l : u
	}), await chrome.action.setBadgeText({
		tabId: t,
		text: r ? m : ""
	}), c && await chrome.action.setBadgeBackgroundColor({
		tabId: t,
		color: c.color
	}), await chrome.action.setTitle({
		tabId: t,
		title: r ? c?.label || e(i ? "supportedTitle" : "genericTitle") : e("unsupportedTitle")
	}), await (r ? chrome.action.enable(t) : chrome.action.disable(t));
}
async function _(e, t, n = 0) {
	if (!Number.isInteger(e)) return;
	let r = `actionState:tab:${e}`;
	t ? await chrome.storage.session.set({ [r]: {
		operation: t,
		count: n
	} }) : await chrome.storage.session.remove(r);
	try {
		await g(e, (await chrome.tabs.get(e)).url);
	} catch (e) {
		h(e);
	}
}
function v(e) {
	return [
		"analysis",
		"choices",
		"selection"
	].map((t) => `${t}:tab:${e}:`);
}
async function y() {
	let e = (e) => e === "analysisCacheIndex" || /^(analysis|choices|selection):(current|playlist):/.test(e);
	for (let t of [chrome.storage.session, chrome.storage.local]) {
		let n = await t.get(null), r = Object.keys(n).filter(e);
		r.length && await t.remove(r);
	}
}
async function b(e) {
	let t = v(e);
	for (let [e, n] of O) if (t.some((t) => e.startsWith(t))) {
		D.delete(e), O.delete(e);
		try {
			n.postMessage({ action: "cancelAnalysis" }), n.disconnect();
		} catch {}
	}
	let n = await chrome.storage.session.get(null), r = Object.keys(n).filter((n) => n === `actionState:tab:${e}` || t.some((e) => n.startsWith(e)));
	r.length && await chrome.storage.session.remove(r);
}
async function x() {
	let [e] = await chrome.tabs.query({
		active: !0,
		currentWindow: !0
	});
	e?.id != null && await g(e.id, e.url);
}
chrome.tabs.onUpdated.addListener((e, t, n) => {
	let r = !!t.url || t.status === "loading";
	(async () => {
		r && await b(e), (t.url || t.status === "complete" || r) && await g(e, t.url || n.url);
	})().catch(h);
}), chrome.tabs.onRemoved.addListener((e) => {
	b(e).catch(h);
}), chrome.tabs.onActivated.addListener(({ tabId: e }) => {
	chrome.tabs.get(e).then((t) => g(e, t.url)).catch(h);
}), chrome.windows?.onFocusChanged?.addListener(() => x().catch(h)), chrome.runtime.onInstalled.addListener(() => x().catch(h)), chrome.runtime.onStartup.addListener(() => x().catch(h)), y().catch(h), x().catch(h);
async function S(e) {
	if (e = t(e), !["analyze", "download"].includes(e?.action)) return e;
	let [n] = await chrome.tabs.query({
		active: !0,
		currentWindow: !0
	}), r = (await chrome.cookies.getAllCookieStores()).find((e) => e.tabIds.includes(n?.id)), i = new URL(e.url).hostname.toLowerCase().replace(/^www\./, ""), a = [e.url];
	(i === "youtube.com" || i.endsWith(".youtube.com") || i === "youtu.be") && a.push("https://www.youtube.com/");
	let o = await Promise.all(a.map((e) => chrome.cookies.getAll({
		url: e,
		...r ? { storeId: r.id } : {}
	}))), s = [...new Map(o.flat().map((e) => [`${e.domain}\n${e.path}\n${e.name}`, e])).values()];
	return {
		...e,
		browserCookies: s.map(({ domain: e, hostOnly: t, path: n, secure: r, expirationDate: i, name: a, value: o }) => ({
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
var C = null, w = { state: "idle" }, T = [], E = [], D = /* @__PURE__ */ new Map(), O = /* @__PURE__ */ new Map();
function k(t) {
	if (!Number.isInteger(t.sourceTabId)) throw Error(e("sourceTabMissing"));
	let n = t.playlistScope === "playlist" ? "playlist" : "current";
	return `analysis:tab:${t.sourceTabId}:${n}:${t.url}`;
}
async function A(e, t) {
	await chrome.storage.session.set({ [e]: t }), chrome.runtime.sendMessage({
		type: s,
		key: e,
		response: t
	}).catch(() => {});
}
async function j(t) {
	let r = k(t);
	if (D.has(r)) return {
		ok: !0,
		message: e("analysisAlreadyRunning"),
		data: D.get(r).data
	};
	let i = await S(t), o = {
		ok: !0,
		message: e("analysisStarted"),
		data: {
			items: [],
			itemLimit: Math.max(1, Math.min(100, Number(t.itemLimit) || 32)),
			truncated: !1,
			analyzing: !0,
			phase: "connectingHost",
			schemaVersion: c,
			startedAt: Date.now()
		}
	};
	D.set(r, o), await _(t.sourceTabId, "analyzing"), await A(r, o);
	let s = chrome.runtime.connectNative(a);
	return O.set(r, s), s.onMessage.addListener(async (i) => {
		let a = D.get(r);
		if (!a) return;
		if (!i?.ok) {
			let n = {
				ok: !1,
				message: i?.message || e("nativeAnalysisFailed")
			};
			D.delete(r), await _(t.sourceTabId, "failed"), await A(r, n), s.disconnect();
			return;
		}
		let o = i.data || {}, c = n(o);
		c.advertised && (a.data.host = c), [
			"off",
			"download",
			"embed"
		].includes(o.thumbnailMode) && (a.data.thumbnailMode = o.thumbnailMode), [
			"off",
			"download",
			"embed"
		].includes(o.subtitleMode) && (a.data.subtitleMode = o.subtitleMode), [
			"off",
			"download",
			"embed"
		].includes(o.chapterMode) && (a.data.chapterMode = o.chapterMode), typeof o.useAria2 == "boolean" && (a.data.useAria2 = o.useAria2), typeof o.aria2Available == "boolean" && (a.data.aria2Available = o.aria2Available), o.outputDirectory && await chrome.storage.local.set({ v2DownloadDirectory: o.outputDirectory }), (o.event === "started" || o.event === "phase") && o.phase ? (a.data.phase = o.phase, await A(r, structuredClone(a))) : o.event === "item" && o.item ? (a.data.items[o.index ?? a.data.items.length] = o.item, await A(r, structuredClone(a))) : o.event === "complete" && (a.data = {
			...a.data,
			itemLimit: o.itemLimit || a.data.itemLimit,
			truncated: !!o.truncated,
			analyzing: !1,
			cachedAt: Date.now()
		}, D.delete(r), O.delete(r), await A(r, structuredClone(a)), await _(t.sourceTabId, "analyzed", a.data.items.length));
	}), s.onDisconnect.addListener(async () => {
		if (!D.has(r)) return;
		let e = {
			ok: !1,
			...m(chrome.runtime.lastError?.message, "analysisDisconnected")
		};
		D.delete(r), O.delete(r), await A(r, e), await _(t.sourceTabId, "failed");
	}), s.postMessage(i), o;
}
async function M(t) {
	let n = k(t), r = D.get(n), i = O.get(n);
	return !r || !i ? {
		ok: !1,
		message: e("analysisNotRunning")
	} : (r.data = {
		...r.data,
		analyzing: !1,
		stopped: !0,
		cachedAt: Date.now()
	}, D.delete(n), O.delete(n), await A(n, structuredClone(r)), await _(t.sourceTabId, r.data.items.length ? "analyzed" : null, r.data.items.length), i.postMessage({ action: "cancelAnalysis" }), {
		ok: !0,
		message: e("analysisStoppedStatus"),
		data: r.data
	});
}
async function N(e) {
	w = {
		...w,
		...e
	}, await chrome.storage.session.set({ activeTask: w }), chrome.runtime.sendMessage({
		type: o,
		task: w
	}).catch(() => {});
}
async function P(e) {
	await N({
		state: "failed",
		...typeof e == "string" ? { message: e } : e,
		finishedAt: Date.now()
	}), await _(w.sourceTabId, "failed"), C = null, T = [], E = [];
}
async function F(t) {
	if (w.id !== t || !["running", "stopping"].includes(w.state)) return;
	C = null, T = [];
	let n = [...E];
	E = [], await N({
		state: "stopped",
		outputPath: n.at(-1) || "",
		outputPaths: n,
		message: e("downloadStopped"),
		finishedAt: Date.now()
	}), await _(w.sourceTabId, "analyzed", w.itemCount);
}
async function I() {
	if (!["running", "stopping"].includes(w.state)) return {
		ok: !1,
		message: e("downloadNotRunning")
	};
	if (w.state === "running") {
		if (T = [], await N({
			state: "stopping",
			message: e("downloadStopping")
		}), C) {
			let e = C;
			try {
				e.postMessage({ action: "cancelDownload" });
			} catch {
				try {
					e.disconnect();
				} catch {}
				await F(w.id);
			}
		} else await F(w.id);
	}
	return {
		ok: !0,
		message: e("downloadStopping"),
		data: { task: w }
	};
}
async function L(t) {
	if (w.id !== t || w.state !== "running") return;
	let r = T.shift();
	if (!r) return;
	let i = w.itemCount - T.length;
	await N({
		title: r.title || r.url,
		url: r.url,
		itemIndex: i,
		currentSourceItemIndex: r.sourceItemIndex,
		downloaded: "",
		total: "",
		speed: "",
		eta: "",
		itemPercent: 0,
		message: e("downloadPreparing")
	});
	let o = chrome.runtime.connectNative(a);
	C = o, o.onMessage.addListener(async (i) => {
		if (w.id !== t || C !== o) return;
		if (!i?.ok) {
			w.state === "stopping" ? await F(t) : await P(i?.message || e("nativeDownloadFailed")), o.disconnect();
			return;
		}
		let a = i.data || {}, s = n(a);
		if (s.advertised && await N({ host: s }), a.event === "stopped") {
			await F(t);
			return;
		}
		if (w.state === "stopping") {
			a.event === "complete" && a.outputPath && E.push(a.outputPath), await F(t);
			return;
		}
		if (a.event === "progress") {
			let t = Number.isFinite(a.percent) ? a.percent : 0;
			await N({
				state: "running",
				percent: (w.completedItems + t / 100) / w.itemCount * 100,
				itemPercent: t,
				downloaded: a.downloaded || "",
				total: a.total || "",
				speed: a.speed || "",
				eta: a.eta || "",
				message: e("downloading")
			});
		} else if (a.event === "complete") {
			a.outputPath && E.push(a.outputPath), C = null;
			let n = w.completedItems + 1, i = Number.isInteger(r.sourceItemIndex) ? [...w.completedSourceItemIndices, r.sourceItemIndex] : w.completedSourceItemIndices;
			T.length ? (await N({
				completedItems: n,
				percent: n / w.itemCount * 100,
				outputPaths: [...E],
				completedSourceItemIndices: i
			}), await L(t)) : (await N({
				state: "completed",
				completedItems: n,
				percent: 100,
				outputPath: E.at(-1) || "",
				outputPaths: [...E],
				completedSourceItemIndices: i,
				itemPercent: 100,
				message: e("downloadCompleted"),
				finishedAt: Date.now()
			}), await _(w.sourceTabId, "completed", n), E = []);
		}
	}), o.onDisconnect.addListener(async () => {
		let e = chrome.runtime.lastError?.message;
		w.id === t && C === o && (w.state === "stopping" ? await F(t) : w.state === "running" && await P(m(e, "nativeDisconnected")));
	}), o.postMessage(r);
}
async function R(t) {
	if (["running", "stopping"].includes(w.state)) return {
		ok: !1,
		message: e("downloadAlreadyRunning")
	};
	if (!Array.isArray(t) || !t.length || t.length > 100 || t.some((e) => e?.action !== "download" || !e.url)) return {
		ok: !1,
		message: e("downloadListInvalid")
	};
	let n = [];
	for (let e of t) n.push(await S(e));
	let r = crypto.randomUUID(), i = n[0];
	return w = {
		id: r,
		state: "running",
		title: i.title || i.url,
		url: i.url,
		sourcePageUrl: i.sourcePageUrl || i.url,
		sourceTabId: i.sourceTabId,
		itemCount: n.length,
		itemIndex: 1,
		completedItems: 0,
		completedSourceItemIndices: [],
		currentSourceItemIndex: i.sourceItemIndex,
		itemPercent: 0,
		percent: 0,
		downloaded: "",
		total: "",
		speed: "",
		eta: "",
		message: e("downloadPreparing"),
		startedAt: Date.now()
	}, T = n, E = [], await N(w), await _(w.sourceTabId, "downloading"), await L(r), {
		ok: !0,
		message: e("downloadStarted"),
		data: { task: w }
	};
}
async function z(e) {
	return R([e]);
}
function B(t, n) {
	chrome.runtime.sendNativeMessage(a, t, async (t) => {
		if (chrome.runtime.lastError) {
			n({
				ok: !1,
				message: chrome.runtime.lastError.message
			});
			return;
		}
		try {
			let r = t?.data?.outputDirectory || t?.data?.directory;
			r && await chrome.storage.local.set({ v2DownloadDirectory: r }), n(t || {
				ok: !1,
				message: e("nativeNoData")
			});
		} catch (e) {
			n({
				ok: !1,
				message: e.message
			});
		}
	});
}
chrome.runtime.onMessage.addListener((t, n, r) => t?.type === "YT_DLP_GUI_TASK_STATUS" ? (chrome.storage.session.get("activeTask").then((e) => {
	r({
		ok: !0,
		data: { task: e.activeTask || w }
	});
}), !0) : t?.type === "YT_DLP_GUI_NATIVE_REQUEST" ? t.request?.action === "cancelDownload" ? (I().then(r).catch((t) => r({
	ok: !1,
	message: t.message || e("downloadStopFailed")
})), !0) : t.request?.action === "cancelAnalysis" ? (M(t.request).then(r).catch((t) => r({
	ok: !1,
	message: t.message || e("analysisStopFailed")
})), !0) : t.request?.action === "analyze" ? (j(t.request).then(r).catch((t) => r({
	ok: !1,
	message: t.message || e("analysisStartFailed")
})), !0) : t.request?.action === "download" ? (z(t.request).then(r).catch(async (e) => {
	let t = C, n = m(e.message, "nativeConnectFailed");
	await P(n), t && t.disconnect(), r({
		ok: !1,
		...n
	});
}), !0) : t.request?.action === "downloadBatch" ? (R(t.request.requests).then(r).catch(async (e) => {
	let t = C, n = m(e.message, "nativeConnectFailed");
	await P(n), t && t.disconnect(), r({
		ok: !1,
		...n
	});
}), !0) : (S(t.request).then((e) => B(e, r)).catch(() => r({
	ok: !1,
	message: e("cookieReadFailed")
})), !0) : !1);
//#endregion
