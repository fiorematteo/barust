use crate::{
    utils::{
        screen_true_height, screen_true_width, set_source_rgba, Atoms, Color, HookSender, Position,
        Rectangle, StatusBarInfo, TimedHooks, WidgetIndex,
    },
    widgets::{ReplaceableWidget, Size, Widget},
    BarustError, Result,
};
use async_channel::{bounded, Receiver};
use cairo::{Context, Operator, XCBConnection, XCBDrawable, XCBSurface, XCBVisualType};
use futures::future::join_all;
use log::{debug, error, warn};
use std::{sync::Arc, thread};
use tokio::{
    select,
    signal::unix::{signal, SignalKind},
    spawn,
};
use xcb::{
    x::{
        Colormap, ColormapAlloc, CreateColormap, CreateWindow, Cw, EventMask, MapWindow, Pixmap,
        VisualClass, Visualtype, Window, WindowClass,
    },
    Connection, Event, Xid,
};

/// Represents the Bar displayed on the screen
pub struct StatusBar {
    background: Color,
    connection: Arc<Connection>,
    regions: Vec<Rectangle>,
    widgets: Vec<ReplaceableWidget>,
    surface: XCBSurface,
    height: u32,
    width: u32,
    window: Window,
    position: Position,
}

impl StatusBar {
    /// Creates a new status bar via [StatusBarBuilder]
    pub fn create() -> StatusBarBuilder {
        debug!("Creating default StatusBarBuilder");
        StatusBarBuilder::default()
    }

    /// Starts the [StatusBar] drawing and event loop
    pub async fn start(mut self) -> Result<()> {
        debug!("Starting loop");
        let (tx, widgets_events) = bounded::<WidgetIndex>(10);

        debug!("Widget setup");
        let info = StatusBarInfo {
            background: self.background,
            regions: self.regions.clone(),
            height: self.height,
            width: self.width,
            position: self.position,
            window: self.window,
        };
        let mut pool = TimedHooks::default();

        let setup_futures = self
            .widgets
            .iter_mut()
            .map(|w| w.setup_or_replace(&info))
            .collect::<Vec<_>>();
        join_all(setup_futures).await;

        for (index, wd) in self.widgets.iter_mut().enumerate() {
            wd.hook_or_replace(HookSender::new(tx.clone(), index), &mut pool)
                .await;
        }

        let update_futures = self
            .widgets
            .iter_mut()
            .map(|w| w.update_or_replace())
            .collect::<Vec<_>>();
        join_all(update_futures).await;

        let signal = stop_on_signal()?;
        let bar_events = bar_event_listener(Arc::clone(&self.connection))?;

        self.generate_regions().await?;
        self.show()?;

        // refresh background for transparent bar?
        self.draw_all().await?;
        self.draw_all().await?;

        pool.start().await;
        self.connection.flush()?;

        loop {
            let mut to_update: Option<WidgetIndex> = None;

            select!(
                id = widgets_events.recv() => {
                    to_update = id.ok();
                }
                _ = bar_events.recv() => {/* just redraw? */ }
                _ = signal.recv() => {
                    // shutdown
                    return Ok(())
                },
            );

            if let Some(to_update) = to_update {
                self.update(to_update).await?;
            }

            let need_relayout = self.generate_regions().await?;
            if need_relayout {
                self.draw_all().await?;
            } else if let Some(to_update) = to_update {
                self.targeted_draw(to_update).await?;
            }
        }
    }

    async fn update(&mut self, index: WidgetIndex) -> Result<()> {
        let wd = &mut self.widgets[index];
        wd.update_or_replace().await;
        Ok(())
    }

    /// Regenerate the regions for the widgets
    /// return true if the regions have changed
    async fn generate_regions(&mut self) -> Result<bool> {
        let context = Context::new(&self.surface)?;
        let mut rectangle = Rectangle {
            x: 0,
            y: 0,
            width: 0,
            height: self.height,
        };

        let static_size: u32 = self
            .widgets
            .iter_mut()
            .map(|wd| {
                if let Ok(Size::Static(width)) = wd.size(&context) {
                    width + 2 * wd.padding()
                } else {
                    2 * wd.padding()
                }
            })
            .sum();

        let flex_widgets = self
            .widgets
            .iter_mut()
            .flat_map(|wd| wd.size(&context))
            .filter(|wd| wd.is_flex())
            .count();

        let flex_size = (self.width - static_size)
            .checked_div(flex_widgets as u32)
            // if there are no flex widgets, use the full width
            .unwrap_or(self.width - static_size);

        let mut need_relayout = false;

        let left = self.widgets.iter_mut().zip(self.regions.iter_mut());

        for (wd, region) in left {
            rectangle.x += wd.padding();
            let widget_width = wd.size_or_replace(&context).await.unwrap_or(flex_size);
            rectangle.width = widget_width;
            if !need_relayout && *region != rectangle {
                need_relayout = true;
            }
            *region = rectangle;
            rectangle.x += widget_width + wd.padding();
        }

        Ok(need_relayout)
    }

    async fn draw_all(&mut self) -> Result<()> {
        assert!(
            self.regions.len() == self.widgets.len(),
            "Regions and widgets length mismatch"
        );

        let widgets = self.widgets.iter_mut();

        let regions: Vec<&Rectangle> = self.regions.iter().collect();

        let context = Context::new(&self.surface)?;
        // clear surface
        context.set_operator(Operator::Clear);
        context.paint()?;
        // paint background
        context.set_operator(Operator::Over);
        set_source_rgba(&context, self.background);
        context.paint()?;

        for (wd, rectangle) in widgets.zip(regions) {
            let cairo_rectangle: cairo::Rectangle = (*rectangle).into();
            let surface = &self.surface.create_for_rectangle(cairo_rectangle)?;
            let context = Context::new(surface)?;
            wd.draw_or_replace(context, rectangle).await;
        }

        self.surface.flush();
        self.connection.flush()?;
        Ok(())
    }

    async fn targeted_draw(&mut self, index: WidgetIndex) -> Result<()> {
        let wd = &mut self.widgets[index];
        let region = self.regions[index];

        let cairo_rectangle: cairo::Rectangle = region.into();
        let surface = &self.surface.create_for_rectangle(cairo_rectangle)?;
        let context = Context::new(surface)?;

        context.set_operator(Operator::Clear);
        context.paint()?;
        context.set_operator(Operator::Over);
        set_source_rgba(&context, self.background);
        context.paint()?;

        wd.draw_or_replace(context, &region).await;

        self.surface.flush();
        self.connection.flush()?;
        Ok(())
    }

    fn show(&self) -> Result<()> {
        self.connection.send_and_check_request(&MapWindow {
            window: self.window,
        })?;
        Ok(())
    }
}

///Used to easily build a [StatusBar]
pub struct StatusBarBuilder {
    xoff: u16,
    yoff: u16,
    width: Option<u16>,
    height: u16,
    position: Position,
    background: Color,
    widgets: Vec<Box<dyn Widget>>,
}

impl Default for StatusBarBuilder {
    fn default() -> Self {
        Self {
            xoff: 0,
            yoff: 0,
            width: None,
            height: 21,
            position: Position::Top,
            background: Color::new(0.0, 0.0, 0.0, 1.0),
            widgets: Vec::new(),
        }
    }
}

impl StatusBarBuilder {
    ///Set the `StatusBar` offset on the x axis
    pub fn xoff(mut self, offset: u16) -> Self {
        self.xoff = offset;
        self
    }

    ///Set the `StatusBar` offset on the y axis
    pub fn yoff(mut self, offset: u16) -> Self {
        self.yoff = offset;
        self
    }

    ///Set the `StatusBar` width
    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    ///Set the `StatusBar` height
    pub fn height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }

    ///Set the `StatusBar` position (top or bottom)
    pub fn position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    ///Set the `StatusBar` background color
    pub fn background(mut self, background: Color) -> Self {
        self.background = background;
        self
    }

    ///Add a widget to the `StatusBar`
    pub fn widget(mut self, widget: Box<dyn Widget>) -> Self {
        self.widgets.push(widget);
        self
    }

    ///Add multiple widgets to the `StatusBar`
    pub fn widgets(mut self, widgets: Vec<Box<dyn Widget>>) -> Self {
        for wd in widgets {
            self.widgets.push(wd);
        }
        self
    }

    ///Build the `StatusBar` with the previously selected options
    pub async fn build(self) -> Result<StatusBar> {
        let (connection, screen_id) = Connection::connect(None)?;
        let connection = Arc::new(connection);

        let width = self
            .width
            .unwrap_or_else(|| screen_true_width(&connection, screen_id));

        let window: Window = connection.generate_id();
        let colormap: Colormap = connection.generate_id();

        let screen = connection
            .get_setup()
            .roots()
            .nth(screen_id as _)
            .unwrap_or_else(|| panic!("cannot find screen:{}", screen_id));

        let depth = screen
            .allowed_depths()
            .find(|d| d.depth() == 32)
            .expect("cannot find valid depth");

        let mut visual_type = depth
            .visuals()
            .iter()
            .find(|v| v.class() == VisualClass::TrueColor)
            .expect("cannot find valid visual type")
            .to_owned();

        connection.send_and_check_request(&CreateColormap {
            alloc: ColormapAlloc::None,
            mid: colormap,
            window: screen.root(),
            visual: visual_type.visual_id(),
        })?;

        connection.send_and_check_request(&CreateWindow {
            depth: depth.depth(),
            wid: window,
            parent: screen.root(),
            x: self.xoff as _,
            y: match self.position {
                Position::Top => self.yoff,
                Position::Bottom => screen_true_height(&connection, screen_id) - self.height,
            } as _,
            width,
            height: self.height,
            border_width: 0,
            class: WindowClass::InputOutput,
            visual: visual_type.visual_id(),
            value_list: &[
                Cw::BackPixmap(Pixmap::none()),
                Cw::BorderPixel(screen.black_pixel()),
                Cw::EventMask(EventMask::all()),
                Cw::Colormap(colormap),
            ],
        })?;

        let atoms = Atoms::new(&connection)?;
        connection.send_and_check_request(&xcb::x::ChangeProperty {
            mode: xcb::x::PropMode::Replace,
            window,
            property: atoms._NET_WM_WINDOW_TYPE,
            r#type: xcb::x::ATOM_ATOM,
            data: &[atoms._NET_WM_WINDOW_TYPE_DOCK],
        })?;

        let bar_size = self.height as u32; // MUST USE u32
        let strut_data = [0, 0, bar_size, 0, 0, 0, 0, 0, 0, width as u32, 0, 0];

        connection.send_and_check_request(&xcb::x::ChangeProperty {
            mode: xcb::x::PropMode::Replace,
            window,
            property: atoms._NET_WM_STRUT,
            r#type: xcb::x::ATOM_CARDINAL,
            data: &strut_data[0..4],
        })?;

        connection.send_and_check_request(&xcb::x::ChangeProperty {
            mode: xcb::x::PropMode::Replace,
            window,
            property: atoms._NET_WM_STRUT_PARTIAL,
            r#type: xcb::x::ATOM_CARDINAL,
            data: &strut_data,
        })?;

        set_window_title(connection.clone(), window, "barust")?;

        let surface = unsafe {
            let conn_ptr = connection.get_raw_conn() as _;
            XCBSurface::create(
                &XCBConnection::from_raw_none(conn_ptr),
                &XCBDrawable(window.resource_id()),
                &XCBVisualType::from_raw_none(&mut visual_type as *mut Visualtype as _),
                i32::from(width),
                i32::from(self.height),
            )?
        };

        connection.flush()?;

        let widgets: Vec<ReplaceableWidget> = self
            .widgets
            .into_iter()
            .map(ReplaceableWidget::new)
            .collect();
        let regions = vec![Rectangle::default(); widgets.len()];

        Ok(StatusBar {
            background: self.background,
            connection,
            height: u32::from(self.height),
            regions,
            widgets,
            surface,
            width: u32::from(width),
            window,
            position: self.position,
        })
    }
}

pub(crate) fn set_window_title(
    connection: Arc<Connection>,
    window: Window,
    title: &str,
) -> xcb::Result<()> {
    let atoms = Atoms::new(&connection)?;
    let mut property_request = xcb::x::ChangeProperty {
        mode: xcb::x::PropMode::Replace,
        window,
        property: atoms.WM_NAME,
        r#type: xcb::x::ATOM_STRING,
        data: title.as_bytes(),
    };
    connection.send_and_check_request(&property_request)?;
    property_request.property = atoms._NET_WM_NAME;
    connection.send_and_check_request(&property_request)?;
    Ok(())
}

fn bar_event_listener(connection: Arc<Connection>) -> Result<Receiver<()>> {
    let (tx, rx) = bounded(10);
    thread::spawn(move || loop {
        if matches!(connection.wait_for_event(), Ok(Event::X(_))) && tx.send_blocking(()).is_err() {
            error!("bar_event_listener channel closed");
            break;
        }
    });
    Ok(rx)
}

fn stop_on_signal() -> std::result::Result<Receiver<()>, BarustError> {
    let (s, r) = bounded(10);
    spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        loop {
            select! {
                _ = sigterm.recv() => warn!("Receive SIGTERM"),
                _ = sigint.recv() => warn!("Receive SIGINT"),
            };
            if s.send(()).await.is_err() {
                error!("signal channel closed");
                break;
            }
        }
    });
    Ok(r)
}
