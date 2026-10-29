# ADR-0003: Output naming and non-replacing final publication

Date: 2026-10-27.

Status: the user confirmed the Issue #33 proposal and detailed architecture. Implementation and Builder validation are recorded in the [handoff](../implementation/ISSUE_33_BUILDER_HANDOFF.md), with evidence limitations stated explicitly. Independent combined review is pending; no #33 review pass is claimed. Issue #32 passed source/controlled-evidence review separately. Agent conversation remains Chinese; development/canonical copy remains English.

## 1. Context

Issue [#33](https://github.com/kannagi0303/yt-dlp-gui-v2/issues/33) reports silent replacement of different videos with the same title and requests filename options. Source inspection finds forced-overwrite flags in direct/online-audio commands, title-derived output paths, overwrite-first subtitle/transcode finalizers, and cached audio's `exists()`-based uniqueness helper. These are static observations, not yet a new runtime reproduction.

Removing a tool flag alone does not coordinate cached copies, FFmpeg, final moves or concurrent jobs. A default ID in the name reduces common collisions but cannot prove that an existing artifact is safe to replace. Issue #32 already requires private reproducible work under the active cache root, protected final outputs and operation-wide cache leases; its gallery runtime has a verified non-replacing publication pattern which can inform a focused Rust implementation.

## 2. Decision

Adopt the following bounded output policy for direct yt-dlp and app audio/conversion jobs, including native requests using that backend:

1. Default to Keep both files; allocate a common numeric suffix for a conflicting artifact group. Offer Skip existing files, with explicit skip semantics. No overwrite mode is introduced.
2. Add Title, Title + video ID, Uploader + title + video ID and a controlled four-field custom basename template with preview. Keep Title as the default. Explicit/legacy saved filenames remain literal and protected. New automatic audio output follows the chosen rule rather than its prior hard-coded template.
3. Capture saved settings/metadata per operation. Introduce compatible naming provenance for queue items; do not reinterpret old nonempty names as disposable/generated text.
4. Stage downloads, fragments, sidecars and processing in unique operation-private work under the active C. Transfer the owner across every subprocess/thread/workflow handoff and retain cache ownership through publication.
5. Reserve one media/sidecar group through shared destination-local OS locks, including across installations. Final filesystem publication is always non-replacing, including a late non-cooperating writer. Same-volume atomic commit and a verified destination-local copy/commit handle cross-volume results.
6. Report actual published paths, explicit skips and partial failures. Preserve every established conflicting file and any already published complete artifact; cleanup owns only this operation's unpublished work.

The full contract is [backend sections 12/13](../backend-reference.md), [frontend section 8](../../EROffice_Frontend_Architecture.md) and product requirements OUTPUT-01 through OUTPUT-11. They define persisted keys, template grammar/fallbacks, save rollback, shared coordination namespace, artifact groups, finalization order, config compatibility and evidence cases. These details are normative for Builder implementation after confirmation.

## 3. Preserved boundaries

- Cache selection remains a next-start setting. Output preferences affect new dispatches without restart; neither setting mutates a running operation's snapshot.
- All media/fragment/processing work remains under checked active cache parents. Destination-local siblings contain completed data being atomically committed. Shared output coordination is final-output metadata, outside disposable-cache ownership.
- Preserve output folders, selected extension/container, playlist/source identity, subtitle languages, file times, actual-path reporting and per-artifact outcomes. Only automatic naming rules/collision suffixes intentionally alter basenames.
- Advanced output configuration may retain its supported naming/directory intent only when it can be represented safely by the shared planner. Unsupported bypassing options fail before media execution. User config files are not rewritten.
- Gallery-dl retains its pinned bundle, native source naming, archive keys, skip and XMP behavior under ADR-0002. No gallery rule UI, dependency change or bundle rebuild is added.
- Reuse existing Rust file-lock, Windows/atomic-publication, SHA, sanitizer, YAML and workflow facilities. Add focused output-policy/transaction modules; no new Rust runtime dependency or general downloader/installer/recovery framework.
- Abrupt process/power loss is not a general resume guarantee. Unknown orphan publication siblings are preserved rather than deleted by filename. Atomicity is per completed file; partial sibling groups are reported accurately and retained.

## 4. Alternatives considered

| Alternative | Assessment |
| --- | --- |
| Only remove `--force-overwrites` / add `--no-overwrites` | Does not cover app copies, overwrite-first postprocessing/finalizers or reservation races. Tool-specific skipping alone can also hide that a different source was not downloaded. |
| Always add video ID to the default name | Useful optional naming rule, but existing-file safety still needs publication protection, including repeated IDs and explicit names. Changes every default name unnecessarily. |
| `exists()` and select the next numbered file | Retains a race between the check and a replacing write; not sufficient as the final boundary. |
| Ask before every conflict | Blocks batches/native jobs and still requires race-safe execution after the answer. A deterministic keep/skip policy fits both entry points. |
| Overwrite with a backup or multi-file rollback framework | Adds destructive replacement/recovery complexity outside the confirmed proposal. Preserve existing files and explicitly report partial publication instead. |

## 5. Consequences, verification and authority

More than command arguments will change: request snapshots, optional queue provenance, the prepared-execution owner, audio/sidecar/transcode finalizers and native actual-result reporting must participate. Operation-private staging uses the existing recognized cache parents. A small marked coordination directory remains beside final outputs; user/unknown content is never adopted as coordination state. Cross-volume publication temporarily needs space for a completed destination-local copy.

Execute OUT-AC-01 through OUT-AC-14 with hashes and actual controlled tool/filesystem/UI evidence in both cache modes. Clearly label injected races/EXDEV/failures and source/native protocol boundaries. Retain earlier failures and #32 review history. Reviewer must examine the combined source diff and new evidence; prior #32 approval does not approve #33.

The user requested that the two issues be submitted together. The combined Git commit follows implementation/validation and the project's review gates. This architecture confirmation does not imply package, version, push, PR, merge, release or real tool replacement authority. `/docs/` remains ignored and is not force-added by this task.

## 6. R-OUT-015 decision: ordinary Stop boundary (2026-10-28)

The user delegated the tradeoff to Builder's recommendation. Builder recommended narrowing the special exited-parent guarantee to ordinary Stop, and the user instructed Builder to use the recommended approach. The boundary below records that choice; the user subsequently confirmed this structural amendment before product/test implementation. It does not close a Reviewer finding.

Keep ordinary cancellation of an active tracked tool, prevent later producers/publication, and retain all operation ownership through actual process/reader settlement. If the initial tracked parent has exited while a descendant still holds an inherited stdout/stderr pipe, accept that Stop or parent-failure reporting may wait for natural descendant completion and pipe EOF. Do not release cache/output ownership, clean live work or claim terminal settlement during that wait. No maximum wait or field occurrence rate is inferred from the controlled fixture.

The stronger R-OUT-011 Windows implementation passed its controlled case, but required platform Job Objects, suspended creation, local Win32 bindings and initial-thread discovery/resumption for all affected commands. The retained evidence establishes that constructed case rather than its real-tool frequency. This decision removes that special guarantee and its dedicated process-tree setup; it does not remove ordinary cancellation, pipe draining, no-clobber publication or protected ownership.

Backend 12.4.1 and OUT-AC-10, frontend 8.3 and product OUTPUT-08/section 7 define the narrowed implementation and acceptance. Preserve the earlier review and evidence verbatim. New validation must distinguish ordinary active-parent cancellation from an exited-parent finite descendant that is deliberately awaited, checking busy ownership during drain and actual settlement afterward in both cache modes. Other open findings, including R-CACHE-007 and R-OUT-012/013/014, retain their separate repair/validation requirements.

## 7. Simplicity repairs: configuration, results and small indirections (2026-10-28)

The user confirmed the remaining repair proposals after the R-OUT-015 implementation. The user confirmed this structural detail before implementation; it is not finding closure or new executed evidence. R-CACHE-007 scheduling is defined separately in ADR-0002 section 4.1.

For R-OUT-012, read/parse/validate the supported effective config once at each logical operation's external-data boundary. Carry a focused typed result through reservation, metadata/format planning, attempts/retries and music preparation/command handoffs. Derive all relevant option ownership from the same comment-aware quoted/escaped argument stream, retaining supported path/output intent and one protected-option rejection pass. Preserve per-use path/storage/publication checks at their genuine external boundaries. Do not add a general config parser/service or speculative change detection.

Re-review amendment for R-OUT-012 (proposal and structural detail confirmed before follow-up implementation): the playback/prefetch boundary includes primary and optional cookie-free retry preparation, both using one captured effective config. Bind before command-specific preparation to remove recursive duplicate storage/tool checks, including the lyrics builder. Main's target selector and path textbox share one render-scoped locked/display observation, then obtain a fresh one on the next render. All ownership/output/protected-option consumers share one option/value interpretation: values consumed by supported options retain value semantics even when spelled `-f` or `-Pfoo`. Preserve the currently supported flat subset, aliases, quoting/escaping, existing precedence and output restrictions. No persistent UI cache, general parser framework, new dependency or runtime option-discovery subprocess is introduced. Any intentional support reduction returns to user discussion first. Backend 12.5.1 and frontend 8.2 specify actual playback/prefetch counts, consistent controls and real local-tool matching-title format/directory cases; model-only evidence does not close those cases.

The launched tool still reads the original resolved config path. Internal result reuse does not guarantee immutable bytes for that separate read; concurrent external config mutation is outside the consistency contract. A later operation reads current configuration anew. Preserve existing config selection/`--ignore-config` behavior; no user-file rewrite, private config snapshot, watcher or lock is added. This explicit limitation avoids promising stronger consistency than the implementation boundary supplies. Product section 7 and backend 12.5.1 state the user workflow and required direct/music comment/format/count tests.

For R-OUT-013, carry disposition, complete published paths and a diagnostic message as typed failure state from the transaction to desktop/native presentation. All later validation/directory/publication failures retain completed paths. The native outer error envelope derives outcome from this state and reuses `outputPath`/`outputPaths`; it never searches English error text to reconstruct PartialFailure. Backend 12.5.2 requires actual outer-envelope fixtures, including a first completed member followed by another member's failure, plus pre-publication failure/success/skip/cancel controls. Preserve complete outputs and existing no-clobber semantics.

For R-OUT-014, inline the one-use constant chapter argument and keep one four-mode template mapping shared by metadata-needs and rendering. The dedicated process-tree forwarding function was already removed by R-OUT-015. Preserve meaningful domain/ownership boundaries, exact chapter arguments and naming behavior; no generic abstraction framework, dependency change or unrelated refactor is introduced. Backend 12.5.3 supplies the bounded implementation scope.

## 8. Protected configuration and metadata-only analysis (2026-10-28)

The follow-up review closes the reported R-CACHE-007/008 triggers but leaves R-OUT-012 open with R-OUT-016 through R-OUT-019. The user confirmed the repair proposal below; the user confirmed this structural amendment on 2026-10-29 before implementation. Source findings and earlier Builder evidence do not constitute executed acceptance of this amendment.

Keep the existing operation-bound effective config and one lexical/option-value/protected interpretation. Normalize one optional leading UTF-8 BOM in memory so the first option receives normal validation; retain the original file and embedded character data. At an option boundary reject standalone `--`, which would terminate parsing of the later application-enforced arguments in the actual tool. A consumed value spelled `--` remains data. The shared boundary governs both output and metadata commands.

Restrict built-in `-t` / `--preset-alias` to the retained non-output preset `sleep`. Reject `mp3`, `aac`, `mp4`, `mkv` and unknown preset values with the supplied option/value and a repair explanation. The known output presets inject extraction/conversion/remux options already forbidden individually by protected output. This is the intentional supported-subset restriction selected in the confirmed proposal. Canonical option interpretation covers existing attached/short/long alias spellings. A preset-expansion framework is unnecessary for this boundary; unfamiliar future presets are not presumed safe.

Bind and validate unbound metadata operations using the same protected result, and reuse already bound configuration during output naming/format resolution and retry preparation. Append explicit `--simulate` and `--skip-download` after config options for full analysis, unknown-source probes, playlist discovery and music metadata queries. Do not rely on JSON dumping to imply simulation. Reject forbidden hooks and unsupported config destinations before execution. Metadata resolution may precede reservation only while it performs no media download, sidecar write, hook or output publication. Retain actual storage/process ownership. Actual downloads continue to support `--no-simulate`, `--force-overwrites` and safely planned relative config output within reserved private work and no-replace publication.

This refines backend 12.5.1 and the OUTPUT-02/06/10 boundary, with detail in backend 12.5.4, product section 7 and frontend 8.2/8.4. It adds no dependency, general parser/alias service, runtime option-discovery process, watcher, private config copy or user-file rewrite. The configured tool still reads its original file; concurrent external mutation and unverified tool versions retain the existing evidence limits.

Acceptance uses actual configured tools, localhost fixtures and preserved established-file hashes in both cache modes and explicit/portable config variants. Cover BOM benign/hook controls; rejected standalone terminators including metadata/preflight routes; preset rejection and harmless `sleep`; actual known-single analysis and unresolved naming preflight under config `--no-simulate`/`--force-overwrites`; unbound hook/unsupported absolute destination rejection; and subsequent protected real output. Record zero media transfer or outside-work output before reservation, one shared application config observation, correct final keep-both/skip behavior, and relevant metadata/music/cache/gallery regressions. Run any GUI checks on a private non-input desktop or silently in the background without interfering with the user's active desktop. Preserve Reviewer history and prior evidence; no review closure, installer/release or combined-commit authorization follows from this design update.
