//! Adaptive layout system for responsive multi-column display with professional spacing

use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};

/// Professional spacing constants for consistent layout
pub struct ProfessionalSpacing;

impl ProfessionalSpacing {
    /// Standard outer margin around the entire application
    pub const OUTER_MARGIN: Margin = Margin {
        vertical: 1,
        horizontal: 2,
    };

    /// Margin between main content and bottom bar
    pub const CONTENT_BOTTOM_SPACING: u16 = 1;

    /// Horizontal spacing between columns in multi-column layout
    pub const COLUMN_SPACING: u16 = 1;

    /// Vertical spacing between widgets in single column
    pub const WIDGET_SPACING: u16 = 1;

    /// Inner padding for widget content areas
    pub const WIDGET_INNER_PADDING: Margin = Margin {
        vertical: 1,
        horizontal: 1,
    };

    /// Minimum height for a widget to be readable
    pub const MIN_WIDGET_HEIGHT: u16 = 4;

    /// Preferred height for optimal widget display
    pub const PREFERRED_WIDGET_HEIGHT: u16 = 8;
}

/// Adaptive layout configuration based on terminal size and content
#[derive(Debug, Clone)]
pub struct AdaptiveLayout {
    /// Terminal width
    terminal_width: u16,
    /// Terminal height
    terminal_height: u16,
    /// Number of widgets to display
    widget_count: usize,
    /// Minimum widget width
    min_widget_width: u16,
    /// Preferred widget width
    preferred_widget_width: u16,
    /// Whether to apply professional spacing
    use_professional_spacing: bool,
}

impl AdaptiveLayout {
    /// Create a new adaptive layout with professional spacing enabled by default
    pub fn new(terminal_width: u16, terminal_height: u16, widget_count: usize) -> Self {
        Self {
            terminal_width,
            terminal_height,
            widget_count,
            min_widget_width: 40,       // Minimum width for readable content
            preferred_widget_width: 60, // Preferred width for optimal display
            use_professional_spacing: true,
        }
    }

    /// Set minimum widget width
    pub fn min_widget_width(mut self, width: u16) -> Self {
        self.min_widget_width = width;
        self
    }

    /// Set preferred widget width  
    pub fn preferred_widget_width(mut self, width: u16) -> Self {
        self.preferred_widget_width = width;
        self
    }

    /// Enable or disable professional spacing
    pub fn professional_spacing(mut self, enabled: bool) -> Self {
        self.use_professional_spacing = enabled;
        self
    }

    /// Apply professional outer margins to an area
    pub fn apply_outer_margins(&self, area: Rect) -> Rect {
        if self.use_professional_spacing && area.width > 4 && area.height > 2 {
            area.inner(ProfessionalSpacing::OUTER_MARGIN)
        } else {
            area
        }
    }

    /// Calculate effective terminal dimensions accounting for professional spacing
    fn effective_dimensions(&self) -> (u16, u16) {
        if self.use_professional_spacing {
            let margin = ProfessionalSpacing::OUTER_MARGIN;
            (
                self.terminal_width.saturating_sub(margin.horizontal * 2),
                self.terminal_height.saturating_sub(
                    margin.vertical * 2 + ProfessionalSpacing::CONTENT_BOTTOM_SPACING,
                ),
            )
        } else {
            (self.terminal_width, self.terminal_height)
        }
    }

    /// Calculate the optimal number of columns based on terminal size and widget count
    pub fn calculate_columns(&self) -> usize {
        let (effective_width, _) = self.effective_dimensions();

        // Account for column spacing in width calculations
        let spacing_adjusted_width = if self.use_professional_spacing {
            effective_width.saturating_sub(ProfessionalSpacing::COLUMN_SPACING)
        } else {
            effective_width
        };

        // Per specification: adaptive column count based on effective width and widget count
        match (spacing_adjusted_width, self.widget_count) {
            // < 160 cols → 1 column always
            (width, _) if width < 160 => 1,

            // 160-239 cols and ≥ 3 widgets → 2 columns
            (width, count) if (160..240).contains(&width) && count >= 3 => 2,

            // ≥ 240 cols and ≥ 5 widgets → 3 columns
            (width, count) if width >= 240 && count >= 5 => 3,

            // Fallback based on available space accounting for spacing
            (width, count) => {
                let max_possible_columns = if self.use_professional_spacing {
                    ((width + ProfessionalSpacing::COLUMN_SPACING)
                        / (self.min_widget_width + ProfessionalSpacing::COLUMN_SPACING))
                        as usize
                } else {
                    (width / self.min_widget_width) as usize
                };

                let desired_columns = if self.use_professional_spacing {
                    ((width + ProfessionalSpacing::COLUMN_SPACING)
                        / (self.preferred_widget_width + ProfessionalSpacing::COLUMN_SPACING))
                        as usize
                } else {
                    (width / self.preferred_widget_width) as usize
                };

                // Use the smaller of: what fits, what's desired, or what we have content for
                std::cmp::min(
                    std::cmp::min(max_possible_columns, desired_columns),
                    count.max(1),
                )
            }
        }
    }

    /// Calculate row count for given column count
    pub fn calculate_rows(&self, columns: usize) -> usize {
        if columns == 0 || self.widget_count == 0 {
            return 0;
        }
        self.widget_count.div_ceil(columns) // Ceiling division
    }

    /// Create layout chunks for the main display area with professional spacing
    pub fn create_main_layout(&self, area: Rect) -> (Vec<Rect>, usize, usize) {
        let columns = self.calculate_columns();
        let rows = self.calculate_rows(columns);

        let min_widget_height = if self.use_professional_spacing {
            ProfessionalSpacing::MIN_WIDGET_HEIGHT
        } else {
            3
        };

        if columns == 1 {
            // Single column layout with professional vertical spacing
            let row_constraints: Vec<Constraint> = if self.use_professional_spacing {
                (0..rows)
                    .flat_map(|i| {
                        let mut constraints = vec![Constraint::Min(min_widget_height)];
                        // Add spacing between widgets (except after last)
                        if i < rows - 1 {
                            constraints
                                .push(Constraint::Length(ProfessionalSpacing::WIDGET_SPACING));
                        }
                        constraints
                    })
                    .collect()
            } else {
                (0..rows)
                    .map(|_| Constraint::Min(min_widget_height))
                    .collect()
            };

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(row_constraints)
                .split(area);

            // Filter out spacing constraints to return only widget areas
            let widget_chunks: Vec<Rect> = if self.use_professional_spacing {
                chunks.iter().step_by(2).copied().collect() // Take every other chunk (skip spacing)
            } else {
                chunks.to_vec()
            };

            (widget_chunks, columns, rows)
        } else {
            // Multi-column layout with professional horizontal spacing
            let column_constraints: Vec<Constraint> = if self.use_professional_spacing {
                (0..columns)
                    .flat_map(|i| {
                        let mut constraints = vec![Constraint::Percentage(100 / columns as u16)];
                        // Add spacing between columns (except after last)
                        if i < columns - 1 {
                            constraints
                                .push(Constraint::Length(ProfessionalSpacing::COLUMN_SPACING));
                        }
                        constraints
                    })
                    .collect()
            } else {
                (0..columns)
                    .map(|_| Constraint::Percentage(100 / columns as u16))
                    .collect()
            };

            let column_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(column_constraints)
                .split(area);

            // Filter out spacing columns to get only content columns
            let content_columns: Vec<Rect> = if self.use_professional_spacing {
                column_chunks.iter().step_by(2).copied().collect() // Take every other chunk (skip spacing)
            } else {
                column_chunks.to_vec()
            };

            // Create row layout within each column with professional spacing
            let mut all_chunks = Vec::new();
            let widgets_per_column = self.widget_count.div_ceil(columns);

            for (col_idx, &column_chunk) in content_columns.iter().enumerate() {
                let start_widget = col_idx * widgets_per_column;
                let end_widget =
                    std::cmp::min(start_widget + widgets_per_column, self.widget_count);
                let widgets_in_column = end_widget - start_widget;

                if widgets_in_column > 0 {
                    let row_constraints: Vec<Constraint> = if self.use_professional_spacing {
                        (0..widgets_in_column)
                            .flat_map(|i| {
                                let mut constraints = vec![Constraint::Min(min_widget_height)];
                                // Add spacing between widgets in column (except after last)
                                if i < widgets_in_column - 1 {
                                    constraints.push(Constraint::Length(
                                        ProfessionalSpacing::WIDGET_SPACING,
                                    ));
                                }
                                constraints
                            })
                            .collect()
                    } else {
                        (0..widgets_in_column)
                            .map(|_| Constraint::Min(min_widget_height))
                            .collect()
                    };

                    let row_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(row_constraints)
                        .split(column_chunk);

                    // Filter out spacing constraints to return only widget areas
                    let widget_chunks: Vec<Rect> = if self.use_professional_spacing {
                        row_chunks.iter().step_by(2).copied().collect() // Take every other chunk (skip spacing)
                    } else {
                        row_chunks.to_vec()
                    };

                    all_chunks.extend(widget_chunks);
                }
            }

            (all_chunks, columns, rows)
        }
    }

    /// Create the complete application layout with professional spacing (main area + bottom bar)
    pub fn create_app_layout(&self, area: Rect) -> (Rect, Rect) {
        // Apply outer margins first
        let content_area = self.apply_outer_margins(area);

        let constraints = if self.use_professional_spacing {
            vec![
                Constraint::Min(5), // Main display area (minimum space)
                Constraint::Length(ProfessionalSpacing::CONTENT_BOTTOM_SPACING), // Spacing between main and bottom bar
                Constraint::Length(3), // Bottom bar (fixed height)
            ]
        } else {
            vec![
                Constraint::Min(5),    // Main display area (minimum space)
                Constraint::Length(3), // Bottom bar (fixed height)
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(content_area);

        if self.use_professional_spacing {
            (chunks[0], chunks[2]) // (main_area, bottom_bar_area) - skip spacing chunk
        } else {
            (chunks[0], chunks[1]) // (main_area, bottom_bar_area)
        }
    }

    /// Get layout info for debugging/display
    pub fn layout_info(&self) -> LayoutInfo {
        let columns = self.calculate_columns();
        let rows = self.calculate_rows(columns);

        LayoutInfo {
            terminal_width: self.terminal_width,
            terminal_height: self.terminal_height,
            widget_count: self.widget_count,
            columns,
            rows,
            widgets_per_column: if columns > 0 {
                self.widget_count.div_ceil(columns)
            } else {
                0
            },
        }
    }
}

/// Layout information for debugging and display
#[derive(Debug, Clone)]
pub struct LayoutInfo {
    pub terminal_width: u16,
    pub terminal_height: u16,
    pub widget_count: usize,
    pub columns: usize,
    pub rows: usize,
    pub widgets_per_column: usize,
}

impl LayoutInfo {
    /// Get a human-readable description of the layout
    pub fn description(&self) -> String {
        format!(
            "{}x{} terminal → {} widgets in {}×{} grid ({} per column)",
            self.terminal_width,
            self.terminal_height,
            self.widget_count,
            self.columns,
            self.rows,
            self.widgets_per_column
        )
    }
}
