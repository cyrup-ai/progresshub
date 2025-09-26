# ProgressHub UI Specification

## CRITICAL IMPLEMENTATION NOTES

### Duplicate Files Explanation
- **Multiple implementations exist for exploration/comparison** - This is INTENTIONAL
- `client.rs` vs `client_fixed.rs` - Different approaches to the same problem
- Multiple collapsible widgets - Testing different UI patterns
- **DO NOT** assume duplicates are mistakes or try to "clean them up"
- **DO NOT** delete files that seem redundant

### Dependencies
- `progress_tracking` was removed from Cargo.toml - this is INTENTIONAL
- The xet-core dependencies are there but may not be the final approach
- Downloads may not actually happen yet - this is a UI prototype first

### Current State
- This is a **UI prototype** - focus on the display, not the backend
- Multiple approaches exist to compare and evaluate
- Not everything is wired together - that's expected at this stage

## Terminal Configuration
- **Font**: FiraCode Nerd Font Mono (embedded)
- **Theme**: CyrupTheme (already implemented in theme.rs)
- **Background**: Clean terminal background (no gradients)
- **Header**: Minimal or removed entirely - no wasted vertical space

## Bottom Bar Layout (Fixed)

The bottom bar contains three widgets that are always visible:

### Remaining Widget (Bottom Left)
- **Position**: Bottom left with padding
- **Alignment**: Text elements right-aligned within widget
- **Padding**: Right padding to prevent flush against progress meter
- **Format**: `Remaining: {models} Models | {files} Files`
  - **Models**: Count of models not yet at 100% completion (remaining only)
  - **Files**: Count of actual files (config.json, model.bin, etc.) not yet at 100% completion
- **Icons**: Each metric should have an appropriate icon preceding the number

### Overall Progress Meter (Bottom Center)
- **Position**: Center bottom with padding left/right
- **Display**: Progress bar showing overall completion
- **Calculation**: 
  ```
  progress_percentage = (total_bytes_downloaded / total_bytes_all_files * 1000.0).round() / 10.0
  ```
  (Shows one decimal place for smooth updates)
- **Height**: Taller than progress bars nested in collapsibles
- **Border**: Different border shape/theme than nested progress bars
- **Color Coding**: Uses CyrupTheme::gauge_style() dynamic coloring:
  - 0-30%: Error color (red)
  - 30-70%: Warning color (yellow/orange)
  - 70-100%: Success color (green)

### Bandwidth Monitor (Bottom Right)
- **Position**: Bottom right with padding
- **Alignment**: Left-aligned within widget area
- **Display**: 
  - Top: Centered text showing `{value} Mb/s` or `{value} Gb/s` (auto-scale based on speed)
  - Bottom: Sparkline graph (2 lines tall, no axes)
- **Graph Details**:
  - Shows 60 seconds of history (matches 1Hz sampling rate)
  - Simple line graph without axes or labels
  - Uses CyrupTheme colors

## Main Display Area

### Scrolling Behavior
- **Bottom-up display**: Newest/most recent models appear at the bottom
- **Auto-scroll**: Once a model scrolls off the top, it's gone from view
- **Scroll indicator**: Visual indicator when models exist above viewport
- **No scrollbar**: Content naturally flows up as new models are added
- **No keyboard/mouse interaction**: Pure display, no user controls

### Model Collapsibles (Per New Spec)
- **Summary row** (always visible):
  - Status icon (pending ▢ / active ▣ / done ✔)
  - Model ID text
  - Overall progress bytes and percentage
  - Mini progress bar + percentage on right side
  - Horizontal bar fills remaining width
- **File list** (when expanded):
  - Indented file entries
  - One line per file: filename, bytes, individual progress bar
  - Scrolls internally if exceeds widget space
  - Each file has its own progress bar
- **States**:
  - All widgets start expanded when first shown
  - Completed models collapse and turn green (remain visible)
  - Thin blank line separates adjacent widgets

### Adaptive Layout Rules (Per New Spec)

1. **Column count** (re-evaluated on terminal resize):
   - < 160 cols → 1 column
   - 160-239 cols and ≥ 3 widgets → 2 columns
   - ≥ 240 cols and ≥ 5 widgets → 3 columns

2. **Width allocation**:
   - Columns share horizontal space equally
   - Bars truncate/ellipsize text (no wrapping)

3. **Height handling**:
   - Standard terminal scrolling when content exceeds viewport

## Program Lifecycle

### Completion Behavior
- When all downloads reach 100%:
  - Display success message ("All downloads completed!" or similar)
  - Wait 3 seconds
  - Auto-exit gracefully

## Important Notes
- **No keyboard shortcuts**: Display only, no interaction (except terminal scrolling)
- **No mouse support**: Pure progress display
- **Cmd+C preserved**: Terminal interrupt still works normally
- **Theme required**: Must use CyrupTheme from theme.rs throughout