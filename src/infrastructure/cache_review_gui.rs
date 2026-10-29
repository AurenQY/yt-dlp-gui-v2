//! Explicitly compiled validation entry point; absent from normal builds.
//! Runs the current NgDlpApp, with only project-local fixture control/state.
//! Native-host registration and self-update startup are outside this UI test.
use crate::{
    app::{
        NgDlpApp,
        state::{AppMode, AppTab},
    },
    infrastructure::{self, AppConfig, storage_paths},
};
use eframe::egui;
use std::{
    fs::{self, File, OpenOptions},
    path::PathBuf,
    time::{Duration, Instant},
};

pub(crate) fn run() -> eframe::Result<()> {
    let root = infrastructure::runtime_paths::application_root();
    assert!(root.starts_with(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/cache-location-validation")
    ));
    infrastructure::runtime_paths::set_application_current_dir().unwrap();
    let (config, tools) = AppConfig::load_runtime();
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 800.0]),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "Cache review GUI - controlled fixture",
        options,
        Box::new(move |cc| {
            let mut app = NgDlpApp::new_with_runtime(cc, config, tools);
            app.review_gui_state().active_tab = AppTab::Options;
            Ok(Box::new(Fixture {
                app,
                root,
                command: String::new(),
                busy: None,
                locked_settings: None,
                runtime_lock: None,
                paint_micros: Vec::new(),
                logic_micros: Vec::new(),
            }))
        }),
    )
}

struct Fixture {
    app: NgDlpApp,
    root: PathBuf,
    command: String,
    busy: Option<storage_paths::StorageUseGuard>,
    locked_settings: Option<File>,
    runtime_lock: Option<File>,
    paint_micros: Vec<u128>,
    logic_micros: Vec<u128>,
}
impl eframe::App for Fixture {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Read the renderer's own pixels on an isolated desktop. Capturing is
        // explicit fixture activity and is never used to wake idle acceptance.
        for event in ctx.input(|input| input.events.clone()) {
            if let egui::Event::Screenshot {
                user_data, image, ..
            } = event
                && let Some(label) = user_data
                    .data
                    .as_ref()
                    .and_then(|data| data.downcast_ref::<String>())
            {
                let rgba: Vec<u8> = image
                    .pixels
                    .iter()
                    .flat_map(|pixel| pixel.to_array())
                    .collect();
                image::save_buffer_with_format(
                    self.root
                        .parent()
                        .unwrap()
                        .join("evidence")
                        .join(format!("{label}.png")),
                    &rgba,
                    image.size[0] as u32,
                    image.size[1] as u32,
                    image::ColorType::Rgba8,
                    image::ImageFormat::Png,
                )
                .unwrap();
            }
        }
        if let Ok(command) = fs::read_to_string(self.root.join("control.txt")) {
            if command != self.command {
                self.command = command.clone();
                match command.trim() {
                    "busy" => self.busy = Some(storage_paths::use_cache().unwrap()),
                    "release" => self.busy = None,
                    "save-fail" => {
                        use std::os::windows::fs::OpenOptionsExt;
                        self.locked_settings = Some(
                            OpenOptions::new()
                                .read(true)
                                .share_mode(1)
                                .open(infrastructure::config::runtime_config_file_path())
                                .unwrap(),
                        );
                    }
                    "save-allow" => self.locked_settings = None,
                    "pending" => {
                        use infrastructure::component_update::{
                            PendingAppUpdateManifest, PendingAppUpdateState,
                        };
                        let staged = storage_paths::cache_directory("temp/manifest")
                            .unwrap()
                            .join("staged/app/fixture.exe");
                        fs::create_dir_all(staged.parent().unwrap()).unwrap();
                        fs::write(&staged, b"synthetic pending-update data; not executable")
                            .unwrap();
                        let manifest = PendingAppUpdateManifest {
                            state: PendingAppUpdateState::DownloadedStaged,
                            version: "2099.1.1".into(),
                            staged_exe: staged,
                            target_exe: self.root.join("yt-dlp-gui.exe"),
                            backup_exe: self.root.join("state/component-update/backup/fixture.exe"),
                            release_url: None,
                            downloaded_at_unix: 0,
                            attempt_count: 0,
                            last_error: None,
                        };
                        infrastructure::yaml_store::write_persistent_yaml(
                            &storage_paths::state_path("component-update/pending_app_update.yaml"),
                            &manifest,
                        )
                        .unwrap();
                    }
                    "gallery-valid" => self.app.review_gui_state().set_gallery_dl_path(
                        self.root
                            .join("runtime/gallery-dl.exe")
                            .display()
                            .to_string(),
                    ),
                    "gallery-refresh" => self.app.review_gui_state().refresh_prepare_report(),
                    "gallery-install-entry" => self
                        .app
                        .review_gui_state()
                        .update_component(infrastructure::ManagedComponentId::GalleryDl),
                    "yt-missing" => self.app.review_gui_state().set_yt_dlp_path(
                        self.root.join("missing/yt-dlp.exe").display().to_string(),
                    ),
                    "config-output" => self.app.review_gui_state().set_yt_dlp_config_path(
                        self.root.join("ui-output.conf").display().to_string(),
                    ),
                    "config-value" => self.app.review_gui_state().set_yt_dlp_config_path(
                        self.root.join("ui-value.conf").display().to_string(),
                    ),
                    "guard-terminator" | "guard-preset" | "guard-metadata" => {
                        let state = self.app.review_gui_state();
                        state.set_app_mode(AppMode::Origin);
                        state.set_yt_dlp_config_path(
                            self.root
                                .join(format!("{}.conf", command.trim()))
                                .display()
                                .to_string(),
                        );
                        state.url_input = fs::read_to_string(self.root.join("fixture-url.txt"))
                            .unwrap()
                            .trim()
                            .into();
                        state.active_tab = AppTab::Main;
                    }
                    "gallery-stale" => {
                        self.app.review_gui_state().set_gallery_dl_path(
                            self.root
                                .join("runtime/gallery-dl.exe")
                                .display()
                                .to_string(),
                        );
                        self.app.review_gui_state().set_gallery_dl_path(
                            self.root
                                .join("missing/gallery-dl.exe")
                                .display()
                                .to_string(),
                        );
                    }
                    "gallery-operation-guard" => {
                        let state = self.app.review_gui_state();
                        let ready = state.gallery_is_ready_for_display();
                        let tools = state.tool_paths.clone();
                        let root = self.root.clone();
                        std::thread::spawn(move || {
                            let manifest = PathBuf::from(&tools.gallery_dl)
                                .parent()
                                .unwrap()
                                .join("bundle.manifest.json");
                            let original = fs::read(&manifest).unwrap();
                            fs::write(&manifest, b"invalid controlled manifest").unwrap();
                            let result =
                                infrastructure::gallery_dl::GalleryDlBackend::from_tool_paths(
                                    &tools,
                                )
                                .analyze_url(
                                    "http://127.0.0.1:9/controlled-gallery",
                                    &tools,
                                    false,
                                );
                            fs::write(&manifest, &original).unwrap();
                            fs::write(root.join("operation-guard.json"), serde_json::to_vec_pretty(&serde_json::json!({
                                "ready_display_before":ready, "rejected":result.is_err(), "error":result.err(),
                                "manifest_restored":fs::read(&manifest).unwrap() == original,
                                "thread":format!("{:?}", std::thread::current().id())
                            })).unwrap()).unwrap();
                        });
                    }
                    "gallery-missing" => self.app.review_gui_state().set_gallery_dl_path(
                        self.root
                            .join("missing/gallery-dl.exe")
                            .display()
                            .to_string(),
                    ),
                    "gallery-incompatible" => self.app.review_gui_state().set_gallery_dl_path(
                        self.root
                            .join("incompatible/gallery-dl.exe")
                            .display()
                            .to_string(),
                    ),
                    "gallery-managed" => self.app.review_gui_state().set_gallery_dl_path(
                        self.root
                            .join("tools/gallery-dl/gallery-dl.exe")
                            .display()
                            .to_string(),
                    ),
                    "gallery-busy" => {
                        let file = storage_paths::open_lock(&storage_paths::state_path(
                            "runtime/gallery-dl-runtime.lock",
                        ))
                        .unwrap();
                        file.try_lock().unwrap();
                        self.runtime_lock = Some(file);
                        self.app.review_gui_state().refresh_prepare_report();
                    }
                    "gallery-release" => {
                        self.runtime_lock = None;
                        self.app.review_gui_state().refresh_prepare_report();
                    }
                    "close" => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                    "output-sample" => {
                        let url = fs::read_to_string(self.root.join("fixture-url.txt")).unwrap();
                        self.app.review_gui_state().review_output_sample(url.trim());
                    }
                    "output-download" | "output-download-again" => {
                        self.app.review_gui_state().review_output_download();
                        self.app.review_gui_state().active_tab = AppTab::Main;
                    }
                    "output-options" => self.app.review_gui_state().active_tab = AppTab::Options,
                    "output-explicit" => {
                        if let Some(item) = self.app.review_gui_state().queue_items.first_mut() {
                            item.selection.name_origin =
                                Some(crate::domain::OutputNameOrigin::Explicit);
                            item.selection.file_name = "Explicit 100% 空间".into();
                        }
                    }
                    "output-partial-feedback" => {
                        if let Some(item) = self.app.review_gui_state().queue_items.first_mut() {
                            let error = format!(
                                "Output publication failed: controlled UI-only partial-failure sample. Complete published files: {:?}",
                                item.last_output_path
                            );
                            item.last_error = Some(error.clone());
                            if let Some(run) = item.workflows.last_mut() {
                                run.state = crate::domain::WorkflowState::Failed;
                                run.error = Some(error);
                            }
                        }
                    }
                    _ => {
                        if let Some(label) = command.trim().strip_prefix("capture:")
                            && !label.is_empty()
                            && label
                                .chars()
                                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                                egui::UserData::new(label.to_owned()),
                            ));
                        }
                    }
                }
            }
        }
        let started = Instant::now();
        self.app.logic(ctx, frame);
        self.logic_micros.push(started.elapsed().as_micros());
        {
            let state = self.app.review_gui_state();
            let data = serde_json::json!({"active_root":state.cache_location_display(),"pending_root":state.pending_cache_location_display(),
                "saved_mode":format!("{:?}",state.saved_cache_location_mode()),"storage_error":state.cache_storage_error(),
                "last_action":state.last_action,"legacy_notice":state.config.legacy_cache_notice,"busy_fixture":self.busy.is_some(),
                "settings_locked":self.locked_settings.is_some(),"build_date":env!("YT_DLP_GUI_BUILD_DATE")});
            let mut data = data;
            data["gallery_ready"] = serde_json::json!(state.gallery_is_ready_for_display());
            data["gallery_pending"] = serde_json::json!(state.gallery_readiness_pending());
            data["gallery_detail"] = serde_json::json!(state.gallery_readiness_detail());
            data["gallery_path"] = serde_json::json!(state.tool_paths.gallery_dl);
            data["gallery_events"] = serde_json::json!(state.review_gallery_events());
            data["clipboard_enabled"] = serde_json::json!(state.clipboard_monitor_enabled());
            data["active_work"] = serde_json::json!(state.has_active_work());
            data["music_active"] = serde_json::json!(state.has_music_playback_activity());
            data["loading_thumbnails"] = serde_json::json!(state.has_loading_thumbnails());
            data["component_update_running"] = serde_json::json!(state.component_update_running());
            data["main_output_observations"] = ctx
                .data(|data| {
                    data.get_temp::<Vec<serde_json::Value>>(egui::Id::new(
                        "review-main-output-observations",
                    ))
                    .unwrap_or_default()
                })
                .into();
            data["gallery_checks"] =
                serde_json::json!(infrastructure::gallery_runtime_policy::review_check_samples());
            data["ui_thread"] = serde_json::json!(format!("{:?}", std::thread::current().id()));
            data["paint_micros"] = serde_json::json!(self.paint_micros);
            data["logic_micros"] = serde_json::json!(self.logic_micros);
            data["control"] = serde_json::json!(self.command);
            data["output_draft"] = serde_json::json!({"naming":format!("{:?}",state.output_draft.naming),"existing":format!("{:?}",state.output_draft.existing),"template":state.output_draft.template});
            data["output_saved"] = serde_json::json!({"naming":format!("{:?}",state.config.output_naming),"existing":format!("{:?}",state.config.output_existing_file),"template":state.config.output_template});
            data["output_feedback"] = serde_json::json!(state.output_settings_feedback);
            data["queue"] = serde_json::json!(state.queue_items.iter().map(|item| serde_json::json!({"file_name":item.selection.file_name,"name_origin":item.selection.name_origin,"last_output":item.last_output_path,"error":item.last_error,"workflow_details":item.workflows.iter().map(|run| &run.detail).collect::<Vec<_>>()})).collect::<Vec<_>>());
            fs::write(
                self.root.join("gui-state.json"),
                serde_json::to_vec_pretty(&data).unwrap(),
            )
            .unwrap();
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let started = Instant::now();
        self.app.ui(ui, frame);
        self.paint_micros.push(started.elapsed().as_micros());
    }
    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        self.app.on_exit(gl);
    }
}

// A bounded test-only barrier separates pending UI input from completion. It
// never requests a repaint; only the real readiness worker notification does.
pub(crate) fn wait_for_readiness_fixture() {
    let root = infrastructure::runtime_paths::application_root();
    let started = Instant::now();
    while root.join("hold-readiness").exists() && started.elapsed() < Duration::from_secs(45) {
        std::thread::sleep(Duration::from_millis(20));
    }
}
