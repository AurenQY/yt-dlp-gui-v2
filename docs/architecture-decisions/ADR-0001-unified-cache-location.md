# ADR-0001: One Application Cache Policy with Restart Activation

- Status: Accepted and implemented with scoped Builder validation; see the [handoff](../implementation/CACHE_LOCATION_BUILDER_STATUS.md). Independent review and complete product acceptance remain outstanding.
- Date: 2026-10-26.
- Source baseline: `047113c` (`2026.10.10`).
- Issue: [Cache location (bug, maybe)? #32](https://github.com/kannagi0303/yt-dlp-gui-v2/issues/32).
- Authority: The user selected route 2, confirmed the original documents and the native-cookie/gallery-dl supplement, authorized implementation, and requested English as the default development language. After the real delegated-playlist check failed, the user confirmed ADR-0002 and the corresponding architecture amendment and authorized scoped implementation/local validation. Builder evidence is not independent review or complete acceptance.

## Context

The Options cache selector currently changes yt-dlp command arguments while GUI caches, cache statistics, and cleanup continue to use a separate directory. Update/install work and startup cleanup also construct portable cache paths independently. As a result, choosing Windows storage does not relocate all application caches.

The directory named `cache` also contains installed-version records, a pending update manifest, recovery backups, process records, and music cue memory. These have different ownership and lifetime from reproducible audio and thumbnail files. Moving every existing cache child to an OS-cleaned directory would risk losing control/recovery state.

Background workers clone tool configuration and capture paths. Immediate location changes would require coordinating active downloads, prefetch, playback, thumbnails, installer/update work, and later completions across two directory layouts.

## Decision

1. Introduce one shared storage path service and freeze its active policy for the lifetime of a process. Every covered cache consumer, usage report, and cleanup operation uses it.
2. Offer two application-level choices: **Application folder (portable)**, the default, and **System temporary folder**. Preserve `v2_cache`/`windows_temp` serialization. Read legacy `yt_dlp_default` as portable and give a migration notice.
3. Resolve portable storage from the existing configured directory. Scope temporary storage to `<system-temp>/yt-dlp-gui/<path-derived-installation-id>/cache`. Do not share the system temporary root between this app and unrelated programs, or one namespace between different application folders.
4. Save new selections immediately but activate them only at next start. Display active and pending folders distinctly. Save failure retains the old saved preference and active policy.
5. Retain old reproducible caches without automatic transfer or removal. Statistics and cleanup cover only active storage; selecting an old location again can reuse its valid entries.
6. Keep persistent state, update records, recovery backups, and music memory below `<application-root>/state`. Import listed legacy persistent sources safely and repeatably, retaining the sources. Protect incomplete migrations and outstanding update transactions from cleanup.
7. Treat the selected cache policy as authoritative for yt-dlp cache/temp placement. Preserve final user outputs, including absolute app requests, by adapting supported output planning; reject unsupported external-output cases clearly rather than silently violating the policy.
8. Keep English as the development authoring language. Preserve the existing UI-locale preference and i18n boundary.
9. Include temporary native-browser Cookie snapshots at `C/temp/native-cookies`, delegated yt-dlp information at `C/yt-dlp`, and gallery-dl per-operation intermediates at `C/gallery-dl-temp/<job-id>`. Keep existing persistent managed credentials at `A/data/cookies`. Use shared storage leases for the full snapshot/job/output-processing lifetime; manual cleanup and 24-hour transient expiry act only after exclusive ownership is obtained.

### Supplement: previously omitted writers

The post-implementation audit found that `NativeCookies::from_request` still creates a temporary snapshot directly under the system temporary root and that gallery-dl's FFmpeg-only helper does not establish delegated cache/work paths. These are concrete application-owned writers, so excluding them would contradict route 2.

Extend the existing storage boundary rather than add another setting. Use exclusive unique snapshot creation, release cleanup, and crash-leftover expiry. For gallery-dl use structured yt-dlp cache options and downloader part-directory controls, with a unique job directory and the same immutable root. Preserve final output, XMP/archive semantics, and metadata-only analysis behavior. Verify single-video and playlist paths against actual supported tools; if those controls cannot preserve the contract, return to design discussion before adopting a different implementation. The precise lifecycle and validation requirements are in backend sections 7.1 and 7.2.

Do not sweep old snapshots from the shared temporary root, relocate persistent credentials, or remove legacy gallery work outside owned active storage. Diagnostic logs retain their existing logging policy; they are not newly classified as disposable cache by this supplement.

The real delegated-playlist test subsequently failed despite `part-directory`. [ADR-0002](ADR-0002-gallery-dl-playlist-runtime.md) proposes the bounded source patch, compatible standalone dependency, per-artifact finalization/reporting, and installer/update protection needed to close that failure. It is the only confirmed exception to this decision's original no-new-dependency boundary. It does not replace the startup snapshot, cache ownership, or persistent-state policy.

The complete path, initialization, persistence, migration, failure, concurrency, and validation contracts are in the [backend reference](../backend-reference.md). Product behavior and UI presentation are in the [requirements](../../EROffice_Architecture_Spec.md) and [frontend architecture](../../EROffice_Frontend_Architecture.md).

## Alternatives considered

### Clarify that the selector affects only yt-dlp

This would be a smaller UI-focused change, but it would not give users control over audio, artwork, thumbnails, and application work files. The user selected the application-wide route.

### Keep all three existing labels as application-wide locations

`Default` currently delegates to yt-dlp configuration/environment and does not define a stable application-owned namespace. Giving it a new system-wide meaning would introduce another mode and change its semantics anyway. The approved design uses two explicit locations with read compatibility for the old token.

### Apply new locations immediately

This requires live cancellation, worker generations, cache ownership rebinding, and cross-location completion handling. Restart activation provides one unambiguous active root and allows existing work to continue normally.

### Automatically move or copy old caches

Audio caches can be large, files may be open, and temporary storage may be on another volume. Automatic transfer would add failure/recovery and progress workflows unrelated to selecting a location. Old caches remain available without automatic transfer.

### Relocate every existing `cache` child to temporary storage

This would include persistent update/recovery and music state. State is separated and imported before generic cleanup can touch legacy sources; only reproducible content follows the cache setting.

## Consequences

- Portable remains the default, with application files concentrated under the application root. Selecting temporary mode explicitly stores disposable caches in an owned system namespace; persistent state remains local to the application.
- Location changes require a restart and old cache copies may continue to occupy space. The UI explains this; no automatic inactive-root cleanup is introduced.
- Temporary caches may be removed by the OS. Later work recreates the same root, while missing staged update files cause an explicit recoverable failure rather than state loss or executable replacement.
- Existing custom portable paths and configuration tokens remain readable. The old yt-dlp-specific delegation behavior is intentionally replaced by application-wide ownership and a migration notice.
- Storage initialization, migration, updater entry points, output planning, and cleanup must be updated together. Changing only `app_cache_root_path()` is insufficient.
- The compatible gallery-dl runtime and locked local build inputs in ADR-0002 are the sole confirmed dependency exception. No new Rust runtime dependency, app package/release action, unrelated refactor, or application-wide language-preference change is part of this decision.

## Acceptance and approval gate

Backend cases `AC-01` through `AC-25` are required evidence categories. Existing preliminary results are distinct from complete acceptance. AC-18/AC-20 have partial fixture evidence; AC-19 retains a real delegated-playlist failure. Cases AC-21 through AC-25 require separate recorded evidence and are not accepted merely because preliminary builds/fixtures pass. Command construction alone cannot prove actual output placement, and updater fixtures do not authorize a real self-update or release.

The earlier document confirmations permitted the existing implementation work. The user has confirmed ADR-0002 and the playlist-runtime amendment and authorized Builder continuation under the workspace's `AGENTS.md`. The authorization covers scoped implementation, local dependency builds, and controlled validation; actual user-tool replacement and release operations remain separate. Implementation delivery must map its changes and evidence back to these document sections and requirements.
