use smithay::{
    backend::renderer::{
        element::solid::SolidColorRenderElement,
        element::{Id, Kind},
        utils::CommitCounter,
        Color32F,
    },
    utils::{Logical, Point, Rectangle, Scale, Size},
    wayland::shell::xdg::ToplevelSurface,
};

pub const DECORATION_HEIGHT: i32 = 28;
pub const PANEL_HEIGHT: i32 = 28;
const DECORATION_FOCUSED: Color32F = Color32F::new(0.28, 0.32, 0.4, 1.0);
const DECORATION_UNFOCUSED: Color32F = Color32F::new(0.18, 0.2, 0.26, 1.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(u64);

pub struct Window {
    id: WindowId,
    toplevel: ToplevelSurface,
    location: Point<i32, Logical>,
    size: Size<i32, Logical>,
    decoration_id: Id,
    decoration_commit: CommitCounter,
    decoration_focused: Color32F,
    decoration_unfocused: Color32F,
    dragging: bool,
    floating: bool,
    forced_floating: bool,
    minimized: bool,
    maximized: bool,
    opacity: f32,
    restore_geometry: Option<(Point<i32, Logical>, Size<i32, Logical>)>,
    restore_floating: bool,
    workspace: usize,
}

impl Window {
    pub fn new(id: WindowId, toplevel: ToplevelSurface, workspace: usize) -> Self {
        Self {
            id,
            toplevel,
            location: (0, 0).into(),
            size: (800, 600).into(),
            decoration_id: Id::new(),
            decoration_commit: CommitCounter::default(),
            decoration_focused: DECORATION_FOCUSED,
            decoration_unfocused: DECORATION_UNFOCUSED,
            dragging: false,
            floating: false,
            forced_floating: false,
            minimized: false,
            maximized: false,
            opacity: 1.0,
            restore_geometry: None,
            restore_floating: false,
            workspace,
        }
    }

    pub fn id(&self) -> WindowId {
        self.id
    }

    pub fn wl_surface(
        &self,
    ) -> &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface {
        self.toplevel.wl_surface()
    }

    pub fn toplevel(&self) -> &ToplevelSurface {
        &self.toplevel
    }

    pub fn location(&self) -> Point<i32, Logical> {
        self.location
    }

    pub fn size(&self) -> Size<i32, Logical> {
        self.size
    }

    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub fn is_minimized(&self) -> bool {
        self.minimized
    }

    pub fn is_maximized(&self) -> bool {
        self.maximized
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    pub fn workspace(&self) -> usize {
        self.workspace
    }

    pub fn set_workspace(&mut self, workspace: usize) -> bool {
        if self.workspace != workspace {
            self.workspace = workspace;
            self.decoration_commit.increment();
            return true;
        }
        false
    }

    pub fn set_dragging(&mut self, dragging: bool) {
        if self.dragging != dragging {
            self.dragging = dragging;
            self.decoration_commit.increment();
        }
    }

    pub fn is_floating(&self) -> bool {
        self.floating || self.forced_floating
    }

    pub fn set_floating(&mut self, floating: bool) -> bool {
        if self.maximized && !floating {
            return false;
        }
        if self.forced_floating && !floating {
            return false;
        }
        if self.floating != floating {
            self.floating = floating;
            self.decoration_commit.increment();
            return true;
        }
        false
    }

    pub fn set_forced_floating(&mut self, forced: bool) -> bool {
        if self.forced_floating != forced {
            self.forced_floating = forced;
            if forced {
                self.floating = true;
            }
            self.decoration_commit.increment();
            return true;
        }
        false
    }

    pub fn set_minimized(&mut self, minimized: bool) -> bool {
        if self.minimized != minimized {
            self.minimized = minimized;
            self.decoration_commit.increment();
            return true;
        }
        false
    }

    pub fn set_maximized(&mut self, maximized: bool, output_size: Size<i32, Logical>) -> bool {
        if self.maximized == maximized {
            return false;
        }

        if maximized {
            self.restore_geometry = Some((self.location, self.size));
            self.restore_floating = self.floating;
            self.maximized = true;
            self.floating = true;
            self.location = (0, 0).into();
            let height = (output_size.h - DECORATION_HEIGHT).max(1);
            self.size = Size::from((output_size.w.max(1), height));
        } else {
            self.maximized = false;
            if let Some((location, size)) = self.restore_geometry.take() {
                self.location = location;
                self.size = size;
            }
            if !self.forced_floating {
                self.floating = self.restore_floating;
            }
        }

        self.decoration_commit.increment();
        true
    }

    pub fn cycle_opacity(&mut self) {
        self.opacity = if self.opacity <= 0.75 {
            1.0
        } else if self.opacity <= 0.9 {
            0.7
        } else {
            0.85
        };
        self.decoration_commit.increment();
    }

    pub fn set_geometry(
        &mut self,
        location: Point<i32, Logical>,
        size: Size<i32, Logical>,
    ) -> bool {
        let changed = self.location != location || self.size != size;
        if changed {
            self.location = location;
            self.size = size;
            self.decoration_commit.increment();
        }
        changed
    }

    pub fn set_size(&mut self, size: Size<i32, Logical>) -> bool {
        if self.size != size {
            self.size = size;
            self.decoration_commit.increment();
            return true;
        }
        false
    }

    pub fn set_location(&mut self, location: Point<i32, Logical>) {
        if self.location != location {
            self.location = location;
            self.decoration_commit.increment();
        }
    }

    pub fn configure(&self) {
        self.toplevel.with_pending_state(|state| {
            state.size = Some(self.size);
        });
        self.toplevel.send_configure();
    }

    pub fn outer_rect(&self) -> Rectangle<i32, Logical> {
        Rectangle::new(self.location, self.outer_size())
    }

    pub fn outer_size(&self) -> Size<i32, Logical> {
        Size::from((self.size.w, self.size.h + DECORATION_HEIGHT))
    }

    pub fn decoration_element(&self, scale: Scale<f64>, focused: bool) -> SolidColorRenderElement {
        let color = if focused {
            self.decoration_focused
        } else {
            self.decoration_unfocused
        };
        let color = Color32F::new(
            color.r(),
            color.g(),
            color.b(),
            (color.a() * self.opacity).clamp(0.05, 1.0),
        );

        let size = Size::from((self.size.w, DECORATION_HEIGHT.max(1)));
        let rect = Rectangle::new(self.location, size).to_physical_precise_round(scale);
        SolidColorRenderElement::new(
            self.decoration_id.clone(),
            rect,
            self.decoration_commit,
            color,
            Kind::Unspecified,
        )
    }

    pub fn set_decoration_colors(&mut self, focused: Color32F, unfocused: Color32F) {
        self.decoration_focused = focused;
        self.decoration_unfocused = unfocused;
    }
}

pub struct WindowManager {
    windows: Vec<Window>,
    focused: Option<WindowId>,
    next_id: u64,
    current_workspace: usize,
    workspace_count: usize,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            focused: None,
            next_id: 1,
            current_workspace: 0,
            workspace_count: 4,
        }
    }

    pub fn set_decoration_colors(&mut self, focused: Color32F, unfocused: Color32F) {
        for window in &mut self.windows {
            window.set_decoration_colors(focused, unfocused);
        }
    }

    pub fn set_workspace_count(&mut self, count: usize) {
        if count > 0 && count != self.workspace_count {
            let old_count = self.workspace_count;
            self.workspace_count = count;
            if self.current_workspace >= count {
                self.current_workspace = count.saturating_sub(1);
            }
            self.windows.retain(|w| w.workspace() < count);
            if self.focused.is_some() {
                if let Some(focused) = self.focused {
                    let still_valid = self.windows.iter().any(|w| w.id() == focused);
                    if !still_valid {
                        self.focused = None;
                    }
                }
            }
            tracing::info!("Workspace count changed from {} to {}", old_count, count);
        }
    }

    pub fn windows(&self) -> &[Window] {
        &self.windows
    }

    pub fn windows_mut(&mut self) -> &mut [Window] {
        &mut self.windows
    }

    pub fn add_window(&mut self, toplevel: ToplevelSurface) -> WindowId {
        let workspace = self.current_workspace;
        let id = WindowId(self.next_id);
        self.next_id += 1;
        self.windows.push(Window::new(id, toplevel, workspace));
        self.focus_window(id);
        id
    }

    pub fn focus_window(&mut self, id: WindowId) -> bool {
        if !self.windows.iter().any(|window| {
            window.id == id && !window.is_minimized() && window.workspace == self.current_workspace
        }) {
            return false;
        }

        self.focused = Some(id);
        if let Some(pos) = self.windows.iter().position(|window| window.id == id) {
            let window = self.windows.remove(pos);
            self.windows.push(window);
        }
        true
    }

    pub fn focus_next(&mut self) -> Option<WindowId> {
        let ids: Vec<WindowId> = self
            .windows
            .iter()
            .filter(|window| !window.is_minimized() && window.workspace == self.current_workspace)
            .map(|window| window.id)
            .collect();
        if ids.is_empty() {
            return None;
        }
        let next = match self.focused {
            None => ids.last().copied(),
            Some(id) => {
                let idx = ids
                    .iter()
                    .position(|window_id| *window_id == id)
                    .unwrap_or(0);
                let next_idx = (idx + 1) % ids.len();
                Some(ids[next_idx])
            }
        };
        if let Some(id) = next {
            self.focus_window(id);
        }
        next
    }

    pub fn focus_prev(&mut self) -> Option<WindowId> {
        let ids: Vec<WindowId> = self
            .windows
            .iter()
            .filter(|window| !window.is_minimized() && window.workspace == self.current_workspace)
            .map(|window| window.id)
            .collect();
        if ids.is_empty() {
            return None;
        }
        let prev = match self.focused {
            None => ids.last().copied(),
            Some(id) => {
                let idx = ids
                    .iter()
                    .position(|window_id| *window_id == id)
                    .unwrap_or(0);
                let prev_idx = if idx == 0 { ids.len() - 1 } else { idx - 1 };
                Some(ids[prev_idx])
            }
        };
        if let Some(id) = prev {
            self.focus_window(id);
        }
        prev
    }

    pub fn focused_window(&self) -> Option<&Window> {
        self.focused.and_then(|id| {
            self.windows
                .iter()
                .find(|window| window.id == id && window.workspace == self.current_workspace)
        })
    }

    pub fn focused_window_mut(&mut self) -> Option<&mut Window> {
        let focused = self.focused?;
        self.windows
            .iter_mut()
            .find(|window| window.id == focused && window.workspace == self.current_workspace)
    }

    pub fn window_at(&self, point: Point<f64, Logical>) -> Option<WindowId> {
        let point = Point::<i32, Logical>::from((point.x as i32, point.y as i32));
        self.windows
            .iter()
            .rev()
            .find(|window| {
                !window.is_minimized()
                    && window.workspace == self.current_workspace
                    && window.outer_rect().contains(point)
            })
            .map(|window| window.id)
    }

    pub fn window_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.iter_mut().find(|window| window.id == id)
    }

    pub fn window_id_for_surface(
        &self,
        surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
    ) -> Option<WindowId> {
        self.windows
            .iter()
            .find(|window| window.wl_surface() == surface)
            .map(|window| window.id)
    }

    pub fn remove_window(&mut self, id: WindowId) -> Option<Window> {
        let pos = self.windows.iter().position(|window| window.id == id)?;
        let removed = self.windows.remove(pos);
        if self.focused == Some(id) {
            self.focused = None;
        }
        Some(removed)
    }

    pub fn set_floating(&mut self, id: WindowId, floating: bool) -> bool {
        self.window_mut(id)
            .map(|window| window.set_floating(floating))
            .unwrap_or(false)
    }

    pub fn set_forced_floating(&mut self, id: WindowId, forced: bool) -> bool {
        self.window_mut(id)
            .map(|window| window.set_forced_floating(forced))
            .unwrap_or(false)
    }

    pub fn set_minimized(&mut self, id: WindowId, minimized: bool) -> bool {
        self.window_mut(id)
            .map(|window| window.set_minimized(minimized))
            .unwrap_or(false)
    }

    pub fn set_maximized(
        &mut self,
        id: WindowId,
        maximized: bool,
        output_size: Size<i32, Logical>,
    ) -> bool {
        self.window_mut(id)
            .map(|window| window.set_maximized(maximized, output_size))
            .unwrap_or(false)
    }

    pub fn current_workspace(&self) -> usize {
        self.current_workspace
    }

    pub fn workspace_count(&self) -> usize {
        self.workspace_count
    }

    pub fn set_current_workspace(&mut self, workspace: usize) -> bool {
        if workspace >= self.workspace_count {
            return false;
        }
        if self.current_workspace != workspace {
            self.current_workspace = workspace;
            self.focused = None;
            return true;
        }
        false
    }

    pub fn next_workspace(&mut self) -> bool {
        let next = (self.current_workspace + 1) % self.workspace_count;
        self.set_current_workspace(next)
    }

    pub fn prev_workspace(&mut self) -> bool {
        let prev = if self.current_workspace == 0 {
            self.workspace_count - 1
        } else {
            self.current_workspace - 1
        };
        self.set_current_workspace(prev)
    }

    pub fn move_window_to_workspace(&mut self, id: WindowId, workspace: usize) -> bool {
        if workspace >= self.workspace_count {
            return false;
        }
        self.window_mut(id)
            .map(|window| window.set_workspace(workspace))
            .unwrap_or(false)
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}
