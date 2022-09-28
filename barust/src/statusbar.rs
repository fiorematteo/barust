use crate::{
    corex::{
        set_source_rgba, Atoms, BarustEvent, Color, _NET_WM_WINDOW_TYPE, _NET_WM_WINDOW_TYPE_DOCK,
    },
    error::{BarustError, Result},
    log_error_and_replace,
    widgets::{Text, Widget},
};
use cairo::{Context, Operator, Rectangle, XCBConnection, XCBDrawable, XCBSurface, XCBVisualType};
use chan::{chan_select, Receiver};
use log::{debug, error, info};
use std::{sync::Arc, thread, time::Duration};
use xcb::{
    x::{
        Colormap, ColormapAlloc, CreateColormap, CreateWindow, Cw, EventMask, MapWindow, Pixmap,
        UnmapWindow, VisualClass, Visualtype, Window, WindowClass,
    },
    Connection, Event, Xid,
};

#[derive(Clone, Copy)]
pub enum Position {
    Top,
    Bottom,
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
}

impl StatusBar {
    /// Creates a new status bar via [StatusBarBuilder]
    pub fn create() -> StatusBarBuilder {
        debug!("Creating default StatusBarBuilder");
        StatusBarBuilder::default()
    }

    /// Starts the [StatusBar] drawing and event loop
    pub fn start(&mut self) -> Result<()> {
        info!("Starting loop");
        let (tx, widgets_events) = chan::sync(0);
        debug!("First update");
        for wd in self
            .left_widgets
            .iter_mut()
            .chain(self.right_widgets.iter_mut())
        {
            log_error_and_replace!(wd, wd.first_update());
            log_error_and_replace!(wd, wd.hook(tx.clone()));
        }
        let signal = chan_signal::notify(&[chan_signal::Signal::INT, chan_signal::Signal::TERM]);
        let timeout = chan::tick(Duration::from_secs(5));
        let bar_events = bar_event_listener(Arc::clone(&self.connection))?;

        self.show()?;
        loop {
            debug!("Looping");
            self.update()?;
            self.draw()?;
            chan_select!(
                timeout.recv() => (),
                widgets_events.recv() => (),
                bar_events.recv() -> event => {
                    if let Some(BarustEvent::Click(x, y)) = event{
                         self.event(x, y);
                    }
                },
                signal.recv() => {
                    for wd in self.left_widgets.iter_mut().chain(&mut self.right_widgets){
                        eprintln!("finishing");
                        log_error_and_replace!(wd, wd.last_update());
                    }
                    return Ok(());
                }
            );
        }
    }

    pub(crate) fn event(&mut self, x: i16, y: i16) {
        if let Some(index) = find_collision(&self.right_regions, x, y) {
            self.right_widgets[index].on_click();
        } else if let Some(index) = find_collision(&self.left_regions, x, y) {
            self.left_widgets[index].on_click();
        }
    }

    pub(crate) fn update(&mut self) -> Result<()> {
        debug!("Updating");
        for wd in self
            .right_widgets
            .iter_mut()
            .chain(self.left_widgets.iter_mut())
        {
            log_error_and_replace!(wd, wd.update());
        }

        let context = Context::new(&self.surface)?;
        let mut rectangle = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: self.height,
        };

        self.left_regions.clear();
        for wd in &mut self.left_widgets {
            rectangle.width = wd.size(&context)?;
            self.left_regions.push(rectangle);
            rectangle.x += wd.size(&context).unwrap_or(0.0);
        }

        let right_size: f64 = self
            .right_widgets
            .iter_mut()
            .flat_map(|wd| wd.size(&context))
            .sum();

        rectangle.x = self.width as f64 - right_size;

        self.right_regions.clear();
        for wd in &mut self.right_widgets {
            rectangle.width = wd.size(&context)?;
            self.right_regions.push(rectangle);
            rectangle.x += rectangle.width;
        }
        Ok(())
    }

    pub(crate) fn draw(&mut self) -> Result<()> {
        if self.left_regions.len() != self.left_widgets.len()
            || self.right_regions.len() != self.right_widgets.len()
        {
            return Err(BarustError::DrawBeforeUpdate);
        }

        let context = Context::new(&self.surface)?;
        context.set_operator(Operator::Clear);
        context.paint()?;
        context.set_operator(Operator::Over);
        set_source_rgba(&context, self.background);
        context.paint()?;

        let widgets = self
            .left_widgets
            .iter_mut()
            .chain(self.right_widgets.iter_mut());

        let regions = self.left_regions.iter().chain(self.right_regions.iter());

        for (wd, rectangle) in widgets.zip(regions) {
            let context = Context::new(&self.surface.create_for_rectangle(*rectangle)?)?;
            log_error_and_replace!(wd, wd.draw(&context, rectangle));
        }

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
        })
    }
}

pub(crate) fn bar_event_listener(connection: Arc<Connection>) -> Result<Receiver<BarustEvent>> {
    let (tx, rx) = chan::sync(0);
    thread::spawn(move || loop {
        if let Ok(Event::X(event)) = connection.wait_for_event() {
            let to_send = match event {
                xcb::x::Event::ButtonPress(data) => {
                    BarustEvent::Click(data.event_x(), data.event_y())
                }
                _ => BarustEvent::Wake,
            };
            tx.send(to_send);
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

    let atoms = Atoms::new(connection);
    connection.send_and_check_request(&xcb::x::ChangeProperty {
        mode: xcb::x::PropMode::Replace,
        window,
        property: atoms.get(_NET_WM_WINDOW_TYPE),
        r#type: xcb::x::ATOM_ATOM,
        data: &[atoms.get(_NET_WM_WINDOW_TYPE_DOCK)],
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
            *$wd = Text::new(
                "Widget Crashed :(",
                &$crate::widgets::WidgetConfig::default(),
                None,
            )
        }
    };
}
