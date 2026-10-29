pub fn text<'a>(key: &'a str) -> &'a str {
    match key {
        "gallery.range.method" => "Metodo di selezione",
        "gallery.range.time" => "Per periodo",
        "gallery.range.posts" => "Per numero di post",
        "gallery.range.date" => "Data iniziale (UTC)",
        "gallery.range.count" => "Numero di post (intero positivo)",
        "gallery.range.custom" => "Personalizzato",
        "gallery.range.positive" => "Intero positivo",
        "gallery.range.invalid" => "Inserisci un intervallo di download valido.",
        "gallery.range.invalid_date" => "Inserisci una data UTC valida (YYYY-MM-DD).",
        "gallery.range.help" => "Aiuto",
        "gallery.range.title" => "Intervallo di download della timeline",
        "gallery.range.time_or_all" => "Seleziona un periodo o tutto",
        "gallery.range.latest" => "Ultimi {count} post",
        "gallery.range.recent24h" => "Ultime 24 ore",
        "gallery.range.days" => "Ultimi {days} giorni",
        "gallery.range.since" => "Dal {date} (UTC)",
        "gallery.status.finishing" => "Finalizzazione",
        "gallery.status.waiting" => "In attesa di download",
        "gallery.status.analyzing" => "Analisi",
        "gallery.posts_count" => "{count} post",
        "gallery.media_count" => "{count} contenuti multimediali",
        "music.volume" => "Volume",
        "gallery.folder_pending" => "Da confermare",
        "gallery.folder_opened" => "Cartella aperta.",
        "gallery.choice.downloader" => "Scegli downloader",
        "gallery.choice.content" => "Scegli contenuto",
        "gallery.choice.add_title" => "Scegli contenuto da aggiungere",
        "gallery.choice.dual_hint" => {
            "Due downloader supportano questo URL. Selezionane almeno uno."
        }
        "gallery.choice.origin_hint" => {
            "Scegli il contenuto da analizzare (modalità classica: un tipo alla volta)."
        }
        "gallery.choice.add_hint" => "Scegli il contenuto da aggiungere",
        "gallery.choice.dual_media" => "Video e immagini possono essere aggiunti insieme.",
        "gallery.choice.one_required" => "Seleziona un downloader.",
        "gallery.choice.tool_required" => "Seleziona almeno un downloader.",
        "gallery.choice.content_required" => "Scegli il contenuto da analizzare o aggiungere.",
        "gallery.choice.cancelled" => "Selezione annullata.",
        "gallery.range.help_steps" => "Scegli il metodo e i limiti, poi conferma per applicarli.",
        "gallery.range.help_multiple" => {
            "Questa fonte comprende più feed. Non è disponibile un limite totale dei post; scegli un periodo o tutto."
        }
        "gallery.range.help_order" => {
            "I post seguono l’ordine del sito e possono includere quelli fissati. Al riavvio si riparte dall’inizio e si saltano i file esistenti."
        }
        "gallery.range.help_time" => {
            "Il periodo si basa sulle date dei post. Instagram salta i vecchi post fissati e si ferma a un post normale fuori intervallo. Altre fonti possono esaminare tutta la timeline. L’assenza di date valide causa un errore."
        }
        "about.core_tools" => "Strumenti principali",
        "about.extension_tools" => "Strumenti aggiuntivi",
        "about.gallery_dl_description" => {
            "Scarica gallerie di immagini e raccolte dai social media."
        }
        "about.aria2_description" => {
            "Downloader esterno per yt-dlp; le prestazioni dipendono dal sito e dalla connessione."
        }
        "about.required" => "Obbligatorio",
        "about.availability.ready" => "Disponibile",
        "about.availability.missing" => "Non trovato",
        "advance.use_aria2_external_downloader" => "Usa aria2 come downloader esterno",
        "tab.advanced" => "Avanzate",
        "advance.browser_integration" => "Integrazione browser",
        "advance.browser_connection" => "Connessione browser",
        "advance.browser_connection_enabled" => "Attiva per {browsers}",
        "advance.browser_connection_disabled" => "Connetti Chrome, Brave ed Edge",
        "advance.browser_connection_enabled_message" => "Connessione browser attivata.",
        "advance.browser_connection_disabled_message" => "Connessione browser disattivata.",
        "tab.about" => "About",
        "about.tools" => "Versioni strumenti",
        "about.current_version" => "Corrente",
        "about.latest_version" => "Più recente",
        "about.author" => "Autore",
        "about.source" => "Origine",
        "about.status" => "Stato",
        "about.message" => "Messaggio",
        "about.check_updates" => "Controlla aggiornamenti",
        "about.update_all" => "Aggiorna tutto",
        "about.install_dependencies" => "Installa dipendenze",
        "about.restart" => "Riavvia",
        "about.open_release" => "Apri Release",
        "about.install" => "Installa",
        "about.update" => "Aggiorna",
        "about.running" => "Controllo aggiornamenti in corso...",
        "about.last_check" => "Ultimo controllo:",
        "about.relative.minutes" => "{count} min",
        "about.relative.hour" => "1 ora",
        "about.relative.hours" => "{count} ore",
        "about.relative.day" => "1 giorno",
        "about.relative.days" => "{count} giorni",
        "about.never_checked" => "Aggiornamenti non ancora controllati",
        "about.no_release_notes_loaded" => {
            "Nessuna nota di rilascio caricata. Premi prima Controlla aggiornamenti."
        }
        "about.ownership.managed_portable" => "Gestito da v2",
        "about.ownership.external" => "Esterno",
        "about.ownership.missing" => "Mancante",
        "about.ownership.unknown" => "Sconosciuto",
        "about.status.unknown" => "Non controllato",
        "about.status.checking" => "Controllo",
        "about.status.up_to_date" => "Aggiornato ✓",
        "about.status.update_available" => "Aggiornamento disponibile ↑",
        "about.status.missing" => "Mancante +",
        "about.status.downloading" => "Download in corso",
        "about.status.downloading_percent" => "Download {percent}%",
        "about.status.staged" => "Preparato",
        "about.status.pending_restart" => "Riavvio in sospeso",
        "about.status.applying" => "Applicazione",
        "about.status.installed" => "Installato",
        "about.status.skipped" => "Saltato",
        "about.status.failed" => "Non riuscito !",
        "tab.options" => "Opzioni",
        "tab.log" => "Registro",
        "main.url_hint" => "URL",
        "action.download" => "Scarica",
        "action.add" => "Aggiungi",
        "action.analyze" => "Analizza",
        "action.stop" => "Ferma",
        "action.stopping" => "Arresto...",
        "action.cut" => "Taglia",
        "action.copy" => "Copia",
        "action.paste" => "Incolla",
        "action.clear" => "Cancella",
        "item.thumbnail" => "Miniatura",
        "item.thumbnail_preview" => "Anteprima miniatura",
        "single.title" => "Titolo",
        "single.description" => "Descrizione",
        "single.info.channel" => "Canale",
        "single.info.date" => "Data",
        "single.info.views" => "Visualizzazioni",
        "item.download_thumbnail" => "Scarica miniatura",
        "media.video" => "Video",
        "media.audio" => "Audio",
        "media.subtitle" => "Sottotitoli",
        "media.section" => "Intervallo",
        "item.file_name" => "Nome file",
        "main.target_folder" => "Cartella di output",
        "picker.title.video" => "Seleziona formato video",
        "picker.title.audio" => "Seleziona formato audio",
        "picker.title.subtitle" => "Seleziona sottotitoli",
        "picker.title.section" => "Seleziona intervallo",
        "action.back" => "Indietro",
        "picker.mode.filter" => "Filtri",
        "picker.mode.table" => "Tabella",
        "action.confirm" => "Conferma",
        "picker.empty_table" => "Nessun formato da mostrare",
        "picker.header.resolution" => "Risoluzione",
        "picker.header.range" => "Intervallo",
        "picker.header.fps" => "FPS",
        "picker.header.format" => "Formato",
        "picker.header.codec" => "Codec",
        "picker.header.size" => "Dimensione",
        "picker.header.sample_rate" => "Frequenza campionamento",
        "picker.filter.resolution" => "Risoluzione",
        "picker.filter.range" => "Intervallo",
        "picker.filter.fps" => "FPS",
        "picker.filter.codec" => "Codec",
        "picker.filter.sample_rate" => "Frequenza campionamento",
        "main.missing_yt_dlp_callout" => {
            "yt-dlp manca. Installalo o scegli yt-dlp.exe in Informazioni."
        }
        "advance.source" => "Sorgente",
        "advance.analysis" => "Analisi",
        "advance.force_analysis" => "Forza analisi",
        "advance.config" => "Configurazione",
        "advance.none" => "Nessuno",
        "advance.network_access" => "Rete e accesso",
        "advance.proxy" => "Proxy",
        "advance.enable_proxy" => "Abilita proxy",
        "advance.certificate" => "Certificato",
        "advance.skip_certificate_verification" => "Salta verifica certificato",
        "advance.use_cookies" => "Usa cookie",
        "advance.enable_cookies" => "Abilita cookie",
        "advance.cookie_source" => "Origine cookie",
        "advance.cookie_source.auto" => "Automatico per sito web",
        "advance.cookie_source.file" => "Usa file (cookies.txt)",
        "advance.cookie_auto" => "Automatico",
        "advance.cookie_auto_note" => "I download usano il Cookie salvato che corrisponde all'URL.",
        "advance.cookie_rescue" => "Recupero Cookie",
        "advance.cookie_file" => "File cookie",
        "advance.get_cookie" => "Ottieni Cookie",
        "advance.cookie" => "Cookie",
        "advance.cookie.off" => "Do not use",
        "advance.cookie.browser" => "Browser",
        "advance.cookie.file" => "File",
        "advance.cookie_file_source" => "File source",
        "advance.cookie_file_custom" => "Custom cookies.txt",
        "advance.cookie_file_auto_select" => "Auto select",
        "advance.cookie_manager_row" => "Management",
        "advance.manage_cookie" => "Manage Cookie",
        "advance.cookie_manager_title" => "Cookie Manager",
        "advance.add_cookie" => "Add Cookie",
        "advance.cookie_manager_empty" => "No Cookie files have been added.",
        "advance.cookie_manager_name" => "Name",
        "advance.cookie_manager_updated" => "Updated",
        "advance.cookie_manager_actions" => "Actions",
        "advance.cookie_manager_refresh" => "Reacquire",
        "advance.cookie_manager_delete" => "Delete",
        "advance.file" => "File",
        "youtube_login_rescue.short_note" => {
            "Apri una finestra del browser dedicata per ottenere i cookie."
        }
        "youtube_login_rescue.title" => "Recupero Cookie",
        "youtube_login_rescue.confirm_heading" => "Apri una finestra di accesso dedicata",
        "youtube_login_rescue.confirm_body" => {
            "Una finestra indipendente di {browser} aprirà l'URL senza usare i dati personali del browser."
        }
        "youtube_login_rescue.target_url_label" => "URL del sito web",
        "youtube_login_rescue.target_url_hint" => "https://www.youtube.com/",
        "youtube_login_rescue.clipboard_prefilled" => "L'URL è stato inserito dagli appunti.",
        "youtube_login_rescue.drop_url_note" => {
            "Incolla un URL o trascina un file .url / di testo."
        }
        "youtube_login_rescue.paste_clipboard" => "Incolla appunti",
        "youtube_login_rescue.cookie_note" => {
            "Accedi lì. Quando i cookie vengono trovati, la finestra si chiude e li applica automaticamente."
        }
        "youtube_login_rescue.no_browser_title" => "Nessun browser supportato trovato",
        "youtube_login_rescue.no_browser_body" => {
            "Per ottenere i cookie servono attualmente Chrome, Brave o Microsoft Edge. Puoi comunque scegliere manualmente cookies.txt."
        }
        "youtube_login_rescue.start" => "Avvia",
        "youtube_login_rescue.opening" => "Apertura di {browser}...",
        "youtube_login_rescue.waiting_for_cdp" => {
            "In attesa della connessione della finestra di accesso di {browser}..."
        }
        "youtube_login_rescue.waiting_for_cookie" => {
            "La finestra di accesso è connessa. In attesa dei cookie del sito web..."
        }
        "youtube_login_rescue.cookie_exported" => "Il Cookie è stato salvato.",
        "youtube_login_rescue.cookie_exported_note" => {
            "Cookie di {site} salvato. I download da quel sito lo useranno automaticamente."
        }
        "youtube_login_rescue.do_not_close_note" => {
            "Tieni aperto il browser di accesso durante questo controllo."
        }
        "youtube_login_rescue.cdp_ready" => "La finestra di accesso è connessa.",
        "youtube_login_rescue.ready_next_step_note" => {
            "Completa l'accesso a YouTube nel browser. L'esportazione dei Cookie sarà aggiunta nel passaggio successivo."
        }
        "youtube_login_rescue.close_login_window" => "Chiudi finestra di accesso",
        "youtube_login_rescue.failed" => "Recupero Cookie non riuscito",
        "youtube_login_rescue.retry" => "Riprova",
        "advance.no_cookies_txt_selected" => "Nessun cookies.txt selezionato",
        "advance.browse" => "Sfoglia",
        "advance.select_netscape_cookies_txt" => "Seleziona cookies.txt Netscape",
        "advance.clear" => "Cancella",
        "advance.browser" => "Browser",
        "advance.default" => "Predefinito",
        "advance.external_downloader" => "Downloader esterno",
        "advance.download_control" => "Controllo download",
        "advance.concurrent_fragments" => "Frammenti simultanei",
        "advance.live_streams" => "Dirette streaming",
        "advance.download_live_streams_from_start_experimental" => {
            "Scarica le dirette dall'inizio (sperimentale)"
        }
        "advance.1_default" => "1 (predefinito)",
        "advance.rate_limit" => "Limite velocità",
        "advance.e_g_2m_800k_leave_empty_for_unlimited" => {
            "es. 2M, 800K; lascia vuoto per illimitato"
        }
        "advance.chapters" => "Capitoli",
        "advance.download_range" => "Intervallo di download",
        "advance.always_show_download_range" => "Mostra sempre la selezione intervallo",
        "advance.chapter_download_compatibility_mode" => "Modalità compatibilità download capitoli",
        "advance.file_time" => "Ora file",
        "advance.file_time.none" => "Non modificare",
        "advance.file_time.upload_date" => "Usa data di caricamento del video",
        "advance.file_time.download_time" => "Usa ora del download",
        "advance.post_processing" => "Post-elaborazione",
        "advance.thumbnail" => "Miniatura",
        "advance.download" => "Scarica",
        "advance.embed" => "Incorpora",
        "advance.subtitles" => "Sottotitoli",
        "advance.download_conversion" => "Converti dopo il download",
        "advance.enable" => "Abilita",
        "advance.settings" => "Impostazioni",
        "item.save_as" => "Salva con nome",
        "item.error" => "Errore",
        "item.all" => "Tutti",
        "item.queued" => "In coda",
        "item.done" => "Completato",
        "item.failed" => "Non riuscito",
        "item.clear_all" => "Cancella tutto",
        "item.add_a_video_url" => "Aggiungi URL video",
        "item.add_an_audio_url" => "Aggiungi URL audio",
        "item.after_adding_choose_the_video_format_here" => "Scegli formato video",
        "item.after_adding_choose_the_audio_format_here" => "Scegli formato audio",
        "item.loading_thumbnail" => "Caricamento miniatura",
        "item.file_actions" => "Azioni file",
        "item.open_file" => "Apri file",
        "item.open_folder" => "Apri cartella",
        "item.copy_path" => "Copia percorso",
        "item.file_not_found_opened_the_output_location" => {
            "File non trovato; aperta la posizione di output."
        }
        "item.opened_output_location" => "Posizione di output aperta.",
        "item.copied_output_path" => "Percorso di output copiato.",
        "prepare.language" => "Lingua",
        "prepare.back" => "Indietro",
        "prepare.auto_detect" => "Rileva automaticamente",
        "prepare.install_the_required_tools_now_or_skip_and_h" => {
            "Installa ora gli strumenti richiesti, oppure salta e gestiscili più tardi in Opzioni."
        }
        "prepare.optional" => "Opzionale",
        "prepare.missing" => "Mancante",
        "prepare.install_later" => "Installa più tardi",
        "prepare.downloading_100" => "Download 100%",
        "prepare.extracting_100" => "Estrazione 100%",
        "prepare.install_failed" => "Installazione fallita",
        "prepare.install_all" => "Installa tutto",
        "prepare.reinstall" => "Reinstalla",
        "prepare.installing" => "Installazione",
        "prepare.skip" => "Salta",
        "prepare.install" => "Installa",
        "prepare.another_tool_is_already_being_installed" => {
            "Un altro strumento è già in installazione."
        }
        "prepare.needs_attention" => "Richiede attenzione",
        "prepare.req.app_folder.title" => "Cartella dell’app",
        "prepare.req.app_folder.description" => {
            "La cartella portabile deve essere scrivibile per salvare impostazioni e dati di supporto."
        }
        "prepare.req.tools_folder.title" => "Cartella strumenti",
        "prepare.req.tools_folder.description" => {
            "La distribuzione delle dipendenze salva qui yt-dlp, FFmpeg e Deno."
        }
        "prepare.req.deployment_temp.title" => "Temporanei di distribuzione",
        "prepare.req.deployment_temp.description" => {
            "L’estrazione di FFmpeg e Deno usa questa cartella temporanea."
        }
        "prepare.req.download_cache.title" => "Cache download",
        "prepare.req.download_cache.description" => {
            "La modalità cache di yt-dlp-gui salva qui la cache di yt-dlp."
        }
        "prepare.req.output_folder.title" => "Cartella di output",
        "prepare.req.output_folder.description" => {
            "Video, audio e sottotitoli vengono salvati qui."
        }
        "prepare.req.output_folder.recommendation" => {
            "Scegli una cartella di output valida dalla schermata principale o dalle Opzioni."
        }
        "prepare.req.config_file.title" => "File di configurazione",
        "prepare.req.config_file.description" => {
            "L’app deve poter salvare lo stato di salto di Prepare e i percorsi degli strumenti."
        }
        "prepare.req.generic_writable_recommendation" => {
            "Scegli una cartella scrivibile e controlla i permessi."
        }
        "prepare.req.config_not_folder" => {
            "Il percorso di configurazione punta a una cartella. Scegli invece un percorso file."
        }
        "prepare.req.config_readonly" => "Il file di configurazione è in sola lettura.",
        "prepare.req.config_readonly_recommendation" => {
            "Consenti la scrittura nel file di configurazione o scegli un’altra cartella dell’app."
        }
        "prepare.req.use_folder_path" => "Scegli un percorso cartella invece di un percorso file.",
        "prepare.req.move_portable_folder" => "Sposta l’app in una cartella portabile scrivibile.",
        "prepare.req.avoid_protected_folder" => {
            "Non mettere l’app portabile in Program Files o nella cartella Windows. Spostala in D:\\Portable o in una cartella utente."
        }
        "prepare.req.move_non_synced_folder" => {
            "Spostala in una cartella non sincronizzata, ad esempio D:\\Portable\\yt-dlp-gui-v2."
        }
        "prepare.req.drive_parent_exists" => {
            "Assicurati che l’unità e la cartella superiore esistano."
        }
        "prepare.req.permission_denied" => {
            "Sposta l’app in una cartella portabile scrivibile. Se Desktop/Documenti/Download falliscono ancora, l’accesso controllato alle cartelle di Defender potrebbe bloccarla."
        }
        "prepare.req.file_in_use" => {
            "Chiudi il programma che potrebbe usare questa cartella oppure scegli un’altra cartella."
        }
        "prepare.req.free_disk_space" => "Libera spazio su disco o scegli un altro disco.",
        "prepare.req.path_too_long" => {
            "Sposta l’app in un percorso più corto, ad esempio D:\\Portable\\yt-dlp-gui-v2."
        }
        "prepare.req.choose_writable_portable_folder" => {
            "Scegli una cartella portabile chiaramente scrivibile e controlla di nuovo."
        }
        "prepare.req.clear_write_test" => {
            "Rimuovi il file di test scrittura rimasto e controlla di nuovo."
        }
        "options.this_url_contains_both_a_video_and_a_playlis" => {
            "Questo URL contiene sia un video sia una playlist"
        }
        "options.detected" => "Rilevato ",
        "options.playlist_prompt" => "Avviso playlist",
        "options.which_one_should_be_loaded" => "Cosa deve essere caricato?",
        "options.both_video_and_playlist_were_detected" => {
            "Sono stati rilevati sia video sia playlist"
        }
        "options.this_playlist_may_contain_many_items" => {
            "Questa playlist può contenere molti elementi."
        }
        "options.playlist_risk.kind.channel_generated" => "Playlist del canale generata da YouTube",
        "options.playlist_risk.kind.youtube_mix_radio" => "YouTube Mix / Radio",
        "options.playlist_risk.kind.youtube_music_album" => "Album/raccolta YouTube Music",
        "options.playlist_risk.kind.liked_videos" => "Video piaciuti",
        "options.playlist_risk.kind.favorites_legacy" => "Vecchia playlist dei preferiti",
        "options.playlist_risk.note.channel_generated" => {
            "Gestisci con cautela questa playlist del canale generata da YouTube."
        }
        "options.playlist_risk.note.youtube_mix_radio" => {
            "Questa playlist Mix / Radio può contenere molti elementi e cambiare nel tempo."
        }
        "options.playlist_risk.note.youtube_music_album" => {
            "Di solito è un album o una raccolta di YouTube Music."
        }
        "options.playlist_risk.note.liked_videos" => {
            "I video piaciuti di solito richiedono accesso o cookie."
        }
        "options.playlist_risk.note.favorites_legacy" => {
            "È un vecchio tipo di playlist dei preferiti e potrebbe non essere più stabile."
        }
        "options.video" => "Video",
        "options.playlist" => "Playlist",
        "options.cancel" => "Annulla",
        "options.load" => "Carica",
        "options.behavior" => "Comportamento",
        "options.format_list" => "Elenco formati",
        "options.unknown_filesize_formats" => "Formati con dimensione sconosciuta",
        "options.unknown_filesize_formats.off" => "Disattivato",
        "options.unknown_filesize_formats.filter" => "Filtra",
        "options.add_action" => "Azione di aggiunta",
        "options.download_directly" => "Scarica direttamente",
        "options.clipboard_change" => "Modifica appunti",
        "options.run_immediately" => "Esegui subito",
        "options.tabs" => "Schede",
        "options.log_tab" => "Scheda registro",
        "options.show_log_tab" => "Mostra registro",
        "options.playlist_2" => "Playlist",
        "options.with_playlist" => "Con playlist",
        "options.ask" => "Chiedi",
        "options.single_video" => "Singolo video",
        "options.full_playlist" => "Playlist completa",
        "options.high_risk_prompt" => "Avviso rischio elevato",
        "options.on" => "Attivo",
        "options.playlist_count" => "Numero playlist",
        "options.limit" => "Limite",
        "options.max" => "Max:",
        "options.items" => " elementi",
        "options.language" => "Lingua",
        "options.current_language" => "Lingua attuale",
        "options.back" => "Indietro",
        "options.choose" => "Scegli",
        "options.auto_detect" => "Rileva automaticamente",
        "options.tool_paths" => "Percorsi strumenti",
        "options.file_actions" => "Azioni file",
        "options.action_button" => "Pulsante azione",
        "options.file_action.show_menu" => "Mostra menu",
        "options.output" => "Download output",
        "options.output.naming" => "File naming",
        "options.output.existing" => "Existing files",
        "options.output.title" => "Title",
        "options.output.title_id" => "Title + video ID",
        "options.output.uploader_title_id" => "Uploader + title + video ID",
        "options.output.custom" => "Custom template",
        "options.output.keep_both" => "Keep both files",
        "options.output.skip" => "Skip existing files",
        "options.output.template" => "Filename template",
        "options.output.template_help" => {
            "Fields: {title}, {id}, {uploader}, {upload_date}. Use {{ and }} for literal braces. Folder paths are not allowed."
        }
        "options.output.preview" => "Filename preview",
        "options.output.preview_help" => {
            "Sample metadata only; no network request. The actual extension and collision suffix are added at download time."
        }
        "options.output.unsaved" => {
            "Unsaved changes. New jobs still use the saved output settings."
        }
        "options.output.save" => "Save output settings",
        "options.output.saved" => "Output settings saved. Applies to new downloads.",
        "options.output.help" => {
            "Applies to new automatic downloads. Existing explicit filenames are kept. Gallery downloads keep their own naming rules. Legacy queued names are kept. Keep both adds (2), (3), etc."
        }
        "options.cache" => "Cache",
        "options.cache_location" => "Posizione cache",
        "options.cache_location.default" => "Predefinito",
        "options.cache_location.portable" => "Cartella dell’applicazione (portatile)",
        "options.cache_location.system_temp" => "Cartella temporanea di sistema",
        "options.cache_current_folder" => "Cartella cache attuale",
        "options.cache_after_restart" => "Dopo il riavvio",
        "options.cache_restart_required" => "Riavvio necessario",
        "options.cache_location_help" => {
            "Le modifiche si applicano dopo il riavvio. La cache precedente rimane al suo posto. Utilizzo e pulizia riguardano la cartella attuale. I dati di stato e ripristino rimangono nella cartella dell’applicazione. Il sistema può eliminare i file temporanei."
        }
        "options.cache_legacy_notice" => {
            "La precedente opzione Predefinito ora usa la cartella dell’applicazione (portatile). Tutte le cache dell’applicazione seguono la posizione selezionata."
        }
        "options.cache_usage" => "Uso",
        "options.cache_usage_detail" => "Totale: {total} · Audio: {audio} · Scaduti: {expired}",
        "options.cache_cleanup" => "Pulizia",
        "options.cache_refresh" => "Aggiorna",
        "options.cache_clear_expired" => "Cancella scaduti",
        "options.cache_clear_audio" => "Cancella audio",
        "options.cache_clear_all" => "Cancella tutto",
        "options.appearance_window" => "Aspetto e finestra",
        "options.notifications" => "Notifiche",
        "options.enable" => "Abilita",
        "options.theme" => "Tema",
        "options.theme_mode.system" => "Segui sistema",
        "options.render_backend" => "Rendering (dopo il riavvio)",
        "options.wgpu_api" => "API grafica (dopo il riavvio)",
        "options.theme_mode.light" => "Chiaro",
        "options.theme_mode.dark" => "Scuro",
        "options.theme_color" => "Colore tema",
        "options.theme_color.off" => "Disattivato",
        "options.theme_color.blue" => "Blu",
        "options.theme_color.soft_blue" => "Azzurro tenue",
        "options.theme_color.purple" => "Viola",
        "options.theme_color.pink" => "Rosa",
        "options.theme_color.green" => "Verde",
        "options.theme_color.orange" => "Arancione",
        "options.theme_color.slate" => "Ardesia",
        "options.ui_scale" => "Scala UI",
        "options.apply" => "Applica",
        "options.current" => "Attuale",
        "options.always_on_top" => "Sempre in primo piano",
        "options.window_position" => "Posizione finestra",
        "options.remember" => "Ricorda",
        "options.window_size" => "Dimensione finestra",
        "options.reinstall" => "Reinstalla",
        "options.installing" => "Installazione",
        "options.install" => "Installa",
        "options.executable" => "eseguibile",
        "main.controlled_by_config" => "Controllato dalla configurazione: ",
        "main.controlled_by_config_2" => "Controllato dalla configurazione",
        "picker.section_tab.chapters" => "Capitoli",
        "picker.section_tab.time_range" => "Intervallo di tempo",
        "picker.section_chapter_instructions" => {
            "Seleziona uno o più capitoli. I capitoli adiacenti diventano un'unica uscita."
        }
        "picker.section_time_instructions" => {
            "Sposta l'indicatore, imposta inizio e fine, quindi aggiungi l'intervallo."
        }
        "picker.section_time_unavailable" => {
            "La durata del video non è disponibile; non è possibile creare un intervallo personalizzato."
        }
        "picker.section_select_all" => "Seleziona tutto",
        "picker.section_from_selected_to_end" => "Dal primo selezionato alla fine",
        "picker.section_set_start" => "Imposta inizio",
        "picker.section_set_end" => "Imposta fine",
        "picker.section_add_range" => "Aggiungi intervallo",
        "picker.section_no_custom_ranges" => "Nessun intervallo di tempo personalizzato.",
        "picker.no_chapters_available" => "Nessun capitolo disponibile.",
        "picker.choose_the_range_to_download_for_this_item_d" => {
            "Scegli l’intervallo da scaricare per questo elemento. Il valore predefinito è l’intero video."
        }
        "picker.chapter_compatibility_mode_is_on_chapter_dow" => {
            "La modalità compatibilità capitoli è attiva: i download dei capitoli useranno un formato a file singolo più stabile."
        }
        "picker.subtitles_will_not_be_downloaded" => "I sottotitoli non verranno scaricati.",
        "picker.no_subtitles_are_available_for_this_video" => {
            "Nessun sottotitolo disponibile per questo video."
        }
        "picker.no_subtitles_are_available_in_this_tab" => {
            "Nessun sottotitolo disponibile in questa scheda."
        }
        "picker.source_language" => "Lingua sorgente",
        "picker.translation_target" => "Destinazione traduzione",
        "picker.tip_youtube_auto_translated_subtitles_are_mo" => {
            "Suggerimento: i sottotitoli YouTube tradotti automaticamente hanno più probabilità di essere limitati rispetto ai sottotitoli originali. Scegli “Nessuna traduzione” se ti serve solo il testo originale."
        }
        "picker.no_subtitles_are_available_for_this_source" => {
            "Nessun sottotitolo disponibile per questa sorgente."
        }
        "picker.target" => "Destinazione",
        "picker.available_subtitles" => "Sottotitoli disponibili",
        "picker.language" => "Lingua",
        "picker.subtitle_tab.none" => "Nessun sottotitolo",
        "picker.subtitle_tab.original" => "Sottotitoli originali",
        "picker.subtitle_tab.automatic" => "Sottotitoli automatici",
        "picker.waiting_analysis" => "In attesa di analisi",
        "picker.audio_from_video" => "Deciso dal formato video",
        "picker.not_selected" => "Non selezionato",
        "picker.full_video" => "Video completo",
        "picker.section_summary.chapters" => "{chapters} capitoli selezionati · {outputs} uscite",
        "picker.section_summary.custom" => "{ranges} intervalli · {outputs} uscite",
        "picker.section_summary.combined" => {
            "{chapters} capitoli + {ranges} intervalli · {outputs} uscite"
        }
        "picker.no_translation" => "Nessuna traduzione",
        "picker.until_end" => "fine",
        "prepare.status.ready" => "Pronto",
        "prepare.status.missing" => "Mancante",
        "prepare.status.warning" => "Richiede attenzione",
        "prepare.status.failed" => "Non riuscito",
        "tool_install.stage.preparing" => "Preparazione",
        "tool_install.stage.downloading" => "Download",
        "tool_install.stage.extracting" => "Estrazione",
        "tool_install.stage.installing" => "Installazione",
        "tool_install.stage.completed" => "Completato",
        "tool_install.stage.failed" => "Non riuscito",
        "item.status.queued" => "In coda",
        "item.status.running" => "In esecuzione",
        "item.status.finished" => "Fatto",
        "item.status.failed" => "Non riuscito",
        "item.status.cancelled" => "Annullato",
        "processing.transcode" => "Transcodifica",
        "transcode.graph.axis.compatibility" => "Compatibilità",
        "transcode.graph.axis.capacity" => "Capacità",
        "transcode.graph.axis.resolution" => "Risoluzione",
        "transcode.graph.axis.format" => "Formato",
        "transcode.graph.compatibility_scope" => "Ambito compatibilità",
        "transcode.graph.capacity_target" => "Obiettivo dimensione",
        "transcode.graph.resolution_limit" => "Limite risoluzione",
        "transcode.graph.format_goal" => "Obiettivo formato",
        "processing.video" => "Video",
        "processing.audio" => "Audio",
        "processing.container" => "Contenitore",
        "processing.subtitle" => "Sottotitoli",
        "processing.choice.source" => "Originale",
        "processing.subtitle.preserve" => "Originale",
        "processing.subtitle.embed" => "Incorpora",
        "processing.subtitle.burn" => "Imprimi",
        "advance.filter_netscape_cookies_txt" => "Netscape cookies.txt",
        "advance.filter_all_files" => "Tutti i file",
        "options.filter_executable" => "Eseguibile",
        "app_mode.origin" => "Modalità Origin",
        "app_mode.standard" => "Modalità standard",
        "app_mode.audio" => "Modalità audio",
        "music.status.completed" => "Fatto",
        "music.status.resolving" => "Risoluzione",
        "music.status.buffering" => "Bufferizzazione",
        "music.status.ready" => "Pronto",
        "music.status.caching" => "Cache in corso",
        "music.status.playing" => "Riproduzione",
        "music.status.paused" => "In pausa",
        "music.status.failed" => "Non riuscito",
        "notification.download_complete" => "Download completato",
        "notification.download_failed" => "Download non riuscito",
        "notification.completed_file" => "Completato: {file}",
        "notification.download_completed" => "Download completato.",
        "notification.no_chapters_saved_full_audio" => {
            "Nessun capitolo rilevato. È stato salvato l’audio completo."
        }
        "options.music_download_format" => "Music download format",
        "options.music_download_audio_label" => "Uscita audio",
        "options.music_download_preference_best" => "Migliore",
        "options.music_download_save_label" => "Modalità di salvataggio",
        "options.music_download_full_audio" => "Audio completo",
        "options.music_download_split_chapters" => "Dividi per capitoli",
        _ => key,
    }
}
