# yt-dlp-gui v2: Cache and Output Frontend Architecture

## 1. Scope and status

This document covers the cache-location workflow for Issue #32 and the download-output workflow for Issue #33. The filename follows this workspace's `AGENTS.md`; the application is yt-dlp-gui v2, built with Rust and egui.

Status: Issue #32 passed source/controlled-evidence review, including the supplied current-build GUI observations, in [Reviewer Round 3](docs/reviews/REVIEW_CACHE_LOCATION_ISSUES.md). The [cache Builder handoff](docs/implementation/CACHE_LOCATION_BUILDER_STATUS.md) retains execution/evidence boundaries. The user confirmed Section 8; its implementation and Builder checks are recorded in the [Issue #33 handoff](docs/implementation/ISSUE_33_BUILDER_HANDOFF.md). Independent combined review is pending. Package/release acceptance remains separate.

Related contracts: [product requirements](EROffice_Architecture_Spec.md), [backend reference](docs/backend-reference.md), [ADR-0001](docs/architecture-decisions/ADR-0001-unified-cache-location.md), and [ADR-0002](docs/architecture-decisions/ADR-0002-gallery-dl-playlist-runtime.md).

Issue #33 decision: [ADR-0003](docs/architecture-decisions/ADR-0003-output-naming-and-no-clobber-publication.md).

## 2. Options > Cache

Keep this workflow inside the existing Cache group in `src/app/ui/options_cache.rs`. Do not add a separate settings page or folder-selection dialog.

| Element | Canonical English text | Source of truth |
| --- | --- | --- |
| Location selector | `Cache location` | Last successfully saved preference, not the active runtime mode |
| Portable choice | `Application folder (portable)` | `v2_cache` |
| Temporary choice | `System temporary folder` | `windows_temp` |
| Active path row | `Current cache folder` | Immutable active runtime cache root |
| Pending path row | `After restart` | Resolved folder for the saved next-start preference; visible when different from active |
| Pending indicator | `Restart required` | Saved and active effective storage locations differ |
| Usage and existing cleanup controls | Preserve their existing intent | Active cache root only |

Show absolute, selectable or copyable path text with wrapping so paths with spaces and non-ASCII characters remain readable. Do not display `yt-dlp default` as a directory. A requested location whose resolution fails displays the error instead of a guessed folder.

## 3. State and transitions

Maintain a strict distinction between the startup `RuntimeStoragePaths` snapshot and the mutable saved preference held by application settings.

| Event | Required behavior |
| --- | --- |
| Settings opened | Read active path from the runtime snapshot and selected choice from the last successfully saved preference. Never resolve usage from a pending preference. |
| Different choice selected | Build a candidate configuration, persist it through the backend contract, then update the displayed saved preference on success. Do not mutate `ToolPaths`, the active snapshot, workers, or thumbnail ownership to apply it immediately. |
| Save succeeded | Show the pending folder and restart notice when it differs from active. Keep playback, downloads, new work, statistics, and cleanup on the active folder. |
| Save failed | Retain the previous saved selection and active location. Surface `Could not save cache location: {error}`. |
| User selects the active location again | Persist that choice and remove the restart notice after successful save. No restart is needed solely to undo a pending change. |
| Application restarted | Initialize the active snapshot from saved settings. Remove pending-location UI when initialization succeeds. |
| Storage or migration unavailable | Show the failing path and operation. Keep the location selector usable to save a repair for next start; disable affected cache-dependent operations and destructive cleanup. |

Do not add a restart button or automatically terminate/relaunch the application for this task.

## 4. Explanations and compatibility notice

Use short stable explanatory UI text with these meanings:

- `Changing this setting takes effect after restart. Existing cache files stay in their previous folder.`
- `Usage and cleanup apply to the current cache folder only.`
- `System temporary cache files may be removed by Windows or other system cleanup tools.`
- `Application state and update recovery data remain in the application folder.`

When the loader detects legacy `yt_dlp_default`, show:

> The old Default cache option has been replaced by Application folder (portable). All application caches now use the selected location.

Show this notice on the migration launch; successful canonical configuration persistence prevents it from recurring on future launches. If persistence fails, report the failure and do not mark the migration as saved. The active normalized mode may be portable for that launch, but no successful-save claim is permitted.

## 5. Refresh and cleanup

- Refresh measures only the active cache's managed disposable content. Exclude legacy persistent-state sources, ownership metadata, and unrelated files as specified by the backend.
- Include native-cookie snapshots and gallery-dl intermediate work in the same current-folder usage. Persistent managed Cookie files remain excluded. There is no separate counter or location selector for these consumers.
- Keep the existing expired-audio, audio-cache, and all-cache actions. Their names do not expand their ownership beyond the active root.
- Backend cleanup returns a structured outcome: completed, busy, blocked by storage/migration failure, or failed. Display removed content and any protected-content explanation accurately.
- While relevant work is using the cache, do not start cleanup or silently cancel the work. Use the feedback `Cache is in use. Try again when current tasks have finished.`
- This busy state also covers a native-cookie snapshot from creation through its final read/removal, and a gallery-dl operation through subprocess exit, output handling, and work-directory release. Saving a new preference during either operation only changes the next-start location.
- Explain pending-update protection when it prevents removal: `Pending update files were kept.` Do not report that every file was removed when some files are protected.
- Cleanup must not target the saved next-start location, inactive old roots, persistent state, or the system temporary directory itself.

Use existing operation-error feedback for native-cookie creation failures or gallery-dl cache/path failures, with the operation and failing path where safe. Do not display or log Cookie values or serialized snapshots. The supplement adds no UI page, restart behavior, or locale preference change.

### 5.1 Gallery-dl compatibility and dependency actions

Use existing dependency readiness, About/tool status, tool-path selection, and per-operation error surfaces. Do not add another cache selector, compatibility override, or settings page. A file's presence and upstream version alone do not establish readiness. Distinguish a missing executable from an installed incompatible build. Block gallery analysis/download preparation on an unverified runtime; keep settings, tool maintenance, and the independent yt-dlp backend usable.

Canonical fixed-English operation feedback has these meanings:

- `This gallery-dl build cannot keep playlist work in the selected cache folder. Install the compatible gallery-dl runtime.`
- `No approved gallery-dl bundle is available locally. The current runtime was kept.`
- `gallery-dl is in use. Try again when current gallery tasks have finished.`
- `This gallery-dl build is pinned for cache compatibility. Unverified upstream updates are not installed.`

Show the actual failing configured path and repair action where useful. Do not describe an incompatible executable as missing or suggest changing the cache location as its repair. The existing install/update button consumes the approved local bundle described in ADR-0002; if absent or invalid, keep the installed runtime and show the reason. Batch component updates report gallery-dl as kept/skipped for compatibility rather than claiming it was updated or silently fetching upstream `latest`. Other components retain their existing behavior. An external configured runtime is never replaced by managed-tool maintenance.

Show upstream version and a concise compatible-build revision in tool details after validation; keep binary hashes, source locks, and protocol field names in diagnostics/evidence. Do not claim that a locally built runtime is an official upstream release or an installed application package.

Playlist completion messages, desktop metadata processing, and native-host streaming use actual completed/skipped artifact paths. Multiple files from one source retain the native source archive key, with exactly one artifact event per file; preserve the source-level post boundary. A failed child must not turn the entire source into a successful completion. Existing-file skips must preserve files and avoid media transfer; no completion is fabricated for an absent aggregate path. Metadata-only preview retains its existing source-level semantics and does not claim that media was downloaded.

### 5.1.1 Background gallery status refresh (R-CACHE-007)

Main/start controls and About component views render the stored gallery readiness result; they never synchronously run inventory hashes, directory scans or capability processes. Startup, gallery-path changes, managed install/update/recovery results and the existing dependency-status refresh request start a background check. Repeated redraws or switching views reuse its pending/result state. Show checking while pending, then the actual Ready, Missing, Incompatible or Busy result with its configured path/detail. A result for a superseded path/installation request must not replace current status. Reuse the existing dependency actions; add no selector, compatibility override or polling setting.

This is display state, not permission to use an unverified runtime. Actual gallery actions still validate under their operation lease and report current preparation errors through the existing surfaces. A stale display observation must never bypass that boundary. Backend 7.3.1 and ADR-0002 section 4.1 define the state/lease distinction. Acceptance must operate real Main/About views with the approved local bundle and record repaint/full-check/capability counts and observed UI responsiveness, including path/managed-install refresh and missing/incompatible/busy states. No latency threshold or current implementation pass is asserted by this design amendment.

Re-review amendment for R-CACHE-007/008: the user confirmed the proposal; the user confirmed this structural detail before follow-up implementation. Gallery completion requests a repaint through the existing GUI context so an otherwise idle production window consumes the ready/error result without user input. Verify startup, path change and managed-install completion with clipboard monitoring off and without an unconditional fixture repaint. Pending keeps the Checking badge/detail, disables About's gallery Install action, and produces checking guidance in Main instead of missing-tool installation guidance. The shared maintenance entry must likewise avoid scheduling gallery installation/update while pending. Preserve the actual post-check Missing/Incompatible/Busy/Ready actions and external-tool ownership. The backend acceptance includes both managed and configured-external pending paths, unchanged external bytes/settings, and normal Missing installation. Add no control, polling preference or compatibility override.

## 6. Language and integration

English is the authoring language and canonical source for new UI copy, documentation, code comments, and diagnostics in this task. Do not change the user's selected locale or make English the application's global UI preference.

Stable labels, choices, and explanatory copy use explicit i18n keys and the existing localization boundary. Preserve the existing canonical key coverage and ordering rules for selectable locales; translation changes are limited to the new or directly changed cache strings. Operation feedback and internal diagnostics remain fixed English under `src/i18n/README.md`.

The primary integration points are `options_cache.rs`, `options_layout.rs`, `state/prepare_actions.rs`, `state/cache_actions.rs`, the application state model, and the directly affected locale entries. Existing untranslated technical paths remain raw data.

## 7. Frontend acceptance

Verify the selector, active/pending rows, restart notice, legacy notice, save-error rollback, disabled/error states, cleanup feedback, compatible/incompatible tool status, local-bundle failures, and update-kept feedback against backend cases `AC-01` through `AC-25`. Include busy/retained-file feedback and current-root usage while a native-cookie snapshot or gallery-dl job is live. Confirm long English labels and long/non-ASCII paths do not overlap neighboring controls. Static mockups or old binaries are not current-build acceptance evidence.

## 8. Issue #33: download output

### 8.1 Settings and persistence

Add one **Download output** group to the existing Options root, adjacent to the existing file-action/cache settings. Reuse the settings form, controls, font fallback and i18n boundary. Keep output-folder and per-item Save As actions at their existing locations.

| Element | Canonical English text / choices | Behavior |
| --- | --- | --- |
| Naming selector | `File naming`: `Title`, `Title + video ID`, `Uploader + title + video ID`, `Custom template` | Initial selection is the last saved preference; Title is the default. |
| Conflict selector | `Existing files`: `Keep both files`, `Skip existing files` | Keep both is the default. No overwrite choice. |
| Custom draft | `Filename template` | Editable only for Custom; show the four supported fields and brace escaping. Extension and folder are separate. Retain the last custom text when another preset is selected. |
| Preview | `Filename preview` | Use the selected/active item's available metadata; otherwise label the example `Sample preview`. Show the rendered basename/extension as selectable wrapped text; collision suffixes are chosen at execution. No metadata network request is made just to preview settings. |
| Save | `Save output settings` | Persist the complete validated draft; disable only when unchanged or invalid. Dirty state is labeled `Unsaved changes`; it must not appear to be the active saved setting. |
| Help | `Applies to new automatic downloads. Existing explicit filenames are kept. Gallery downloads keep their own naming rules.` | Running jobs retain their captured policy; cache location still changes only after restart. |

Preset previews use `{title}`, `{title} [{id}]`, and `{uploader} - {title} [{id}]` respectively. Preview is indicative: a collision suffix is chosen at execution/publication and must not be promised from a settings-only preview. Validate unsupported fields, unbalanced braces, unsafe literal characters and empty templates inline. Do not silently save an invalid custom draft or discard unrelated settings.

Successful save: `Output settings saved. Applies to new downloads.` Failure: `Could not save output settings: {error}`. Retain both the prior saved setting and the user's editable draft on failure so retry is possible. Draft controls remain distinct from saved job defaults. An invalid stored preference reports its field and safe effective fallback; it does not reset the entire application configuration.

### 8.2 Automatic and explicit names

New queue items carry automatic/explicit naming provenance. Metadata refresh and output-preference changes must not overwrite an explicit filename. Save As marks the item explicit. New automatic jobs resolve the current saved rule at dispatch and freeze the resulting policy/metadata. Already running jobs and their retries keep that snapshot.

Legacy persisted nonempty filenames with no provenance are preserved as explicit; legacy empty names use automatic naming. Explain this compatibility rule in settings help/diagnostic details when applicable. No new batch renaming or retrospective migration control is introduced.

External yt-dlp output configuration takes precedence over the app naming preset where the existing supported configuration subset can be safely planned. For those items show `Filename controlled by yt-dlp configuration`; the collision policy remains enforced. App-wide output settings stay editable for other items. Unsafe/unsupported output configuration fails preflight with a repair explanation before media work starts.

R-OUT-012 re-review amendment (proposal and structural detail confirmed before follow-up implementation): within each Main render, compute the output-control config observation once and share its locked/display decision between the target selector and path textbox. A later render obtains a fresh observation; do not persist this as UI configuration state. Playback/prefetch primary and cookie-free retry commands separately share their operation's effective config as specified in backend 12.5.1. Option-looking values consumed by supported options do not control the selector, format or output directory. Acceptance checks both controls together and retains the actual local-tool matching-title cases and operation read/parse/validation counts. Existing UI choices and config precedence remain the same.

Protected-configuration follow-up for R-OUT-016 through R-OUT-019 (proposal confirmed on 2026-10-28; structural detail confirmed on 2026-10-29 before implementation): use existing item/result and analysis-error surfaces for config rejection, with English diagnostics identifying the supplied option/value and a repair action. For a standalone config `--`, ask the user to remove it. For an unsupported built-in preset, identify the preset and direct the user to the app's corresponding audio/conversion operation or supported individual config options. `sleep` remains the supported built-in preset. A benign leading UTF-8 BOM needs no warning or configuration rewrite.

Analyze, discovery and naming preflight must remain visibly analysis/preparation operations: no media download, completed-output feedback or nominal output file may result from config `--no-simulate`. A rejected config ends that attempt with the existing error feedback; it must not trigger an unprotected fallback. Actual downloads retain supported config precedence and the existing collision/result flow after protected preparation. Reuse backend 12.5.4 for explicit/portable input and metadata-only enforcement, including one shared config result per operation. Add no setting, bypass switch, preset-expansion UI or native protocol change.

### 8.3 Collision and completion feedback

- Keep both: report `Saved as {path}` using the actual suffixed result, and retain the concrete path for open/reveal/history actions. Media and its sidecars use the same selected basename; chapter exports select one unoccupied chapter directory.
- Skip: report `Skipped existing destination: {path}` and the selected policy. Do not label the source newly downloaded or run conversion/embedding against the existing file. A conflicting thumbnail or subtitle may cause the item to skip; show which existing artifact caused the decision.
- Live reservation: show `Output destination is in use. Waiting for it to become available.` when Skip must wait to distinguish active work from a completed file. Waiting remains cancellable. Keep both may choose another free basename.
- Partial publication: report `Output publication failed: {error}` and list any complete files this operation already published. Retain those paths without presenting the entire requested group as complete. Existing files never become rollback targets. The disposition and path list come from typed transaction/result state for desktop and native consumers; diagnostic wording cannot select the outcome. Native error responses reuse `outputPath`/`outputPaths` in the existing envelope as specified in backend 12.5.2.
- Cancellation: preserve established files; stop/drain owned producers under the ordinary-Stop boundary in backend 12.4.1 before releasing output/cache ownership. If an exited parent's descendant still holds a pipe, retain the stopping/waiting state and busy ownership until natural drain completes. Do not show terminal cancellation or a completed group while consumers remain unsettled. Only unpublished files belonging to the operation are cleanup candidates. This boundary was selected by the user on 2026-10-28 for R-OUT-015 and the user confirmed the structural amendment before implementation; no new setting or action is introduced.

Show output feedback in the relevant item/result area and the Options group for settings-save feedback; do not depend on a footer below a full-height scroll area. Wrap English text and long/non-ASCII paths. Keep error text, paths and diagnostics free of Cookie data. Native-host responses report the same actual destination and explicit skipped/failed outcome through their existing response envelope, rather than exposing cache staging paths as completed outputs.

### 8.4 Frontend acceptance and scope

Capture current-source UI evidence for both selectors, all preset/custom previews, invalid custom input, successful persistence/restart, save failure/retry, unsaved-versus-saved state, explicit filename priority, suffixed completion, skip and partial/error feedback. Include long English labels, CJK/literal-percent names and a narrow window. Map to backend `OUT-AC-01` through `OUT-AC-14` and preserve the #32 cache-setting checks affected by the combined change. Record actual interactions separately from internal state assertions and compile-only checks. Approval of this document authorizes only the subsequent agreed implementation scope; it is not implementation or Reviewer acceptance.

For the protected-configuration follow-up, verify readable rejection detail for the terminator/preset and safe metadata-only/preparation feedback using the existing views. Keep config output-control state consistent within one Main render. GUI checks run silently or on a private non-input desktop, with project-local state and application-renderer captures; do not switch the user's active desktop, move its cursor, send global input or steal foreground focus. Identify controlled setup/window-message actions separately from physical user interactions. These GUI observations supplement the actual tool/transfer/hash evidence required by backend 12.5.4.
