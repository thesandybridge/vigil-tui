use ratatui::layout::Rect;

use crate::config::ZoneConfig;

pub struct ZoneLayout {
    pub id: String,
    pub rect: ComputedRect,
    pub min_w: u16,
    pub min_h: u16,
}

/// Stores either absolute-percentage data or pre-computed character positions.
pub enum ComputedRect {
    /// Percentage-based (absolute layout mode)
    Percent {
        pct_x: u16,
        pct_y: u16,
        pct_w: u16,
        pct_h: u16,
    },
    /// Fixed character positions (rows layout mode, computed at layout time)
    Fixed {
        x: u16,
        y: u16,
        w: u16,
        h: u16,
    },
}

impl ZoneLayout {
    pub fn to_rect(&self, terminal_width: u16, terminal_height: u16) -> Rect {
        match &self.rect {
            ComputedRect::Percent {
                pct_x,
                pct_y,
                pct_w,
                pct_h,
            } => {
                let x = (*pct_x as u32 * terminal_width as u32 / 100) as u16;
                let y = (*pct_y as u32 * terminal_height as u32 / 100) as u16;
                let w = (*pct_w as u32 * terminal_width as u32 / 100) as u16;
                let h = (*pct_h as u32 * terminal_height as u32 / 100) as u16;
                Rect::new(x, y, w, h)
            }
            ComputedRect::Fixed { x, y, w, h } => {
                // Width is still percentage-based (of terminal width)
                let actual_w = (*w as u32 * terminal_width as u32 / 100) as u16;
                let actual_x = (*x as u32 * terminal_width as u32 / 100) as u16;
                Rect::new(actual_x, *y, actual_w, *h)
            }
        }
    }
}

/// Build layouts for absolute positioning mode.
pub fn build_absolute(zones: &[ZoneConfig], widgets: &[Box<dyn crate::widget::Widget>]) -> Vec<ZoneLayout> {
    zones
        .iter()
        .zip(widgets.iter())
        .map(|(zone, widget)| {
            let (min_w, min_h) = widget.min_size();
            ZoneLayout {
                id: zone.id.clone(),
                rect: ComputedRect::Percent {
                    pct_x: zone.x,
                    pct_y: zone.y,
                    pct_w: zone.width,
                    pct_h: zone.height,
                },
                min_w: zone.min_width.unwrap_or(min_w),
                min_h: zone.min_height.unwrap_or(min_h),
            }
        })
        .collect()
}

/// A row in the stacked layout, containing one or more columns (zones).
struct RowGroup {
    row_id: u16,
    /// Fixed height in chars, or None = fill remaining space
    fixed_height: Option<u16>,
    /// (zone_index, width_pct) pairs
    columns: Vec<(usize, u16)>,
}

/// Build layouts for rows (stacked) mode.
/// Zones are grouped by `row` field. Within a row, zones are placed as columns.
/// Heights are in characters. Rows without explicit height share remaining space.
pub fn build_rows(
    zones: &[ZoneConfig],
    widgets: &[Box<dyn crate::widget::Widget>],
    terminal_height: u16,
) -> Vec<ZoneLayout> {
    // Group zones into rows, preserving config order
    let mut rows: Vec<RowGroup> = Vec::new();
    let mut auto_row_counter = 0u16;
    let mut row_order: Vec<u16> = Vec::new();

    for (i, zone) in zones.iter().enumerate() {
        let row_id = zone.row.unwrap_or_else(|| {
            auto_row_counter += 1;
            auto_row_counter * 1000 // high numbers to avoid collision with explicit row IDs
        });

        if let Some(existing) = rows.iter_mut().find(|r| r.row_id == row_id) {
            // Add as column to existing row
            let width_pct = if zone.width == 0 { 100 } else { zone.width };
            existing.columns.push((i, width_pct));
        } else {
            // New row
            let (_, widget_min_h) = widgets[i].min_size();
            let fixed_height = if zone.height > 0 {
                Some(zone.height)
            } else {
                // No explicit height — use widget min as the fill-minimum
                Some(widget_min_h)
            };

            let width_pct = if zone.width == 0 { 100 } else { zone.width };
            row_order.push(row_id);
            rows.push(RowGroup {
                row_id,
                fixed_height,
                columns: vec![(i, width_pct)],
            });
        }
    }

    // Sort rows by the order they first appeared
    let ordered_rows: Vec<&RowGroup> = row_order
        .iter()
        .map(|id| rows.iter().find(|r| r.row_id == *id).unwrap())
        .collect();

    // Compute heights: fixed rows get their height, fill rows share the rest
    let total_fixed: u16 = ordered_rows
        .iter()
        .filter_map(|r| r.fixed_height)
        .sum();

    let fill_count = ordered_rows
        .iter()
        .filter(|r| r.fixed_height.is_none())
        .count() as u16;

    let remaining = terminal_height.saturating_sub(total_fixed);
    let fill_height = if fill_count > 0 {
        remaining / fill_count
    } else {
        0
    };

    // Assign positions
    let mut current_y: u16 = 0;
    let mut placement_order: Vec<(usize, ZoneLayout)> = Vec::new();

    for row in &ordered_rows {
        let row_height = row.fixed_height.unwrap_or(fill_height);

        let total_explicit_width: u16 = row.columns.iter().map(|(_, w)| *w).sum();
        let needs_auto = total_explicit_width == 0;

        let mut current_x_pct: u16 = 0;
        for (zone_idx, width_pct) in &row.columns {
            let w = if needs_auto {
                100 / row.columns.len() as u16
            } else {
                *width_pct
            };

            let (min_w, min_h) = widgets[*zone_idx].min_size();
            placement_order.push((*zone_idx, ZoneLayout {
                id: zones[*zone_idx].id.clone(),
                rect: ComputedRect::Fixed {
                    x: current_x_pct,
                    y: current_y,
                    w,
                    h: row_height,
                },
                min_w: zones[*zone_idx].min_width.unwrap_or(min_w),
                min_h: zones[*zone_idx].min_height.unwrap_or(min_h),
            }));

            current_x_pct += w;
        }

        current_y += row_height;
    }

    // Return in original zone order
    placement_order.sort_by_key(|(idx, _)| *idx);
    placement_order.into_iter().map(|(_, layout)| layout).collect()
}

pub fn check_terminal_size(
    zones: &[&ZoneLayout],
    terminal_width: u16,
    terminal_height: u16,
) -> Option<(u16, u16)> {
    let mut required_w: u16 = 0;
    let mut required_h: u16 = 0;

    for zone in zones {
        match &zone.rect {
            ComputedRect::Percent { pct_w, pct_h, .. } => {
                if *pct_w > 0 {
                    let needed_w = (zone.min_w as u32 * 100 / *pct_w as u32) as u16;
                    required_w = required_w.max(needed_w);
                }
                if *pct_h > 0 {
                    let needed_h = (zone.min_h as u32 * 100 / *pct_h as u32) as u16;
                    required_h = required_h.max(needed_h);
                }
            }
            ComputedRect::Fixed { w, .. } => {
                // Width is percentage-based
                if *w > 0 {
                    let needed_w = (zone.min_w as u32 * 100 / *w as u32) as u16;
                    required_w = required_w.max(needed_w);
                }
                // Height is fixed chars — just need enough total height
                // (handled by summing all row heights)
            }
        }
    }

    // For rows mode, compute total required height from fixed rows
    let total_fixed_h: u16 = zones
        .iter()
        .filter_map(|z| match &z.rect {
            ComputedRect::Fixed { h, y, .. } => Some(y + h),
            _ => None,
        })
        .max()
        .unwrap_or(0);

    if total_fixed_h > 0 {
        required_h = required_h.max(total_fixed_h);
    }

    if terminal_width < required_w || terminal_height < required_h {
        Some((required_w, required_h))
    } else {
        None
    }
}
