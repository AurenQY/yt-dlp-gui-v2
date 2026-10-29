//! R-OUT-020: actual playback/prefetch workers settle preparation rejection logs.
use super::review_round1::new_state;
use super::review_round3::music_item;
use super::*;
use serde_json::json;

pub(super) fn run(root: &Path, tools: &ToolPaths, server: &Server) {
    for explicit in [false, true] {
        for (kind, bytes, expected_counts, diagnostic) in [
            (
                "unsupported",
                b"--exec \"echo forbidden\"\n".as_slice(),
                [1, 1, 1],
                "--exec",
            ),
            (
                "malformed",
                b"--user-agent \"unfinished\n".as_slice(),
                [1, 1, 0],
                "quot",
            ),
            (
                "unreadable",
                b"\xff\xfe\xff".as_slice(),
                [1, 0, 0],
                "Could not inspect",
            ),
        ] {
            for playback in [true, false] {
                let route = if playback { "playback" } else { "prefetch" };
                let label = format!(
                    "resolve-log-{}-{kind}-{route}",
                    if explicit { "explicit" } else { "portable" }
                );
                let folder = root.join(&label);
                fs::create_dir_all(&folder).unwrap();
                let config = if explicit {
                    folder.join("external.conf")
                } else {
                    Path::new(&tools.yt_dlp)
                        .parent()
                        .unwrap()
                        .join("yt-dlp.conf")
                };
                fs::write(&config, bytes).unwrap();
                fs::write(folder.join("config-before.conf"), bytes).unwrap();
                let mut selected = tools.clone();
                selected.yt_dlp_config = if explicit {
                    config.display().to_string()
                } else {
                    String::new()
                };
                let mut state = new_state(root, &selected, &label);
                let source = format!("{}/resolve-log/{label}.html", server.address);
                let id = music_item(&mut state, &source, "Rejected preparation");
                state
                    .queue_item_mut_by_id(id)
                    .unwrap()
                    .selection
                    .use_cookies = true;
                let before = crate::infrastructure::output_config_observation_counts(&config);
                let exited_before = crate::infrastructure::exited_tracked_test_pids();
                let http_before =
                    fs::read_to_string(root.join("http-requests.jsonl")).unwrap_or_default();
                let media_before = server.media_get_count();
                if playback {
                    state.resolve_music_item_for_playback_with_session(id, 7);
                } else {
                    state.start_music_prefetch_for_item(id);
                    assert_eq!(state.music.music_prefetch_pending_item_id, Some(id));
                    assert!(state.music.music_prefetch_started_at.is_some());
                }
                let action_id = state.tool_log_actions().back().unwrap().id;
                assert_eq!(
                    state.tool_log_actions().back().unwrap().status,
                    ToolLogStatus::Running
                );
                let mut events = Vec::new();
                let mut finished = false;
                while !finished {
                    let event = state
                        .music_stream_result_rx
                        .recv_timeout(Duration::from_secs(10))
                        .unwrap();
                    match &event {
                        MusicStreamResolveEvent::PreparationFailed {
                            action_id: actual,
                            action,
                            error,
                        } => {
                            assert_eq!(*actual, action_id);
                            assert_eq!(
                                action,
                                if playback {
                                    "resolve stream"
                                } else {
                                    "prefetch resolve"
                                }
                            );
                            assert!(error.contains(diagnostic), "{kind}: {error}");
                            events.push(json!({"event":"preparation-failed","action_id":actual,"action":action,"error":error}));
                        }
                        MusicStreamResolveEvent::Resolve {
                            play_after_resolve,
                            result,
                            ..
                        } => {
                            assert_eq!(*play_after_resolve, playback);
                            let error = result.as_ref().err().unwrap();
                            assert!(error.contains(diagnostic));
                            events.push(json!({"event":"resolve","error":error}));
                            finished = true;
                        }
                        _ => {
                            panic!("Preparation rejection unexpectedly emitted a tool/import event")
                        }
                    }
                    state.apply_music_stream_result(event);
                }
                assert_eq!(events.len(), 2);
                assert!(state.music_stream_result_rx.try_recv().is_err());
                let terminal = if playback {
                    ToolLogStatus::Failed
                } else {
                    ToolLogStatus::Skipped
                };
                let action = state
                    .tool_log_actions()
                    .iter()
                    .find(|a| a.id == action_id)
                    .unwrap();
                assert_eq!(action.status, terminal);
                assert_eq!(action.steps.len(), 1);
                let step = &action.steps[0];
                assert_eq!(step.status, terminal);
                assert_eq!(step.tool, "v2");
                assert_eq!(step.action, "prepare configuration");
                assert!(step.command.is_empty());
                let detail = step.detail.as_ref().unwrap();
                assert!(detail.contains("before launching yt-dlp") && detail.contains(diagnostic));
                let counts = crate::infrastructure::output_config_observation_counts(&config);
                let observed: [usize; 3] = std::array::from_fn(|i| counts[i] - before[i]);
                assert_eq!(observed, expected_counts);
                assert_eq!(
                    crate::infrastructure::exited_tracked_test_pids(),
                    exited_before
                );
                assert!(crate::infrastructure::tracked_test_pids().is_empty());
                assert_eq!(
                    fs::read_to_string(root.join("http-requests.jsonl")).unwrap_or_default(),
                    http_before
                );
                assert_eq!(server.media_get_count(), media_before);
                assert_eq!(fs::read(&config).unwrap(), bytes);
                let item = state.queue_item_by_id(id).unwrap();
                if playback {
                    assert_eq!(item.compact_music_state, Some(CompactMusicState::Failed));
                    assert!(
                        matches!(&item.metadata_state, MetadataState::Failed(error) if error.contains(diagnostic))
                    );
                    assert!(item.last_error.as_ref().unwrap().contains(diagnostic));
                    assert!(
                        state
                            .music
                            .music_player_error
                            .as_ref()
                            .unwrap()
                            .contains(diagnostic)
                    );
                    assert!(state.last_action.contains(diagnostic));
                } else {
                    assert!(state.music.music_prefetch_pending_item_id.is_none());
                    assert!(state.music.music_prefetch_active_item_id.is_none());
                    assert!(state.music.music_prefetch_started_at.is_none());
                    assert!(state.music.music_player_error.is_none());
                    assert!(item.last_error.is_none());
                }
                assert!(state.music.music_playback.is_none());
                fs::write(folder.join("case-result.json"), serde_json::to_vec_pretty(&json!({
                    "route":route,"explicit":explicit,"kind":kind,"application_counts":observed,
                    "action_id":action_id,"action_status":format!("{:?}",action.status),
                    "steps":[{"status":format!("{:?}",step.status),"tool":step.tool,"action":step.action,
                        "command":step.command,"detail":step.detail}],"events":events,
                    "playback_error":state.music.music_player_error,"item_error":item.last_error,
                    "prefetch_pending":state.music.music_prefetch_pending_item_id,
                    "prefetch_started":state.music.music_prefetch_started_at.is_some(),
                    "new_tool_processes":0,"tool_attempt_events":0,"cookie_free_retries":0,
                    "http_unchanged":true,"media_gets_before":media_before,"media_gets_after":server.media_get_count(),
                    "config_unchanged":true,"audio_device_handoff_executed":false
                })).unwrap()).unwrap();
            }
        }
    }
    println!(
        "R-OUT-020 actual resolve-event acceptance passed: {}",
        root.display()
    );
}
