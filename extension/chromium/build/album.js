import { $ as e, A as t, C as n, Ct as r, E as i, G as a, I as o, J as s, K as c, L as l, M as u, O as d, Q as f, R as p, St as m, T as h, U as g, V as _, W as v, X as y, Z as ee, _ as b, a as te, at as ne, b as x, c as re, ct as S, d as ie, dt as C, et as w, ft as T, ht as ae, i as oe, it as E, j as D, k as se, l as ce, lt as le, mt as ue, nt as O, ot as de, pt as k, q as fe, rt as pe, st as A, t as me, tt as j, u as he, ut as M, v as ge, w as _e, wt as N, x as ve, xt as ye, z as be } from "./chunks/framework7-interaction-policy-C1jqd_Km.js";
import { t as P } from "./chunks/i18n-SwgWtGUP.js";
import { c as F, f as xe, i as Se, m as Ce, n as we, p as Te, r as Ee, t as De, u as Oe } from "./chunks/gallery-timeline-session-CJYptyAy.js";
//#region node_modules/.pnpm/spotlight.js@0.7.8/node_modules/spotlight.js/src/js/helper.js
function I(e, t) {
	R(e, t, !0);
}
function L(e, t) {
	R(e, t);
}
function R(e, t, n) {
	e.classList[n ? "add" : "remove"](t);
}
function z(e, t, n) {
	n = "" + n, e["_s_" + t] !== n && (e.style.setProperty(t, n), e["_s_" + t] = n);
}
var ke = 0;
function Ae(e, t) {
	t && (z(e, "transition", "none"), t()), ke ||= e.clientTop && 0, t && z(e, "transition", "");
}
function je(e, t) {
	e.firstChild.nodeValue = t;
}
function B(e, t) {
	return (t || document).getElementsByClassName(e);
}
function V(e, t, n, r) {
	Me(!0, e, t, n, r);
}
function Me(e, t, n, r, i) {
	t[(e ? "add" : "remove") + "EventListener"](n, r, i || i === !1 ? i : !0);
}
function H(e, t) {
	e.stopPropagation(), t && e.preventDefault();
}
function Ne(e, t) {
	let n = Pe("a"), r = t.src;
	n.href = r, n.download = r.substring(r.lastIndexOf("/") + 1), e.appendChild(n), n.click(), e.removeChild(n);
}
function Pe(e) {
	return document.createElement(e);
}
function Fe(e, t) {
	z(e, "display", t ? "" : "none");
}
function Ie(e, t) {
	z(e, "visibility", t ? "" : "hidden");
}
function Le(e, t) {
	z(e, "transition", t ? "" : "none");
}
//#endregion
//#region node_modules/.pnpm/spotlight.js@0.7.8/node_modules/spotlight.js/src/js/config.js
var Re = [
	"theme",
	"download",
	"play",
	"page",
	"close",
	"autofit",
	"zoom-in",
	"zoom-out",
	"prev",
	"next",
	"fullscreen"
], ze = {
	page: 1,
	close: 1,
	autofit: 1,
	"zoom-in": 1,
	"zoom-out": 1,
	prev: 1,
	next: 1,
	fullscreen: 1
}, U = {
	BACKSPACE: 8,
	ESCAPE: 27,
	SPACEBAR: 32,
	LEFT: 37,
	RIGHT: 39,
	UP: 38,
	NUMBLOCK_PLUS: 107,
	PLUS: 187,
	DOWN: 40,
	NUMBLOCK_MINUS: 109,
	MINUS: 189
}, W = Pe("div");
W.id = "spotlight", W.innerHTML = "<div class=spl-spinner></div><div class=spl-track><div class=spl-scene><div class=spl-pane></div></div></div><div class=spl-header><div class=spl-page> </div></div><div class=spl-progress></div><div class=spl-footer><div class=spl-title> </div><div class=spl-description> </div><div class=spl-button> </div></div><div class=spl-prev></div><div class=spl-next></div>";
//#endregion
//#region node_modules/.pnpm/spotlight.js@0.7.8/node_modules/spotlight.js/src/js/parser.js
var Be = {}, Ve = Pe("video");
function He(e, t, n, r) {
	let i, a;
	if (r !== "node") {
		let e = Object.keys(n);
		for (let o = 0, s; o < e.length; o++) if (s = e[o], s.length > 3 && s.indexOf("src") === 0) {
			if (r === "video") {
				let e = Be[s];
				if (e) {
					if (e > 0) {
						i = n[s];
						break;
					}
				} else if (Ve.canPlayType("video/" + s.substring(3).replace("-", "").toLowerCase())) {
					Be[s] = 1, i = n[s];
					break;
				} else Be[s] = -1;
			} else {
				let e = parseInt(s.substring(4), 10);
				if (e) {
					let r = Math.abs(t - e);
					(!a || r < a) && (a = r, i = n[s]);
				}
			}
		}
	}
	return i || n.src || n.href || e.src || e.href;
}
//#endregion
//#region node_modules/.pnpm/spotlight.js@0.7.8/node_modules/spotlight.js/src/js/spotlight.js
var Ue = {}, We = navigator.connection, Ge = window.devicePixelRatio || 1, G, K, Ke, qe, Je, Ye, Xe, Ze, Qe, $e, et, tt, nt, rt, q, J, it, Y, at, ot, st, ct, lt, ut, dt, ft, pt, mt, ht, gt, _t, vt, yt, bt, xt, St, Ct, wt, Tt, X, Et, Z, Dt = Pe("img"), Ot, kt, At, jt, Mt, Nt, Pt, Ft, It, Lt, Rt, zt, Bt, Vt, Ht, Q, Ut, Wt, Gt, Kt;
V(document, "click", Xt);
function qt() {
	if (Tt) return;
	Tt = document.body, Ot = t("scene"), kt = t("header"), At = t("footer"), jt = t("title"), Mt = t("description"), Nt = t("button"), Pt = t("prev"), Ft = t("next"), Lt = t("page"), zt = t("progress"), Bt = t("spinner"), Et = [t("pane")], Jt("close", Nn), Tt[Gt = "requestFullscreen"] || Tt[Gt = "msRequestFullscreen"] || Tt[Gt = "webkitRequestFullscreen"] || Tt[Gt = "mozRequestFullscreen"] || (Gt = ""), Gt ? (Kt = Gt.replace("request", "exit").replace("mozRequest", "mozCancel").replace("Request", "Exit"), It = Jt("fullscreen", wn)) : Re.pop(), Jt("autofit", En), Jt("zoom-in", Dn), Jt("zoom-out", On), Jt("theme", Tn), Rt = Jt("play", gn), Jt("download", Mn), V(Pt, "click", Fn), V(Ft, "click", In);
	let e = t("track");
	V(e, "mousedown", xn), V(e, "mousemove", Cn), V(e, "mouseleave", Sn), V(e, "mouseup", Sn), V(e, "touchstart", xn, { passive: !1 }), V(e, "touchmove", Cn, { passive: !0 }), V(e, "touchend", Sn), V(Nt, "click", function() {
		vt ? vt(q, Y) : _t && (location.href = _t);
	});
	function t(e) {
		return Ue[e] = B("spl-" + e, W)[0];
	}
}
function Jt(e, t) {
	let n = Pe("div");
	return n.className = "spl-" + e, V(n, "click", t), kt.appendChild(n), Ue[e] = n;
}
function Yt(e) {
	let t = Ue[e];
	t && (kt.removeChild(t), Ue[e] = null);
}
function Xt(e) {
	let t = e.target.closest(".spotlight");
	if (t) {
		H(e, !0);
		let n = t.closest(".spotlight-group");
		it = B("spotlight", n);
		for (let e = 0; e < it.length; e++) if (it[e] === t) {
			ot = n && n.dataset, Qt(e + 1);
			break;
		}
	}
}
function Zt(e, t, n) {
	it = e, t && (ot = t, lt = t.onshow, ut = t.onchange, dt = t.onclose, n ||= t.index), Qt(n);
}
function Qt(e) {
	if (J = it.length, J) {
		Tt || qt(), lt && lt(e);
		let t = Et[0], n = t.parentNode;
		for (let e = Et.length; e < J; e++) {
			let r = t.cloneNode(!1);
			z(r, "left", e * 100 + "%"), n.appendChild(r), Et[e] = r;
		}
		X || (Tt.appendChild(W), sn()), q = e || 1, Le(Ot), zn(!0), Gt && on(), jn();
	}
}
function $(e, t) {
	let n = Y[e];
	return n === void 0 ? t : (n = "" + n, n !== "false" && (n || t));
}
function $t(e) {
	Y = {}, ot && Object.assign(Y, ot), Object.assign(Y, e.dataset || e), at = Y.media, vt = Y.onclick, ht = Y.theme, yt = Y.class, pt = $("autohide", !0), st = $("infinite"), ct = $("progress", !0), mt = $("autoslide"), gt = $("preload", !0), _t = Y.buttonHref, bt = mt && parseFloat(mt) || 7, rt || ht && Tn(ht), yt && I(W, yt), yt && Ae(W);
	let t = Y.control;
	if (t) {
		let e = typeof t == "string" ? t.split(",") : t;
		for (let e = 0; e < Re.length; e++) Y[Re[e]] = !1;
		for (let t = 0; t < e.length; t++) {
			let n = e[t].trim();
			n === "zoom" ? Y["zoom-in"] = Y["zoom-out"] = !0 : Y[n] = !0;
		}
	}
	let n = Y.animation;
	if (xt = St = Ct = !n, wt = !1, n) {
		let e = typeof n == "string" ? n.split(",") : n;
		for (let t = 0; t < e.length; t++) {
			let n = e[t].trim();
			n === "scale" ? xt = !0 : n === "fade" ? St = !0 : n === "slide" ? Ct = !0 : n && (wt = n);
		}
	}
	ft = Y.fit;
}
function en(e) {
	e ? Ae(Z, en) : (Le(Ot, Ct), z(Z, "opacity", +!St), ln(xt && .8), wt && I(Z, wt));
}
function tn(e) {
	if (X = Et[e - 1], Z = X.firstChild, q = e, Z) An(), ft && I(Z, ft), en(!0), wt && L(Z, wt), St && z(Z, "opacity", 1), xt && z(Z, "transform", ""), z(Z, "visibility", "visible"), Ht && (Dt.src = Ht), mt && _n(Q);
	else {
		let t = Vt.media, n = $("spinner", !0);
		if (t === "video") nn(n, !0), Z = Pe("video"), Z.onloadedmetadata = function() {
			Z === this && (Z.onerror = null, Z.width = Z.videoWidth, Z.height = Z.videoHeight, cn(), nn(n), tn(e));
		}, Z.poster = Y.poster, Z.preload = gt ? "auto" : "metadata", Z.controls = $("controls", !0), Z.autoplay = Y.autoplay, Z.playsinline = $("inline"), Z.muted = $("muted"), Z.src = Vt.src, X.appendChild(Z);
		else if (t === "node") {
			Z = Vt.src, typeof Z == "string" && (Z = document.querySelector(Z)), Z && (Z._root || (Z._root = Z.parentNode), cn(), X.appendChild(Z), tn(e));
			return;
		} else nn(n, !0), Z = Pe("img"), Z.onload = function() {
			Z === this && (Z.onerror = null, nn(n), tn(e), cn());
		}, Z.src = Vt.src, X.appendChild(Z);
		Z && (n || z(Z, "visibility", "visible"), Z.onerror = function() {
			Z === this && (Pn(Z), I(Bt, "error"), nn(n));
		});
	}
}
function nn(e, t) {
	e && R(Bt, "spin", t);
}
function rn() {
	return document.fullscreen || document.fullscreenElement || document.webkitFullscreenElement || document.mozFullScreenElement;
}
function an() {
	if (sn(), Z && cn(), Gt) {
		let e = rn();
		R(It, "on", e), e || on();
	}
}
function on() {
	Fe(It, screen.availHeight - window.innerHeight > 0);
}
function sn() {
	Je = W.clientWidth, Ye = W.clientHeight;
}
function cn() {
	Xe = Z.clientWidth, Ze = Z.clientHeight;
}
function ln(e) {
	z(Z, "transform", "translate(-50%, -50%) scale(" + (e || Qe) + ")");
}
function un(e, t) {
	z(X, "transform", e || t ? "translate(" + e + "px, " + t + "px)" : "");
}
function dn(e, t, n) {
	t ? Ae(Ot, function() {
		dn(e, !1, n);
	}) : z(Ot, "transform", "translateX(" + (-e * 100 + (n || 0)) + "%)");
}
function fn(e) {
	Me(e, window, "keydown", mn), Me(e, window, "wheel", hn), Me(e, window, "resize", an), Me(e, window, "popstate", pn);
}
function pn(e) {
	X && e.state.spl && Nn(!0);
}
function mn(e) {
	if (X) {
		let t = Y["zoom-in"] !== !1;
		switch (e.keyCode) {
			case U.BACKSPACE:
				t && En();
				break;
			case U.ESCAPE:
				Nn();
				break;
			case U.SPACEBAR:
				mt && gn();
				break;
			case U.LEFT:
				Fn();
				break;
			case U.RIGHT:
				In();
				break;
			case U.UP:
			case U.NUMBLOCK_PLUS:
			case U.PLUS:
				t && Dn();
				break;
			case U.DOWN:
			case U.NUMBLOCK_MINUS:
			case U.MINUS: t && On();
		}
	}
}
function hn(e) {
	if (X && Y["zoom-in"] !== !1) {
		let t = e.deltaY;
		t = (t < 0 ? 1 : t ? -1 : 0) * .5, t < 0 ? On() : Dn();
	}
}
function gn(e, t) {
	(typeof e == "boolean" ? e : !Q) === !Q && (Q = Q ? clearTimeout(Q) : 1, R(Rt, "on", Q), t || _n(Q));
}
function _n(e) {
	ct && (Ae(zt, function() {
		z(zt, "transition-duration", ""), z(zt, "transform", "");
	}), e && (z(zt, "transition-duration", bt + "s"), z(zt, "transform", "translateX(0)"))), e && (Q = setTimeout(In, bt * 1e3));
}
function vn() {
	pt && (Wt = Date.now() + 2950, Ut || (I(W, "menu"), yn(3e3)));
}
function yn(e) {
	Ut = setTimeout(function() {
		let e = Date.now();
		e >= Wt ? (L(W, "menu"), Ut = 0) : yn(Wt - e);
	}, e);
}
function bn(e) {
	typeof e == "boolean" && (Ut = e ? Ut : 0), Ut ? (Ut = clearTimeout(Ut), L(W, "menu")) : vn();
}
function xn(e) {
	H(e, !0), $e = !0, et = !1;
	let t = e.touches;
	(t &&= t[0]) && (e = t), tt = Xe * Qe <= Je, Ke = e.pageX, qe = e.pageY, Le(X);
}
function Sn(e) {
	if (H(e), $e) {
		if (!et) bn();
		else {
			if (tt && et) {
				let e = G < -(Je / 7) && (q < J || st), t = e || G > Je / 7 && (q > 1 || st);
				(e || t) && (dn(q - 1, !0, G / Je * 100), e && In() || t && Fn()), G = 0, un();
			}
			Le(X, !0);
		}
		$e = !1;
	}
}
function Cn(e) {
	if (H(e), $e) {
		let t = e.touches;
		(t &&= t[0]) && (e = t);
		let n = (Xe * Qe - Je) / 2;
		G -= Ke - (Ke = e.pageX), tt || (G > n ? G = n : G < -n && (G = -n), Ze * Qe > Ye && (n = (Ze * Qe - Ye) / 2, K -= qe - (qe = e.pageY), K > n ? K = n : K < -n && (K = -n))), et = !0, un(G, K);
	} else vn();
}
function wn(e) {
	let t = rn();
	(typeof e != "boolean" || e !== !!t) && (t ? document[Kt]() : W[Gt]());
}
function Tn(e) {
	typeof e != "string" && (e = rt ? "" : ht || "white"), rt !== e && (rt && L(W, rt), e && I(W, e), rt = e);
}
function En(e) {
	typeof e == "boolean" && (nt = !e), nt = Qe === 1 && !nt, R(Z, "autofit", nt), z(Z, "transform", ""), Qe = 1, G = 0, K = 0, cn(), Le(X), un();
}
function Dn(e) {
	let t = Qe / .65;
	t <= 50 && (An(), G /= .65, K /= .65, un(G, K), kn(t));
}
function On(e) {
	let t = Qe * .65;
	An(), t >= 1 && (t === 1 ? G = K = 0 : (G *= .65, K *= .65), un(G, K), kn(t));
}
function kn(e) {
	Qe = e || 1, ln();
}
function An() {
	nt && En();
}
function jn() {
	history.pushState({ spl: 1 }, ""), history.pushState({ spl: 2 }, ""), Le(W, !0), I(Tt, "hide-scrollbars"), I(W, "show"), fn(!0), sn(), vn(), mt && gn(!0, !0);
}
function Mn() {
	Ne(Tt, Z);
}
function Nn(e) {
	setTimeout(function() {
		Tt.removeChild(W), X = Z = Vt = Y = ot = it = lt = ut = dt = vt = null;
	}, 200), L(Tt, "hide-scrollbars"), L(W, "show"), wn(!1), fn(), history.go(e === !0 ? -1 : -2), Ht && (Dt.src = ""), Q && gn(), Z && Pn(Z), Ut &&= clearTimeout(Ut), rt && Tn(), yt && L(W, yt), dt && dt();
}
function Pn(e) {
	if (e._root) e._root.appendChild(e), e._root = null;
	else {
		let t = e.parentNode;
		t && t.removeChild(e), e = e.src = e.onerror = "";
	}
}
function Fn(e) {
	if (e && vn(), J > 1) {
		if (q > 1) return Ln(q - 1);
		if (st) return dn(J, !0), Ln(J);
	}
}
function In(e) {
	if (e && vn(), J > 1) {
		if (q < J) return Ln(q + 1);
		if (st) return dn(-1, !0), Ln(1);
		Q && gn();
	}
}
function Ln(e) {
	if (e !== q) {
		Q ? (clearTimeout(Q), _n()) : vn();
		let t = e > q;
		return q = e, zn(t), !0;
	}
}
function Rn(e) {
	let t = it[q - 1];
	$t(t);
	let n = We && We.downlink, r = Math.max(Ye, Je) * Ge;
	n && n * 1200 < r && (r = n * 1200);
	let i;
	if (Vt = {
		media: at,
		src: He(t, r, Y, at),
		title: $("title", t.alt || t.title || (i = t.firstElementChild) && (i.alt || i.title))
	}, Ht && (Dt.src = Ht = ""), gt && e && (t = it[q])) {
		let e = t.dataset || t, n = e.media;
		(!n || n === "image") && (Ht = He(t, r, e, n));
	}
	for (let e = 0; e < Re.length; e++) {
		let t = Re[e];
		Fe(Ue[t], $(t, ze[t]));
	}
}
function zn(e) {
	if (G = 0, K = 0, Qe = 1, Z) {
		if (Z.onerror) Pn(Z);
		else {
			let e = Z;
			setTimeout(function() {
				e && Z !== e && (Pn(e), e = null);
			}, 650), en(), un();
		}
	}
	Rn(e), dn(q - 1), L(Bt, "error"), tn(q), Le(X), un();
	let t = Vt.title, n = $("description"), r = $("button"), i = t || n || r;
	i && (t && je(jt, t), n && je(Mt, n), r && je(Nt, r), Fe(jt, t), Fe(Mt, n), Fe(Nt, r), z(At, "transform", pt === "all" ? "" : "none")), pt || I(W, "menu"), Ie(At, i), Ie(Pt, st || q > 1), Ie(Ft, st || q < J), je(Lt, J > 1 ? q + " / " + J : ""), ut && ut(q, Y);
}
var Bn = {
	init: qt,
	theme: Tn,
	fullscreen: wn,
	download: Mn,
	autofit: En,
	next: In,
	prev: Fn,
	goto: Ln,
	close: Nn,
	zoom: kn,
	menu: bn,
	show: Zt,
	play: gn,
	addControl: Jt,
	removeControl: Yt
};
//#endregion
//#region src/album/album-post-info.js
function Vn(e = {}) {
	let t = (e) => typeof e == "string" ? e.trim() : "", n = [t(e.creator), t(e.date)].filter(Boolean).join(" · ") || t(e.siteName), r = t(e.description) || t(e.title);
	return {
		title: n === r ? "" : n,
		description: r
	};
}
//#endregion
//#region src/album/album-spotlight-slides.js
function Hn(e, t = {}, n = []) {
	return e.map((e, r) => {
		let i = Vn(n[r] || t);
		return {
			src: e.url,
			title: i.title,
			description: i.description,
			...xe(e) ? {
				media: "video",
				fit: "contain",
				controls: !0,
				autoplay: !1,
				poster: e.thumbnail || "",
				title: !1,
				description: !1,
				button: !1,
				control: [
					"page",
					"prev",
					"next",
					"close"
				]
			} : {}
		};
	});
}
//#endregion
//#region src/album/album-viewer-wheel.js
function Un(e, t, n = window) {
	let r = -Infinity, i = -Infinity, a = 0, o = (n) => {
		if (t() !== "navigate" || !n.target?.closest?.("#spotlight")) return;
		if (n.target.closest(".spl-footer, .spl-header") || document.fullscreenElement?.tagName === "VIDEO") {
			n.stopImmediatePropagation();
			return;
		}
		if (n.ctrlKey || n.metaKey) {
			n.stopImmediatePropagation();
			return;
		}
		if (!n.deltaY || Math.abs(n.deltaX) > Math.abs(n.deltaY)) {
			n.stopImmediatePropagation();
			return;
		}
		n.preventDefault(), n.stopImmediatePropagation();
		let o = n.timeStamp;
		if ((o - r > 180 || Math.sign(a) !== Math.sign(n.deltaY)) && (a = 0), r = o, a += n.deltaY * (n.deltaMode === 1 ? 16 : n.deltaMode === 2 ? 800 : 1), Math.abs(a) < 40 || o - i < 500) return;
		i = o;
		let s = Math.sign(a);
		a = 0, s > 0 ? e.next() : e.prev();
	};
	return n.addEventListener("wheel", o, {
		capture: !0,
		passive: !1
	}), () => n.removeEventListener("wheel", o, { capture: !0 });
}
//#endregion
//#region src/album/album-video-sources.js
function Wn(e) {
	let t = Array.isArray(e?.fallbackUrls) ? e.fallbackUrls.slice(0, 16) : [];
	return [...new Set([e?.url, ...t].filter(Oe))];
}
function Gn(e, t, { resumePlayback: n = !0, onSourceChange: r = () => {} } = {}) {
	let i = Wn(t), a = "";
	function o(e, t, n) {
		F("album-video.source", {
			status: e,
			phase: `${t}_${n + 1}`,
			itemCount: i.length
		});
	}
	function s(t) {
		let a = e.getAttribute("src");
		if (!a) return;
		let s = i.indexOf(a);
		if (s < 0) return;
		if (s + 1 >= i.length) {
			o("failed", "exhausted", s);
			return;
		}
		t.stopImmediatePropagation(), o("loading", "fallback", s + 1);
		let c = n && !e.paused;
		e.src = i[s + 1], e.load(), r(i[s + 1]), c && e.play().catch(() => {});
	}
	function c() {
		let t = e.getAttribute("src"), n = i.indexOf(t);
		n < 0 || t === a || (a = t, o("completed", n ? "fallback" : "primary", n));
	}
	return e.addEventListener("error", s, !0), e.addEventListener("loadeddata", c), () => {
		e.removeEventListener("error", s, !0), e.removeEventListener("loadeddata", c);
	};
}
//#endregion
//#region src/album/album-viewer.js
function Kn(e) {
	return e.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}
function qn(e, t, n = {}, r = [], { session: i, report: a = () => {}, wheelMode: o = () => "zoom" } = {}) {
	if (xe(e[t]) && String(e[t].extension).toLowerCase() === "m3u8") {
		chrome.tabs.create({ url: e[t].url });
		return;
	}
	let s = Hn(e, n, r);
	s.map((t, n) => {
		if (xe(e[n])) return "";
		let r = `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="640" height="360"><text x="20" y="40" fill="#8c8c8c">${Kn(P("albumProcessingNumber", { number: n + 1 }))}</text></svg>`)}`;
		return i && (t.src = r), r;
	});
	let c = !1, l = -1, u = 0, d = "", f = () => {};
	async function p(t) {
		let n = t - 1, r = ++u;
		if (l = n, f(), f = () => {}, xe(e[n])) {
			let t = document.querySelectorAll("#spotlight .spl-pane")[n]?.querySelector("video");
			t && (f = Gn(t, e[n]));
			return;
		}
		if (i) try {
			let t = await i.prepare(e[n]);
			if (c || l !== n || u !== r) return;
			let o = document.querySelectorAll("#spotlight .spl-pane")[n]?.querySelector("img");
			if (!o) throw Error("Viewer provided no replaceable image element.");
			d && URL.revokeObjectURL(d), d = URL.createObjectURL(t.blob), o.src = d, a(t.xmp ? "" : P("albumOriginalNoXmp"));
		} catch (e) {
			!c && l === n && a(String(e?.message || e));
		}
	}
	let m = Un(Bn, o);
	try {
		Bn.show(s, {
			index: t + 1,
			preload: !1,
			autohide: !1,
			control: [
				"page",
				"prev",
				"next",
				"autofit",
				"zoom",
				"fullscreen",
				"close"
			],
			theme: "dark",
			onchange: p,
			onclose: () => {
				c = !0, f(), m(), d && URL.revokeObjectURL(d);
			}
		});
	} catch (e) {
		throw f(), m(), e;
	}
}
function Jn() {
	Bn.close();
}
//#endregion
//#region src/album/album-justified-layout.js
function Yn(e, t) {
	let n = Number(e?.width), r = Number(e?.height), i = Number.isFinite(t) && t > 0 ? t : n > 0 && r > 0 ? n / r : 1;
	return Math.max(.2, Math.min(8, i));
}
function Xn(e, t, n = /* @__PURE__ */ new Map(), r = 6) {
	if (!Number.isFinite(t) || t <= 0 || !e.length) return [];
	let i = Math.max(1, t), a = Math.max(0, r), o = Math.max(135, Math.min(235, i * .22)), s = [], c = [], l = 0;
	function u(e) {
		if (!c.length) return;
		let t = Math.max(1, i - a * (c.length - 1)) / l, n = e || t <= o * 1.15, r = n ? t : Math.min(o, t);
		s.push({
			id: c[0].index,
			height: r,
			justified: n,
			items: c.map(({ index: e, ratio: t }) => ({
				index: e,
				ratio: t,
				width: t * r
			}))
		}), c = [], l = 0;
	}
	return e.forEach((e, t) => {
		let r = Yn(e, n.get(t));
		c.push({
			index: t,
			ratio: r
		}), l += r, l * o + a * (c.length - 1) >= i && u(!0);
	}), u(!1), s;
}
//#endregion
//#region src/album/album-post-stream.js
function Zn(e = {}) {
	let t = Array.isArray(e.posts) && e.posts.length ? e.posts : [e], n = [], r = [], i = [], a = [];
	for (let o of t) {
		let t = (Array.isArray(o?.media) ? o.media : []).filter((e) => Oe(e?.url));
		if (!t.length) continue;
		let s = n.length, c = {
			title: o.title || "",
			creator: o.creator || "",
			description: o.description || "",
			date: o.date || "",
			...Qn(o.date),
			siteName: o.siteName || e.siteName || "",
			sourcePageUrl: o.sourcePageUrl || e.sourcePageUrl || "",
			postId: o.postId || "",
			count: t.length
		};
		n.push(c), a.push({
			kind: "post",
			postIndex: s,
			width: .9,
			height: 1
		});
		for (let e of t) {
			let t = r.length;
			r.push(e), i.push(c), a.push({
				kind: "media",
				postIndex: s,
				mediaIndex: t,
				width: e.width,
				height: e.height
			});
		}
	}
	return e.timeline && a.push({
		kind: "continue",
		width: .9,
		height: 1
	}), {
		posts: n,
		media: r,
		tiles: a,
		postByMediaIndex: i
	};
}
function Qn(e = "") {
	let t = /^(\d{4}-\d{2}-\d{2})(?:[T ](\d{2}:\d{2}(?::\d{2})?))?/.exec(String(e));
	return {
		dateOnly: t?.[1] || "",
		timeOnly: t?.[2] || ""
	};
}
//#endregion
//#region src/album/album-auto-download.js
function $n(e) {
	let t = new URL(e).searchParams.get("download");
	return t === "files" || t === "zip" ? t : null;
}
async function er(e, t) {
	let n = new URL(t), r = $n(t), i = n.searchParams.get("id");
	if (!r || !i || !/^[0-9a-f-]{36}$/i.test(i)) return null;
	let a = `galleryAutoDownload:${i}`;
	return (await e.get(a))[a] === r ? (await e.remove(a), r) : null;
}
//#endregion
//#region src/image/gallery-image-session.js
function tr() {
	let e = /* @__PURE__ */ new Map(), t = /* @__PURE__ */ new Set(), n = (e) => JSON.stringify([
		e.url,
		e.archiveKey,
		e.canonicalMetadata
	]), r = [], i = /* @__PURE__ */ new Set(), a = !1, o = 0;
	function s() {
		for (let [t, n] of e) {
			if (o <= 67108864) break;
			n.value && (o -= n.value.blob.size, e.delete(t));
		}
	}
	function c() {
		for (; !a && i.size < 2 && r.length;) {
			let t = r.shift();
			if (t.signal?.aborted) {
				e.delete(t.key), t.reject(t.signal.reason);
				continue;
			}
			let n = new AbortController(), l = () => n.abort(t.signal.reason);
			t.signal?.addEventListener("abort", l, { once: !0 }), i.add(n), Ee(t.item, { signal: n.signal }).then((n) => {
				if (a) throw new DOMException("Album closed", "AbortError");
				let r = {
					blob: new Blob([n.bytes], { type: n.mime }),
					xmp: n.xmp
				}, i = e.get(t.key);
				i && (i.value = r, o += r.blob.size, s()), t.resolve(r);
			}).catch((n) => {
				e.delete(t.key), t.reject(n);
			}).finally(() => {
				t.signal?.removeEventListener("abort", l), i.delete(n), c();
			});
		}
	}
	function l(t, { signal: i } = {}) {
		if (a) return Promise.reject(new DOMException("Album closed", "AbortError"));
		if (i?.aborted) return Promise.reject(i.reason);
		let o = n(t), s = e.get(o);
		return s ? (e.delete(o), e.set(o, s), u(s.promise, i)) : (s = {}, s.promise = new Promise((e, n) => r.push({
			item: t,
			key: o,
			signal: i,
			resolve: e,
			reject: n
		})), e.set(o, s), c(), u(s.promise, i));
	}
	function u(e, t) {
		return t ? new Promise((n, r) => {
			let i = () => r(t.reason);
			t.addEventListener("abort", i, { once: !0 }), e.then((e) => {
				t.removeEventListener("abort", i), n(e);
			}, (e) => {
				t.removeEventListener("abort", i), r(e);
			}), t.aborted && i();
		}) : e;
	}
	function d() {
		a = !0;
		for (let e of i) e.abort();
		for (let e of r.splice(0)) e.reject(new DOMException("Album closed", "AbortError"));
		e.clear(), t.clear(), o = 0;
	}
	return {
		prepare: l,
		dispose: d,
		hasDisplayed: (e) => t.has(n(e)),
		markDisplayed: (e) => {
			a || t.add(n(e));
		}
	};
}
//#endregion
//#region src/album/AlbumImage.svelte
var nr = s("<img/>"), rr = s("<span class=\"album-image-error\" role=\"status\"> </span>"), ir = s("<span class=\"album-image-pending\"> </span>"), ar = s("<div class=\"album-image\"><!></div>");
function or(t, n) {
	m(n, !1);
	let r = i(n, "item", 8), o = i(n, "session", 8), s = i(n, "index", 8), u = _e(), f = T(), p = T(""), _ = T(""), v = T(!1), y = T(!1);
	function ee(e) {
		o().markDisplayed(r()), u("load", e);
	}
	h(() => {
		let e = !0, t = !1;
		function n() {
			O(p) && URL.revokeObjectURL(O(p)), k(p, "");
		}
		async function i() {
			if (!(O(v) || O(p))) {
				k(v, !0), k(_, "");
				try {
					let n = await o().prepare(r());
					e && t && (k(y, !o().hasDisplayed(r())), k(p, URL.createObjectURL(n.blob)));
				} catch (t) {
					e && k(_, String(t?.message || t));
				} finally {
					k(v, !1);
				}
			}
		}
		let a = new IntersectionObserver((e) => {
			t = e[0].isIntersecting, t ? i() : n();
		}, { rootMargin: "120px" });
		return a.observe(O(f)), () => {
			e = !1, a.disconnect(), n();
		};
	}), d();
	var b = ar(), te = S(b), ne = (t) => {
		var n = nr();
		let r;
		A((e) => {
			D(n, "src", O(p)), D(n, "alt", e), r = l(n, 1, "", null, r, { "album-image-first-reveal": O(y) });
		}, [() => (j(P), j(s()), E(() => P("albumImage", { number: s() + 1 })))]), e("load", n, ee), w(n), c(t, n);
	}, x = (e) => {
		var t = rr(), n = M(t, !0);
		A(() => a(n, O(_))), c(e, t);
	}, re = (e) => {
		var t = ir(), n = M(t, !0);
		A((e) => a(n, e), [() => (O(v), j(P), E(() => O(v) ? P("albumProcessingImage") : P("albumWaiting")))]), c(e, t);
	};
	g(te, (e) => {
		O(p) ? e(ne) : O(_) ? e(x, 1) : e(re, -1);
	}), N(b), se(b, (e) => k(f, e), () => O(f)), c(t, b), ye();
}
//#endregion
//#region src/album/album-video-preview.js
function sr(e) {
	return Oe(e?.thumbnail) && e.thumbnail !== e.url ? e.thumbnail : "";
}
function cr(e, t, n, r = () => {}) {
	let i = !1, a = !1, o = !1, s = !1, c = "", l = 0, u = !1, d = !1, f = [
		"mp4",
		"webm",
		"mov"
	].includes(String(n.extension).toLowerCase()), p = !!sr(n);
	t.muted = !0, t.loop = !0, t.playsInline = !0;
	function m() {
		l++, u = !1, d = !1, t.pause(), c &&= (t.removeAttribute("src"), t.load(), ""), r("idle");
	}
	function h() {
		if (s || !f || !o || document.hidden) {
			m();
			return;
		}
		if (!c) {
			if (p && !(i || a)) {
				r("ready");
				return;
			}
			l++, c = n.url, t.preload = "metadata", t.src = c, r("loading"), t.load();
		}
		if (!(i || a)) {
			u = !1, t.pause(), r(d ? "ready" : "loading");
			return;
		}
		if (u || !t.paused) return;
		u = !0, t.preload = "auto";
		let e = l;
		t.play().then(() => {
			!s && l === e && (u = !1, i || a ? r("playing") : (t.pause(), r(d ? "ready" : "loading")));
		}).catch(() => {
			!s && l === e && (u = !1, r(d ? "ready" : "error"));
		});
	}
	let g = () => {
		!c || s || p || i || a || (t.currentTime = 0);
	}, _ = () => {
		!c || s || t.seeking || t.readyState < 2 || (d = !0, i || a || r("ready"));
	}, v = () => {
		i = !0, h();
	}, y = () => {
		i = !1, h();
	}, ee = () => {
		a = !0, h();
	}, b = () => {
		a = !1, h();
	}, te = () => {
		c && !s && r("error");
	}, ne = Gn(t, n, {
		resumePlayback: !1,
		onSourceChange: (e) => {
			l++, c = e, d = !1, u = !1, r("loading"), h();
		}
	});
	e.addEventListener("pointerenter", v), e.addEventListener("pointerleave", y), e.addEventListener("focusin", ee), e.addEventListener("focusout", b), t.addEventListener("error", te), t.addEventListener("loadedmetadata", g), t.addEventListener("loadeddata", _), t.addEventListener("seeked", _), document.addEventListener("visibilitychange", h);
	let x = new IntersectionObserver((e) => {
		o = e[0].isIntersecting, h();
	});
	return x.observe(e), () => {
		s = !0, ne(), x.disconnect(), e.removeEventListener("pointerenter", v), e.removeEventListener("pointerleave", y), e.removeEventListener("focusin", ee), e.removeEventListener("focusout", b), t.removeEventListener("error", te), t.removeEventListener("loadedmetadata", g), t.removeEventListener("loadeddata", _), t.removeEventListener("seeked", _), document.removeEventListener("visibilitychange", h), m();
	};
}
//#endregion
//#region src/album/AlbumVideo.svelte
var lr = s("<span class=\"album-video\"> </span>"), ur = s("<video class=\"album-video-preview\" loop=\"\" playsinline=\"\" preload=\"none\"></video> <!>", 3);
function dr(t, n) {
	m(n, !1);
	let r = T(), o = i(n, "item", 8), s = T(), l = T("idle"), u = _e();
	h(() => cr(O(s).parentElement, O(s), o(), (e) => {
		k(l, e);
	})), ne(() => j(o()), () => {
		k(r, sr(o()));
	}), de(), d();
	var f = ur(), p = le(f);
	p.muted = !0, se(p, (e) => k(s, e), () => O(s));
	var _ = C(p, 2), v = (e) => {
		var t = lr(), n = M(t);
		A((e) => a(n, `▶ ${e ?? ""}`), [() => (j(o()), E(() => String(o().extension).toUpperCase()))]), c(e, t);
	};
	g(_, (e) => {
		O(l) !== "playing" && e(v);
	}), A((e) => {
		D(p, "poster", O(r)), D(p, "aria-label", e);
	}, [() => (j(P), E(() => P("albumMutedVideoPreview")))]), e("loadedmetadata", p, (e) => u("load", e)), c(t, f), ye();
}
//#endregion
//#region src/album/album-timeline-browse.js
function fr(e) {
	if (!e?.timeline) return e;
	if (e.timeline.browsing) return {
		...e,
		timeline: {
			...e.timeline,
			range: { kind: "all" }
		}
	};
	if (Number.isInteger(e.timeline.seedNextStart) && e.timeline.seedNextStart > 1) return {
		...e,
		timeline: {
			...e.timeline,
			browsing: !0,
			range: { kind: "all" },
			postStart: e.timeline.seedNextStart,
			hasMore: e.timeline.seedHasMore !== !1
		}
	};
	let t = e.posts?.[0];
	return {
		...e,
		...t ? {
			posts: [t],
			media: void 0
		} : {},
		timeline: {
			...e.timeline,
			browsing: !0,
			range: { kind: "all" },
			pageSize: 1,
			postStart: 1,
			hasMore: !0
		}
	};
}
function pr(e, t) {
	let n = t, r = !1, i = new IntersectionObserver((e) => {
		!r && n.enabled && !document.hidden && e.some((e) => e.isIntersecting) && n.load();
	}, {
		root: e.closest(".page-content"),
		threshold: 0
	});
	function a() {
		i.disconnect(), !r && n.enabled && !document.hidden && i.observe(e);
	}
	return document.addEventListener("visibilitychange", a), a(), {
		update(e) {
			n = e, a();
		},
		destroy() {
			r = !0, i.disconnect(), document.removeEventListener("visibilitychange", a);
		}
	};
}
//#endregion
//#region src/album/album-page-forecast.js
var mr = 1.5;
function hr(e, t, n = /* @__PURE__ */ new Map()) {
	e = e.filter((e) => e.kind !== "continue");
	let r = Xn(e, t, n);
	return r.reduce((e, t) => e + t.height, 0) + Math.max(0, r.length - 1) * 6;
}
function gr({ tiles: e = [], width: t, targetHeight: n, measuredRatios: r = /* @__PURE__ */ new Map(), limit: i = 10 }) {
	let a = Math.max(1, Math.min(10, Math.floor(i) || 1));
	if (!(t > 0) || !(n > 0)) return 1;
	let o = [], s = /* @__PURE__ */ new Map();
	e.forEach((e, t) => {
		if (e.kind === "continue") return;
		o.push({ ...e });
		let n = s.get(e.postIndex) || [];
		n.push({
			...e,
			width: Yn(e, r.get(t)),
			height: 1
		}), s.set(e.postIndex, n);
	});
	let c = [...s.values()].slice(-6).sort((e, n) => hr(e, t) - hr(n, t)), l = c.slice(0, Math.max(1, Math.ceil(c.length / 2)));
	if (!c.length) return 1;
	for (let e = 1; e <= a; e++) if (o.push(...l[(e - 1) % l.length]), hr(o, t, r) >= n) return e;
	return a;
}
function _r(e, t = 0) {
	let n = e?.closest(".page-content");
	if (!n) return 0;
	let r = e.getBoundingClientRect().top - n.getBoundingClientRect().top + n.scrollTop, i = n.scrollTop + n.clientHeight * mr - r;
	return Math.max(i, (Number.isFinite(t) ? Math.max(0, t) : 0) + n.clientHeight);
}
//#endregion
//#region src/album/album-timeline-client.js
function vr(e, t, n) {
	return t?.throwIfAborted(), new Promise((r, i) => {
		let a = chrome.runtime.connect({ name: "yt-dlp-gui-timeline-browse" }), o = !1, s = !1, c = Promise.resolve(), l = (e, n) => {
			o || (o = !0, s = !0, t?.removeEventListener("abort", d), a.disconnect(), e ? i(e) : r(n));
		}, u = (e, t) => {
			s || (s = !0, c.then(() => l(e, t), (e) => l(e)));
		}, d = () => l(t.reason);
		t?.addEventListener("abort", d, { once: !0 }), a.onMessage.addListener((e) => {
			s || (e?.event === "post" ? (c = c.then(() => {
				if (!o) return n(e.page);
			}), c.catch((e) => l(e))) : e?.event === "complete" ? u(null, e.page) : e?.event === "error" && u(Error(e.message || P("timelineReadFailed"))));
		}), a.onDisconnect.addListener(() => {
			s || u(Error(chrome.runtime.lastError?.message || P("nativeDisconnected")));
		}), a.postMessage({ request: e }), t?.aborted && d();
	});
}
//#endregion
//#region src/album/AlbumApp.svelte
var yr = s("<span class=\"album-navbar-title\"> </span>"), br = s("<div class=\"album-navbar-actions\"><span class=\"album-selected-count\" aria-live=\"polite\"> </span> <!> <!> <!> <!> <!></div>"), xr = s("<!> <!>", 1), Sr = s("<div class=\"album-status\" role=\"status\" aria-live=\"polite\"> </div>"), Cr = s("<time class=\"album-post-card-date\"> </time>"), wr = s("<span class=\"album-post-card-time\"> </span>"), Tr = s("<article class=\"album-post-card\"><button type=\"button\" class=\"album-post-card-open\"><span class=\"album-post-card-date-line\"><!> <!></span> <span class=\"album-post-card-heading\"> </span> <span class=\"album-post-card-content\"> </span> <span class=\"album-post-card-meta\"> </span></button></article>"), Er = s("<article class=\"album-post-card\"><button type=\"button\" class=\"album-post-card-open album-continuation\"><!> <span class=\"album-post-card-heading\"> </span> <span class=\"album-post-card-content\" role=\"status\"> </span></button></article>"), Dr = s("<article><div class=\"album-image-frame\"><button type=\"button\" class=\"album-preview\"><!></button> <label class=\"album-select-toggle\"><input type=\"checkbox\"/></label></div> <div class=\"album-tile-footer\"><span> </span></div></article>"), Or = s("<div></div>"), kr = s("<a target=\"_blank\" rel=\"noopener noreferrer\"> </a>"), Ar = s("<header class=\"album-post-dialog-header\"><strong> </strong> <button type=\"button\">✕</button></header> <p class=\"album-post-dialog-meta\"> </p> <p class=\"album-post-dialog-text\"> </p> <!>", 1), jr = s("<!> <section class=\"album-content\"><!> <div class=\"album-grid\"></div></section> <dialog class=\"album-post-dialog\"><!></dialog>", 1);
function Mr(n, i) {
	m(i, !1);
	let s = tr(), v = matchMedia("(prefers-color-scheme: dark)"), ee = T(v.matches), te = "zoom", ne = T(), w = T(null), E = T([]), de = T([]), j = T([]), _e = [], F = T(null), Ee = T(), I = T(/* @__PURE__ */ new Set()), L = T(""), R = T(!1), z = T(), ke = T([]), Ae = /* @__PURE__ */ new Map(), je, B = T(!1), V = T(""), Me, H = T(!0);
	function Ne(e) {
		k(w, e);
		let t = Zn(e);
		k(E, t.media), k(de, t.posts), k(j, t.tiles), _e = t.postByMediaIndex, Fe();
	}
	async function Pe() {
		if (!O(H) || O(B) || O(R) || !O(w)?.timeline?.hasMore) return;
		k(B, !0), k(V, ""), Me = new AbortController();
		let e = hr(O(j), O(z)?.clientWidth, Ae), t = _r(O(z), e);
		async function n(e, t = !1) {
			if (!O(H) || Me.signal.aborted) return;
			let n = De(O(w), e, { allowSameCursor: t });
			Ne(n), await pe(), O(H) && !Me.signal.aborted && await chrome.storage.session.set({ [je]: n });
		}
		try {
			do
				await n(await vr({
					...we(O(w)),
					postCount: gr({
						tiles: O(j),
						width: O(z)?.clientWidth,
						targetHeight: t,
						measuredRatios: Ae
					})
				}, Me.signal, (e) => n(e)), !0);
			while (O(H) && !document.hidden && O(w).timeline.hasMore && hr(O(j), O(z)?.clientWidth, Ae) < t);
		} catch (e) {
			O(H) && k(V, String(e?.message || e));
		} finally {
			O(H) && k(B, !1);
		}
	}
	function Fe() {
		O(z) && k(ke, Xn(O(j), O(z).clientWidth, Ae));
	}
	function Ie(e, t) {
		let n = t.currentTarget, r = n.naturalWidth || n.videoWidth, i = n.naturalHeight || n.videoHeight;
		if (!r || !i) return;
		let a = Yn(null, r / i), o = Ae.get(e);
		if (o != null && Math.abs(o - a) < .01) return;
		let s = O(j)[e];
		if (s?.kind !== "media") return;
		let c = O(E)[s.mediaIndex];
		c?.width > 0 && c?.height > 0 || (Ae = new Map(Ae).set(e, a), Fe());
	}
	h(() => {
		let e = oe(chrome.storage, (e) => {
			te = e;
		}, (e) => {
			k(L, P("albumPreferencesFailed", { message: String(e?.message || e) }));
		}), t = (e) => {
			k(ee, e.matches);
		};
		v.addEventListener("change", t);
		let n = new ResizeObserver(Fe);
		O(z) && n.observe(O(z));
		let r = !0;
		return Te(chrome.storage.session, location.href).then(async (e) => {
			if (!r) return;
			je = e.key, Ne(fr(e.gallery));
			let t = await er(chrome.storage.session, location.href);
			if (r && t) {
				k(I, new Set(O(E).map((e, t) => t))), k(R, !0), k(ne, new AbortController()), k(L, P("albumDownloadAuthorized"));
				try {
					await Se({
						gallery: e.gallery,
						media: O(E),
						selected: O(I),
						asZip: t === "zip",
						permissionsAlreadyGranted: !0,
						session: s,
						signal: O(ne).signal,
						progress: (e) => {
							r && k(L, e);
						}
					});
				} catch (e) {
					r && k(L, e?.name === "AbortError" ? P("downloadStopped") : P("browserDownloadFailed", { message: String(e?.message || e) }));
				} finally {
					r && k(R, !1);
				}
			}
		}).catch((e) => {
			r && k(L, String(e?.message || e));
		}), () => {
			r = !1, k(H, !1), e(), n.disconnect(), v.removeEventListener("change", t), O(ne)?.abort(), Me?.abort(new DOMException("Album closed", "AbortError")), Jn(), s.dispose();
		};
	});
	function Le(e) {
		let t = new Set(O(I));
		t.has(e) ? t.delete(e) : t.add(e), k(I, t);
	}
	function Re() {
		k(I, new Set(O(E).map((e, t) => t)));
	}
	function ze() {
		k(I, /* @__PURE__ */ new Set());
	}
	async function U(e) {
		try {
			qn(O(E), e, O(w), _e, {
				session: s,
				wheelMode: () => te,
				report: (e) => {
					k(L, e);
				}
			}), k(L, "");
		} catch (e) {
			k(L, P("albumViewerFailed", { message: String(e?.message || e) }));
		}
	}
	function W(e) {
		k(F, e), O(Ee)?.showModal();
	}
	async function Be(e) {
		if (!(O(R) || O(B) || !O(I).size)) {
			k(R, !0), k(ne, new AbortController()), k(L, P("downloadPreparing"));
			try {
				await Se({
					gallery: O(w),
					media: O(E),
					selected: O(I),
					asZip: e,
					session: s,
					signal: O(ne).signal,
					progress: (e) => {
						k(L, e);
					}
				});
			} catch (e) {
				k(L, e?.name === "AbortError" ? P("downloadStopped") : String(e?.message || e || P("downloadFailed")));
			} finally {
				k(R, !1);
			}
		}
	}
	d(), x(n, {
		name: "yt-dlp-gui Album",
		id: "me.kannagi.ytdlpgui.album",
		theme: "md",
		get touch() {
			return me;
		},
		get dark() {
			return O(ee);
		},
		children: (n, i) => {
			ve(n, {
				main: !0,
				children: (n, i) => {
					re(n, {
						name: "gallery-album",
						class: "album-page",
						children: (n, i) => {
							var d = jr(), m = le(d);
							ce(m, {
								children: (e, t) => {
									var n = xr(), i = le(n);
									he(i, {
										children: (e, t) => {
											var n = yr(), r = M(n);
											A((e) => a(r, `${(O(w)?.siteName || "Gallery") ?? ""} · ${e ?? ""}`), [() => P("album")]), c(e, n);
										},
										$$slots: { default: !0 }
									});
									var o = C(i, 2);
									ie(o, {
										children: (e, t) => {
											var n = br(), i = S(n), o = M(i, !0), s = C(i, 2);
											{
												let e = ue(() => O(R) || O(B) || !O(E).length || O(I).size === O(E).length);
												b(s, {
													small: !0,
													outline: !0,
													get disabled() {
														return O(e);
													},
													onClick: Re,
													children: (e, t) => {
														r();
														var n = y();
														A((e) => a(n, e), [() => P("selectAll")]), c(e, n);
													},
													$$slots: { default: !0 }
												});
											}
											var l = C(s, 2);
											{
												let e = ue(() => O(R) || O(B) || O(I).size === 0);
												b(l, {
													small: !0,
													outline: !0,
													get disabled() {
														return O(e);
													},
													onClick: ze,
													children: (e, t) => {
														r();
														var n = y();
														A((e) => a(n, e), [() => P("clearSelection")]), c(e, n);
													},
													$$slots: { default: !0 }
												});
											}
											var u = C(l, 2);
											{
												let e = ue(() => O(R) || O(B) || O(I).size === 0);
												b(u, {
													small: !0,
													fill: !0,
													class: "primary-action",
													get disabled() {
														return O(e);
													},
													onClick: () => Be(!1),
													children: (e, t) => {
														r();
														var n = y();
														A((e) => a(n, e), [() => P("download")]), c(e, n);
													},
													$$slots: { default: !0 }
												});
											}
											var d = C(u, 2);
											{
												let e = ue(() => O(R) || O(B) || O(I).size === 0);
												b(d, {
													small: !0,
													outline: !0,
													get disabled() {
														return O(e);
													},
													onClick: () => Be(!0),
													children: (e, t) => {
														r();
														var n = y("ZIP");
														c(e, n);
													},
													$$slots: { default: !0 }
												});
											}
											var f = C(d, 2), p = (e) => {
												b(e, {
													small: !0,
													fill: !0,
													onClick: () => O(ne)?.abort(),
													children: (e, t) => {
														r();
														var n = y();
														A((e) => a(n, e), [() => P("stop")]), c(e, n);
													},
													$$slots: { default: !0 }
												});
											};
											g(f, (e) => {
												O(R) && e(p);
											}), N(n), A((e, t) => {
												D(n, "aria-label", e), a(o, t);
											}, [() => P("albumActions"), () => P("albumSelected", {
												count: O(I).size,
												total: O(E).length
											})]), c(e, n);
										},
										$$slots: { default: !0 }
									}), c(e, n);
								},
								$$slots: { default: !0 }
							});
							var h = C(m, 2), v = S(h), ee = (e) => {
								var t = Sr(), n = M(t, !0);
								A(() => a(n, O(L))), c(e, t);
							};
							g(v, (e) => {
								O(L) && e(ee);
							});
							var te = C(v, 2);
							_(te, 5, () => O(ke), (e) => e.id, (e, n) => {
								var r = Or();
								_(r, 5, () => O(n).items, (e) => e.index, (e, r) => {
									let i = ue(() => O(j)[O(r).index]);
									var d = fe(), m = le(d), h = (e) => {
										let t = ue(() => O(de)[O(i).postIndex]);
										var s = Tr(), l = S(s), u = S(l), d = S(u), p = (e) => {
											var n = Cr(), r = M(n, !0);
											A(() => {
												D(n, "datetime", O(t).dateOnly), a(r, O(t).dateOnly);
											}), c(e, n);
										};
										g(d, (e) => {
											O(t).dateOnly && e(p);
										});
										var m = C(d, 2), h = (e) => {
											var n = wr(), r = M(n, !0);
											A(() => a(r, O(t).timeOnly)), c(e, n);
										};
										g(m, (e) => {
											O(t).timeOnly && e(h);
										}), N(u);
										var _ = C(u, 2), v = M(_, !0), y = C(_, 2), ee = M(y, !0), b = C(y, 2), te = M(b, !0);
										N(l), N(s), A((e, t, i) => {
											o(s, O(n).justified ? `flex: ${O(r).ratio} 1 0%;` : `flex: 0 0 ${O(r).width}px;`), a(v, e), a(ee, t), a(te, i);
										}, [
											() => O(t).creator || O(t).title || O(t).siteName || P("post"),
											() => O(t).description || O(t).title || P("postViewInfo"),
											() => P("galleryMediaCount", { count: O(t).count })
										]), f("click", l, () => W(O(t))), c(e, s);
									}, _ = (e) => {
										var t = Er(), i = S(t), s = S(i), l = (e) => {
											ge(e, { size: 32 });
										};
										g(s, (e) => {
											O(B) && e(l);
										});
										var u = C(s, 2), d = M(u, !0), p = C(u, 2), m = M(p, !0);
										N(i), N(t), be(t, (e, t) => pr?.(e, t), () => ({
											load: Pe,
											enabled: O(H) && O(w).timeline.hasMore && !O(B) && !O(R) && !O(V)
										})), A((e, s) => {
											o(t, O(n).justified ? `flex: ${O(r).ratio} 1 0%;` : `flex: 0 0 ${O(r).width}px;`), i.disabled = O(B) || O(R) || !O(w).timeline.hasMore, a(d, e), a(m, s);
										}, [() => O(B) ? P("albumReading") : O(w).timeline.hasMore ? O(V) ? P("albumReadRetry") : P("albumReadMore") : P("albumReadDone"), () => O(V) || (O(w).timeline.hasMore ? P("albumReadHint") : P("albumNoMore"))]), f("click", i, Pe), c(e, t);
									}, v = (e) => {
										let d = ue(() => O(i).mediaIndex), m = ue(() => O(E)[O(d)]);
										var h = Dr(), _ = S(h), v = S(_), y = S(v), ee = (e) => {
											dr(e, {
												get item() {
													return O(m);
												},
												$$events: { load: (e) => Ie(O(r).index, e.detail) }
											});
										}, b = ae(() => xe(O(m))), te = (e) => {
											or(e, {
												get item() {
													return O(m);
												},
												get session() {
													return s;
												},
												get index() {
													return O(d);
												},
												$$events: { load: (e) => Ie(O(r).index, e.detail) }
											});
										};
										g(y, (e) => {
											O(b) ? e(ee) : e(te, -1);
										}), N(v);
										var ne = C(v, 2), x = S(ne);
										t(x), N(ne), N(_);
										var re = C(_, 2), ie = S(re), T = M(ie, !0);
										N(re), N(h), A((e, t, i, s, c, d) => {
											l(h, 1, e), o(h, O(n).justified ? `flex: ${O(r).ratio} 1 0%;` : `flex: 0 0 ${O(r).width}px;`), D(v, "aria-label", t), u(x, i), x.disabled = O(R) || O(B) && O(w).timeline?.postStart === 1, D(x, "aria-label", s), D(ie, "title", c), a(T, d);
										}, [
											() => p(O(I).has(O(d)) ? "album-tile selected" : "album-tile"),
											() => P("albumViewItem", { number: O(d) + 1 }),
											() => O(I).has(O(d)),
											() => P("albumSelectItem", { number: O(d) + 1 }),
											() => Ce(O(m), O(d)),
											() => Ce(O(m), O(d))
										]), f("click", v, () => U(O(d))), f("change", x, () => Le(O(d))), c(e, h);
									};
									g(m, (e) => {
										O(i).kind === "post" ? e(h) : O(i).kind === "continue" ? e(_, 1) : e(v, -1);
									}), c(e, d);
								}), N(r), A(() => {
									l(r, 1, p(O(n).justified ? "album-row justified" : "album-row")), o(r, `height: ${O(n).height}px`);
								}), c(e, r);
							}), N(te), se(te, (e) => k(z, e), () => O(z)), N(h);
							var x = C(h, 2), re = S(x), T = (e) => {
								var t = Ar(), n = le(t), r = S(n), i = M(r, !0), o = C(r, 2);
								N(n);
								var s = C(n, 2), l = M(s, !0), u = C(s, 2), d = M(u, !0), p = C(u, 2), m = (e) => {
									var t = kr(), n = M(t);
									A((e) => {
										D(t, "href", O(F).sourcePageUrl), a(n, `${e ?? ""} ↗`);
									}, [() => P("postOpenOriginal")]), c(e, t);
								}, h = ae(() => Oe(O(F).sourcePageUrl));
								g(p, (e) => {
									O(h) && e(m);
								}), A((e, t, n, r) => {
									a(i, e), D(o, "aria-label", t), a(l, n), a(d, r);
								}, [
									() => O(F).creator || O(F).siteName || P("postInfo"),
									() => P("postCloseInfo"),
									() => [O(F).date, P("galleryMediaCount", { count: O(F).count })].filter(Boolean).join(" · "),
									() => O(F).description || O(F).title || P("postEmpty")
								]), f("click", o, () => O(Ee)?.close()), c(e, t);
							};
							g(re, (e) => {
								O(F) && e(T);
							}), N(x), se(x, (e) => k(Ee, e), () => O(Ee)), A((e, t) => {
								D(h, "aria-label", e), D(te, "aria-label", t);
							}, [() => P("albumMedia"), () => P("albumGrid")]), e("close", x, () => k(F, null)), c(n, d);
						},
						$$slots: { default: !0 }
					});
				},
				$$slots: { default: !0 }
			});
		},
		$$slots: { default: !0 }
	}), ye();
}
ee(["click", "change"]), n.use(te), v(Mr, { target: document.getElementById("app") });
//#endregion
