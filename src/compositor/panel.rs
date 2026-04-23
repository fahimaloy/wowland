use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::{Id, Kind};
use smithay::backend::renderer::utils::CommitCounter;
use smithay::backend::renderer::Color32F;
use smithay::utils::{Logical, Point, Rectangle, Scale, Size};

pub const PANEL_HEIGHT: i32 = 28;

const PANEL_BG: Color32F = Color32F::new(0.15, 0.17, 0.21, 1.0);

pub struct Panel {
    id: Id,
    commit: CommitCounter,
    current_workspace: usize,
    workspace_count: usize,
    workspace_rects: Vec<(Id, CommitCounter)>,
}

impl Panel {
    pub fn new(workspace_count: usize) -> Self {
        Self {
            id: Id::new(),
            commit: CommitCounter::default(),
            current_workspace: 0,
            workspace_count,
            workspace_rects: Vec::new(),
        }
    }

    pub fn update(&mut self, current_workspace: usize, workspace_count: usize) {
        self.current_workspace = current_workspace;
        self.workspace_count = workspace_count;
        self.commit.increment();
        for rect in &mut self.workspace_rects {
            rect.1.increment();
        }
    }

    pub fn render_elements(
        &mut self,
        scale: Scale<f64>,
        output_size: Size<i32, Logical>,
    ) -> Vec<SolidColorRenderElement> {
        let mut elements = Vec::new();

        let size = Size::from((output_size.w, PANEL_HEIGHT));
        let rect = Rectangle::new(Point::from((0, 0)), size).to_physical_precise_round(scale);
        elements.push(SolidColorRenderElement::new(
            self.id.clone(),
            rect,
            self.commit,
            PANEL_BG,
            Kind::Unspecified,
        ));

        let workspace_width = 30i32;
        let gap = 4;
        let start_x = 10;

        for i in 0..self.workspace_count {
            let x = start_x + i as i32 * (workspace_width + gap);
            let rect = Rectangle::new(
                Point::from((x, 6)),
                Size::from((workspace_width, PANEL_HEIGHT - 12)),
            )
            .to_physical_precise_round(scale);
            let color = if i == self.current_workspace {
                Color32F::new(0.28, 0.32, 0.4, 1.0)
            } else {
                Color32F::new(0.12, 0.14, 0.18, 1.0)
            };

            if self.workspace_rects.len() <= i {
                self.workspace_rects
                    .push((Id::new(), CommitCounter::default()));
            }

            elements.push(SolidColorRenderElement::new(
                self.workspace_rects[i].0.clone(),
                rect,
                self.workspace_rects[i].1,
                color,
                Kind::Unspecified,
            ));
        }

        elements
    }
}
