# Cache Location: Reviewer Issues

## Review baseline and decision

- Latest re-review date: 2026-10-29 (Australia/Perth); earlier rounds are retained below.
- Role: Reviewer. Only review reports were created/updated. No product, test, configuration, design, dependency, or build file was changed by Reviewer.
- Worktree: `C:/Users/SKY/orca/workspaces/yt-dlp-gui-v2/cache-location-bug-maybe`.
- Branch: `AurenQY/cache-location-bug-maybe`.
- Source baseline: `047113c1b550be2cd2347257eef22d5604659eea` (`2026.10.10`), plus the current uncommitted Builder changes.
- Scope: Round 1 covered all 47 changed tracked files and additional task files; Round 2 covered the 51-file tracked diff and its repair material. Round 3 covered the then-current 55-file tracked diff, the new export/import regression sources, output ownership and streaming teardown repairs, and the supplied GUI evidence. Later combined re-reviews and the current source boundary are recorded below and in the companion output report. The two root architecture documents, backend reference and ADR-0001/0002 remain the contract. `/docs/` is ignored by Git; this report and the local design/handoff documents are not tracked or published.
- Requirements: CACHE-01 through CACHE-14; backend AC-01 through AC-25. The accepted gallery-runtime amendment is included, rather than treating its dependency changes as unrelated work.
- Historical decision, Round 3: **Passed for the documented cache-location implementation and the supplied controlled validation scope.** R-CACHE-001 through R-CACHE-005 and V-CACHE-001 were closed with the evidence qualifications below.
- Historical combined-review decision, 2026-10-27: the #33 re-review found R-CACHE-006 and did not pass the combined changes. That finding was subsequently repaired and independently re-reviewed as recorded below.
- Historical combined-review decision, Round 6: **Passed for the then-reviewed #32/#33 source and supplied controlled Windows evidence.** R-CACHE-006 remains closed for its reported lookup/link/expiry boundary; R-OUT-011 remains historical evidence for its stronger controlled Windows scenario, since narrowed by the user's R-OUT-015 decision. This was not installed-package, unrestricted live-service or release acceptance.
- Current combined-review decision, 2026-10-29: **Passed for the combined #32/#33 source and supplied controlled Windows evidence.** R-CACHE-007/008 and R-OUT-012 through R-OUT-020 retain their documented closures, including the new closure of R-OUT-020; R-OUT-015 is closed under the separately confirmed narrower Stop contract. Installed-package, real-user and release acceptance are not implied. See `REVIEW_ISSUE_33_OUTPUT_ISSUES.md`.

## Issue ledger: current combined-review status

| ID | Severity | Kind | Status | Summary |
| --- | --- | --- | --- | --- |
| R-CACHE-001 | High / P1 | Lifecycle defect | Closed, Round 2 | Detached playback producers now independently retain their storage leases. |
| R-CACHE-002 | Medium / P2 | External output expansion | Closed, Round 2 | The reported home-expansion forms are rejected before download preparation. |
| R-CACHE-003 | Medium / P2 | Direct-tool path safety | Closed, Round 2 | Enabled direct information-cache/temp children now use the checked directory helper. |
| R-CACHE-004 | High / P1 | Deletion safety | Closed, Round 3 | Existing ancestors resolve to filesystem ownership; alias rejection, startup and cached export routes are covered. |
| R-CACHE-005 | Medium / P2 | Streaming lifecycle | Closed, Round 3 | Actual import owners retain leases through subprocess wait and output teardown. |
| V-CACHE-001 | Medium / P2 | GUI evidence | Closed, Round 3 | Final controlled-binary screenshots, settings/state records and operations now supply the previously missing evidence. |
| R-CACHE-006 | High / P1 | Deletion safety | Closed, subsequent re-review | Lookup no longer deletes; checked audio paths, link rejection and guarded owned-entry expiry cover the reported boundary. |
| R-CACHE-007 | Medium / P2 | UI responsiveness / background completion | Closed, follow-up re-review | Background completion requests UI repaint, with controlled idle wakeup evidence. |
| R-CACHE-008 | Medium / P2 | Pending status / install action | Closed, follow-up re-review | Pending controls and maintenance entry defer installation in the supplied managed/external cases. |

The original issue descriptions and prior re-reviews below are historical records of their respective source/evidence states. Closed code findings were independently checked in source; transfer/regression executions and GUI interactions remain Builder evidence, not Reviewer reruns. Reviewer independently inspected the supplied images and retained files. R-CACHE-006 and its subsequent closure are recorded at the end.

## R-CACHE-001: Playback does not retain ownership of detached download work

- Locations: `src/app/music_stream.rs:1442`, `1479-1518`, `1521-1527`, and the stop return at `1573-1578`; downloader implementations at `1984` and `2175`. Cleanup entry points: `src/app/state/cache_actions.rs:115-154`.
- Contract: CACHE-10; backend section 8 and AC-11; frontend section 5 requires cleanup to refuse active work without cancelling it.
- Trigger: start playback with an incomplete, sufficiently slow cache download; stop playback or encounter a decoder/open failure before the transfer finishes. After unrelated cover/analysis work releases its own leases, invoke audio/all-cache cleanup in the same application instance while the detached transfer is still running.
- Actual behavior: `run_stream_playback` holds the new storage lease in its own stack. Its two `thread::spawn` closures do not receive that lease or acquire another long-lived lease. Both pass `None` for cancellation, and their downloader functions can continue writing after playback returns. These functions use direct subprocess/HTTP paths, so the lease in `run_tracked_command_output` does not protect them. Individual manifest writes can acquire short leases, but do not protect the media transfer between those writes. Exclusive cleanup can therefore start during active download work.
- Expected behavior: every detached producer retains storage ownership from before its first cache access until subprocess/HTTP activity and final cache bookkeeping end, including retries and parent playback failure/stop.
- User impact: cleanup can remove or partially remove live download files, fail partway through on Windows sharing restrictions, or race a transfer that recreates cache content after cleanup. The UI can report cleanup completion without excluding the continuing producer.
- Cause: a lease scoped to the playback consumer was assumed to cover a separately owned producer with a longer lifetime.
- Evidence classification: static control-flow and lifetime analysis; no playback or transfer was launched by Reviewer. Existing lock tests exercise manually retained guards, not this producer/consumer lifetime split.
- Repair acceptance: Builder must cover both yt-dlp and HTTP branches with a controlled slow transfer in each cache mode. Stop/fail the playback owner while transfer work remains alive; cleanup must remain busy until that work settles. Include the cookie-free retry boundary and verify cleanup cannot delete live files or race subsequent writes. Keep all fixture data below the project root and use synthetic/local sources.

## R-CACHE-002: External `~` output templates evade validation

- Locations: `src/infrastructure/tools.rs:2234-2257`; the accepted config can retain output control at `858-859`.
- Contract: CACHE-11; backend section 7 and AC-13. Environment-dependent or absolute external output templates must be normalized safely or visibly rejected.
- Trigger: use a flat explicit/portable yt-dlp config with `-o '~/cache-review-preview/%(id)s.%(ext)s'` and perform an ordinary download without an explicit app output override.
- Actual behavior: `validate_external_output_config` checks roots, drive syntax, `$`, percent-variable forms, and parent components, but not a leading `~`. This template passes those conditions. yt-dlp expands the home shorthand before joining its output paths, creating an absolute template that takes precedence over `-P temp:...`.
- Expected behavior: detect and reject this bypass before work, or resolve/represent it in a form that actually preserves the cache staging policy.
- User impact: intermediate downloads can be written beside the final media under the user profile rather than under the selected active cache. Switching to system-temporary cache does not enforce the documented policy for this supported config route.
- Cause: validation operates on the unexpanded template without accounting for yt-dlp's home-directory expansion.
- Evidence classification: static Rust validator analysis plus an independent **read-only path-planning simulation**, using the existing project-local yt-dlp `2026.10.21` Python package. No transfer, directory creation, settings save, or network call was executed.

The simulation instantiated `YoutubeDL` with `auto_init=False`, `cachedir=False`, the template above, and a project-local `paths.temp`, then called `prepare_filename` for a synthetic `fixture` MP4:

```text
template=~/cache-review-preview/%(id)s.%(ext)s
planned_final=C:\Users\SKY\cache-review-preview\fixture.mp4
planned_temp=C:\Users\SKY\cache-review-preview\fixture.mp4
```

- Repair acceptance: add regression coverage for `~/...` and the platform-supported home-expansion variants, including typed sidecar templates. Demonstrate rejection before writes or actual contained intermediate placement with controlled yt-dlp execution. Retain working relative output templates and intentional information-cache disabling.

## R-CACHE-003: Direct yt-dlp follows linked managed subdirectories

- Locations: `src/infrastructure/tools.rs:1194-1206`; compare the checked managed-directory helper at `src/infrastructure/storage_paths.rs:358-368` and root-only validation in `use_cache` at `446-453`.
- Contract: CACHE-09/10/11; backend sections 2, 7, and 8; AC-04/11/12. Root and managed-subdirectory validation must prevent writes outside the chosen cache through links/junctions.
- Trigger: keep the active cache root itself valid, but make `C/yt-dlp-temp` a junction to another directory before a normal direct yt-dlp download. The analogous `C/yt-dlp` junction redirects generated information-cache files when the extractor creates them. The junction can already exist at startup; no adversarial timing race is required.
- Actual behavior: storage initialization and `use_cache` validate the root/ownership marker, not those two children. `apply_common_yt_dlp_args` directly joins and passes the child paths to the external process. Unlike gallery/native snapshot preparation, it does not call the checked directory helper or reject links for these paths. The external process follows the junction and writes outside `C`.
- Expected behavior: fail with a path-specific storage error before tool work when a managed path is linked or otherwise unsafe; do not write through it or fall back elsewhere.
- User impact: cache and work placement escapes the selected root and no longer matches usage/cleanup. The implemented cleanup junction protection does not protect this write path.
- Cause: adoption of the immutable root did not also apply the shared managed-subdirectory validation at the direct-tool boundary.
- Evidence classification: static call-path analysis. The existing Windows junction test verifies cleanup/root rejection, not a valid root with a linked direct-tool child. Reviewer did not create a junction or launch a writing tool.
- Repair acceptance: Builder must prepare project-local sentinel directories and Windows junctions for both children in both cache modes, then exercise the production direct analysis/download routes. Expect actionable failure before outside writes; preserve outside sentinels and prove no alternate-root writes. Retain successful normal direct download behavior. Distinguish generated information-cache evidence from a transfer that never creates an information cache.

## R-CACHE-004: Startup-only output separation does not protect later final outputs

- Locations: `src/infrastructure/storage_paths.rs:252-256` and `609-620`; `src/app/state/options_actions.rs:219-230`; output preparation at `src/infrastructure/tools.rs:1934-1947` and gallery destination preparation at `src/infrastructure/gallery_dl.rs:372-389`.
- Contract: CACHE-10/11; backend sections 2 and 8 and AC-10. Completed user downloads and unrecognized/user-owned content must not become disposable cache merely because they fall below a cache-named subtree.
- Trigger: start successfully with the normal output directory outside `C`. In the same session use the existing output selector to choose `C/audio/user-export` (or a corresponding disposable subtree), finish a download, and then invoke audio/all-cache cleanup. A custom portable cache makes this overlap possible without the user choosing a system temporary namespace.
- Actual behavior: the output/cache separation check runs only during the one-time storage initialization, using the startup default output directory. The mutable output setter and per-operation output planners do not enforce it later. Once the download lease ends, `clear_disposable` recursively removes every file in `audio`, including the completed user output, without consulting final-output ownership. The audio-only cleanup has the same exposure. Per-item/native/gallery destinations similarly are not covered by the startup default alone.
- Expected behavior: final-output routes and destructive cache ownership must remain disjoint throughout the process, and cleanup must preserve completed/user-owned files. An output change must not silently create an allowed download that is subsequently treated as disposable work.
- User impact: a successful completed download can be permanently deleted by the application's cleanup action. Saving that overlapping output directory can also make the next startup fail the separation check, despite being accepted in the prior session.
- Cause: dynamic output ownership is checked only at initialization while the deletion allowlist treats whole subtrees as disposable. This is an unclosed requirement in the new cleanup contract; it is not presented as an independently reproduced new baseline regression.
- Evidence classification: static end-to-end flow analysis. Reviewer did not create output files, download media, or invoke deletion.
- Repair acceptance: Builder must exercise output changes after initialization, explicit per-item paths, and desktop/native/gallery output routes with project-local fixtures. Either reject unsafe destinations before any download writes or establish equivalent persistent output protection. Verify final media survives cleanup and restart, while actual disposable cache remains removable. Include unknown/user sentinel files at the applicable ownership boundary rather than testing only an unrelated file at the cache root.

## V-CACHE-001: Current-build GUI acceptance remains unverified

- Locations: `EROffice_Frontend_Architecture.md` sections 3, 5, and 7; backend AC-03/09/16; `docs/implementation/CACHE_LOCATION_BUILDER_STATUS.md` sections 3 and 5.
- Trigger/scope: interactive selector changes, rollback after failed save, active versus pending paths, legacy/storage error notices, cleanup busy/retained feedback, and long/non-ASCII paths/labels in the actual current GUI.
- Available result: Builder explicitly states that `target/debug/yt-dlp-gui.exe` was compiled but not launched for GUI acceptance. The retained source build and scoped i18n checks do not demonstrate visible layout, interactions, disabled states, or actual user-facing error feedback.
- Expected evidence: current-source GUI observations mapped to the cases above, with source/build identity, the controlled application/settings roots, steps, and outcomes. Do not infer package or complete product acceptance from this evidence.
- User impact: the reviewed delivery cannot yet establish the required interactive behavior or layout under the documented acceptance contract.
- Cause: important acceptance work is explicitly outstanding, rather than a discovered assertion failure. No particular visual defect is asserted by this issue.
- Evidence classification: missing acceptance evidence, corroborated by Builder's own scope statement. Reviewer did not launch the GUI because it would save settings/state outside the sole authorized report write.
- Closure requirement: after code fixes, Builder or a user-designated executor must supply controlled current-build GUI evidence. The available entry point is the worktree's `target/debug/yt-dlp-gui.exe`, reported build date `2026.10.10`, baseline `047113c` plus this task's uncommitted source. Debug startup uses the project root and can save configuration, queues, and persistent state; it must not be treated as a read-only launch. Use the confirmed compatible bundle for gallery-specific status checks. No personal account or real download service is necessary for these GUI cases. No installer/package was supplied or requested for human acceptance in this round.

## Round 1 verification performed and evidence limits

### Independently performed without filesystem mutation

- Read the complete tracked diff and inventory/read the additional task source/test/build inputs, with focused reads of unchanged integration callers.
- `git diff --check`: passed for tracked changes. This command alone does not cover added files.
- Recomputed every locked build-input hash, both patch hashes, both modified upstream-source hashes, and the retained upstream ZIP hash: matched the current lock.
- Recomputed the approved bundle's canonical inventory digest and all **2,057 managed-file hashes** at `target/gallery-dl-cache-policy/artifacts/windows-x86_64/gallery-dl/`: matched approved ID `gallery-dl-1.32.18-cache1-b8633ef19b682484`; no unlisted loadable runtime files found. This establishes the inspected artifact identity/integrity, not independent executable behavior or rebuild reproducibility.
- Parsed all added dependency Python scripts and `storage_gallery_fixture.py` with `ast.parse` using Python `-B`: passed; no bytecode or output files were generated.
- Executed only the in-memory yt-dlp filename planning described in R-CACHE-002. It demonstrates tool path semantics, not a completed filesystem transfer.
- Confirmed the local design, handoff, and report paths are ignored by Git. Nothing was staged.

### Builder evidence inspected, not rerun by Reviewer

- `target/cache-location-runtime-tests-final.log`: records **26 passed, 0 failed, 4 ignored**. Some child test entries are environment-gated; separate parent executions supply the relevant tool fixtures. These are Builder results, not independent Reviewer test passes.
- `target/cache-location-runtime-build-final.log`: records a successful debug source build.
- `target/cache-location-runtime-source-final.log`, `target/cache-location-runtime-standalone-final.log`: recorded source and standalone gallery fixture outcomes in both modes.
- `target/cache-location-runtime-source-postprocess-2.log`, `target/cache-location-runtime-standalone-postprocess-final.log`: recorded local FFmpeg cases.
- `target/cache-location-runtime-publication-final.log`: records four publication tests; cross-volume behavior is injected, not a real second-volume transfer.
- `target/cache-location-runtime-transactions-final.log`: records both-mode bundle transaction/recovery cases.
- `target/cache-location-real-tool-final.log`: records four earlier direct yt-dlp placement combinations. Its test binary predates the gallery amendments, as the handoff states; it is not relabeled as an execution of the entire current source.
- The Builder handoff retains the original unmodified gallery failure. Neither that failure nor older investigation failures are superseded into a pass by this review.

### Not performed / not established

- No `cargo test`, `cargo build`, writing test binary, dependency build, fixture generation, app launch, cleanup, installer, install/update transaction, or real-service request was executed by Reviewer.
- No actual independent transfer/cleanup reproduction of R-CACHE-001/003/004 was performed; Builder must produce the requested filesystem evidence after repair. R-CACHE-002 has only the explicitly identified read-only path simulation plus static validation analysis.
- No current-build GUI acceptance, live playback, browser account/cookie integration, arbitrary external-tool configurations, real second-volume publication, installed-package acceptance, or release readiness is established.
- Any future real-site or login validation may call external services and use personal accounts and requires its own clearly identified evidence/scope. None was necessary or used here.

## Revision history

- Round 1, 2026-10-27: initial review; R-CACHE-001 through R-CACHE-004 and V-CACHE-001 opened. Builder should address the stable IDs directly. Subsequent re-review must update this same file and retain prior findings and evidence boundaries.
- Round 2, 2026-10-27: re-reviewed the Builder repairs in this same file. Closed R-CACHE-001/002/003; retained R-CACHE-004 with the residual cases below and V-CACHE-001; opened R-CACHE-005. Overall decision remains changes required.
- Round 3, 2026-10-27: closed the remaining R-CACHE-004, R-CACHE-005 and V-CACHE-001 after source, regression-artifact and final GUI-evidence review. R-CACHE-001/002/003 remain closed. No new actionable finding; review passed within the stated scope and limits.

## Round 2 re-review: baseline and verified evidence

Builder response: `docs/implementation/CACHE_LOCATION_REVIEW_FIXES.md`. HEAD remains `047113c1b550be2cd2347257eef22d5604659eea`; current changes are uncommitted. No test, build, app/fixture launch, transfer, installation or deletion was executed by Reviewer in this round. Only this report was updated.

Independent read-only checks:

- All **81 source/document/input hashes** and **7 log hashes** in `target/cache-review-evidence.json` match the current files. The incoming Round 1 report hash also matched before this authorized Reviewer update.
- The normal debug EXE, test EXE and GUI fixture EXE all match their recorded hashes. This ties the inspected material to the Builder evidence snapshot; it is not an independent build or execution.
- Recomputed all **2,057 gallery bundle file hashes** and the canonical inventory digest: matched the unchanged approved inventory `b8633ef19b6824843111ba1f38170f5bd080a9c10770ae132d05e9ce68159e2b`.
- `git diff --check` passed. Inspected the new repair source/test files and compile-only GUI fixture wiring, along with the relevant unchanged output/worker callers.
- Read retained detached-producer completion JSON in all four mode/backend cases: each records 1,048,576 bytes and busy-until-settled behavior. These records were generated by Builder.
- Performed the read-only Windows identity checks and in-memory yt-dlp output planning described under R-CACHE-004 below. No output file or fixture was created by those checks.

Builder results inspected, without Reviewer reruns:

| Evidence | Recorded result and boundary |
| --- | --- |
| `target/cache-review-tests-final.log` | 29 passed, 0 failed, 6 ignored. Environment-gated child entries are not treated as real-tool passes by themselves. |
| `target/cache-review-build-final.log` | Normal debug source build completed. |
| `target/cache-review-r001-final.log` | Actual HTTP and standalone yt-dlp detached-producer/cookie-free-retry fixtures, both modes, completed after the playback owner returned. |
| `target/cache-review-r003-info-final.log` | Both-mode junction/output boundary checks and actual versioned information-cache generation using the pinned Python yt-dlp engine. This is source-engine evidence, not standalone-extractor acceptance. |
| `target/cache-review-direct-tool-final.log` | Four actual standalone ordinary/absolute download combinations, with final-output cleanup/restart preservation. |
| `target/cache-review-gallery-final.log` | Actual compatible gallery EXE cases in both modes. Builder identifies this run as preceding the final direct-config literal-percent refinement, which did not change gallery/runtime code. |
| `target/cache-review-gui-build-final.log` and fixture `gui-state.json` | Compiled GUI material and initial internal state only. No screenshots or interactive/layout acceptance. |

### R-CACHE-001: closed for the detached playback producers

Current `src/app/music_stream.rs:1482-1484` and `1513-1515` acquire each producer's `StorageUseGuard` before `thread::spawn` and move it into the closure. Its lexical lifetime covers the complete download/fallback and result reporting, independently of the playback owner's stack. This closes the specific stop/failure lifetime defect.

`src/app/music_stream_storage_tests.rs` invokes the production playback function with an already-stopped consumer and waits for actual detached HTTP and real yt-dlp producers. The retained log and four completion JSON records cover both modes, the initial 403/fallback boundary, busy cleanup and final bytes. Closure is based on independent static lifetime analysis plus inspection of this Builder execution evidence. It is not a Reviewer transfer rerun or audio-decoder/device acceptance. The separate streaming-import gap is recorded as new R-CACHE-005 rather than reopening this repaired producer case.

### R-CACHE-002: closed for the reported home-expansion forms

Current `src/infrastructure/tools.rs::validate_external_output_config` rejects a leading `~` after the typed output prefix is removed. The production download path invokes it before output planning. Reviewed tests include `~/...`, Windows `~\\...`, named-user and typed thumbnail/subtitle forms, relative templates, literal-percent directories, and retained `--no-cache-dir`.

The original reported template no longer passes the source guard. The scoped suite and production-preparation fixtures supply Builder evidence; no rejected template was executed by Reviewer. This closes the specific tilde-expansion finding, without declaring every arbitrary external yt-dlp configuration accepted.

### R-CACHE-003: closed for the linked direct-tool children

Current `src/infrastructure/tools.rs:1167-1215` returns `Result`, obtains enabled `yt-dlp` and required `yt-dlp-temp` paths through `storage_paths::cache_directory`, and propagates preparation errors to the direct callers. Disabled information caching does not create/use the information-cache directory. The checked helper rejects the managed child/ancestor links before command preparation succeeds.

Reviewed `storage_review_tests.rs` covers actual Windows junctions in both modes for the two children, production analysis/download preparation rejection, and unchanged outside sentinels. The pinned engine case produces versioned information-cache JSON under the selected cache root. These are Builder executions inspected alongside the source. They close the concrete pre-existing-child junction finding; they do not prove protection against arbitrary hostile filesystem mutation after preparation.

### R-CACHE-004: remains open after partial repair

The new checks correctly reject ordinary lexical paths under the active, portable and known temporary cache roots. Mutable defaults, direct downloads, native configured destinations and gallery final-download preparation now use the boundary. The metadata-only gallery exception remains private and retains its no-media-write flags. However, two remaining paths still permit final outputs to acquire disposable-cache ownership.

**A. Windows aliases bypass the new ownership comparison.**

- Current location: `src/infrastructure/storage_paths.rs:372-403`, especially `379-395`.
- Trigger: a configured or per-operation final destination uses the Windows 8.3 spelling of an ancestor of the cache root, such as the actual measured short path below, followed by `audio/user-export`. Restored/configured paths can preserve this spelling; no link or concurrent path swap is required.
- Actual: the helper rejects links and normalizes lexical `.`/`..`, case, separators and the extended prefix, but compares the remaining strings without resolving existing filesystem ancestors to their actual identity. A short-path spelling is therefore outside the long-path string while referring to the same directory. `reject_links` does not reject an ordinary 8.3 directory alias.
- Read-only evidence: used the existing Builder fixture root `target/cache-location-validation/review-real-info-25464-0/app/cache`. `GetShortPathNameW` returned:

```text
C:\Users\SKY\orca\WORKSP~1\YT-DLP~1\CACHE-~1\target\CACHE-~1\RE5DBD~1\app\cache
same_directory=true
```

The current comparator's lexical rules evaluated in memory return `lexical_overlap=false`. Using this short spelling in an in-memory pinned yt-dlp `YoutubeDL` instance, with `windowsfilenames=True`, `cachedir=False`, `auto_init=False` and `outtmpl=fixture.mp4`, yields a final path whose Windows-resolved identity is:

```text
<worktree>\target\cache-location-validation\review-real-info-25464-0\app\cache\audio\user-export\fixture.mp4
```

This is inside the actual disposable audio subtree. Windows `os.path.samefile` also independently confirmed that the existing `cache.`, `cache ` and `cache` directory spellings have the same identity, while the new comparator misses them. Direct yt-dlp sanitizes some trailing-dot/space components; the 8.3 example above specifically survives that tool's filename planning and is the demonstrated direct-tool bypass.

- Expected: compare actual Windows ownership for existing ancestors, or reject unsupported alias spellings, before permitting a final output. Preserve correct handling when the final child directory does not yet exist.
- Impact: a permitted completed output can still resolve inside `C/audio` and later be deleted by audio/all-cache cleanup.
- Evidence classification: independent read-only directory identity observations, static Rust comparator analysis and in-memory real-tool path planning. The Rust application was not launched and no download/deletion was performed. Acceptance requires Builder to exercise the production validator and actual final-output lifecycle with these aliases.

**B. Cached audio exports also retain the unvalidated filesystem spelling.**

- Current locations: `src/app/state/music_download_actions.rs:76-77`; `src/app/state/music_worker_helpers.rs:4-75`, `492-519`, `522-586`.
- Trigger: the output alias in A is admitted as the configured or mutable output selection, and a complete cached track is saved through `CacheCopy` or `CacheConvert`. For example, an existing `<cache-short-spelling>/audio` directory can remain configured: `config.rs::normalize_stored_path` preserves existing absolute aliases, and the startup output check also compares lexical paths. This does not depend on audio queue snapshots preserving a separate output preference.
- Actual: the worker resolves `item.selection.output_dir`, chooses a cached-export branch and creates/copies/converts to `job.output_dir` without `validate_output_destination`. The added guard in `ToolPaths::prepare_music_download` protects the online branch only. `prepare_queue_item_for_audio_mode` does not validate the destination. Raw `fs::copy` and the FFmpeg output path retain the supplied directory spelling, so they also lack yt-dlp's incidental sanitization of trailing-dot/space path components. A worker cache-use lease prevents concurrent cleanup but does not make its final output safe after that lease is released.
- Expected: all final audio-export branches validate output ownership before output directory creation, copying or FFmpeg execution, regardless of how the job's destination was obtained.
- Impact: the export can report success for a final file inside disposable cache and subsequent cleanup can delete it. This is a reachable cached-export manifestation of A, as well as a missing per-operation defense; it is not counted as a separate independently reproduced issue.
- Cause/evidence classification: incomplete caller coverage, established by static source tracing; no cached export or FFmpeg process was executed by Reviewer. The existing Round 2 tests exercise direct yt-dlp and gallery/native boundaries, not these two cached-export branches.

**Repair acceptance for the remaining R-CACHE-004 scope:** Builder must verify production rejection/protection for Windows 8.3 aliases, relevant trailing-dot/space/extended-path spellings, non-existing descendants of existing aliased parents, and restored/per-item cached audio copy/conversion targets. Cover active and known inactive roots in both modes. Rejected cases must make no final-output writes. Valid completed files and neighboring unknown user files must survive audio/all-cache cleanup and restart. Keep the original four-combination direct-download regression. Do not describe a string comparison or directory-identity observation alone as an actual successful export/cleanup regression.

### R-CACHE-005: streaming batch/flat imports release storage ownership before execution

- Severity/status: Medium / P2, open. Newly found in Round 2 within the task's shared cache-lifetime contract; not asserted to have been newly introduced by the repair.
- Locations: `src/infrastructure/tools.rs:1167-1215`; `src/app/batch_add_worker.rs:42-93`; `src/app/state/queue_batch_helpers.rs:131-180`; `src/app/state/music_batch_actions.rs:271-295`; `src/app/state/queue_input_helpers.rs:242-259`. `src/infrastructure/process_guard.rs::track_child_process` is diagnostic/process tracking only.
- Contract: CACHE-01/10; backend sections 4/8 and AC-11. Information-cache writers and subprocess owners must retain a storage lease throughout actual use; cleanup must return busy while they run.
- Trigger: run a sufficiently slow desktop playlist/batch import or music flat update that uses the direct yt-dlp information cache, with no unrelated active cache leases, and invoke all-cache cleanup in the same instance while that subprocess is still running. A synthetic local extractor can deterministically pause around an actual information-cache read/write for regression evidence.
- Actual behavior: these owners receive a plain `std::process::Command`, spawn it directly and retain no `StorageUseGuard`. The new `_storage_use` in `apply_common_yt_dlp_args` protects preparation only and is dropped when that helper returns. Their `track_child_process` guard does not own a storage lease, and they bypass `run_tracked_command_output`, whose execution-wide lease therefore does not apply. The same-instance cleanup entry points do not test batch/flat worker activity separately.
- Expected behavior: storage ownership extends from preparation into subprocess execution, cancellation/wait and output completion for every direct streaming owner; cleanup cannot start in that interval.
- User impact: cleanup can remove/recreate the information-cache tree while the tool is reading/writing it, race later writes, or report successful clearing while the live import repopulates the cache. Busy behavior is inconsistent with the corrected playback/download owners.
- Cause: the preparation API returns only `Command`; execution owners using direct spawning were omitted from the lifetime integration. This issue is independent of the now-fixed direct-child junction validation in R-CACHE-003.
- Evidence classification: static ownership/call-path analysis. No import process, real-site request or cache deletion was executed by Reviewer. The current tests hold an explicit guard or inspect command preparation, so they do not establish the lifetime of these actual streaming owners.
- Repair acceptance: Builder must exercise the production batch and music flat-import owners with a controlled delayed extractor/process in each mode, retaining actual information-cache behavior where relevant. Check cleanup busy during startup, cache access, cancellation and subprocess/output teardown; check eventual release and safe cleanup. Prove no active-file deletion or writes racing a completed cleanup. Test the execution owner rather than holding an artificial outer guard around the assertion.

### V-CACHE-001: remains open; prepared material is not GUI acceptance

Builder supplied a current-source fixture executable and a compile-only controller, but still explicitly reports no screenshot or interactive/layout verification. The native-pipe error is **Builder-reported**; Reviewer did not retest the connection or launch the writing GUI. Initial `gui-state.json` reports the portable root, `V2Cache` and legacy notice; it cannot establish what was visible or clickable. The final fixture binary was rebuilt after the initial non-visual launch, so that launch is not evidence of executing the final fixture binary either.

Available designated-executor entry point:

- EXE: `<worktree>/target/cache-location-validation/gui-review/app/cache-review-gui.exe`.
- Verified SHA-256: `4d1dcba150cb87069621a1eb9f9c5e087f43307346f0a7197072408fb918cf4e`.
- Source: baseline `047113c` plus the repairs fingerprinted in `target/cache-review-evidence.json`; build date `2026.10.10`; explicit `cache_review_gui` configuration. This is a controlled source-build fixture, not an installer.
- Application/settings root: `<worktree>/target/cache-location-validation/gui-review/app`; use child TEMP/TMP `<worktree>/target/cache-location-validation/gui-review/system-temp`.
- Exact steps/expected outcomes/feedback template: `docs/implementation/CACHE_LOCATION_REVIEW_FIXES.md`, section `V-CACHE-001: available GUI material and concrete blocker`.

Those steps save fixture configuration/state and can delete fixture cache, so they were not run under the Reviewer's read-only validation boundary. They require no personal account or real download service. The controller is gated by explicit compile cfg and absent from normal builds; the fixture skips native registration/automatic updater startup, which remain outside its acceptance scope. Resume the existing current-build GUI acceptance plan after the remaining code repairs. V-CACHE-001 cannot close from compilation, internal state JSON, or hashes alone.

## Round 3 re-review: current closure decision

Date: 2026-10-27. HEAD/branch remain the recorded baseline; the current implementation is uncommitted. Reviewed the current section `Current response to review Round 2` of `docs/implementation/CACHE_LOCATION_REVIEW_FIXES.md`, the full repair delta and its integration paths. The prior report hash matched Builder's preserved incoming-report record before this authorized update.

Only this review report was updated. Reviewer did not run a writing test/build, create fixtures, launch or interact with the application, transfer media, invoke cleanup, install a tool, or use an account/service. Independent verification consisted of source review, read-only integrity/byte checks, Python parsing and visual inspection of existing screenshots. No subagent was used.

### R-CACHE-004: closed

Current implementation evidence:

- `storage_paths.rs:374-429` resolves the deepest existing ancestor with filesystem canonicalization, then appends absent descendants and normalizes lexical parent components. This addresses the demonstrated short-name ancestor bypass even when the intended final directory does not yet exist.
- Windows ambiguous trailing-dot/space components and unsupported namespaces are rejected; drive-relative/root-relative outputs are refused. Normal/extended disk and UNC representations are handled through actual ancestor resolution before ownership comparison. Link/junction rejection remains in the path boundary.
- `validate_output_for_paths` compares the resolved destination against the resolved active/portable/temporary roots. Startup at `storage_paths.rs:247` now uses this same boundary rather than its former lexical-only output check.
- `music_download_actions.rs:79` rejects unsafe per-item destinations before a workflow starts. `music_worker_helpers.rs:497` and `530` independently validate the cached copy/conversion output directories before their filesystem/tool work. This covers both the scheduler route and direct worker jobs called without the scheduler.

Reviewed `cache_review_round2_tests.rs` and `target/cache-review-round2-real-accepted.log`: both modes exercise genuine 8.3 names (the fixture asserts they differ from long names), ordinary/extended spellings, trailing-dot/space rejection, existing destinations and absent descendants for all three root references. The production scheduler and actual production worker branch selection are checked, rather than only the shared helper. Separate initializer phases check aliased startup outputs in portable/temporary mode, including an inactive portable root.

The same retained fixtures execute actual valid CacheCopy and FFmpeg CacheConvert exports, audio/all-cache cleanup, and a separate restart phase. Reviewer independently read the resulting copied WAV (8,898 bytes) and FLAC (9,551 bytes) in each mode: all four match their full expected-byte sidecars; both neighboring `notes.txt` sentinels remain intact. The refreshed standalone direct-tool log also records the ordinary/absolute output combinations in both modes with retained final outputs.

Closure basis: independent source/caller analysis and read-only inspection of the above Builder execution evidence. No Reviewer export, FFmpeg execution or deletion rerun is claimed. Hostile concurrent replacement of filesystem components and every possible namespace spelling are not inferred to pass from these cases.

### R-CACHE-005: closed

The three actual owners now acquire a storage-use guard before preparing commands and retain it in the scope that owns execution:

- `batch_add_worker.rs:55-64`: the lease travels with the prepared command into the complete worker scope, including stop/cancel, child wait, stderr consumption and final events. The stdout-error path terminates/waits and drains before returning.
- `music_batch_actions.rs:285`: the worker operation owns its lease through early first-entry stop, wait and stderr-thread join; read/wait failures settle the process/output before returning.
- `queue_input_helpers.rs:249`: the complete import operation owns its lease, drains stderr concurrently and completes child/output teardown on errors. Process tracking remains separate from storage ownership.

Reviewed the actual-owner fixture and its Python extractor/launcher. It uses the pinned real yt-dlp engine cache API and does not lend an artificial storage guard to the test assertions. `target/cache-review-round2-real-accepted.log` records seven cases per mode: batch completion, batch cancellation, flat update, flat list and inherited-stderr-drain cases for all three owners. The detached pipe helper performs no cache writes. Reviewer independently inspected all **14** retained `assertions.json` records: every case records busy startup/cache access, eventual release and no late cache write; cancellation/drain-specific flags match their cases. The source assertions also check cleanup only succeeds after completion and observe no repopulation for 150 ms afterward.

Closure basis: independent lifetime/control-flow review plus inspection of Builder's actual-owner regression evidence. Error paths that require OS I/O failures remain statically inspected rather than independently fault-injected. The repaired lifetime boundary does not depend on `Command` preparation or diagnostic process tracking alone.

### V-CACHE-001: closed for controlled current-build GUI evidence

The prior missing evidence is now supplied. Reviewer independently viewed all **14 final-case screenshots** cited in the current Builder response and cross-checked their retained state/settings records:

| Requirement | Images independently inspected | Supported observation |
| --- | --- | --- |
| Readable long English/Chinese paths and legacy notice | `50-final-legacy.png`, `51-final-narrow.png` | Chinese path glyphs are readable; labels, long paths/help and the legacy notice wrap at normal and 766-pixel outer width without overlapping the cache controls. |
| Settings save rollback | `53-final-save-failed.png` | Visible save/publish error; portable selection/active root retained; no pending-location row. The recorded settings SHA-256 equals the independently hashed `final-save-fail-before.yaml` bytes. |
| Save success with immutable active root | `55-final-pending.png` | Saved temporary choice and restart row appear while the displayed current folder remains portable. |
| Busy cleanup/current-root usage | `56-final-busy.png` | Visible busy message and 75-byte total/30-byte audio usage, consistent with the retained active audio plus protected staging fixture. |
| Retained pending data/success feedback | `58-final-pending-ready.png`, `59-final-retained.png` | Audio usage changes from 30 bytes to zero; protected 45-byte staging remains; feedback explicitly reports pending update files kept. Recorded before/after staging hashes are equal. |
| Undo pending choice | `61-final-toggle-pending.png`, `63-final-undo.png` | Reselecting portable removes the pending row and reports that the chosen location is already active. |
| Restart/legacy persistence | `40-latest-temp-restart.png` | Temporary root becomes active; pending row and legacy notice are absent. |
| Storage error, disabled cleanup and next-start repair | `41-latest-storage-error.png`, `42-latest-disabled-cleanup.png`, `45-latest-repair-saved.png`, `46-latest-recovered-temp.png` | Error path/feedback and disabled cleanup are visible; the retained state is unchanged after the disabled-action attempt; a temporary next-start repair can be saved; restart removes the error/pending row and activates temporary storage. |

All image names above are below `target/cache-location-validation/gui-review/evidence-round2/`. The capture/launch helpers, window/action JSON, diagnostic state, settings/staging records and current fixture binary were inspected as supplied evidence. A successful action JSON alone was not used to establish a visible result.

Final GUI fixture EXE: `target/cache-location-validation/gui-review/app/cache-review-gui.exe`; independently verified SHA-256 **`3cb9edd630856d4f99c38dd6767ee765cd1b454b0f9f66a5077b9e207b58ec0b`**. Earlier images/binaries and the earlier native-pipe blocker remain historical. This current evidence uses the real NgDlpApp logic/UI under the explicit compile-only fixture configuration, with isolated project-local state. Normal builds omit its entry point/controller/root override.

The associated UI repairs are limited to the existing contract: `app/mod.rs::apply_fonts` observes cache-path/error scripts independently of queue revisions, `options_cache.rs:129` makes operation feedback visible in the cache section, and `prepare_actions.rs` distinguishes undo-to-active from a saved pending change. Source review and the inspected images agree with the intended behavior.

The retained failed fixture-controller attempt `57-final-retained-action.json`/`latest-final-portable.stderr.log` remains a failure: a controller write raced hashing and hit Windows sharing error 32. It is not a production update failure or a passed GUI run. The separately recorded `58`/`59` retry provides the successful preparation/cleanup evidence. This does not erase the earlier failed attempt.

Closure basis: current controlled-binary GUI evidence is now available and independently inspectable, closing the original absence-of-evidence finding. Reviewer reviewed screenshots/files and did **not** personally execute the clicks or application. This does not establish installer, complete accessibility, sound-device, native registration, browser protocol or automatic updater startup acceptance. No human end-user acceptance is claimed.

### Round 3 independent checks and remaining boundaries

- All **90 source/document/input hashes**, **5 current log hashes**, **119 retained real-fixture file hashes** and **259 GUI-evidence file hashes** in `target/cache-review-round2-evidence.json` match current files. Some evidence includes failed/historical investigation material; matching its hash is an integrity result, not a pass result.
- The current normal debug EXE, test EXE and final GUI fixture EXE match their recorded hashes. Recomputed all **2,057** gallery bundle managed-file hashes and its canonical inventory: unchanged and matching the approved `b8633ef19b6824843111ba1f38170f5bd080a9c10770ae132d05e9ce68159e2b` identity.
- `git diff --check` passed; the added Python streaming fixture parsed successfully with `ast.parse` and Python `-B`. No generated bytecode or test artifact was created by Reviewer.
- Builder's current suite records **30 passed, 0 failed, 7 ignored** in `target/cache-review-round2-tests-accepted.log`; the ignored actual-owner/direct-tool parents have the separate inspected executions. Normal debug build and explicit fixture build logs completed; five unused-import warnings belong to the alternate fixture entry point. These are Builder results, not Reviewer build/test reruns. The log suffix `accepted` is a Builder artifact label and did not confer review approval by itself.
- Previous gallery/FFmpeg, cookie, migration and detached-playback executions remain historical evidence with their original scope. They were not rerun or relabeled as executions of every current source change. Their relevant unchanged code/input identity and the current suite/targeted repairs were checked.
- No new dependency/runtime/architecture change, package, installation, version, commit, push, PR, merge, release or real-user tool replacement was performed by Reviewer. Files remain in the same task worktree; only this ignored local review report changed.

At the end of Round 3, all six then-recorded review entries were closed. Live external services/accounts, full browser/native integration, arbitrary external configurations, hostile concurrent filesystem mutation, full audio-device behavior, real second-volume publication and installed-package acceptance remained outside that review. The later combined re-review below supersedes the current-pass implication.

## Combined re-review: R-CACHE-006

- **Severity/status:** High / P1, Open. This is a newly discovered defect in the combined #32/#33 source; it was present at the earlier cache-review baseline, not introduced by the Round 1 #33 repair. The prior six IDs and their closure evidence remain historical.
- **Locations:** `src/app/state/music_cache_playlist_actions.rs:13-33` scans `C/audio` and calls `fs::remove_dir_all` for a stale manifest. `src/app/state/music_runtime.rs:529-545` gives the deserialized manifest default values, including `updated_unix_seconds = 0`. `src/app/state/cache_actions.rs:179-183` returns the raw active root; `music_cache_playlist_actions.rs:158-160` appends `audio`. In contrast, the checked directory and lock boundaries are `src/infrastructure/storage_paths.rs:355-363`, `547-574`, and `728-742`.
- **Trigger:** Audio Mode queue preparation, prefetch, playback lookup or music download invokes `complete_music_cache_hit_for_item`; it iterates all immediate audio-cache children before checking their source identity. An `audio` parent that is a junction/reparse link to an external directory can expose an ordinary external child containing a parseable stale `manifest.yaml`. A mapping such as `{}` deserializes with the default timestamp 0 and is treated as expired.
- **Actual behavior:** The lookup follows the parent when calling `read_dir`/`is_dir`, reads the manifest and recursively deletes that child at line 32. It performs no `require_ready`, link/reparse validation, source/ownership check or exclusive cleanup lock before deletion, and ignores any deletion error. Because the junction is a parent of the deleted child rather than the deletion target itself, resolving the path can reach and remove that external child. No user-selected cleanup action is needed. The same loop can delete a stale entry belonging to another source while playback or another producer holds a shared cache-use lease; the lookup does not consult that lease.
- **Expected behavior:** Cache lookup must not delete unknown or externally linked content. Expiry cleanup must use the active-root/ownership/link boundary and coordinate with active users using the documented exclusive cleanup mechanism, while preserving correct TTL semantics and reporting failures appropriately. A read/lookup can instead leave stale entries for an authorized cleanup path.
- **User impact:** A normal Audio Mode action can remove files outside the application cache in the link scenario, or disrupt an active cache consumer in the race scenario. The impact is potential data loss, not just an incorrect cache-usage count.
- **Cause:** Legacy opportunistic expiry deletion remains inside the cache-hit search and bypasses the #32 storage service. The explicit cleanup UI's guard does not protect this independent call path.
- **Evidence classification:** Static source/caller analysis only. Reviewer did not create a junction, run the application, perform deletion, or verify behavior on any user files. The current #33 repair manifest's 119 source/document/input hashes, including this unchanged file and this report before its update, matched on read-only inspection.
- **Repair acceptance:** Builder should exercise the actual lookup entry point in isolated project-local fixtures for both cache modes: a linked `audio` parent and/or linked child pointing to a separate sentinel tree; a parseable incomplete/stale manifest next to unknown files; active playback/producer ownership while an expired candidate is encountered; and ordinary stale owned entries. Assert all external/unknown and live-consumer sentinel hashes remain unchanged, no traversal or deletion occurs without exclusive ownership, and expiry only occurs through a correctly validated guarded path. Include actual errors/retention behavior and report whether a junction test was available; preserve R-CACHE-001 through 005 regressions.

### Subsequent re-review: R-CACHE-006 closed

- `music_cache_playlist_actions.rs:13-84` acquires a shared cache-use guard, obtains the checked active `audio` directory, rejects linked children/manifests/media, and skips stale entries without deleting them. `music_metadata_helpers.rs:395-426` likewise rejects links, retains stale entries and checks the cached source identity.
- Explicit expiry cleanup uses the checked active audio root; its UI caller obtains exclusive cleanup ownership. `music_cache_dir_is_expired` requires a nonempty source/extension, a nonzero old timestamp and the expected generated key, and rejects linked directories/manifests. Incomplete default mappings and unknown neighbors are not selected for expiry.
- Reviewer inspected `src/app/state/output_review_round2_tests.rs` and Builder's `target/output-round2-actual-final.log`. Both modes use real Windows parent/child junctions within project-local fixtures, retain the external sentinel and `{}` unknown neighbor, hold a shared cache guard while lookup encounters a stale entry, and then remove an ordinary stale owned entry under exclusive cleanup. Holding the guard is controlled ownership evidence, not an independently performed playback run. Reviewer independently read the retained external sentinel bytes in both modes and checked the manifest's 121 source/input, 990 artifact and 3 executable hashes before changing either review report.
- Builder's prior cache-owner and gallery regressions are recorded in `target/output-round2-cache-owners.log` and `target/output-round2-gallery.log`; these precede only the final additional external-option rejection list. Reviewer did not rerun tests, create links, execute deletion or run the app. Arbitrary hostile filesystem mutation and installed-package behavior remain outside the tested scope.
- Closure is limited to R-CACHE-006 and the supplied controlled boundary. The #33 output issues recorded in the companion review remain open, so there is no combined review pass.

### Combined re-review after output Round 6

At that round, Reviewer re-read the #33 repair and its evidence inventory `target/output-round5-evidence.json`, SHA-256 `3c91a869f048fcc6aad0f3b38d5b7ea3d9f712db22af1e299206a1cabac1a142`. R-OUT-011 was closed for the reported Windows parent-exited-first producer/pipe boundary, and the final-source GUI evidence was rechecked. The then-recorded findings were closed for their reported scopes. The subsequent simplicity review below supersedes the earlier overall pass. No Reviewer test, build, app launch, cleanup, staging, commit, push or release was performed.

## R-CACHE-007: Full gallery runtime verification runs in the UI redraw path

- **Severity/status:** High / P1, Open. This is a current source regression discovered by static analysis; it is separate from the previously closed gallery/cache cases.
- **Locations:** `src/app/mod.rs:454-455`, `src/app/ui/main_tab_template.rs:18-25,200-209`, `src/app/ui/main_tab_url_action.rs:79-108`, `src/infrastructure/gallery_dl.rs:44-51,62-69,103-105`, and `src/infrastructure/gallery_runtime_policy.rs:135-191,208-245,293-335`. About-tab callers include `src/app/ui/about_tab.rs:162-194,290-316` and `src/app/state/prepare_actions.rs:669-670`. `src/infrastructure/prepare_check.rs:177-188` also discards an availability error and calls readiness a second time to recover it.
- **Trigger:** A compatible gallery-dl bundle is installed and the main tab is drawn in Standard or Origin mode, audio display and direct-download-on-add are off, and the start control is not showing a running spinner. Each qualifying UI repaint invokes `is_available()` synchronously. The About component view calls the same availability path from multiple rendering helpers.
- **Actual behavior:** The formerly cheap existence check now acquires a runtime guard, validates and hashes every listed runtime file, scans the tree, starts the capability subprocess and waits for it, all before the UI call returns. In the retained approved local artifact, Reviewer counted 2,057 manifest entries totaling 57,455,738 bytes; `sha256_hex_file` reads each complete file. Thus each successful qualifying check performs that work again; the capability process has a ten-second timeout. The actual elapsed time, redraw count and responsiveness were not measured in this review.
- **Expected behavior / impact:** UI rendering should consume an already obtained readiness result and remain responsive, while real gallery operations still perform the approved compatibility/ownership checks under the operation lease. A displayed status can be refreshed off the rendering path when its configured path or managed installation changes. Repeated complete verification and process creation on the UI thread can visibly stall normal interaction and wastes I/O/CPU; repeatedly rebuilding or probing the same status also obscures the actual boundary. ADR-0002 requires verified readiness, and permits reuse only under its stated identity/lease conditions; it does not require hashing the bundle for each paint. Do not silently weaken verification before actual use.
- **Cause:** `GalleryDlRuntime::availability_error` and `dependency_tool_is_available` were changed from cheap checks to full `acquire`/`readiness`, while existing immediate-mode render callers were left unchanged. The error-to-bool conversion in `prepare_check` causes another readiness call for its detail.
- **Evidence classification:** Static source/caller analysis and read-only counting/summing of the retained approved bundle manifest. No app, capability process, test or timing experiment was run by Reviewer. The earlier GUI evidence does not measure this compatible-bundle repaint path.
- **Repair acceptance:** Under a separately authorized Builder scope, show the real main and About views using the approved project-local compatible bundle: repeated repaints must not cause repeated full inventory hashes or capability processes on the UI thread. Record actual check counts and responsiveness/timing rather than asserting a guessed threshold. Verify status refresh after configured-path and managed-install changes, missing/incompatible/busy states, and that actual gallery operations still acquire the required lease and validate the approved runtime. Preserve existing functional/cache regressions. Use controlled local material only; no personal account or live service is needed.

## Simplicity re-review: 2026-10-28

### R-CACHE-007: partially repaired; remains open

- **Severity/status:** Medium / P2, Open. The original P1 finding concerned full inventory hashing and capability execution synchronously in qualifying redraws. Those operations have moved to a background worker, so the remaining issue is the conditional completion/wakeup defect below. The original P1 description is retained as historical evidence.
- **Locations:** `src/app/state/gallery_readiness_actions.rs:13-50`, `src/app/state/background_poll_actions.rs:4-5`, `src/app/mod.rs:424-451`, `src/app/state/queue_item_actions.rs:95-106`, `src/infrastructure/config.rs:938`, and the controlled fixture at `src/infrastructure/cache_review_gui.rs:221-252`. Contract: `docs/backend-reference.md:158-164`.
- **Trigger:** The startup, configured-path or managed-maintenance refresh starts a background gallery readiness check. The check completes after the production window becomes idle: no active work/music, loading thumbnails, clipboard monitor or other pending redraw/input. Clipboard monitoring is disabled by default.
- **Actual behavior:** The worker sends its result through an `mpsc` channel but does not request a repaint or otherwise wake the UI. The receiver is consumed only when `NgDlpApp::logic` calls `poll_background_work`. Gallery pending is absent from `has_active_work`, and the normal logic schedules no repaint under the stated idle conditions. The local eframe event loop can wait for another event once no repaint is requested. The completed status can therefore remain at Checking or the previous visible state until unrelated interaction or another repaint.
- **Expected behavior / user impact:** A completed background check should cause the existing UI to consume and display the result without unrelated user input. Otherwise the user can see a stale checking/missing state despite completed verification, including after starting the app or changing/installing a runtime.
- **Cause:** Moving verification to a worker added a channel but did not connect worker completion to the UI wakeup mechanism. The actual operation's independent runtime lease/verification remains necessary and is not implicated by this display-only observation.
- **Evidence classification:** Static source/caller analysis; Reviewer did not run the application or measure an idle production session. Builder's controlled GUI fixture explicitly calls `ctx.request_repaint_after(Duration::from_millis(100))` unconditionally, unlike production. The retained `31-checking.png` and `31-ready.png` show checking/ready in that fixture, but its periodic redraw can consume a result that would not wake a sleeping production window. The fixture's 40-frame main/About measurements and zero additional checks are useful controlled evidence that rendering no longer repeats full verification; they do not establish idle completion wakeup. Reviewer independently inspected those retained artifacts, not the GUI actions.
- **Repair acceptance:** Connect a completed check to the existing UI wakeup/repaint mechanism with a focused notification; do not add a generic scheduler, periodic full probe or repaint-time hashing. With current production scheduling and no unconditional fixture repaint, verify startup, configured-path change and managed-install completion automatically leave pending for the actual ready/error result while idle, including clipboard monitoring disabled. Record check/capability counts for ordinary repaints and retain actual gallery-operation lease/validation, missing/incompatible/busy and stale-completion checks. Use controlled project-local runtime data; no account or live service is necessary.

### R-CACHE-008: pending readiness is treated as missing by install controls

- **Severity/status:** Medium / P2, Open; newly discovered in this re-review.
- **Locations:** `src/app/state/gallery_readiness_actions.rs:76-78`, `src/app/state/prepare_actions.rs:433-450,567-574,681-685`, `src/app/ui/about_tab.rs:235-240,419-438`, and `src/app/ui/main_tab_url_action.rs:101-106`. Contract: `docs/backend-reference.md:158-164`.
- **Trigger:** About displays gallery-dl while its readiness check is pending and no component update is currently running. The configured runtime may already be valid and may be an external path.
- **Actual behavior:** `gallery_is_ready_for_display` returns false both for pending and for a non-ready result. `dependency_tool_is_installed` forwards that bool; the About action cell interprets false as missing and displays an Install button enabled whenever no component update is running. If clicked, `update_component` also skips its installed-external-tool guard because the same bool is false, and can schedule managed installation, or report Busy if the runtime lease prevents maintenance. This requires a user click; it does not mean installation happens automatically or that an external runtime is overwritten. The main URL action can likewise show the missing yt-dlp guidance while a potentially usable gallery runtime is still checking.
- **Expected behavior / user impact:** Pending must remain distinguishable from an established missing/incompatible result for actionable controls. Until the check resolves, installation should be unavailable or deferred rather than presented as the remedy for a known absence. Otherwise the user can be guided into an unnecessary managed install or a confusing Busy result while a valid configured runtime is being checked.
- **Cause:** A display readiness boolean is reused as an installed/missing decision for UI and maintenance policy, discarding the existing pending distinction.
- **Evidence classification:** Static source/caller analysis plus inspection of Builder's retained `target/cache-location-validation/gui-simplicity/evidence/31-checking.png`, which visibly shows Checking and Install together. Source shows the button is enabled when the update-running flag is false; Reviewer did not click it or run the installer. The screenshot by itself does not establish the outcome of clicking or any external-file modification.
- **Repair acceptance:** In actual Main/About flows, exercise pending compatible managed and configured-external runtimes with no update running: install must be disabled or deferred until the check resolves; Ready must not offer a spurious install. Verify an actually missing runtime still exposes the intended install flow, incompatible/busy results keep their correct meaning, and external configured runtimes are not replaced. Cover the main action's pending guidance and normal post-check behavior. Reuse the existing readiness state rather than introducing a generic status framework.

### Re-review evidence and boundary

Before these report updates, Reviewer checked `target/output-simplicity-evidence.json`, SHA-256 `9e94013d0ba3bc78c3cc43da62b8d262478f44ffa8a61228e49d445a7f10eabb`: 131 current source/document/input entries, 127 pre-implementation snapshots, seven prior manifests, both previous review reports, four executables, 317 GUI build inputs and 9,193 retained artifacts all matched. After the report updates, the other 129 source/input entries still match; the two report hashes intentionally differ from the Builder's earlier inventory. All 120 Git-visible task paths are included in the source inventory. `git diff --check` passed. The Builder's final logs and controlled screenshots are evidence of Builder executions, not Reviewer reruns. Reviewer used read-only source/caller analysis, hashing, retained artifact/image inspection and the in-memory config model described in the output report; no test suite, app, real tool, installation, build, cleanup or network/account workflow was executed. Only the two review reports were changed. The combined #32/#33 review remains **not passed** because R-CACHE-007, R-CACHE-008 and R-OUT-012 remain open. Installed-package, real-user, arbitrary external-service and release acceptance remain outside this review.

## Follow-up re-review: 2026-10-28

### R-CACHE-007: closed for the reported controlled Windows scope

`src/app/state/gallery_readiness_actions.rs:13-55` retains a shared repaint binding across replacement requests. After successful delivery, the worker requests an egui repaint; `src/app/mod.rs:163-164` binds the actual application context after state construction. Normal logic consumes the result. A startup result already delivered before binding is available to the initial frame; a superseded receiver cannot replace the current result. The actual gallery operation still independently validates under its lease.

The previous fixture's unconditional 100-ms repaint is removed. Reviewer inspected the retained `32-private-startup`, `48-external`, `57-managed`, `62-main`, `67-missing`, `70-incompatible`, `73-busy`, `76-release` and `106-feedback` idle records under `target/cache-location-validation/gui-followup/evidence/`: the fixture records a quiet interval with clipboard monitoring off, then worker completion/wake request and UI consumption without intervening input. Reviewer also inspected the fixture's test-only bounded worker barrier and absence of its former periodic repaint. The latest `106`/`107` case uses the final feedback-clearing source; earlier successful GUI cases used the preceding source generation as documented in Builder's handoff. The specific implementation of worker wakeup was unchanged by that final feedback correction.

Builder's controlled 12-paint Main/About windows report no new gallery verification and runtime checks off the UI thread. This supports the reported repair without claiming an arbitrary latency threshold or physical user interaction. Reviewer inspected retained records/screenshots; Builder operated the private-desktop fixture and the Reviewer did not launch it. This closes R-CACHE-007 for its reported idle-wakeup and repeated-check cases, not installed-package or unrestricted real-user acceptance.

### R-CACHE-008: closed for the reported controlled Windows scope

`src/app/ui/about_tab.rs:425-440` disables the gallery Install control while pending; `src/app/state/prepare_actions.rs:404-413,446-450` defers pending gallery maintenance, and Main supplies disabled checking guidance when gallery readiness is needed and yt-dlp is unavailable. Result consumption clears only the corresponding transient checking feedback. Reviewer examined the external/managed pending state records and the screenshots `45-external-pending.png`, `51-managed-missing.png`, `61-main-checking.png` and `107-feedback-ready.png`. The retained `47`/`56` maintenance-entry records show pending, no update running and Checking feedback; `107` shows Ready and cleared transient feedback. The Builder's directed missing-install action and completed post-install readiness are retained in `52`/`53`/`57`/`58`; configured external path remains unchanged in the reported pending cases. These are controlled GUI actions and evidence inspection, not Reviewer clicks or proof of installed-package behavior.

This closes the original pending-as-missing action/guidance finding for its reported cases. The normal real gallery operation still performs its own compatibility/lease check: the Builder's controlled operation-guard receipt records rejection after fixture-manifest invalidation despite stored Ready, followed by manifest restoration.

### Follow-up evidence and combined boundary

Before updating either report, Reviewer checked `target/output-followup-evidence.json`, SHA-256 `2b3558d8401c2536c38d4101f93e9991051452af2f123d2a834f60f7db55870a`: all 138 source/document/input entries, 328 pre-follow-up snapshots, eight previous manifests, both incoming review reports, 9,860 retained artifacts, six executables and 319 GUI build-input entries matched. All 127 Git-visible changed/untracked paths are included in that inventory. The two report hashes intentionally change after this update. `git diff --check` passed with line-ending warnings. Builder's 41-pass/eight-ignored unit result, actual-tool logs and operated controlled GUI are retained Builder evidence; no test suite, build, GUI, tool, installer, cleanup or network/account workflow was executed by Reviewer. Two in-memory standard-library config models and the associated static analyses are described in the output report. Reviewer changed only the two review reports.

The combined review is **not passed**: R-OUT-012 remains open for effective-config acceptance, with the newly identified R-OUT-016 through R-OUT-019 documented in `REVIEW_ISSUE_33_OUTPUT_ISSUES.md`. Installed-package, real-user, physical second-volume, arbitrary external-service and release acceptance remain outside the reviewed evidence.

## Protected-configuration re-review: 2026-10-29

R-CACHE-007 and R-CACHE-008 remain closed for their previously reported controlled Windows scopes; the new phase does not rerun the prior gallery idle-wakeup acceptance. Reviewer inspected Builder's current protected-config source, retained local-tool receipts and private-desktop screenshots. R-OUT-016 through R-OUT-019 are closed for their reported protected-config cases; their individual closure evidence and limitations are in `REVIEW_ISSUE_33_OUTPUT_ISSUES.md`. R-OUT-012 remains open because one playlist or music-stream metadata operation can still repeatedly bind/read/parse/validate its selected external config; the complete trigger and acceptance are in that output report. The combined #32/#33 review is therefore **not passed**.

Before updating either report, Reviewer independently checked `target/output-guard-evidence.json`, SHA-256 `dfed0508bbb100637975681f08e317177025b857794b5b25c8ab8e5c1e312efa`: 139 source/document/input entries, 330 pre-implementation snapshots, nine previous manifests, both incoming review reports, 3,641 artifacts, six recorded executables and three actual tool binaries matched. Of 320 GUI build inputs, 319 match current files and the remaining earlier test-only source matches its archived `target/guard-test2-inputs` bytes; the recorded final test-only change adds a consumed `--` value. All 128 Git-visible task paths appear in the source inventory. The two report hashes intentionally change after this update. Builder's 43-pass/eight-ignored unit run, focused local-tool cases and controlled GUI actions are Builder evidence; Reviewer did not rerun a test suite, build, GUI, tool, installer, cleanup or network/account workflow. `git diff --check` passed for the tracked diff with line-ending warnings. Reviewer changed only the two review reports; installed-package, physical second-volume and release/end-user acceptance remain outside scope.

## Complete metadata-operation re-review: 2026-10-29

R-CACHE-007/008 retain their existing controlled Windows closures; this backend follow-up did not rerun their GUI acceptance. R-OUT-012 is closed for the reported repeated application-level config observations after review of the operation-scoped propagation and Builder's full playlist/music receipts. The same repair introduces R-OUT-020, an open processing-log completion regression when music configuration is rejected before launch. The combined review remains **not passed**. The complete trigger, acceptance criteria and controlled-evidence boundaries are in `REVIEW_ISSUE_33_OUTPUT_ISSUES.md`.

Before these report updates, Reviewer checked `target/output-metadata-evidence.json` (SHA-256 `d87a2a167a5667baf0673f0a44ced8102187bf7f258936bc8a7e6da85dcb1af1`): 141 source/input entries, 331 pre-implementation snapshots, ten prior manifests, two incoming reports, 3,641 prior artifacts, 1,583 current artifacts, eight recorded executables and three tool binaries matched; all 130 Git-visible changed/untracked paths were included. The final verified test inputs match current code and tests; two subsequent differences are Builder handoff prose. The two report hashes intentionally change after this update. `git diff --check` passed for tracked files with line-ending warnings. Builder's test/tool logs are retained Builder evidence; Reviewer performed read-only source/evidence checks and no tests, build, GUI, tool, installation, cleanup or account/network workflow. Installed-package, real-user, physical second-volume, native Unix, unrestricted external-service and release acceptance remain outside the reviewed evidence.

## Preparation-rejection log re-review: 2026-10-29

R-CACHE-007/008 retain their established controlled Windows closures; this backend follow-up did not rerun their GUI acceptance. R-OUT-020 is closed after reviewing the worker/event/log repair and Builder's 24 actual-entry controlled rejection cases. R-OUT-012 retains its closure, with Builder's complete metadata operation parent rerun in both modes. No issue remains open in the recorded combined scope, so the combined #32/#33 source and supplied controlled Windows evidence **pass this review**. The output report records the individual evidence and limits.

Before these report updates, Reviewer independently checked `target/output-resolve-log-evidence.json` (SHA-256 `a7d30e76d31ec9b20e61d736fa2c2a73aab213654afbee903d22666a4a75c259`): 142 source/input entries, 332 pre-implementation snapshots, eleven prior manifests, both incoming reports, 5,224 prior artifacts, 646 current artifacts, four recorded executables and three tool binaries matched; all 131 Git-visible changed/untracked paths were included. The 333 recorded test inputs match current code and tests; only two subsequent differences are Builder handoff prose. The two report hashes intentionally change after this update. `git diff --check` passed for tracked files with line-ending warnings. Builder's logs are Builder evidence; Reviewer performed read-only source/evidence checks and no tests, build, GUI, tool, installation, cleanup or account/network workflow. Prior GUI evidence retains its historical source boundary; installed-package, audio-device, real-user, physical second-volume, native Unix, unrestricted external-service and release acceptance remain unverified.
