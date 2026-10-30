use std::path::PathBuf;

use licht_core::{
    CoreError, GlobalLaunch, Settings, SharedCache, apply_global_launch, configured_java,
    download_concurrency, game_cache, load_settings, memory_slider_max, recommended_max_memory_mb,
    save_settings, total_memory_mb,
};

fn cache(name: &str) -> SharedCache {
    let root = std::env::temp_dir().join(format!("licht-settings-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    SharedCache::at(root)
}

fn settings_file(cache: &SharedCache) -> PathBuf {
    cache.root().join("settings.json")
}

fn sample(download_concurrency: u32) -> Settings {
    Settings {
        schema: 1,
        theme: licht_core::LauncherTheme::Dark,
        username: "Steve".to_string(),
        fullscreen: false,
        width: None,
        height: None,
        max_memory_mb: 2048,
        jvm_arguments: Vec::new(),
        java25: None,
        java21: None,
        java17: None,
        java8: None,
        download_concurrency,
        write_concurrency: 8,
        download_speed_limit_bytes_per_second: None,
        data_directory: None,
        disable_custom_title_bar: false,
        automatic_updates: true,
        language: licht_core::LauncherLanguage::English,
        hardware_acceleration: true,
        minimize_to_tray: true,
        minimize_on_game_start: false,
        restore_on_game_exit: false,
    }
}

#[test]
fn automatic_updates_migrate_without_enabling_previously_disabled_downloads() {
    for (index, (check, install, expected)) in [
        (true, true, true),
        (true, false, false),
        (false, true, false),
        (false, false, false),
    ]
    .into_iter()
    .enumerate()
    {
        let cache = cache(&format!("old-update-preferences-{index}"));
        std::fs::create_dir_all(cache.root()).expect("root");
        std::fs::write(settings_file(&cache), serde_json::to_vec(&serde_json::json!({"schema":1,"checkForUpdates":check,"installUpdatesOnRestart":install})).expect("json")).expect("legacy settings");
        let loaded = load_settings(&cache).expect("migrate preferences");
        assert_eq!(loaded.automatic_updates, expected);
        save_settings(&cache, &loaded).expect("save new preference");
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(settings_file(&cache)).expect("saved file"))
                .expect("json");
        assert_eq!(saved["automaticUpdates"], expected);
        assert!(saved.get("checkForUpdates").is_none());
        assert!(saved.get("installUpdatesOnRestart").is_none());
    }
}

#[test]
fn recommended_memory_is_half_the_ram_inside_the_bounds() {
    assert_eq!(recommended_max_memory_mb(8192), 4096);
    assert_eq!(recommended_max_memory_mb(3000), 1280);
    assert_eq!(recommended_max_memory_mb(1024), 1024);
    assert_eq!(memory_slider_max(100_000), 100_000);
    assert_eq!(memory_slider_max(1000), 1000);
    assert!(total_memory_mb() >= 512);
}

#[test]
fn a_missing_file_returns_defaults_and_is_not_created() {
    let cache = cache("missing");

    let settings = load_settings(&cache).expect("default");

    assert_eq!(settings.schema, 1);
    assert!(settings.automatic_updates);
    assert_eq!(settings.language, licht_core::LauncherLanguage::English);
    assert!(settings.hardware_acceleration);
    assert!(settings.minimize_to_tray);
    assert!(!settings.minimize_on_game_start);
    assert!(!settings.restore_on_game_exit);
    assert_eq!(settings.theme, licht_core::LauncherTheme::Dark);
    assert!(!settings.fullscreen);
    assert_eq!(settings.width, Some(854));
    assert_eq!(settings.height, Some(480));
    assert_eq!(settings.download_concurrency, 8);
    assert_eq!(settings.write_concurrency, 8);
    assert_eq!(
        settings.max_memory_mb,
        recommended_max_memory_mb(total_memory_mb())
    );
    assert!(settings.data_directory.is_none());
    assert!(!cache.root().exists());
}

#[test]
fn a_saved_value_roundtrips() {
    let cache = cache("roundtrip");

    let saved = save_settings(&cache, &sample(4)).expect("save");
    let loaded = load_settings(&cache).expect("load");

    assert_eq!(saved.schema, 1);
    assert_eq!(saved.download_concurrency, 4);
    assert_eq!(loaded, saved);
    assert!(settings_file(&cache).is_file());
}

#[test]
fn general_preferences_roundtrip_and_legacy_files_use_defaults() {
    let cache = cache("general-preferences");
    std::fs::create_dir_all(cache.root()).expect("root");
    std::fs::write(settings_file(&cache), r#"{"schema":1}"#).expect("legacy settings");
    let mut settings = load_settings(&cache).expect("legacy defaults");
    assert!(settings.hardware_acceleration);
    assert!(settings.minimize_to_tray);
    assert!(!settings.minimize_on_game_start);
    assert!(!settings.restore_on_game_exit);
    settings.hardware_acceleration = false;
    settings.minimize_to_tray = false;
    settings.minimize_on_game_start = true;
    settings.restore_on_game_exit = true;
    save_settings(&cache, &settings).expect("save preferences");
    assert_eq!(load_settings(&cache).expect("reload"), settings);
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(settings_file(&cache)).expect("settings"))
            .expect("json");
    assert_eq!(saved["language"], "en");
    assert_eq!(saved["minimizeToTray"], false);
}

#[test]
fn unsupported_languages_are_rejected_at_the_settings_boundary() {
    let cache = cache("unsupported-language");
    std::fs::create_dir_all(cache.root()).expect("root");
    std::fs::write(settings_file(&cache), r#"{"schema":1,"language":"pt-BR"}"#).expect("settings");
    assert!(matches!(load_settings(&cache), Err(CoreError::Settings(_))));
}

#[test]
fn legacy_mib_limits_are_loaded_as_exact_bytes_per_second() {
    let cache = cache("legacy-speed-limit");
    std::fs::create_dir_all(cache.root()).expect("root");
    std::fs::write(
        settings_file(&cache),
        r#"{"schema":1,"downloadSpeedLimitMiB":3}"#,
    )
    .expect("legacy settings");
    let loaded = load_settings(&cache).expect("load legacy limit");
    let serialized = serde_json::to_value(&loaded).expect("settings json");
    assert_eq!(
        serialized["downloadSpeedLimitBytesPerSecond"],
        3 * 1024 * 1024
    );
    assert!(serialized.get("downloadSpeedLimitMiB").is_none());
    save_settings(&cache, &loaded).expect("save migrated settings");
    assert_eq!(
        load_settings(&cache).expect("reload migrated limit"),
        loaded
    );
}

#[test]
fn byte_limits_take_precedence_and_remain_safe_for_the_frontend() {
    let cache = cache("byte-speed-limit");
    std::fs::create_dir_all(cache.root()).expect("root");
    std::fs::write(
        settings_file(&cache),
        r#"{"schema":1,"downloadSpeedLimitMiB":3,"downloadSpeedLimitBytesPerSecond":1000}"#,
    )
    .expect("settings");
    assert_eq!(
        load_settings(&cache)
            .expect("byte limit")
            .download_speed_limit_bytes_per_second,
        Some(1000)
    );
    let mut settings = sample(4);
    settings.download_speed_limit_bytes_per_second = Some(9_007_199_254_740_992);
    assert!(matches!(
        save_settings(&cache, &settings),
        Err(CoreError::SettingsDownloadSpeed)
    ));
    assert_eq!(
        load_settings(&cache)
            .expect("previous limit")
            .download_speed_limit_bytes_per_second,
        Some(1000)
    );
}

#[test]
fn global_speed_limit_roundtrips_and_rejects_zero() {
    let cache = cache("speed-limit");
    let mut settings = sample(4);
    assert_eq!(
        load_settings(&cache)
            .expect("defaults")
            .download_speed_limit_bytes_per_second,
        None
    );
    settings.download_speed_limit_bytes_per_second = Some(3000);
    save_settings(&cache, &settings).expect("save speed limit");
    assert_eq!(
        load_settings(&cache)
            .expect("load speed limit")
            .download_speed_limit_bytes_per_second,
        Some(3000)
    );
    settings.download_speed_limit_bytes_per_second = Some(0);
    assert!(matches!(
        save_settings(&cache, &settings),
        Err(CoreError::SettingsDownloadSpeed)
    ));
    assert_eq!(
        load_settings(&cache)
            .expect("previous valid value")
            .download_speed_limit_bytes_per_second,
        Some(3000)
    );
    settings.download_speed_limit_bytes_per_second = None;
    save_settings(&cache, &settings).expect("remove limit");
}

#[test]
fn an_old_file_keeps_concurrency_and_fills_the_new_fields() {
    let cache = cache("old");
    let path = settings_file(&cache);
    std::fs::create_dir_all(cache.root()).expect("root");
    let original = "{\"schema\":1,\"downloadConcurrency\":4}\n";
    std::fs::write(&path, original).expect("write");

    let loaded = load_settings(&cache).expect("load");

    assert_eq!(loaded.download_concurrency, 4);
    assert_eq!(loaded.write_concurrency, 8);
    assert_eq!(loaded.theme, licht_core::LauncherTheme::Dark);
    assert!(!loaded.fullscreen);
    assert_eq!(loaded.width, Some(854));
    assert_eq!(loaded.height, Some(480));
    assert!(loaded.java21.is_none());
    assert_eq!(
        loaded.max_memory_mb,
        recommended_max_memory_mb(total_memory_mb())
    );
    assert_eq!(std::fs::read_to_string(&path).expect("unchanged"), original);
}

#[test]
fn an_explicit_null_window_stays_empty() {
    let cache = cache("null-window");
    let path = settings_file(&cache);
    std::fs::create_dir_all(cache.root()).expect("root");
    std::fs::write(
        &path,
        "{\"schema\":1,\"downloadConcurrency\":8,\"width\":null,\"height\":null}\n",
    )
    .expect("write");

    let loaded = load_settings(&cache).expect("load");

    assert!(loaded.width.is_none());
    assert!(loaded.height.is_none());
}

#[test]
fn legacy_transparency_preferences_are_ignored_and_removed_on_save() {
    let cache = cache("legacy-transparency");
    std::fs::create_dir_all(cache.root()).expect("root");
    for disabled in [true, false] {
        let legacy = serde_json::json!({"schema": 1, "disableTransparency": disabled});
        std::fs::write(
            settings_file(&cache),
            serde_json::to_vec(&legacy).expect("json"),
        )
        .expect("legacy settings");
        let settings = load_settings(&cache).expect("load legacy settings");
        assert_eq!(settings.theme, licht_core::LauncherTheme::Dark);
        save_settings(&cache, &settings).expect("save");
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(settings_file(&cache)).expect("saved file"))
                .expect("json");
        assert!(saved.get("disableTransparency").is_none());
        assert_eq!(load_settings(&cache).expect("reload"), settings);
    }
}

#[test]
fn values_outside_one_to_sixteen_are_rejected_without_writing() {
    let cache = cache("range");
    let path = settings_file(&cache);

    let low = save_settings(&cache, &sample(0)).expect_err("zero");
    assert!(matches!(low, CoreError::SettingsConcurrency));
    assert!(!path.exists());

    save_settings(&cache, &sample(8)).expect("valid");
    let before = std::fs::read_to_string(&path).expect("file");
    let high = save_settings(&cache, &sample(17)).expect_err("seventeen");
    assert!(matches!(high, CoreError::SettingsConcurrency));
    assert_eq!(std::fs::read_to_string(&path).expect("unchanged"), before);
}

#[test]
fn write_concurrency_outside_one_to_sixteen_is_rejected() {
    let cache = cache("write-range");
    let mut low = sample(8);
    low.write_concurrency = 0;
    assert!(matches!(
        save_settings(&cache, &low).expect_err("zero"),
        CoreError::SettingsWriteConcurrency
    ));

    let mut high = sample(8);
    high.write_concurrency = 17;
    assert!(matches!(
        save_settings(&cache, &high).expect_err("seventeen"),
        CoreError::SettingsWriteConcurrency
    ));
    assert!(!settings_file(&cache).exists());
}

#[test]
fn unsupported_themes_and_bad_memory_are_rejected() {
    let cache = cache("theme");
    let mut settings = serde_json::to_value(sample(8)).expect("settings");
    settings["theme"] = "unknown".into();
    assert!(serde_json::from_value::<Settings>(settings).is_err());

    let mut memory = sample(8);
    memory.max_memory_mb = 511;
    assert!(matches!(
        save_settings(&cache, &memory).expect_err("memory"),
        CoreError::SettingsMemory
    ));
    assert!(!settings_file(&cache).exists());
}

#[test]
fn exact_memory_values_survive_save_and_relocated_settings_fit_the_current_machine() {
    let cache = cache("exact-memory");
    let mut settings = sample(8);
    settings.max_memory_mb = 513;
    save_settings(&cache, &settings).expect("save exact memory");
    assert_eq!(load_settings(&cache).expect("reload").max_memory_mb, 513);
    let mut stored = serde_json::to_value(settings).expect("json");
    stored["maxMemoryMb"] = u32::MAX.into();
    std::fs::write(
        settings_file(&cache),
        serde_json::to_vec(&stored).expect("json"),
    )
    .expect("settings from another machine");
    assert_eq!(
        load_settings(&cache)
            .expect("adapt to current RAM")
            .max_memory_mb,
        memory_slider_max(total_memory_mb())
    );
}

#[test]
fn all_supported_themes_survive_save_and_reload() {
    let cache = cache("theme-roundtrip");
    for theme in [
        licht_core::LauncherTheme::Light,
        licht_core::LauncherTheme::Dark,
        licht_core::LauncherTheme::System,
    ] {
        let mut settings = sample(8);
        settings.theme = theme;
        save_settings(&cache, &settings).expect("save theme");
        assert_eq!(load_settings(&cache).expect("reload theme").theme, theme);
    }
}

#[test]
fn a_relative_directory_is_rejected_and_the_default_launcher_root_is_stored_as_empty() {
    let cache = cache("directory");
    let mut relative = sample(8);
    relative.data_directory = Some("games".to_string());
    assert!(matches!(
        save_settings(&cache, &relative).expect_err("relative"),
        CoreError::SettingsDirectory
    ));

    let parent = cache.root().display().to_string();
    let mut defaults = sample(8);
    defaults.data_directory = Some(parent);
    let saved = save_settings(&cache, &defaults).expect("default parent");
    assert!(saved.data_directory.is_none());
}

#[test]
fn a_custom_directory_is_remembered_without_moving_files() {
    let home = cache("home");
    let chosen = std::env::temp_dir().join(format!("licht-data-dir-{}", std::process::id()));
    let mut settings = sample(4);
    settings.data_directory = Some(chosen.display().to_string());

    save_settings(&home, &settings).expect("save");
    let game = game_cache(&home).expect("game cache");

    assert_eq!(game.root(), chosen.join("Licht Launcher").join("data"));
    assert!(settings_file(&home).is_file());
    assert!(!chosen.join("Licht Launcher").join("settings.json").exists());
    assert_eq!(download_concurrency(&game, &home).expect("concurrency"), 4);
}

#[test]
fn a_configured_java_is_used_only_when_the_file_exists() {
    let cache = cache("java");
    std::fs::create_dir_all(cache.root()).expect("root");
    let executable = cache.root().join("java");
    std::fs::write(&executable, b"java").expect("file");
    let mut settings = sample(8);
    settings.java21 = Some(executable.display().to_string());

    assert_eq!(configured_java(&settings, 21).expect("java 21"), executable);
    assert!(configured_java(&settings, 17).is_none());

    settings.java21 = Some(cache.root().join("missing").display().to_string());
    assert!(configured_java(&settings, 21).is_none());
}

#[test]
fn global_launch_inserts_memory_before_the_main_class() {
    let main_class = "net.minecraft.client.main.Main";
    let mut command = vec![
        "java".to_string(),
        "-cp".to_string(),
        "app.jar".to_string(),
        main_class.to_string(),
        "--username".to_string(),
        "Steve".to_string(),
    ];
    apply_global_launch(
        &mut command,
        main_class,
        &GlobalLaunch {
            max_memory_mb: 2048,
            jvm_arguments: vec!["-XX:+UseG1GC".to_string()],
            fullscreen: false,
            width: Some(1280),
            height: Some(720),
        },
    );

    let main = command
        .iter()
        .position(|part| part == main_class)
        .expect("main");
    assert_eq!(command[main - 3], "-Xms512M");
    assert_eq!(command[main - 2], "-Xmx2048M");
    assert_eq!(command[main - 1], "-XX:+UseG1GC");
    assert!(command.windows(2).any(|pair| pair == ["--width", "1280"]));
    assert!(command.windows(2).any(|pair| pair == ["--height", "720"]));
    assert!(!command.iter().any(|part| part == "--fullscreen"));

    let mut fullscreen = vec!["java".to_string(), main_class.to_string()];
    apply_global_launch(
        &mut fullscreen,
        main_class,
        &GlobalLaunch {
            max_memory_mb: 2048,
            jvm_arguments: Vec::new(),
            fullscreen: true,
            width: Some(1280),
            height: Some(720),
        },
    );
    assert!(fullscreen.iter().any(|part| part == "--fullscreen"));
    assert!(!fullscreen.iter().any(|part| part == "--width"));
    assert!(!fullscreen.iter().any(|part| part == "--height"));
}

#[test]
fn an_unknown_schema_is_left_in_place() {
    let cache = cache("schema");
    let path = settings_file(&cache);
    std::fs::create_dir_all(cache.root()).expect("root");
    let original = "{\"schema\":2,\"downloadConcurrency\":8}\n";
    std::fs::write(&path, original).expect("write");

    let loaded = load_settings(&cache).expect_err("schema");
    let saved = save_settings(&cache, &sample(4)).expect_err("save");

    assert!(matches!(loaded, CoreError::SettingsSchema { schema: 2 }));
    assert!(matches!(saved, CoreError::SettingsSchema { schema: 2 }));
    assert_eq!(std::fs::read_to_string(&path).expect("unchanged"), original);
}

#[test]
fn invalid_json_is_left_in_place() {
    let cache = cache("invalid");
    let path = settings_file(&cache);
    std::fs::create_dir_all(cache.root()).expect("root");
    let original = "{ not json";
    std::fs::write(&path, original).expect("write");

    let loaded = load_settings(&cache).expect_err("json");
    let saved = save_settings(&cache, &sample(4)).expect_err("save");

    assert!(matches!(loaded, CoreError::Settings(_)));
    assert!(matches!(saved, CoreError::Settings(_)));
    assert_eq!(std::fs::read_to_string(&path).expect("unchanged"), original);
}
