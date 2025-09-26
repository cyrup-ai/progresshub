# Ratatui v0.30.0 Components Guide

## Overview
Ratatui v0.30.0 introduces enhanced component architecture with better composition, state management, and reusability patterns.

## Core Component Concepts

### Widget Trait
The foundation of all components:
```rust
use ratatui::{Frame, widgets::Widget, buffer::Buffer, layout::Rect};

pub trait Widget {
    fn render(self, area: Rect, buf: &mut Buffer);
}

// Stateful widget variant
pub trait StatefulWidget {
    type State;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State);
}
```

### Basic Component Structure
```rust
use ratatui::{
    style::{Color, Style},
    widgets::{Block, Paragraph},
    text::Line,
};

pub struct StatusWidget {
    title: String,
    status: String,
    style: Style,
}

impl StatusWidget {
    pub fn new(title: String, status: String) -> Self {
        Self {
            title,
            status,
            style: Style::default(),
        }
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for StatusWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title(self.title)
            .border_style(self.style);
            
        let paragraph = Paragraph::new(Line::from(self.status))
            .block(block)
            .style(self.style);
            
        paragraph.render(area, buf);
    }
}
```

## Stateful Components

### State Management Pattern
```rust
use ratatui::widgets::ListState;

pub struct ModelListWidget {
    items: Vec<String>,
    block: Option<Block<'static>>,
    style: Style,
}

pub struct ModelListState {
    list_state: ListState,
    selected: Option<usize>,
}

impl ModelListState {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            selected: None,
        }
    }
    
    pub fn select_next(&mut self, len: usize) {
        let next = match self.selected {
            Some(i) => (i + 1) % len,
            None => 0,
        };
        self.selected = Some(next);
        self.list_state.select(Some(next));
    }
    
    pub fn select_previous(&mut self, len: usize) {
        let previous = match self.selected {
            Some(i) => if i == 0 { len - 1 } else { i - 1 },
            None => 0,
        };
        self.selected = Some(previous);
        self.list_state.select(Some(previous));
    }
}

impl StatefulWidget for ModelListWidget {
    type State = ModelListState;
    
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let items: Vec<ListItem> = self.items
            .into_iter()
            .map(|item| ListItem::new(Line::from(item)))
            .collect();
            
        let list = List::new(items)
            .block(self.block.unwrap_or_default())
            .style(self.style)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");
            
        StatefulWidget::render(list, area, buf, &mut state.list_state);
    }
}
```

## Composite Components

### Layout-Based Components
```rust
use ratatui::layout::{Constraint, Layout, Direction};

pub struct DashboardWidget {
    header: HeaderWidget,
    sidebar: SidebarWidget,
    main_content: MainContentWidget,
    footer: FooterWidget,
}

impl DashboardWidget {
    pub fn new() -> Self {
        Self {
            header: HeaderWidget::default(),
            sidebar: SidebarWidget::default(),
            main_content: MainContentWidget::default(),
            footer: FooterWidget::default(),
        }
    }
}

impl Widget for DashboardWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),      // Header
                Constraint::Min(0),         // Main area
                Constraint::Length(1),      // Footer
            ])
            .split(area);
            
        let content_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(20),     // Sidebar
                Constraint::Min(0),         // Main content
            ])
            .split(main_layout[1]);
            
        // Render components
        self.header.render(main_layout[0], buf);
        self.sidebar.render(content_layout[0], buf);
        self.main_content.render(content_layout[1], buf);
        self.footer.render(main_layout[2], buf);
    }
}
```

## Advanced Component Patterns

### Builder Pattern Components
```rust
pub struct ProgressWidget {
    value: f64,
    max: f64,
    label: Option<String>,
    style: Style,
    block: Option<Block<'static>>,
    show_percentage: bool,
}

impl ProgressWidget {
    pub fn new(value: f64, max: f64) -> Self {
        Self {
            value,
            max,
            label: None,
            style: Style::default(),
            block: None,
            show_percentage: true,
        }
    }
    
    pub fn label<S: Into<String>>(mut self, label: S) -> Self {
        self.label = Some(label.into());
        self
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    
    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }
    
    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }
}

impl Widget for ProgressWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let ratio = (self.value / self.max).clamp(0.0, 1.0);
        
        let label = if self.show_percentage {
            format!("{:.1}%", ratio * 100.0)
        } else {
            self.label.unwrap_or_default()
        };
        
        let gauge = Gauge::default()
            .block(self.block.unwrap_or_default())
            .gauge_style(self.style)
            .ratio(ratio)
            .label(label);
            
        gauge.render(area, buf);
    }
}
```

### Event-Driven Components
```rust
use crossterm::event::{Event, KeyCode};

pub trait EventHandler {
    type Result;
    
    fn handle_event(&mut self, event: Event) -> Self::Result;
}

pub struct InteractiveListWidget {
    items: Vec<String>,
    state: ListState,
    on_select: Option<Box<dyn Fn(usize)>>,
}

impl InteractiveListWidget {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            items,
            state: ListState::default(),
            on_select: None,
        }
    }
    
    pub fn on_select<F>(mut self, callback: F) -> Self 
    where
        F: Fn(usize) + 'static,
    {
        self.on_select = Some(Box::new(callback));
        self
    }
}

impl EventHandler for InteractiveListWidget {
    type Result = bool; // true if event was handled
    
    fn handle_event(&mut self, event: Event) -> Self::Result {
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Down => {
                    let len = self.items.len();
                    let next = match self.state.selected() {
                        Some(i) => (i + 1) % len,
                        None => 0,
                    };
                    self.state.select(Some(next));
                    true
                }
                KeyCode::Up => {
                    let len = self.items.len();
                    let previous = match self.state.selected() {
                        Some(i) => if i == 0 { len - 1 } else { i - 1 },
                        None => 0,
                    };
                    self.state.select(Some(previous));
                    true
                }
                KeyCode::Enter => {
                    if let Some(selected) = self.state.selected() {
                        if let Some(ref callback) = self.on_select {
                            callback(selected);
                        }
                    }
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    }
}
```

## Component Lifecycle

### Initialization and Cleanup
```rust
pub trait ComponentLifecycle {
    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn update(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct AsyncDataWidget {
    data: Option<Vec<String>>,
    loading: bool,
    error: Option<String>,
}

impl ComponentLifecycle for AsyncDataWidget {
    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.loading = true;
        self.error = None;
        // Start async data loading
        Ok(())
    }
    
    fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Check for new data, update state
        Ok(())
    }
    
    fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.data = None;
        self.loading = false;
        Ok(())
    }
}
```

## Real-World Component Examples

### Bandwidth Monitor Component
```rust
use std::collections::VecDeque;

pub struct BandwidthWidget {
    history: VecDeque<f64>,
    current_speed: f64,
    max_points: usize,
    style: Style,
    unit: String,
}

impl BandwidthWidget {
    pub fn new(max_points: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(max_points),
            current_speed: 0.0,
            max_points,
            style: Style::default(),
            unit: "Mbps".to_string(),
        }
    }
    
    pub fn update_speed(&mut self, speed: f64) {
        self.current_speed = speed;
        
        if self.history.len() >= self.max_points {
            self.history.pop_front();
        }
        self.history.push_back(speed);
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for BandwidthWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let data: Vec<u64> = self.history
            .iter()
            .map(|&speed| speed as u64)
            .collect();
            
        let sparkline = Sparkline::default()
            .block(Block::bordered().title("Bandwidth"))
            .style(self.style)
            .data(&data);
            
        sparkline.render(area, buf);
        
        // Render current speed overlay
        let speed_text = format!("{:.1} {}", self.current_speed, self.unit);
        let speed_area = Rect {
            x: area.x + area.width - speed_text.len() as u16 - 2,
            y: area.y,
            width: speed_text.len() as u16 + 1,
            height: 1,
        };
        
        let speed_widget = Paragraph::new(speed_text)
            .style(self.style.add_modifier(Modifier::BOLD));
            
        speed_widget.render(speed_area, buf);
    }
}
```

### Download Progress Component
```rust
pub struct DownloadProgressWidget {
    downloads: Vec<DownloadItem>,
    layout_mode: LayoutMode,
}

#[derive(Clone)]
pub struct DownloadItem {
    pub name: String,
    pub progress: f64,
    pub speed: f64,
    pub status: DownloadStatus,
}

#[derive(Clone)]
pub enum LayoutMode {
    Compact,
    Detailed,
    Grid,
}

impl DownloadProgressWidget {
    pub fn new(downloads: Vec<DownloadItem>) -> Self {
        Self {
            downloads,
            layout_mode: LayoutMode::Detailed,
        }
    }
    
    pub fn layout_mode(mut self, mode: LayoutMode) -> Self {
        self.layout_mode = mode;
        self
    }
}

impl Widget for DownloadProgressWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.layout_mode {
            LayoutMode::Compact => self.render_compact(area, buf),
            LayoutMode::Detailed => self.render_detailed(area, buf),
            LayoutMode::Grid => self.render_grid(area, buf),
        }
    }
}

impl DownloadProgressWidget {
    fn render_compact(self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self.downloads
            .into_iter()
            .map(|download| {
                let progress_bar = "█".repeat((download.progress * 20.0) as usize);
                let line = format!(
                    "{:<20} [{}] {:.1}%",
                    download.name,
                    progress_bar,
                    download.progress * 100.0
                );
                ListItem::new(Line::from(line))
            })
            .collect();
            
        let list = List::new(items)
            .block(Block::bordered().title("Downloads"));
            
        list.render(area, buf);
    }
    
    fn render_detailed(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                self.downloads
                    .iter()
                    .map(|_| Constraint::Length(3))
                    .collect::<Vec<_>>()
            )
            .split(area);
            
        for (i, download) in self.downloads.into_iter().enumerate() {
            if i < chunks.len() {
                let gauge = Gauge::default()
                    .block(Block::bordered().title(download.name))
                    .gauge_style(match download.status {
                        DownloadStatus::Completed => Style::default().fg(Color::Green),
                        DownloadStatus::Failed => Style::default().fg(Color::Red),
                        _ => Style::default().fg(Color::Yellow),
                    })
                    .ratio(download.progress)
                    .label(format!("{:.1}% @ {:.1} MB/s", 
                        download.progress * 100.0, download.speed));
                        
                gauge.render(chunks[i], buf);
            }
        }
    }
    
    fn render_grid(self, area: Rect, buf: &mut Buffer) {
        let cols = 2;
        let rows = (self.downloads.len() + cols - 1) / cols;
        
        let row_constraints: Vec<Constraint> = (0..rows)
            .map(|_| Constraint::Length(3))
            .collect();
            
        let row_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(area);
            
        for (row, chunk) in row_chunks.iter().enumerate() {
            let col_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(*chunk);
                
            for col in 0..cols {
                let index = row * cols + col;
                if index < self.downloads.len() {
                    let download = &self.downloads[index];
                    let gauge = Gauge::default()
                        .block(Block::bordered().title(download.name.clone()))
                        .ratio(download.progress);
                        
                    gauge.render(col_chunks[col], buf);
                }
            }
        }
    }
}
```

## Component Testing

### Unit Testing Components
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{buffer::Buffer, layout::Rect};
    
    #[test]
    fn test_status_widget_render() {
        let widget = StatusWidget::new(
            "Test".to_string(),
            "Running".to_string()
        );
        
        let area = Rect::new(0, 0, 20, 5);
        let mut buffer = Buffer::empty(area);
        
        widget.render(area, &mut buffer);
        
        // Assert buffer contents
        let content = buffer.content();
        assert!(content.iter().any(|cell| cell.symbol().contains("Test")));
        assert!(content.iter().any(|cell| cell.symbol().contains("Running")));
    }
}
```

## Best Practices

### Component Design
- Keep components focused on a single responsibility
- Use builder patterns for complex configuration
- Implement proper state management for interactive components
- Make components reusable and composable

### Performance
- Avoid expensive calculations in render methods
- Cache computed values when possible
- Use appropriate layout constraints
- Minimize buffer allocations

### Maintainability
- Document component APIs thoroughly
- Use type-safe state management
- Implement proper error handling
- Follow consistent naming conventions

### Example Integration
```rust
// In your main application
fn render_app(frame: &mut Frame, app: &mut App) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.area());
    
    // Header with bandwidth monitoring
    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(20),
        ])
        .split(main_layout[0]);
    
    let title_widget = Paragraph::new("ProgressHub")
        .block(Block::bordered())
        .style(app.theme.primary);
    frame.render_widget(title_widget, header_layout[0]);
    
    let bandwidth_widget = BandwidthWidget::new(60)
        .style(app.theme.accent);
    frame.render_widget(bandwidth_widget, header_layout[1]);
    
    // Main content with download progress
    let progress_widget = DownloadProgressWidget::new(app.downloads.clone())
        .layout_mode(app.layout_mode);
    frame.render_widget(progress_widget, main_layout[1]);
    
    // Footer with status
    let status_widget = StatusWidget::new(
        "Status".to_string(),
        app.get_status_message(),
    ).style(app.theme.secondary);
    frame.render_widget(status_widget, main_layout[2]);
}
```
