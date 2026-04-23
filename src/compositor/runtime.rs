use std::{os::unix::io::OwnedFd, sync::Arc};

use smithay::backend::winit as WinitBackend;
use smithay::backend::winit::WinitEvent;
use smithay::input::keyboard::KeyboardHandle;
use smithay::{
    backend::{
        input::{
            AbsolutePositionEvent, ButtonState, Event, InputEvent, KeyState, KeyboardKeyEvent,
            MouseButton, PointerButtonEvent,
        },
        renderer::{
            damage::OutputDamageTracker,
            element::{
                render_elements,
                solid::SolidColorRenderElement,
                surface::{render_elements_from_surface_tree, WaylandSurfaceRenderElement},
                Kind,
            },
            gles::GlesRenderer,
            utils::on_commit_buffer_handler,
            Color32F, ImportAll, Renderer,
        },
    },
    delegate_compositor, delegate_data_device, delegate_seat, delegate_shm, delegate_xdg_shell,
    input::{keyboard::FilterResult, Seat, SeatHandler, SeatState},
    reexports::wayland_server::{protocol::wl_seat, Display},
    utils::{Logical, Physical, Rectangle, Scale, Serial, Size, Transform},
    wayland::{
        buffer::BufferHandler,
        compositor::{
            with_states, with_surface_tree_downward, CompositorClientState, CompositorHandler,
            CompositorState, SurfaceAttributes, TraversalAction,
        },
        selection::{
            data_device::{
                ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
            },
            SelectionHandler,
        },
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
            XdgToplevelSurfaceData,
        },
        shm::{ShmHandler, ShmState},
    },
};
use wayland_protocols::xdg::shell::server::xdg_toplevel;
use wayland_server::{
    backend::{ClientData, ClientId, DisconnectReason},
    protocol::{
        wl_buffer,
        wl_surface::{self, WlSurface},
    },
    Client, ListeningSocket,
};
#[allow(unused_imports)]
use winit::platform::pump_events::PumpStatus;

use crate::compositor::{
    config, input,
    input::Action,
    launcher::AppLauncher,
    layout::LayoutEngine,
    panel::Panel,
    window::{WindowId, WindowManager, DECORATION_HEIGHT, PANEL_HEIGHT},
};

render_elements! {
    AppRenderElement<R> where R: Renderer + ImportAll;
    Surface=WaylandSurfaceRenderElement<R>,
    Solid=SolidColorRenderElement,
}

const BACKGROUND: Color32F = Color32F::new(0.12, 0.14, 0.18, 1.0);
const MIN_FLOAT_WIDTH: i32 = 160;
const MIN_FLOAT_HEIGHT: i32 = 120;

pub fn run(config_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    run_winit(config_path)
}

fn init_logging() {
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().init();
    }
}

pub fn run_winit(config_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let mut display: Display<App> = Display::new()?;
    let dh = display.handle();

    let compositor_state = CompositorState::new::<App>(&dh);
    let shm_state = ShmState::new::<App>(&dh, vec![]);
    let mut seat_state = SeatState::new();
    let seat = seat_state.new_wl_seat(&dh, "wowland");

    let config = config::load_config_with_fallback(config_path);
    let bindings = input::resolve_keybindings(&config.keybindings);
    let super_is_alt = config::super_is_alt(&config);

    let mut state = App {
        compositor_state,
        xdg_shell_state: XdgShellState::new::<App>(&dh),
        shm_state,
        seat_state,
        data_device_state: DataDeviceState::new::<App>(&dh),
        seat,
        keyboard: None,
        windows: WindowManager::new(),
        layout: LayoutEngine::default(),
        input: input::InputState::new(bindings, super_is_alt),
        floating_app_ids: config.floating_app_ids.clone(),
        super_is_alt,
        damage_tracker: OutputDamageTracker::new(Size::from((1, 1)), 1.0, Transform::Flipped180),
        output_size: Size::from((1, 1)),
        layout_dirty: true,
        needs_redraw: true,
        should_exit: false,
        panel: Panel::new(4),
        launcher: AppLauncher::new(),
    };

    state.launcher.load_desktop_files();

    if let Some(focused) = &config.decoration_focused {
        if let Some(color) = config::parse_hex_color(focused) {
            if let Some(unfocused) = &config.decoration_unfocused {
                if let Some(unfocused_color) = config::parse_hex_color(unfocused) {
                    state.windows.set_decoration_colors(color, unfocused_color);
                }
            }
        }
    }

    if let Some(gaps_config) = &config.gaps {
        state.layout.gaps.inner = gaps_config.inner.unwrap_or(0);
        state.layout.gaps.outer = gaps_config.outer.unwrap_or(0);
    }

    if let Some(workspace_config) = &config.workspace {
        state.windows.set_workspace_count(workspace_config.count);
    }

    state.panel = Panel::new(state.windows.workspace_count());

    let listener = ListeningSocket::bind_auto("wowland", 1..=32)?;
    let (mut backend, mut winit) = WinitBackend::init::<GlesRenderer>()?;
    let start_time = std::time::Instant::now();
    let keyboard = state.seat.add_keyboard(Default::default(), 200, 200)?;
    state.keyboard = Some(keyboard.clone());

    if let Some(name) = listener.socket_name() {
        // SAFETY: set once before spawning any clients to avoid races.
        unsafe {
            std::env::set_var("WAYLAND_DISPLAY", name);
        }
        tracing::info!("Wayland socket: {}", name.to_string_lossy());
    }

    loop {
        let status = winit.dispatch_new_events(|event| match event {
            WinitEvent::Resized { .. } => {}
            WinitEvent::Input(event) => match event {
                InputEvent::Keyboard { event } => {
                    let key_state = event.state();
                    let serial = Serial::from(0);
                    let time = event.time_msec();
                    keyboard.input::<(), _>(
                        &mut state,
                        event.key_code(),
                        key_state,
                        serial,
                        time,
                        |state, mods, keysym_handle| {
                            state.input.update_modifiers(mods);
                            if key_state == KeyState::Pressed {
                                if let Some(sym) = input::key_from_handle(&keysym_handle) {
                                    if let Some(action) = state.input.action_for(mods, sym) {
                                        state.apply_action(action);
                                        return FilterResult::Intercept(());
                                    }
                                }
                            }
                            FilterResult::Forward
                        },
                    );
                }
                InputEvent::PointerMotionAbsolute { event } => {
                    let position = event.position_transformed(state.output_size);
                    state.input.update_pointer_location(position.x, position.y);
                    state.handle_pointer_motion(position.x, position.y);
                }
                InputEvent::PointerButton { event } => {
                    state.handle_pointer_button(event.button(), event.state());
                }
                _ => {}
            },
            _ => (),
        });

        if state.should_exit {
            return Ok(());
        }

        match status {
            PumpStatus::Continue => (),
            PumpStatus::Exit(_) => return Ok(()),
        }

        let size = backend.window_size();
        let scale_factor = backend.scale_factor();
        let logical_size = size.to_logical(1);
        if logical_size != state.output_size {
            state.output_size = logical_size;
            state.layout_dirty = true;
            state.needs_redraw = true;
            state.damage_tracker =
                OutputDamageTracker::new(size, scale_factor, Transform::Flipped180);
        }

        if state.layout_dirty {
            state.apply_layout();
        }

        if let Some(stream) = listener.accept()? {
            let client = display
                .handle()
                .insert_client(stream, Arc::new(ClientState::default()))?;
            tracing::info!("Client connected: {:?}", client.id());
        }

        display.dispatch_clients(&mut state)?;
        display.flush_clients()?;

        if !state.needs_redraw {
            continue;
        }

        let buffer_age = backend.buffer_age().unwrap_or(0);
        let frame_damage: Option<Vec<Rectangle<i32, Physical>>>;
        {
            let (renderer, mut framebuffer) = backend.bind()?;
            let mut elements = Vec::new();
            let scale = Scale::from(1.0);

            state.panel.update(
                state.windows.current_workspace(),
                state.windows.workspace_count(),
            );
            for panel_elem in state.panel.render_elements(scale, state.output_size) {
                elements.push(AppRenderElement::Solid(panel_elem));
            }

            for window in state.windows.windows() {
                if window.workspace() != state.windows.current_workspace() {
                    continue;
                }
                if window.is_minimized() {
                    continue;
                }
                let focused = state
                    .windows
                    .focused_window()
                    .map(|focused| focused.id() == window.id())
                    .unwrap_or(false);
                elements.push(AppRenderElement::Solid(
                    window.decoration_element(scale, focused),
                ));
                let window_origin = (
                    window.location().x,
                    window.location().y + DECORATION_HEIGHT + PANEL_HEIGHT,
                );
                let surface_elements = render_elements_from_surface_tree(
                    renderer,
                    window.wl_surface(),
                    window_origin,
                    1.0,
                    window.opacity(),
                    Kind::Unspecified,
                )
                .into_iter()
                .map(AppRenderElement::Surface);
                elements.extend(surface_elements);
            }

            let render_result = state.damage_tracker.render_output(
                renderer,
                &mut framebuffer,
                buffer_age,
                &elements,
                BACKGROUND,
            )?;
            frame_damage = render_result.damage.map(|damage| damage.to_vec());
        }

        for window in state.windows.windows() {
            if window.workspace() != state.windows.current_workspace() {
                continue;
            }
            if window.is_minimized() {
                continue;
            }
            send_frames_surface_tree(window.wl_surface(), start_time.elapsed().as_millis() as u32);
        }

        backend.submit(frame_damage.as_deref())?;
        state.needs_redraw = false;
    }
}

fn send_frames_surface_tree(surface: &wl_surface::WlSurface, time: u32) {
    with_surface_tree_downward(
        surface,
        (),
        |_, _, &()| TraversalAction::DoChildren(()),
        |_surf, states, &()| {
            for callback in states
                .cached_state
                .get::<SurfaceAttributes>()
                .current()
                .frame_callbacks
                .drain(..)
            {
                callback.done(time);
            }
        },
        |_, _, &()| true,
    );
}

#[derive(Default)]
struct ClientState {
    compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {
        tracing::info!("Client initialized");
    }

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {
        tracing::info!("Client disconnected");
    }
}

struct App {
    compositor_state: CompositorState,
    xdg_shell_state: XdgShellState,
    shm_state: ShmState,
    seat_state: SeatState<Self>,
    data_device_state: DataDeviceState,
    seat: Seat<Self>,
    keyboard: Option<KeyboardHandle<Self>>,
    windows: WindowManager,
    layout: LayoutEngine,
    input: input::InputState,
    floating_app_ids: Vec<String>,
    super_is_alt: bool,
    damage_tracker: OutputDamageTracker,
    output_size: Size<i32, Logical>,
    layout_dirty: bool,
    needs_redraw: bool,
    should_exit: bool,
    panel: Panel,
    launcher: AppLauncher,
}

impl App {
    fn apply_layout(&mut self) {
        let workspace = self.windows.current_workspace();
        self.layout
            .apply(self.output_size, self.windows.windows_mut(), workspace);
        self.layout_dirty = false;
        self.needs_redraw = true;
    }

    fn apply_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_exit = true,
            Action::NextLayout => {
                self.layout.mode = self.layout.mode.next();
                self.layout_dirty = true;
            }
            Action::PrevLayout => {
                self.layout.mode = self.layout.mode.prev();
                self.layout_dirty = true;
            }
            Action::FocusNext => {
                if let Some(id) = self.windows.focus_next() {
                    self.set_focus(id);
                }
            }
            Action::FocusPrev => {
                if let Some(id) = self.windows.focus_prev() {
                    self.set_focus(id);
                }
            }
            Action::ToggleFloat => {
                if let Some(window) = self.windows.focused_window() {
                    let id = window.id();
                    let new_state = !window.is_floating();
                    if self.windows.set_floating(id, new_state) {
                        self.layout_dirty = true;
                        self.needs_redraw = true;
                    }
                }
            }
            Action::ToggleMaximize => {
                if let Some(window) = self.windows.focused_window() {
                    let id = window.id();
                    let new_state = !window.is_maximized();
                    if self.set_maximized(id, new_state) {
                        self.layout_dirty = true;
                        self.needs_redraw = true;
                    }
                }
            }
            Action::ToggleMinimize => {
                if let Some(window) = self.windows.focused_window() {
                    let id = window.id();
                    let new_state = !window.is_minimized();
                    if self.windows.set_minimized(id, new_state) {
                        if new_state {
                            if let Some(next_id) = self.windows.focus_next() {
                                self.set_focus(next_id);
                            }
                        } else {
                            self.set_focus(id);
                        }
                        self.layout_dirty = true;
                        self.needs_redraw = true;
                    }
                }
            }
            Action::CloseFocused => {
                if let Some(window) = self.windows.focused_window() {
                    window.toplevel().send_close();
                    self.needs_redraw = true;
                }
            }
            Action::CycleOpacity => {
                if let Some(window) = self.windows.focused_window_mut() {
                    window.cycle_opacity();
                    self.needs_redraw = true;
                }
            }
            Action::WorkspaceNext => {
                if self.windows.next_workspace() {
                    self.reset_pointer_grabs();
                    self.refocus_current_workspace();
                    self.layout_dirty = true;
                    self.needs_redraw = true;
                    tracing::info!(
                        "Workspace switched to {}",
                        self.windows.current_workspace() + 1
                    );
                }
            }
            Action::WorkspacePrev => {
                if self.windows.prev_workspace() {
                    self.reset_pointer_grabs();
                    self.refocus_current_workspace();
                    self.layout_dirty = true;
                    self.needs_redraw = true;
                    tracing::info!(
                        "Workspace switched to {}",
                        self.windows.current_workspace() + 1
                    );
                }
            }
            Action::MoveToWorkspaceNext => {
                if let Some(window_id) = self.active_window_id() {
                    let next =
                        (self.windows.current_workspace() + 1) % self.windows.workspace_count();
                    if self.windows.move_window_to_workspace(window_id, next) {
                        self.refocus_current_workspace();
                        self.layout_dirty = true;
                        self.needs_redraw = true;
                        tracing::info!("Moved window to workspace {}", next + 1);
                    }
                }
            }
            Action::MoveToWorkspacePrev => {
                if let Some(window_id) = self.active_window_id() {
                    let current = self.windows.current_workspace();
                    let prev = if current == 0 {
                        self.windows.workspace_count() - 1
                    } else {
                        current - 1
                    };
                    if self.windows.move_window_to_workspace(window_id, prev) {
                        self.refocus_current_workspace();
                        self.layout_dirty = true;
                        self.needs_redraw = true;
                        tracing::info!("Moved window to workspace {}", prev + 1);
                    }
                }
            }
            Action::Spawn { command } => {
                tracing::info!("Spawning: {}", command);
                if let Err(e) = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&command)
                    .spawn()
                {
                    tracing::error!("Failed to spawn command: {}", e);
                }
            }
            Action::Launcher { query } => {
                let query = query.as_deref().unwrap_or("");
                let matches = self.launcher.search(query);
                if let Some(first) = matches.first() {
                    tracing::info!("Launching: {}", first.name);
                    if let Err(e) = self.launcher.spawn(&first.name) {
                        tracing::error!("Failed to launch app: {}", e);
                    }
                }
            }
        }
    }

    fn set_focus(&mut self, id: WindowId) {
        let was_focused = self.windows.focused_window().map(|window| window.id());
        if !self.windows.focus_window(id) {
            return;
        }
        if let (Some(window), Some(keyboard)) =
            (self.windows.focused_window(), self.keyboard.clone())
        {
            keyboard.set_focus(self, Some(window.wl_surface().clone()), Serial::from(0));
        }
        let now_focused = self.windows.focused_window().map(|window| window.id());
        if was_focused != now_focused {
            self.needs_redraw = true;
        }
    }

    fn handle_pointer_motion(&mut self, x: f64, y: f64) {
        if let Some(resize) = self.input.resize_state() {
            if let Some(window) = self.windows.window_mut(resize.window_id) {
                let dx = x - resize.start_pointer.0;
                let dy = y - resize.start_pointer.1;
                let new_w = (resize.start_size.0 + dx as i32).max(MIN_FLOAT_WIDTH);
                let new_h = (resize.start_size.1 + dy as i32).max(MIN_FLOAT_HEIGHT);
                if window.set_size(Size::from((new_w, new_h))) {
                    window.configure();
                    self.needs_redraw = true;
                }
            }
            return;
        }

        if let Some(drag) = self.input.drag_state() {
            if let Some(window) = self.windows.window_mut(drag.window_id) {
                let new_location = (x - drag.offset.0, y - drag.offset.1);
                window.set_location((new_location.0 as i32, new_location.1 as i32).into());
                self.needs_redraw = true;
            }
            return;
        }

        if let Some(window_id) = self.windows.window_at((x, y).into()) {
            self.set_focus(window_id);
        }
    }

    fn handle_pointer_button(&mut self, button: Option<MouseButton>, state: ButtonState) {
        if state == ButtonState::Released {
            if let Some(drag) = self.input.drag_state() {
                if let Some(window) = self.windows.window_mut(drag.window_id) {
                    window.set_dragging(false);
                }
                self.input.end_drag();
            }
            if self.input.resize_state().is_some() {
                self.input.end_resize();
            }
            return;
        }

        if button != Some(MouseButton::Left) && button != Some(MouseButton::Right) {
            return;
        }

        let mods = self.input.modifiers();
        if !self.super_pressed(&mods) {
            return;
        }

        let (x, y) = self.input.pointer_location();
        if let Some(window_id) = self.windows.window_at((x, y).into()) {
            self.set_focus(window_id);
            if let Some(window) = self.windows.window_mut(window_id) {
                if !window.is_floating() {
                    return;
                }
                if window.is_maximized() {
                    return;
                }
                if button == Some(MouseButton::Left) {
                    window.set_dragging(true);
                    let offset = (
                        x - window.location().x as f64,
                        y - window.location().y as f64,
                    );
                    self.input.begin_drag(window_id, offset);
                } else if button == Some(MouseButton::Right) {
                    let size = window.size();
                    self.input.begin_resize(window_id, (x, y), (size.w, size.h));
                }
            }
        }
    }

    fn apply_floating_rule(&mut self, surface: &ToplevelSurface) {
        let app_id = match toplevel_app_id(surface) {
            Some(app_id) => app_id,
            None => return,
        };
        let should_float = self.floating_app_ids.iter().any(|rule| rule == &app_id);
        if let Some(window_id) = self.windows.window_id_for_surface(surface.wl_surface()) {
            if self.windows.set_forced_floating(window_id, should_float) {
                self.layout_dirty = true;
            }
        }
    }

    fn super_pressed(&self, mods: &smithay::input::keyboard::ModifiersState) -> bool {
        if self.super_is_alt {
            mods.alt
        } else {
            mods.logo
        }
    }

    fn set_maximized(&mut self, window_id: WindowId, maximized: bool) -> bool {
        let output_size = self.output_size;
        if !self
            .windows
            .set_maximized(window_id, maximized, output_size)
        {
            return false;
        }
        if let Some(window) = self.windows.window_mut(window_id) {
            window.toplevel().with_pending_state(|state| {
                if maximized {
                    state.states.set(xdg_toplevel::State::Maximized);
                } else {
                    state.states.unset(xdg_toplevel::State::Maximized);
                }
                state.size = Some(window.size());
            });
            window.toplevel().send_configure();
        }
        self.needs_redraw = true;
        true
    }

    fn refocus_current_workspace(&mut self) {
        if let Some(id) = self.windows.focus_next() {
            self.set_focus(id);
        }
    }

    fn reset_pointer_grabs(&mut self) {
        if let Some(drag) = self.input.drag_state() {
            if let Some(window) = self.windows.window_mut(drag.window_id) {
                window.set_dragging(false);
            }
            self.input.end_drag();
        }
        if self.input.resize_state().is_some() {
            self.input.end_resize();
        }
    }

    fn active_window_id(&self) -> Option<WindowId> {
        if let Some(window) = self.windows.focused_window() {
            return Some(window.id());
        }
        let (x, y) = self.input.pointer_location();
        self.windows.window_at((x, y).into())
    }
}

impl BufferHandler for App {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl XdgShellHandler for App {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Activated);
        });
        surface.send_configure();
        let window_id = self.windows.add_window(surface.clone());
        if let Some(app_id) = toplevel_app_id(&surface) {
            let should_float = self.floating_app_ids.iter().any(|rule| rule == &app_id);
            self.windows.set_forced_floating(window_id, should_float);
        }
        self.set_focus(window_id);
        self.layout_dirty = true;
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {}

    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {}

    fn reposition_request(
        &mut self,
        _surface: PopupSurface,
        _positioner: PositionerState,
        _token: u32,
    ) {
    }

    fn app_id_changed(&mut self, surface: ToplevelSurface) {
        self.apply_floating_rule(&surface);
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        if let Some(window_id) = self.windows.window_id_for_surface(surface.wl_surface()) {
            if self.set_maximized(window_id, true) {
                self.layout_dirty = true;
            }
        }
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        if let Some(window_id) = self.windows.window_id_for_surface(surface.wl_surface()) {
            if self.set_maximized(window_id, false) {
                self.layout_dirty = true;
            }
        }
    }

    fn minimize_request(&mut self, surface: ToplevelSurface) {
        if let Some(window_id) = self.windows.window_id_for_surface(surface.wl_surface()) {
            if self.windows.set_minimized(window_id, true) {
                if let Some(id) = self.windows.focus_next() {
                    self.set_focus(id);
                }
                self.layout_dirty = true;
            }
        }
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        if let Some(window_id) = self.windows.window_id_for_surface(surface.wl_surface()) {
            self.windows.remove_window(window_id);
            if let Some(next_id) = self.windows.focus_next() {
                self.set_focus(next_id);
            }
            self.layout_dirty = true;
        }
    }
}

impl SelectionHandler for App {
    type SelectionUserData = ();
}

impl DataDeviceHandler for App {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for App {}

impl ServerDndGrabHandler for App {
    fn send(&mut self, _mime_type: String, _fd: OwnedFd, _seat: Seat<Self>) {}
}

impl CompositorHandler for App {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        if self.windows.window_id_for_surface(surface).is_some() {
            self.needs_redraw = true;
        }
    }
}

impl ShmHandler for App {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl SeatHandler for App {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&WlSurface>) {}

    fn cursor_image(
        &mut self,
        _seat: &Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }
}

delegate_xdg_shell!(App);
delegate_compositor!(App);
delegate_shm!(App);
delegate_seat!(App);
delegate_data_device!(App);

fn toplevel_app_id(surface: &ToplevelSurface) -> Option<String> {
    with_states(surface.wl_surface(), |states| {
        states
            .data_map
            .get::<XdgToplevelSurfaceData>()
            .and_then(|data| data.lock().ok().and_then(|guard| guard.app_id.clone()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_is_opaque() {
        assert!(BACKGROUND.is_opaque());
    }
}
