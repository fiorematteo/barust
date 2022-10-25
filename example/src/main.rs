use barust::{
    corex::{Color, HookSender, TimedHooks},
    error::{Erc, Result},
    statusbar::{Position, StatusBar},
    widgets::{
        ActiveWindow, Battery, Clock, Cpu, Disk, Spacer, Systray, Volume, Widget, WidgetConfig,
        WidgetError, Wlan, Workspace,
    },
};
use log::LevelFilter;
use std::fmt::Display;

const _WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
const _BLACK: Color = Color::new(0.0, 0.0, 0.0, 1.0);
const _GREEN: Color = Color::new(0.0, 1.0, 0.0, 1.0);
const _RED: Color = Color::new(1.0, 0.2, 0.2, 1.0);
const PURPLE: Color = Color::new(0.8, 0.0, 1.0, 1.0);
const BLANK: Color = Color::new(0.0, 0.0, 0.0, 0.0);

fn main() -> Result<()> {
    let level = LevelFilter::Info;
    simple_logging::log_to_file("/home/matteo/.local/share/barust.log", level)?;

    log_panics::Config::new()
        .backtrace_mode(log_panics::BacktraceMode::Resolved)
        .install_panic_hook();

    let wd_config = WidgetConfig {
        font: "DejaVu Sans Mono",
        font_size: 16.0,
        ..WidgetConfig::default()
    };

    let mut bar = StatusBar::create()
        .position(Position::Top)
        .background(BLANK)
        .left_widgets(vec![
            Spacer::new(20.0),
            FilteredWorkspace::new::<&str>(
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
                        std::process::Command::new("pulsemixer")
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
                        std::process::Command::new("pulsemixer")
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
                Some(&|| {}),
            ),
            Clock::new("🕓 %H:%M %d/%m/%Y", &wd_config, None),
        ])
        .build()?;
    bar.start()
}

#[derive(Debug)]
struct FilteredWorkspace {
    inner: Workspace,
    ignored_workspaces: Vec<String>,
}

impl FilteredWorkspace {
    fn new<T: ToString>(
        active_workspace_color: Color,
        internal_padding: f64,
        config: &WidgetConfig,
        ignored_workspaces: &[T],
    ) -> Box<Self> {
        let inner = *Workspace::new(active_workspace_color, internal_padding, config, None);
        Box::new(Self {
            inner,
            ignored_workspaces: ignored_workspaces.iter().map(|w| w.to_string()).collect(),
        })
    }
}

impl Widget for FilteredWorkspace {
    fn draw(
        &self,
        context: &cairo::Context,
        rectangle: &cairo::Rectangle,
    ) -> barust::widgets::Result<()> {
        self.inner.draw(context, rectangle)
    }

    fn size(&self, context: &cairo::Context) -> barust::widgets::Result<f64> {
        self.inner.size(context)
    }

    fn padding(&self) -> f64 {
        self.inner.padding()
    }

    fn update(&mut self) -> barust::widgets::Result<()> {
        self.inner.update().map_err(FilteredWorkspaceError::from)?;

        if self.ignored_workspaces.is_empty() {
            return Err(FilteredWorkspaceError::EmptyFilter.into());
        }

        let mut i = 0;
        while i < self.inner.workspaces.len() {
            let (ref name, _) = self.inner.workspaces[i];
            if self.ignored_workspaces.contains(name) {
                self.inner.workspaces.remove(i);
            } else {
                i += 1;
            }
        }
        Ok(())
    }

    fn hook(&mut self, sender: HookSender, pool: &mut TimedHooks) -> barust::widgets::Result<()> {
        self.inner.hook(sender, pool)
    }
}

impl Display for FilteredWorkspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FilteredWorkspace")
    }
}

#[derive(Debug, derive_more::Display, derive_more::From, derive_more::Error)]
enum FilteredWorkspaceError {
    EmptyFilter,
    Inner(WidgetError),
}

impl From<FilteredWorkspaceError> for WidgetError {
    fn from(v: FilteredWorkspaceError) -> Self {
        Self::CustomWidget(Erc::new(v))
    }
}
