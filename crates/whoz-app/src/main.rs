use mimalloc::MiMalloc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tracing::info;
use whoz_bridge::{ChromiumBackend, ChromiumFlagsBuilder, EngineConfig, EngineHandle};
use whoz_core::{Tab, TabId, TabSuspensionPolicy, WhozConfig, MemoryGovernor, ProcessPool};
use whoz_extensions::ExtensionManager;
use whoz_sync::{BookmarkItem, GoogleToken, NigoriCryptor, SyncClient};
use whoz_ui::BrowserWindowShell;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("===========================================================");
    info!("   WHOZ BROWSER: Ultra-Lightweight Chromium Engine in Rust ");
    info!("   Target: 85% Less RAM, Instant Boot, 100% Chrome Parity  ");
    info!("===========================================================");

    let config = WhozConfig::default();

    // 1. Initialize Memory Governor and Bounded Process Pool
    let suspension_policy = TabSuspensionPolicy::default();
    let governor = MemoryGovernor::new(suspension_policy);
    let mut process_pool = ProcessPool::new(config.max_renderer_processes);

    // 2. Build Stripped Flags for Chromium Base (disables telemetry, AI guides, WebUI bloat)
    let engine_flags = ChromiumFlagsBuilder::new()
        .with_user_data_dir(config.user_data_dir.to_str().unwrap_or("./data"))
        .build();

    let engine_config = EngineConfig {
        base_binary_dir: config
            .chrome_base_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(r"C:\Program Files\Google\Chrome\Application")),
        user_data_dir: config.user_data_dir.clone(),
        flags: engine_flags,
    };

    let mut engine = EngineHandle::initialize(ChromiumBackend::EmbeddedContentApi, engine_config)?;

    // 3. Initialize Google Sync Subsystem in Rust
    let mut sync_client = SyncClient::new();
    let salt = b"whoz_browser_sync_salt_2026";
    let cryptor = NigoriCryptor::from_passphrase("secure_master_passphrase", salt);
    sync_client.set_cryptor(cryptor);

    // Add sample bookmark entity
    sync_client.add_local_bookmark(BookmarkItem {
        id: "bm_google".to_string(),
        parent_id: "bm_root".to_string(),
        title: "Google Search".to_string(),
        url: "https://www.google.com".to_string(),
        is_folder: false,
        position_in_parent: 0,
    });

    let mock_token = GoogleToken {
        access_token: "ya29.mock_oauth_token_whoz_browser".to_string(),
        refresh_token: Some("1//mock_refresh_token".to_string()),
        expires_at: Instant::now() + std::time::Duration::from_secs(3600),
        email: Some("afrizalyogi@aryatama.co.id".to_string()),
    };

    sync_client.sync_cycle(&mock_token).await?;

    // 4. Initialize Extension Manager
    let extension_manager = ExtensionManager::new();
    info!(
        extension_count = extension_manager.count(),
        "Extension subsystem active (Manifest V3 + V2)"
    );

    // 5. Native Rust Window Shell
    let mut ui_shell = BrowserWindowShell::new("Whoz Browser", 1280, 800);

    // 6. Simulate Tab Management & Memory Governor in action
    let mut tabs: HashMap<TabId, Tab> = HashMap::new();

    // Tab 1: Active
    let t1_id = 1;
    let mut t1 = Tab::new(t1_id, "https://www.google.com".to_string());
    t1.title = "Google Search".to_string();
    let _p1 = process_pool.assign_process_for_tab(t1_id);
    engine.navigate_tab(t1_id, &t1.url);
    tabs.insert(t1_id, t1);
    ui_shell.set_active_tab(t1_id, "https://www.google.com".to_string());

    // Tab 2: Simulating an idle tab that aged 90 seconds
    let t2_id = 2;
    let mut t2 = Tab::new(t2_id, "https://news.ycombinator.com".to_string());
    t2.title = "Hacker News".to_string();
    t2.last_active = Instant::now() - std::time::Duration::from_secs(90);
    process_pool.assign_process_for_tab(t2_id);
    tabs.insert(t2_id, t2);

    // Tab 3: Another idle tab
    let t3_id = 3;
    let mut t3 = Tab::new(t3_id, "https://github.com".to_string());
    t3.title = "GitHub".to_string();
    t3.last_active = Instant::now() - std::time::Duration::from_secs(120);
    process_pool.assign_process_for_tab(t3_id);
    tabs.insert(t3_id, t3);

    // Run Memory Governor pass
    let stats = governor.evaluate_and_reclaim(&mut tabs, Some(t1_id));

    info!("-----------------------------------------------------------");
    info!("               MEMORY GOVERNOR REPORT                      ");
    info!("-----------------------------------------------------------");
    info!("Total tabs created      : {}", tabs.len());
    info!("Active tabs             : {}", stats.active_tabs_count);
    info!("Suspended idle tabs     : {}", stats.suspended_tabs_count);
    info!("Saved Memory            : {} MB", stats.saved_memory_kb / 1024);
    info!("Current Estimated RAM   : {} MB", stats.total_allocated_kb / 1024);
    info!("(vs Official Chrome 3 tabs baseline: ~650 - 900 MB)");
    info!("-----------------------------------------------------------");
    info!("Whoz Browser successfully initialized and ready to serve!");

    Ok(())
}
