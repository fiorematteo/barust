use barust::{
    corex::Color,
    error::Result,
    statusbar::{Position, StatusBar},
    widgets::{
        ActiveWindow, Battery, Brightness, Clock, Cpu, Disk, FilteredWorkspaces, Spacer, Systray,
        Volume, WidgetConfig, Wlan,
    },
};
use log::LevelFilter;
use std::env;
use std::{fs::OpenOptions, process::Command, time::Duration};

const PURPLE: Color = Color::new(0.8, 0.0, 1.0, 1.0);
const BLANK: Color = Color::new(0.0, 0.0, 0.0, 0.0);

fn main() -> Result<()> {
    setup_logger();

    let wd_config = WidgetConfig {
        font: "DejaVu Sans Mono",
        font_size: 16.0,
        hide_timeout: Duration::from_secs(5),
        ..WidgetConfig::default()
    };

    let bar = StatusBar::create()
        .position(Position::Top)
        .background(BLANK)
        .left_widgets(vec![
            Spacer::new(20.0),
            FilteredWorkspaces::new::<&str>(
                PURPLE,
                10.0,
                &WidgetConfig {
                    padding: 0.0,
                    ..wd_config
                },
                &["scratchpad", "pulsemixer"],
            ),
            ActiveWindow::new(&wd_config, None)?,
        ])
        .right_widgets(vec![
            Systray::new(20.0, &wd_config)?,
            Disk::new("💾 %f", "/", &wd_config, None),
            Wlan::new("📡 %e", "wlp1s0".to_string(), &wd_config, Some(&|| {})),
            Cpu::new("💻 %p%", &wd_config, None)?,
            Battery::new("%i %c%", None, &wd_config, None)?,
            Volume::new(
                "%i %p",
                &|()| -> Option<f64> {
                    let out = String::from_utf8(
                        Command::new("pulsemixer")
                            .arg("--get-volume")
                            .output()
                            .ok()?
                            .stdout,
                    )
                    .ok()?;
                    let out = out.split(' ').collect::<Vec<_>>();
                    out.first()?.parse::<f64>().ok()
                },
                &|()| -> Option<bool> {
                    String::from_utf8(
                        Command::new("pulsemixer")
                            .arg("--get-mute")
                            .output()
                            .ok()?
                            .stdout,
                    )
                    .ok()
                    .map(|out| out == *"1\n")
                },
                None,
                &wd_config,
                None,
            ),
            Brightness::new(
                "%b%",
                &|()| -> Option<i32> {
                    String::from_utf8(Command::new("light").output().ok()?.stdout)
                        .ok()?
                        .strip_suffix('\n')?
                        .parse::<f64>()
                        .ok()
                        .map(|n| n as _)
                },
                &wd_config,
                None,
            ),
            Clock::new("🕓 %H:%M %d/%m/%Y", &wd_config, None),
        ])
        .build()?;
    bar.start()
}

fn setup_logger() {
    let args = env::args().collect::<Vec<_>>();

    let level = args
        .iter()
        .map(|s| {
            let s: &str = s;
            match s {
                "--trace" => LevelFilter::Trace,
                "--debug" => LevelFilter::Debug,
                "--info" => LevelFilter::Info,
                "--warn" => LevelFilter::Warn,
                "--error" => LevelFilter::Error,
                _ => LevelFilter::Error,
            }
        })
        .max()
        .unwrap_or(LevelFilter::Warn);

    if args.contains(&String::from("--stderr")) {
        simple_logging::log_to_stderr(level);
    } else {
        simple_logging::log_to(
            OpenOptions::new()
                .append(true)
                .open("/home/matteo/.local/share/barust.log")
                .unwrap(),
            level,
        );
        log_panics::Config::new()
            .backtrace_mode(log_panics::BacktraceMode::Resolved)
            .install_panic_hook();
    }
}
