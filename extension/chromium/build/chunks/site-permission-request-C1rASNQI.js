//#region src/shared/site-permission-request.js
var e = ["http://*/*", "https://*/*"];
async function t(t) {
	return !t.length || await chrome.permissions.contains({ origins: t }) ? !0 : chrome.permissions.request({ origins: e });
}
//#endregion
export { t };
