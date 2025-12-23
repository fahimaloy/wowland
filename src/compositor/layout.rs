use smithay::utils::{Logical, Rectangle, Size};

use crate::compositor::window::{Window, DECORATION_HEIGHT};

#[derive(Debug, Clone, Copy)]
pub enum LayoutMode {
    MasterStack { ratio: f32 },
    Grid,
}

impl Default for LayoutMode {
    fn default() -> Self {
        LayoutMode::MasterStack { ratio: 0.5 }
    }
}

impl LayoutMode {
    pub fn next(self) -> Self {
        match self {
            LayoutMode::MasterStack { .. } => LayoutMode::Grid,
            LayoutMode::Grid => LayoutMode::MasterStack { ratio: 0.6 },
        }
    }

    pub fn prev(self) -> Self {
        match self {
            LayoutMode::MasterStack { .. } => LayoutMode::Grid,
            LayoutMode::Grid => LayoutMode::MasterStack { ratio: 0.6 },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Gaps {
    pub inner: i32,
    pub outer: i32,
}

impl Default for Gaps {
    fn default() -> Self {
        Self { inner: 0, outer: 0 }
    }
}

#[derive(Debug, Default)]
pub struct LayoutEngine {
    pub mode: LayoutMode,
    pub gaps: Gaps,
}

impl LayoutEngine {
    pub fn apply(&self, output: Size<i32, Logical>, windows: &mut [Window]) {
        let active: Vec<usize> = windows
            .iter()
            .enumerate()
            .filter(|(_, window)| !window.is_dragging() && !window.is_floating() && !window.is_minimized())
            .map(|(idx, _)| idx)
            .collect();

        if active.is_empty() {
            return;
        }

        let rects = match self.mode {
            LayoutMode::MasterStack { ratio } => master_stack_rects(active.len(), output, self.gaps, ratio),
            LayoutMode::Grid => grid_rects(active.len(), output, self.gaps),
        };

        for (slot, rect) in rects.into_iter().enumerate() {
            let idx = active[slot];
            let client_height = (rect.size.h - DECORATION_HEIGHT).max(1);
            let client_size = Size::from((rect.size.w.max(1), client_height));
            if windows[idx].set_geometry(rect.loc, client_size) {
                windows[idx].configure();
            }
        }
    }
}

fn master_stack_rects(
    count: usize,
    output: Size<i32, Logical>,
    gaps: Gaps,
    ratio: f32,
) -> Vec<Rectangle<i32, Logical>> {
    let area = apply_outer_gaps(output, gaps.outer);
    if count == 1 {
        return vec![area];
    }

    let master_width = ((area.size.w as f32) * ratio) as i32;
    let stack_width = area.size.w - master_width - gaps.inner;

    let master = Rectangle::new(area.loc, Size::from((master_width, area.size.h)));
    let stack_origin = (area.loc.x + master_width + gaps.inner, area.loc.y).into();
    let stack: Rectangle<i32, Logical> = Rectangle::new(stack_origin, Size::from((stack_width, area.size.h)));

    let mut rects = Vec::with_capacity(count);
    rects.push(master);

    let stack_count = count - 1;
    let stack_height_total = stack.size.h - gaps.inner * (stack_count as i32 - 1).max(0);
    let stack_height = (stack_height_total / stack_count as i32).max(1);

    for i in 0..stack_count {
        let y = stack.loc.y + i as i32 * (stack_height + gaps.inner);
        rects.push(Rectangle::new((stack.loc.x, y).into(), Size::from((stack.size.w, stack_height))));
    }

    rects
}

fn grid_rects(count: usize, output: Size<i32, Logical>, gaps: Gaps) -> Vec<Rectangle<i32, Logical>> {
    let area = apply_outer_gaps(output, gaps.outer);
    let columns = (count as f32).sqrt().ceil() as i32;
    let rows = ((count as f32) / columns as f32).ceil() as i32;

    let total_inner_w = gaps.inner * (columns - 1).max(0);
    let total_inner_h = gaps.inner * (rows - 1).max(0);
    let cell_w = ((area.size.w - total_inner_w) / columns).max(1);
    let cell_h = ((area.size.h - total_inner_h) / rows).max(1);

    let mut rects = Vec::with_capacity(count);
    for i in 0..count {
        let col = i as i32 % columns;
        let row = i as i32 / columns;
        let x = area.loc.x + col * (cell_w + gaps.inner);
        let y = area.loc.y + row * (cell_h + gaps.inner);
        rects.push(Rectangle::new((x, y).into(), Size::from((cell_w, cell_h))));
    }

    rects
}

fn apply_outer_gaps(output: Size<i32, Logical>, outer: i32) -> Rectangle<i32, Logical> {
    let size = Size::from((
        (output.w - outer * 2).max(1),
        (output.h - outer * 2).max(1),
    ));
    Rectangle::new((outer, outer).into(), size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn master_stack_rects_returns_master_and_stack() {
        let rects = master_stack_rects(2, Size::from((1000, 800)), Gaps::default(), 0.6);
        assert_eq!(rects.len(), 2);
        assert!(rects[0].size.w > rects[1].size.w);
    }

    #[test]
    fn grid_rects_count_matches() {
        let rects = grid_rects(5, Size::from((1000, 800)), Gaps::default());
        assert_eq!(rects.len(), 5);
    }
}
