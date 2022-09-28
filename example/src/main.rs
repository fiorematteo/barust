use barust::{
    corex::Color,
    error::{Erc, Result},
    statusbar::{Position, StatusBar},
    widgets::{
        Battery, BatteryIcons, Clock, Cpu, Network, Widget, WidgetConfig, WidgetError, Workspace,
    },
};

const _WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
const _BLACK: Color = Color::new(0.0, 0.0, 0.0, 1.0);
const _GREEN: Color = Color::new(0.0, 1.0, 0.0, 1.0);
const _RED: Color = Color::new(1.0, 0.2, 0.2, 1.0);
const PURPLE: Color = Color::new(0.8, 0.0, 1.0, 1.0);
const BLANK: Color = Color::new(0.0, 0.0, 0.0, 0.0);

fn main() -> Result<()> {
    let icons = BatteryIcons {
        full: String::from(" "),
        most: String::from(" "),
        half: String::from(" "),
        few: String::from(" "),
        empty: String::from(" "),
        charging: String::from("🗲"),
    };

    env_logger::init();

    let wd_config = WidgetConfig::default();
    let mut bar = StatusBar::create()
        .position(Position::Bottom)
        .background(BLANK)
        .left_widgets(vec![FilteredWorkspace::new::<&str>(
            PURPLE,
            10.0,
            &WidgetConfig {
                padding: 0.0,
                ..wd_config
            },
            &["scratchpad", "pulsemixer"],
        )])
        .right_widgets(vec![
            Cpu::new("%p%", &wd_config, None)?,
            Network::new("%s %n", "wlp1s0".to_string(), None, &wd_config, None),
            Battery::new("%i  %c%", Some(icons), &wd_config, None)?,
            Clock::new("%H:%M %d/%m/%Y", &wd_config, None),
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

    fn hook(&mut self, sender: chan::Sender<()>) -> barust::widgets::Result<()> {
        self.inner.hook(sender)
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
