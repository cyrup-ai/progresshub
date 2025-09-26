use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, StatefulWidget, Widget},
};

use crate::ui::theme::CyrupTheme;

/// State for managing collapsible component expansion
#[derive(Debug, Clone, Default)]
pub struct CollapsibleState {
    /// Whether the content is expanded or collapsed
    pub expanded: bool,
    /// Whether this item is currently selected
    pub selected: bool,
}

impl CollapsibleState {
    /// Create new collapsible state
    pub fn new(expanded: bool) -> Self {
        Self {
            expanded,
            selected: false,
        }
    }

    /// Toggle the expanded state
    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }

    /// Set expanded state
    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }

    /// Set selected state
    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }
}

/// A function that can render content to a buffer
pub type ContentRenderer<'a> = Box<dyn FnOnce(Rect, &mut Buffer) + 'a>;

/// Collapsible widget that can be expanded or collapsed
pub struct Collapsible<'a> {
    /// Title to display in the header
    title: Line<'a>,
    /// Function to render content when expanded
    content_renderer: Option<ContentRenderer<'a>>,
    /// Optional outer block
    block: Option<Block<'a>>,
    /// Style for the header area
    header_style: Style,
    /// Style when selected
    selected_style: Style,
    /// Collapsed indicator (usually ►)
    collapsed_indicator: &'static str,
    /// Expanded indicator (usually ▼)
    expanded_indicator: &'static str,
    /// Height when collapsed
    collapsed_height: u16,
}

impl<'a> Collapsible<'a> {
    /// Create new collapsible widget with title
    pub fn new<T>(title: T) -> Self
    where
        T: Into<Line<'a>>,
    {
        Self {
            title: title.into(),
            content_renderer: None,
            block: None,
            header_style: CyrupTheme::primary_text(),
            selected_style: CyrupTheme::selected_style(),
            collapsed_indicator: "►",
            expanded_indicator: "▼",
            collapsed_height: 1,
        }
    }

    /// Set content renderer function
    pub fn content<F>(mut self, renderer: F) -> Self
    where
        F: FnOnce(Rect, &mut Buffer) + 'a,
    {
        self.content_renderer = Some(Box::new(renderer));
        self
    }

    /// Set content from a widget
    pub fn content_widget<W>(mut self, widget: W) -> Self
    where
        W: Widget + 'a,
    {
        self.content_renderer = Some(Box::new(move |area, buf| {
            widget.render(area, buf);
        }));
        self
    }

    /// Set outer block for styling
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set header style
    pub fn header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }

    /// Set selected style
    pub fn selected_style(mut self, style: Style) -> Self {
        self.selected_style = style;
        self
    }

    /// Set custom indicators
    pub fn indicators(mut self, collapsed: &'static str, expanded: &'static str) -> Self {
        self.collapsed_indicator = collapsed;
        self.expanded_indicator = expanded;
        self
    }

    /// Set collapsed height
    pub fn collapsed_height(mut self, height: u16) -> Self {
        self.collapsed_height = height.max(1);
        self
    }
}

impl<'a> StatefulWidget for Collapsible<'a> {
    type State = CollapsibleState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Apply outer block if provided
        let inner_area = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        if inner_area.height == 0 {
            return;
        }

        // Choose appropriate indicator based on state
        let indicator = if state.expanded {
            self.expanded_indicator
        } else {
            self.collapsed_indicator
        };

        // Build header line with indicator and title
        let mut header = self.title.clone();
        header.spans.insert(
            0,
            Span::styled(
                format!("{indicator} "),
                if state.selected {
                    self.selected_style
                } else {
                    CyrupTheme::muted_style()
                },
            ),
        );

        // Determine layout based on expansion state
        if state.expanded {
            // Split area for header and content
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(1)])
                .split(inner_area);

            // Render header
            let header_style = if state.selected {
                self.selected_style
            } else {
                self.header_style
            };

            buf.set_line(chunks[0].x, chunks[0].y, &header, chunks[0].width);
            for x in chunks[0].x..chunks[0].x + chunks[0].width {
                if let Some(cell) = buf.cell_mut((x, chunks[0].y)) {
                    cell.set_style(header_style);
                }
            }

            // Render content if we have space and a renderer
            if let Some(renderer) = self.content_renderer
                && chunks[1].height > 0
            {
                renderer(chunks[1], buf);
            }
        } else {
            // Collapsed: only show header within collapsed height
            let header_area = Rect {
                height: self.collapsed_height.min(inner_area.height),
                ..inner_area
            };

            let header_style = if state.selected {
                self.selected_style
            } else {
                self.header_style
            };

            buf.set_line(header_area.x, header_area.y, &header, header_area.width);
            for x in header_area.x..header_area.x + header_area.width {
                if let Some(cell) = buf.cell_mut((x, header_area.y)) {
                    cell.set_style(header_style);
                }
            }
        }
    }
}

// Stateless rendering - defaults to collapsed
impl<'a> Widget for Collapsible<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = CollapsibleState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}
