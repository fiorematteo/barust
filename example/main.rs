mod elite;
mod qtile;

use crate::qtile::QtileStatusProvider;
use barust::{
    statusbar::StatusBar,
    utils::{Color, Position},
    widgets::*,
    xdg_data, Result,
};
use elite::Titans;
use log::LevelFilter;
use std::{env, time::Duration};

const PURPLE: Color = Color::new(0.8, 0.0, 1.0, 1.0);
const BLANK: Color = Color::new(0.0, 0.0, 0.0, 0.0);

#[tokio::main]
async fn main() -> Result<()> {
    setup_logger();
    // #[cfg(debug_assertions)]
    // console_subscriber::init();

    let wd_config = WidgetConfig {
        font: "DejaVuSansM Nerd Font Propo".to_owned(),
        font_size: 17.0,
        hide_timeout: Duration::from_secs(5),
        ..WidgetConfig::default()
    };

    let widgets: Vec<Box<dyn Widget>> = vec![
        Spacer::new(20).await,
        Workspaces::new(
            PURPLE,
            10,
            &WidgetConfig {
                padding: 0,
                ..wd_config.clone()
            },
            WorkspaceFilter,
            QtileStatusProvider::new().await?,
        )
        .await,
        ActiveWindow::new(&WidgetConfig {
            flex: true,
            ..wd_config.clone()
        })
        .await?,
        Systray::new(
            10,
            &WidgetConfig {
                padding: 0,
                ..wd_config.clone()
            },
        )
        .await?,
        // Weather::new(
        //     &"%city %icon %cur (%min/%max)",
        //     MeteoIcons::default(),
        //     &wd_config,
        //     OpenMeteoProvider::new(),
        // )
        // .await,
        // Icon::new("test.svg", 21, &wd_config)?,
        Mail::new(
            "(fiorematteo2002) %c 📧",
            GmailLogin::new("fiorematteo2002@gmail.com", "client_secret.json"),
            None,
            None,
            &wd_config,
        )
        .await?,
        Mail::new(
            "(m.fiorina1) %c 📧",
            GmailLogin::new("m.fiorina1@campus.unimib.it", "client_secret.json"),
            None,
            None,
            &wd_config,
        )
        .await?,
        // Icon::new(xdg_config()?.join("interceptor.png"), 21, &wd_config)?,
        Titans::new(&wd_config).await,
        Disk::new("💾 %f", "/", &wd_config).await,
        Wlan::new("📡 %e", "wlp1s0".to_string(), &wd_config).await,
        Cpu::new("💻 %p󱉸", &wd_config).await?,
        Battery::new("%i %c󱉸", None, &wd_config, NotifySend::default()).await?,
        Volume::new(
            "%i %p",
            Box::new(PulseaudioProvider::new().await.unwrap()),
            None,
            &wd_config,
        )
        .await,
        Brightness::new("%i %p󱉸", None, None, &wd_config).await?,
        Clock::new("🕓 %H:%M %d/%m/%Y", &wd_config).await,
    ];
    StatusBar::create()
        .height(25)
        .position(Position::Top)
        .background(BLANK)
        .widgets(widgets)
        .build()
        .await?
        .start()
        .await
}

#[derive(Debug)]
struct WorkspaceFilter;

impl WorkspaceHider for WorkspaceFilter {
    fn should_hide(&self, workspace: &str, status: &WorkspaceStatus) -> bool {
        if ["scratchpad", "pulsemixer"].contains(&workspace) {
            return true;
        }
        !matches!(status, WorkspaceStatus::Active | WorkspaceStatus::Used)
    }
}

fn setup_logger() {
    let args = env::args().collect::<Vec<_>>();

    let mut level = LevelFilter::Info;
    for arg in &args {
        level = match arg.as_str() {
            "--trace" => LevelFilter::Trace,
            "--debug" => LevelFilter::Debug,
            "--info" => LevelFilter::Info,
            "--warn" => LevelFilter::Warn,
            "--error" => LevelFilter::Error,
            _ => continue,
        }
    }

    let handle = log2::open(xdg_data().unwrap().join("log.txt").to_str().unwrap())
        .level(level)
        .tee(args.contains(&String::from("--stderr")))
        .module_filter(|module| module.contains("barust"))
        .start();
    // dropping handle stops the logger
    std::mem::forget(handle);
}
