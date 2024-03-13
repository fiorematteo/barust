use crate::utils::{
    hook_sender::RightLeft, screen_true_height, screen_true_width, set_source_rgba, Atoms, Color,
    HookSender, Position, Rectangle, ResettableTimer, StatusBarInfo, TimedHooks, WidgetID,
};
use crate::{
    widgets::{ReplaceableWidget, Size, Widget},
    BarustError, Result,
};
use async_channel::{bounded, Receiver};
use cairo::{Context, Operator, XCBConnection, XCBDrawable, XCBSurface, XCBVisualType};
use futures::future::join_all;
use futures::StreamExt;
use log::{debug, error};
use signal_hook::consts::signal::{SIGINT, SIGTERM};
use signal_hook_tokio::Signals;
use std::{ffi::c_int, sync::Arc, thread, time::Duration};
use tokio::{select, spawn};
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
    left_regions: Vec<Rectangle>,
    left_widgets: Vec<ReplaceableWidget>,
    right_regions: Vec<Rectangle>,
    right_widgets: Vec<ReplaceableWidget>,
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
        let (tx, widgets_events) = bounded::<WidgetID>(10);

        debug!("Widget setup");
        let info = StatusBarInfo {
            background: self.background,
            left_regions: self.left_regions.clone(),
            right_regions: self.right_regions.clone(),
            height: self.height,
            width: self.width,
            position: self.position,
            window: self.window,
        };
        let mut pool = TimedHooks::default();

        let setup_futures = self
            .left_widgets
            .iter_mut()
            .chain(self.right_widgets.iter_mut())
            .map(|w| w.setup_or_replace(&info))
            .collect::<Vec<_>>();
        join_all(setup_futures).await;

        for (index, wd) in self.left_widgets.iter_mut().enumerate() {
            wd.hook_or_replace(
                HookSender::new(tx.clone(), (RightLeft::Left, index)),
                &mut pool,
            )
            .await;
        }
        for (index, wd) in self.right_widgets.iter_mut().enumerate() {
            wd.hook_or_replace(
                HookSender::new(tx.clone(), (RightLeft::Right, index)),
                &mut pool,
            )
            .await;
        }

        let update_futures = self
            .left_widgets
            .iter_mut()
            .chain(self.right_widgets.iter_mut())
            .map(|w| w.update_or_replace())
            .collect::<Vec<_>>();
        join_all(update_futures).await;

        let signal = notify(&[SIGINT, SIGTERM])?;
        let bar_events = bar_event_listener(Arc::clone(&self.connection))?;

        self.generate_regions().await?;
        self.show()?;
        self.draw().await?;
        pool.start().await;
        self.connection.flush()?;

        let mut draw_timer = ResettableTimer::new(Duration::from_millis(1000 / 60));
        loop {
            let mut to_update: Option<WidgetID> = None;
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
            // greedy updating
            while let Ok(to_update) = widgets_events.try_recv() {
                self.update(to_update).await?;
            }

            self.generate_regions().await?;

            if draw_timer.is_done() {
                draw_timer.reset();
                self.draw().await?;
            }
        }
    }

    async fn update(&mut self, (side, index): WidgetID) -> Result<()> {
        let wd = match side {
            RightLeft::Left => &mut self.left_widgets[index],
            RightLeft::Right => &mut self.right_widgets[index],
        };
        wd.update_or_replace().await;
        Ok(())
    }

    async fn generate_regions(&mut self) -> Result<()> {
        let context = Context::new(&self.surface)?;
        let mut rectangle = Rectangle {
            x: 0,
            y: 0,
            width: 0,
            height: self.height,
        };

        let static_size: u32 = self
            .left_widgets
            .iter_mut()
            .chain(&mut self.right_widgets)
            .map(|wd| {
                if let Ok(Size::Static(width)) = wd.size(&context) {
                    width + 2 * wd.padding()
                } else {
                    2 * wd.padding()
                }
            })
            .sum();

        let flex_widgets = self
            .left_widgets
            .iter_mut()
            .chain(&mut self.right_widgets)
            .flat_map(|wd| wd.size(&context))
            .filter(|wd| wd.is_flex())
            .count();

        let flex_size = (self.width - static_size) / flex_widgets as u32;

        self.left_regions.clear();
        for wd in &mut self.left_widgets {
            rectangle.x += wd.padding();
            let widget_width = wd.size_or_replace(&context).await.unwrap_or(flex_size);
            rectangle.width = widget_width;
            self.left_regions.push(rectangle);
            rectangle.x += widget_width + wd.padding();
        }

        self.right_regions.clear();
        for wd in &mut self.right_widgets {
            rectangle.x += wd.padding();
            let widget_width = wd.size_or_replace(&context).await.unwrap_or(flex_size);
            rectangle.width = widget_width;
            self.right_regions.push(rectangle);
            rectangle.x += widget_width + wd.padding();
        }
        Ok(())
    }

    async fn draw(&mut self) -> Result<()> {
        assert!(
            self.left_regions.len() == self.left_widgets.len()
                && self.right_regions.len() == self.right_widgets.len(),
            "Regions and widgets length mismatch"
        );

        let widgets = self
            .left_widgets
            .iter_mut()
            .chain(self.right_widgets.iter_mut());

        let regions: Vec<&Rectangle> = self
            .left_regions
            .iter()
            .chain(self.right_regions.iter())
            .collect();

        // double buffer to prevent flickering
        let tmp_surface = self.surface.create_similar_image(
            cairo::Format::ARgb32,
            self.width as _,
            self.height as _,
        )?;

        for (wd, rectangle) in widgets.zip(regions) {
            let cairo_rectangle: cairo::Rectangle = (*rectangle).into();
            let surface = &tmp_surface.create_for_rectangle(cairo_rectangle)?;
            let context = Context::new(surface)?;
            wd.draw_or_replace(context, rectangle).await;
        }
        tmp_surface.flush();

        let context = Context::new(&self.surface)?;
        // clear surface
        context.set_operator(Operator::Clear);
        context.paint()?;
        // paint background
        context.set_operator(Operator::Over);
        set_source_rgba(&context, self.background);
        context.paint()?;
        // copy tmp_surface
        context.set_source_surface(&tmp_surface, 0.0, 0.0)?;
        context.paint()?;
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
    left_widgets: Vec<Box<dyn Widget>>,
    right_widgets: Vec<Box<dyn Widget>>,
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
            left_widgets: Vec::new(),
            right_widgets: Vec::new(),
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

    ///Add a widget to the `StatusBar` on the left
    pub fn left_widget(mut self, widget: Box<dyn Widget>) -> Self {
        self.left_widgets.push(widget);
        self
    }

    ///Add multiple widgets to the `StatusBar` on the left
    pub fn left_widgets(mut self, widgets: Vec<Box<dyn Widget>>) -> Self {
        for wd in widgets {
            self.left_widgets.push(wd);
        }
        self
    }

    ///Add a widget to the `StatusBar` on the right
    pub fn right_widget(mut self, widget: Box<dyn Widget>) -> Self {
        self.right_widgets.push(widget);
        self
    }

    ///Add multiple widgets to the `StatusBar` on the right
    pub fn right_widgets(mut self, widgets: Vec<Box<dyn Widget>>) -> Self {
        for wd in widgets {
            self.right_widgets.push(wd);
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

        Ok(StatusBar {
            background: self.background,
            connection,
            height: u32::from(self.height),
            left_regions: Vec::new(),
            left_widgets: self
                .left_widgets
                .into_iter()
                .map(ReplaceableWidget::new)
                .collect(),
            right_regions: Vec::new(),
            right_widgets: self
                .right_widgets
                .into_iter()
                .map(ReplaceableWidget::new)
                .collect(),
            surface,
            width: u32::from(width),
            window,
            position: self.position,
        })
    }
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

fn notify(signals: &[c_int]) -> std::result::Result<Receiver<c_int>, BarustError> {
    let (s, r) = bounded(10);
    let mut signals = Signals::new(signals)?;
    spawn(async move {
        while let Some(signal) = signals.next().await {
            if s.send(signal).await.is_err() {
                break;
            }
        }
    });
    Ok(r)
}
