use crate::{
    corex::{
        notify, set_source_rgba, Atoms, Color, HookSender, StatusBarEvent, TimedHooks, WidgetID,
    },
    error::{BarustError, Result},
    log_error_and_replace,
    widgets::{Text, Widget},
};
use cairo::{Context, Operator, Rectangle, XCBConnection, XCBDrawable, XCBSurface, XCBVisualType};
use crossbeam_channel::{bounded, select, tick, Receiver};
use log::{debug, error, info};
use signal_hook::consts::{SIGINT, SIGTERM};
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use xcb::{
    x::{
        Colormap, ColormapAlloc, CreateColormap, CreateWindow, Cw, EventMask, MapWindow, Pixmap,
        UnmapWindow, VisualClass, Visualtype, Window, WindowClass,
    },
    Connection, Event, Xid,
};

#[derive(Clone, Copy, Debug)]
pub enum Position {
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug)]
pub enum RightLeft {
    Right,
    Left,
}

#[derive(Debug)]
pub struct StatusBarInfo {
    pub background: Color,
    pub left_regions: Vec<Rectangle>,
    pub right_regions: Vec<Rectangle>,
    pub height: f64,
    pub width: f64,
    pub position: Position,
}

impl StatusBarInfo {
    pub fn new(bar: &StatusBar) -> Self {
        Self {
            background: bar.background,
            left_regions: bar.left_regions.clone(),
            right_regions: bar.right_regions.clone(),
            height: bar.height,
            width: bar.width,
            position: bar.position,
        }
    }
}

/// Represents the Bar displayed on the screen
pub struct StatusBar {
    background: Color,
    connection: Arc<Connection>,
    left_regions: Vec<Rectangle>,
    left_widgets: Vec<Box<dyn Widget>>,
    right_regions: Vec<Rectangle>,
    right_widgets: Vec<Box<dyn Widget>>,
    surface: XCBSurface,
    height: f64,
    width: f64,
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
    pub fn start(mut self) -> Result<()> {
        info!("Starting loop");
        let (tx, widgets_events) = bounded::<WidgetID>(10);

        debug!("Widget setup");
        let info = StatusBarInfo::new(&self);
        let mut pool = TimedHooks::default();
        for (index, wd) in self.left_widgets.iter_mut().enumerate() {
            log_error_and_replace!(wd, wd.setup(&info));
            log_error_and_replace!(
                wd,
                wd.hook(
                    HookSender::new(tx.clone(), (RightLeft::Left, index)),
                    &mut pool
                )
            );
        }
        for (index, wd) in self.right_widgets.iter_mut().enumerate() {
            log_error_and_replace!(wd, wd.setup(&info));
            log_error_and_replace!(
                wd,
                wd.hook(
                    HookSender::new(tx.clone(), (RightLeft::Right, index)),
                    &mut pool
                )
            );
        }
        let signal = notify(&[SIGINT, SIGTERM])?;
        let timeout = tick(Duration::from_secs(10));
        let bar_events = bar_event_listener(Arc::clone(&self.connection))?;

        self.generate_regions()?;
        self.draw()?;
        self.show()?;

        let mut draw_timer = Instant::now();
        loop {
            let mut to_update: Option<WidgetID> = None;
            debug!("Looping");
            select!(
                recv(timeout) ->  _ => debug!("timeout triggered"),
                recv(widgets_events) -> id => {
                    if let Ok(id) = id{
                        to_update=Some(id)
                    }
                },
                recv(bar_events) -> event => {
                    if let Ok(StatusBarEvent::Click(x, y)) = event {
                         self.event(x, y);
                    }
                },
                recv(signal) -> _ => {
                    for wd in self.left_widgets.iter_mut().chain(&mut self.right_widgets){
                        log_error_and_replace!(wd, wd.last_update());
                    }
                    return Ok(());
                }
            );
            if let Some(to_update) = to_update {
                self.update(to_update)?;
            }
            self.generate_regions()?;

            if draw_timer.elapsed() > Duration::from_millis(10) {
                draw_timer = Instant::now();
                self.draw()?;
            }
        }
    }

    pub(crate) fn event(&mut self, x: i16, y: i16) {
        if let Some(index) = find_collision(&self.right_regions, x, y) {
            self.right_widgets[index].on_click();
        } else if let Some(index) = find_collision(&self.left_regions, x, y) {
            self.left_widgets[index].on_click();
        }
    }

    pub(crate) fn update(&mut self, to_update: WidgetID) -> Result<()> {
        debug!("Updating");
        match to_update {
            (RightLeft::Left, index) => {
                let wd = &mut self.left_widgets[index];
                log_error_and_replace!(wd, wd.update());
            }
            (RightLeft::Right, index) => {
                let wd = &mut self.right_widgets[index];
                log_error_and_replace!(wd, wd.update());
            }
        }
        Ok(())
    }

    pub(crate) fn generate_regions(&mut self) -> Result<()> {
        let context = Context::new(&self.surface)?;
        let mut rectangle = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: self.height,
        };

        self.left_regions.clear();
        for wd in &mut self.left_widgets {
            let widget_width = wd.size(&context)?;
            rectangle.width = widget_width;
            self.left_regions.push(rectangle);
            rectangle.x += widget_width;
        }

        let right_size: f64 = self
            .right_widgets
            .iter_mut()
            .flat_map(|wd| wd.size(&context))
            .sum();

        rectangle.x = self.width as f64 - right_size;

        self.right_regions.clear();
        for wd in &mut self.right_widgets {
            let widget_width = wd.size(&context)?;
            rectangle.width = widget_width;
            self.right_regions.push(rectangle);
            rectangle.x += widget_width;
        }
        Ok(())
    }

    pub(crate) fn draw(&mut self) -> Result<()> {
        if self.left_regions.len() != self.left_widgets.len()
            || self.right_regions.len() != self.right_widgets.len()
        {
            return Err(BarustError::DrawBeforeUpdate);
        }

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

        let contexts: Vec<_> = regions
            .iter()
            .map(|region| -> Result<Context> {
                let surface = &tmp_surface.create_for_rectangle(**region)?;
                Ok(Context::new(surface)?)
            })
            .collect();

        for ((wd, rectangle), context) in widgets.zip(regions).zip(contexts) {
            log_error_and_replace!(wd, wd.draw(&context?, rectangle));
        }

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

        self.connection.flush()?;
        Ok(())
    }

    pub fn show(&self) -> Result<&Self> {
        self.connection.send_and_check_request(&MapWindow {
            window: self.window,
        })?;
        Ok(self)
    }

    pub fn hide(&self) -> Result<&Self> {
        self.connection.send_and_check_request(&UnmapWindow {
            window: self.window,
        })?;
        Ok(self)
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
    pub fn xoff(&mut self, offset: u16) -> &mut Self {
        self.xoff = offset;
        self
    }

    ///Set the `StatusBar` offset on the y axis
    pub fn yoff(&mut self, offset: u16) -> &mut Self {
        self.yoff = offset;
        self
    }

    ///Set the `StatusBar` width
    pub fn width(&mut self, width: u16) -> &mut Self {
        self.width = Some(width);
        self
    }

    ///Set the `StatusBar` height
    pub fn height(&mut self, height: u16) -> &mut Self {
        self.height = height;
        self
    }

    ///Set the `StatusBar` position (top or bottom)
    pub fn position(&mut self, position: Position) -> &mut Self {
        self.position = position;
        self
    }

    ///Set the `StatusBar` background color
    pub fn background(&mut self, background: Color) -> &mut Self {
        self.background = background;
        self
    }

    ///Add a widget to the `StatusBar` on the left
    pub fn left_widget(&mut self, widget: Box<dyn Widget>) -> &mut Self {
        self.left_widgets.push(widget);
        self
    }

    ///Add multiple widgets to the `StatusBar` on the left
    pub fn left_widgets(&mut self, widgets: Vec<Box<dyn Widget>>) -> &mut Self {
        for wd in widgets {
            self.left_widgets.push(wd);
        }
        self
    }

    ///Add a widget to the `StatusBar` on the right
    pub fn right_widget(&mut self, widget: Box<dyn Widget>) -> &mut Self {
        self.right_widgets.push(widget);
        self
    }

    ///Add multiple widgets to the `StatusBar` on the right
    pub fn right_widgets(&mut self, widgets: Vec<Box<dyn Widget>>) -> &mut Self {
        for wd in widgets {
            self.right_widgets.push(wd);
        }
        self
    }

    ///Build the `StatusBar` with the previously selected options
    pub fn build(&mut self) -> Result<StatusBar> {
        let (connection, screen_id) = Connection::connect(None)?;
        let connection = Arc::new(connection);

        let width = self
            .width
            .unwrap_or_else(|| screen_true_width(&connection, screen_id));

        let (window, surface) = create_xwindow(
            &connection,
            screen_id,
            self.xoff,
            self.yoff,
            width,
            self.height,
            self.position,
        )?;

        Ok(StatusBar {
            background: self.background,
            connection,
            height: self.height as _,
            left_regions: Vec::new(),
            left_widgets: self.left_widgets.drain(..).collect(),
            right_regions: Vec::new(),
            right_widgets: self.right_widgets.drain(..).collect(),
            surface,
            width: width as _,
            window,
            position: self.position,
        })
    }
}

pub(crate) fn bar_event_listener(connection: Arc<Connection>) -> Result<Receiver<StatusBarEvent>> {
    let (tx, rx) = bounded(10);
    thread::spawn(move || loop {
        if let Ok(Event::X(event)) = connection.wait_for_event() {
            let event = match event {
                xcb::x::Event::ButtonPress(data) => {
                    StatusBarEvent::Click(data.event_x(), data.event_y())
                }
                _ => StatusBarEvent::Wake,
            };
            if tx.send(event).is_err() {
                break;
            }
        }
    });
    Ok(rx)
}

pub(crate) fn create_xwindow(
    connection: &Connection,
    screen_id: i32,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    position: Position,
) -> Result<(Window, XCBSurface)> {
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
        x: x as _,
        y: match position {
            Position::Top => y,
            Position::Bottom => screen_true_height(connection, screen_id) - height,
        } as _,
        width,
        height,
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

    let atoms = Atoms::new(connection)?;
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
            width as _,
            height as _,
        )?
    };

    connection.flush()?;
    Ok((window, surface))
}

pub(crate) fn screen_true_width(connection: &Connection, screen_id: i32) -> u16 {
    connection
        .get_setup()
        .roots()
        .nth(screen_id as _)
        .unwrap_or_else(|| panic!("cannot find screen:{}", screen_id))
        .width_in_pixels()
}

pub(crate) fn screen_true_height(connection: &Connection, screen_id: i32) -> u16 {
    connection
        .get_setup()
        .roots()
        .nth(screen_id as _)
        .unwrap_or_else(|| panic!("cannot find screen:{}", screen_id))
        .height_in_pixels()
}

pub(crate) fn find_collision(regions: &[Rectangle], x: i16, y: i16) -> Option<usize> {
    regions
        .iter()
        .enumerate()
        .find(|(_, r)| {
            r.x < x as f64 && r.x + r.width > x as f64 && r.y < y as f64 && r.y + r.width > y as f64
        })
        .map(|(index, _)| index)
}

#[macro_export]
macro_rules! log_error_and_replace {
    ( $wd:expr, $r:expr ) => {
        if let Err(e) = $r {
            error!("{:?}", e);
            error!("Replacing widget with default");
            *$wd = Text::new(
                "Widget Crashed :(",
                &$crate::widgets::WidgetConfig::default(),
                None,
            )
        }
    };
}
