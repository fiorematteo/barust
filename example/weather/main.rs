use barust::statusbar::StatusBar;
use barust::utils::{Color, Position};
use barust::widgets::{Clock, MeteoIcons, Spacer, Text, Weather, WidgetConfig};
use std::io::stdout;
use std::time::Duration;

#[tokio::main]
async fn main() -> barust::Result<()> {
    simple_logging::log_to(stdout(), log::LevelFilter::Debug);

    let black = Color::new(0.0, 0.0, 0.0, 1.0);
    let green = Color::new(0.0, 1.0, 0.3, 1.0);

    let wd_config = WidgetConfig {
        font: "GohuFont 11 Nerd Font".into(),
        font_size: 24.0,
        hide_timeout: Duration::from_secs(60),
        fg_color: green,
        ..WidgetConfig::default()
    };
    let _bar = StatusBar::create()
        .position(Position::Bottom)
        .background(black)
        .left_widgets(vec![
            Clock::new("🕓 %H:%M %d/%m/%Y", &wd_config).await,
            Spacer::new(10).await,
        ])
        .right_widgets(vec![
            Weather::new(
                &"󰴖 %cit %cod %cur%cur-u   %min%min-u  %max%max-u",
                MeteoIcons::default(),
                &wd_config,
            )
            .await,
            Clock::new("🕓 %H:%M %d/%m/%Y", &wd_config).await,
            Text::new(
                &"",
                &WidgetConfig {
                    font_size: 20.0,
                    flex: true,
                    ..WidgetConfig::default()
                },
            )
            .await,
        ])
        .height(30)
        .build()
        .await?
        .start()
        .await;
    Ok(())
}
