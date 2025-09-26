# Ratatui v0.30 Guide for ProgressHub

## Critical Changes in v0.30
- Widget trait now requires `fn render(self, area: Rect, buf: &mut Buffer)`
- No more `Frame` parameter in render methods
- StatefulWidget for interactive components
- Direct buffer manipulation for custom widgets

## Essential Patterns for ProgressHub

### 1. Basic Widget Implementation (for static components)
```rust
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::Widget,
};

struct MyWidget {
    text: String,
    style: Style,
}

impl Widget for MyWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_string(area.x, area.y, &self.text, self.style);
    }
}
```

### 2. Frame Rendering in App (app.rs pattern)
```rust
terminal.draw(|frame| {
    let area = frame.area();
    
    // Apply background style to entire frame
    frame.render_widget(Block::default().style(CyrupTheme::default_style()), area);
    
    // Layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),    // Header
            Constraint::Min(0),       // Content
            Constraint::Length(4),    // Bottom bar (taller for sparkline)
        ])
        .split(area);
    
    // Render widgets
    frame.render_widget(my_widget, chunks[0]);
})?;
```

### 3. Sparkline for Bandwidth Graph
```rust
use ratatui::widgets::{Sparkline, Block};

// In bottom bar rendering:
let sparkline = Sparkline::default()
    .block(Block::default())
    .data(&bandwidth_history) // Vec<u64> of values
    .style(CyrupTheme::gauge_style(0.5))
    .max(100); // Max expected value

frame.render_widget(sparkline, area);
```

### 4. StatefulWidget for Collapsibles
```rust
use ratatui::widgets::StatefulWidget;

struct CollapsibleWidget {
    title: String,
    content: Vec<String>,
}

struct CollapsibleState {
    is_expanded: bool,
}

impl StatefulWidget for CollapsibleWidget {
    type State = CollapsibleState;
    
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Render title row
        let title_style = if state.is_expanded {
            CyrupTheme::selected_style()
        } else {
            CyrupTheme::default_style()
        };
        
        let icon = if state.is_expanded { "▼" } else { "▶" };
        buf.set_string(area.x, area.y, &format!("{} {}", icon, self.title), title_style);
        
        // Render content if expanded
        if state.is_expanded && area.height > 1 {
            for (i, line) in self.content.iter().enumerate() {
                if i + 1 < area.height as usize {
                    buf.set_string(area.x + 2, area.y + i as u16 + 1, line, CyrupTheme::secondary_style());
                }
            }
        }
    }
}

// In app.rs:
frame.render_stateful_widget(collapsible, area, &mut collapsible_state);
```

### 5. Progress Bar (Gauge)
```rust
use ratatui::widgets::{Gauge, Block, Borders};

let progress = 0.65; // 65%
let gauge = Gauge::default()
    .block(Block::default().borders(Borders::ALL))
    .gauge_style(CyrupTheme::gauge_style(progress))
    .percent((progress * 100.0) as u16)
    .label(format!("{:.1}%", progress * 100.0));

frame.render_widget(gauge, area);
```

### 6. Layout with Adaptive Columns
```rust
// Determine column count based on terminal width
let column_count = match frame.area().width {
    0..=159 => 1,
    160..=239 => if downloads.len() >= 3 { 2 } else { 1 },
    _ => if downloads.len() >= 5 { 3 } else { 2 },
};

// Create column constraints
let constraints = vec![Constraint::Ratio(1, column_count); column_count];
let columns = Layout::default()
    .direction(Direction::Horizontal)
    .constraints(constraints)
    .split(content_area);
```

### 7. List with Scroll (for model list)
```rust
use ratatui::widgets::{List, ListItem, ListState};

// Create list items (bottom-up: reverse before display)
let items: Vec<ListItem> = downloads
    .iter()
    .rev() // Bottom-up display
    .map(|d| {
        let style = match d.status {
            DownloadStatus::Completed => CyrupTheme::progress_completed(),
            DownloadStatus::Downloading => CyrupTheme::progress_downloading(),
            _ => CyrupTheme::progress_pending(),
        };
        ListItem::new(d.model_name.clone()).style(style)
    })
    .collect();

let list = List::new(items)
    .block(Block::default())
    .highlight_style(CyrupTheme::selected_style());

// Render with state for selection/scrolling
frame.render_stateful_widget(list, area, &mut list_state);
```

### 8. Event Handling Pattern (from async example)
```rust
use crossterm::event::{Event, EventStream};
use futures::StreamExt;

let mut event_stream = EventStream::new();

tokio::select! {
    event = event_stream.next() => {
        if let Some(Ok(Event::Key(key))) = event {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Up => list_state.previous(),
                KeyCode::Down => list_state.next(),
                _ => {}
            }
        }
    }
    // Other async streams...
}
```

## Common Pitfalls & Solutions

1. **"Widget trait not implemented"**
   - You're using old v0.28 patterns
   - Check that render takes `(self, area, buf)` not `(self, area, buf, frame)`

2. **Sparkline not showing**
   - Data must be `&[u64]` not `&[f64]`
   - Convert: `data.iter().map(|&x| x as u64).collect()`

3. **Colors not applying**
   - Apply base style to Block first: `Block::default().style(theme_style)`
   - Then add widget-specific styles

4. **Layout overflow**
   - Always check `area.height` before rendering rows
   - Use `area.intersection()` to clip rendering area

## ProgressHub-Specific Snippets

### Bottom Bar with 3 Widgets
```rust
let bottom_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
        Constraint::Ratio(1, 3),  // Remaining
        Constraint::Ratio(1, 3),  // Progress
        Constraint::Ratio(1, 3),  // Bandwidth
    ])
    .split(bottom_bar_area);
```

### Model Card Title Bar
```rust
// In buffer rendering for collapsible
let fill_width = area.width.saturating_sub(title.len() as u16 + progress_text.len() as u16 + 2);
let separator = "─".repeat(fill_width as usize);
let full_line = format!("{} {} {}", title, separator, progress_text);
buf.set_string(area.x, area.y, &full_line, style);
```

### Auto-exit After Completion
```rust
// In app state
if all_downloads_complete && !self.exit_timer_started {
    self.exit_timer = Some(tokio::time::Instant::now());
    self.exit_message = Some("All downloads completed! Exiting in 3 seconds...".to_string());
}

if let Some(timer) = self.exit_timer {
    if timer.elapsed() > Duration::from_secs(3) {
        self.should_quit = true;
    }
}
```

Be Useful. Not Thorough.