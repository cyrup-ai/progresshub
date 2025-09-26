# Ratatui v0.30.0 Theming Guide

## Overview
Ratatui v0.30.0 introduces significant enhancements to theming, including a fluent styling API, RGB color support, and more flexible style composition.

## Core Theming Components

### Style API
The new fluent API allows chaining style methods:

```rust
use ratatui::style::{Color, Modifier, Style, Stylize};

// Fluent API (v0.30.0+)
let style = Style::new()
    .fg(Color::Green)
    .bg(Color::Black)
    .bold()
    .italic();

// Alternative fluent syntax
let style = Style::default().green().bold().bg(Color::Rgb(50, 50, 50));
```

### Color Support
Enhanced color support including RGB values:

```rust
use ratatui::style::Color;

// Basic colors
Color::Red
Color::Green
Color::Blue
Color::Yellow
Color::Magenta
Color::Cyan
Color::White
Color::Black
Color::Gray
Color::DarkGray

// Light variants
Color::LightRed
Color::LightGreen
Color::LightBlue
Color::LightYellow
Color::LightMagenta
Color::LightCyan

// RGB colors (new in v0.30.0)
Color::Rgb(255, 100, 50)
Color::Rgb(100, 200, 100)  // Pastel green
Color::Rgb(200, 100, 100)  // Soft red
```

### Modifiers
Text styling modifiers:

```rust
use ratatui::style::Modifier;

Modifier::BOLD
Modifier::DIM
Modifier::ITALIC
Modifier::UNDERLINED
Modifier::REVERSED
Modifier::CROSSED_OUT
Modifier::RAPID_BLINK
Modifier::HIDDEN
```

## Creating a Theme System

### Theme Structure
```rust
pub struct AppTheme {
    pub primary: Style,
    pub secondary: Style,
    pub success: Style,
    pub warning: Style,
    pub error: Style,
    pub info: Style,
    pub background: Style,
    pub foreground: Style,
    pub accent: Style,
}

impl AppTheme {
    pub fn default() -> Self {
        Self {
            primary: Style::new().fg(Color::Cyan).bold(),
            secondary: Style::new().fg(Color::Gray),
            success: Style::new().fg(Color::Green).bold(),
            warning: Style::new().fg(Color::Yellow).bold(),
            error: Style::new().fg(Color::Red).bold(),
            info: Style::new().fg(Color::Blue),
            background: Style::new().bg(Color::Black),
            foreground: Style::new().fg(Color::White),
            accent: Style::new().fg(Color::Magenta).bold(),
        }
    }
    
    pub fn cyrup() -> Self {
        Self {
            primary: Style::new().fg(Color::Rgb(100, 255, 100)).bold(),
            secondary: Style::new().fg(Color::Rgb(200, 200, 200)),
            success: Style::new().fg(Color::Rgb(100, 200, 100)).bold(),
            warning: Style::new().fg(Color::Rgb(255, 200, 100)).bold(),
            error: Style::new().fg(Color::Rgb(255, 160, 122)).bold(),
            info: Style::new().fg(Color::Rgb(100, 230, 230)),
            background: Style::new().bg(Color::Black),
            foreground: Style::new().fg(Color::White),
            accent: Style::new().fg(Color::Rgb(255, 100, 255)).bold(),
        }
    }
}
```

### Widget-Specific Theming
```rust
impl AppTheme {
    pub fn gauge_style(&self) -> Style {
        self.primary.bg(Color::Black).add_modifier(Modifier::ITALIC)
    }
    
    pub fn progress_bar(&self, progress: f64) -> Style {
        match progress {
            p if p < 0.3 => self.error,
            p if p < 0.7 => self.warning,
            _ => self.success,
        }
    }
    
    pub fn list_item_selected(&self) -> Style {
        self.primary.bg(Color::DarkGray)
    }
    
    pub fn block_border(&self) -> Style {
        self.secondary
    }
}
```

## Theme Application Patterns

### Widget Theming
```rust
// Block with themed borders
let block = Block::bordered()
    .title("Downloads")
    .border_style(theme.block_border())
    .title_style(theme.primary);

// Gauge with dynamic coloring
let gauge = Gauge::default()
    .block(Block::new().title("Progress"))
    .gauge_style(theme.progress_bar(progress))
    .ratio(progress);

// List with selection highlighting
let list = List::new(items)
    .block(Block::bordered().title("Models"))
    .highlight_style(theme.list_item_selected())
    .highlight_symbol("> ");
```

### Conditional Styling
```rust
// Status-based coloring
let status_style = match status {
    DownloadStatus::Pending => theme.secondary,
    DownloadStatus::Downloading => theme.info,
    DownloadStatus::Completed => theme.success,
    DownloadStatus::Failed => theme.error,
};

// Speed-based bandwidth widget
let speed_style = match bandwidth_mbps {
    x if x < 10.0 => theme.error,
    x if x < 50.0 => theme.warning, 
    x if x < 100.0 => theme.success,
    _ => theme.accent,
};
```

## Advanced Theming Techniques

### Style Composition
```rust
// Combining styles
let combined_style = base_style
    .patch(theme.primary)
    .add_modifier(Modifier::BOLD);

// Style inheritance
impl Widget for CustomWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let style = self.base_style
            .patch(self.theme.primary)
            .add_modifier(if self.selected { 
                Modifier::BOLD 
            } else { 
                Modifier::empty() 
            });
    }
}
```

### Dark/Light Theme Support
```rust
pub enum ThemeVariant {
    Dark,
    Light,
}

impl AppTheme {
    pub fn new(variant: ThemeVariant) -> Self {
        match variant {
            ThemeVariant::Dark => Self::dark(),
            ThemeVariant::Light => Self::light(),
        }
    }
    
    fn dark() -> Self {
        Self {
            background: Style::new().bg(Color::Black),
            foreground: Style::new().fg(Color::White),
            primary: Style::new().fg(Color::Cyan).bold(),
            // ... rest of dark theme
        }
    }
    
    fn light() -> Self {
        Self {
            background: Style::new().bg(Color::White),
            foreground: Style::new().fg(Color::Black),
            primary: Style::new().fg(Color::Blue).bold(),
            // ... rest of light theme
        }
    }
}
```

## Best Practices

### Performance
- Define styles once and reuse them
- Avoid creating styles in render loops
- Use theme constants for frequently used styles

### Consistency
- Create semantic color mappings (success, error, warning)
- Use consistent modifiers across similar UI elements
- Maintain visual hierarchy through color intensity

### Accessibility
- Ensure sufficient color contrast
- Don't rely solely on color for information
- Test with different terminal color schemes

### Example Usage
```rust
// In your app
struct App {
    theme: AppTheme,
    // ... other fields
}

impl App {
    fn render_bandwidth_widget(&self, frame: &mut Frame, area: Rect) {
        let speed_style = self.theme.speed_indicator(self.bandwidth_mbps);
        
        let bandwidth_text = vec![
            Line::from(vec![
                Span::styled("🌐 ", speed_style),
                Span::styled(
                    format!("{:.1} Mbps", self.bandwidth_mbps),
                    speed_style.add_modifier(Modifier::BOLD)
                ),
            ])
        ];
        
        let widget = Paragraph::new(bandwidth_text)
            .block(Block::bordered().border_style(self.theme.block_border()));
            
        frame.render_widget(widget, area);
    }
}
```

## Theme Configuration
Themes can be loaded from configuration files or environment variables:

```rust
// From config file
impl AppTheme {
    pub fn from_config(config: &Config) -> Result<Self> {
        Ok(Self {
            primary: parse_style(&config.colors.primary)?,
            secondary: parse_style(&config.colors.secondary)?,
            // ... rest of theme parsing
        })
    }
}

// Environment variable support
impl AppTheme {
    pub fn from_env() -> Self {
        let variant = std::env::var("THEME_VARIANT")
            .unwrap_or_else(|_| "dark".to_string());
            
        match variant.as_str() {
            "light" => Self::light(),
            "cyrup" => Self::cyrup(),
            _ => Self::dark(),
        }
    }
}
```
