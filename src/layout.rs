use ratatui::layout::Rect;

use crate::config::ZoneConfig;

pub struct ZoneLayout {
    pub id: String,
    pub rect: ComputedRect,
    pub min_w: u16,
    pub min_h: u16,
    /// Row y-position for stacked zones (used by check_terminal_size to group columns)
    pub row_y: Option<u16>,
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
        let (x, y, w, h) = match &self.rect {
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
                (x, y, w, h)
            }
            ComputedRect::Fixed {
                x: fx,
                y: fy,
                w: fw,
                h: fh,
            } => {
                let x = (*fx as u32 * terminal_width as u32 / 100) as u16;
                let w = (*fw as u32 * terminal_width as u32 / 100) as u16;
                (x, *fy, w, *fh)
            }
        };
        // Clamp to terminal bounds to prevent buffer overflow from rounding
        let w = w.min(terminal_width.saturating_sub(x));
        let h = h.min(terminal_height.saturating_sub(y));
        Rect::new(x, y, w, h)
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
                row_y: None,
            }
        })
        .collect()
}

/// A column within a row, potentially containing multiple vertically stacked zones.
struct ColumnGroup {
    col_id: u16,
    width_pct: u16,
    /// (zone_index, height) pairs — zones stacked vertically within this column
    zones: Vec<(usize, u16)>,
}

/// A row in the stacked layout, containing one or more columns.
struct RowGroup {
    row_id: u16,
    /// Fixed height in chars, or None = fill remaining space
    fixed_height: Option<u16>,
    columns: Vec<ColumnGroup>,
}

/// Build layouts for rows (stacked) mode.
/// Zones are grouped by `row` field, then by `col` within each row.
/// Multiple zones sharing the same row+col are stacked vertically within that column.
pub fn build_rows(
    zones: &[ZoneConfig],
    widgets: &[Box<dyn crate::widget::Widget>],
    terminal_height: u16,
) -> Vec<ZoneLayout> {
    // Phase 1: Group zones into rows, then into columns within rows
    let mut rows: Vec<RowGroup> = Vec::new();
    let mut auto_row_counter = 0u16;
    let mut row_order: Vec<u16> = Vec::new();
    // Track auto-col counters per row to avoid collision
    let mut auto_col_counters: std::collections::HashMap<u16, u16> = std::collections::HashMap::new();

    for (i, zone) in zones.iter().enumerate() {
        let row_id = zone.row.unwrap_or_else(|| {
            auto_row_counter += 1;
            auto_row_counter * 1000
        });

        let width_pct = if zone.width == 0 { 100 } else { zone.width };
        let zone_height = zone.height;

        if let Some(existing) = rows.iter_mut().find(|r| r.row_id == row_id) {
            // Determine col_id for this zone
            let col_id = zone.col.unwrap_or_else(|| {
                let counter = auto_col_counters.entry(row_id).or_insert(10000);
                *counter += 1;
                *counter
            });

            if let Some(col) = existing.columns.iter_mut().find(|c| c.col_id == col_id) {
                // Add zone to existing column (vertical stacking)
                col.zones.push((i, zone_height));
            } else {
                // New column in existing row
                existing.columns.push(ColumnGroup {
                    col_id,
                    width_pct,
                    zones: vec![(i, zone_height)],
                });
            }
        } else {
            // New row
            let (_, widget_min_h) = widgets[i].min_size();
            let fixed_height = if zone_height > 0 {
                Some(zone_height)
            } else {
                Some(widget_min_h)
            };

            let col_id = zone.col.unwrap_or_else(|| {
                let counter = auto_col_counters.entry(row_id).or_insert(10000);
                *counter += 1;
                *counter
            });

            row_order.push(row_id);
            rows.push(RowGroup {
                row_id,
                fixed_height,
                columns: vec![ColumnGroup {
                    col_id,
                    width_pct,
                    zones: vec![(i, zone_height)],
                }],
            });
        }
    }

    // Phase 2: Compute row heights
    // Row height = max across columns of (sum of zone heights in that column)
    // Re-derive fixed_height now that all zones are grouped
    for row in &mut rows {
        let max_col_height: u16 = row
            .columns
            .iter()
            .map(|col| {
                col.zones.iter().map(|(idx, h)| {
                    if *h > 0 {
                        *h
                    } else {
                        let (_, min_h) = widgets[*idx].min_size();
                        min_h
                    }
                }).sum::<u16>()
            })
            .max()
            .unwrap_or(0);

        row.fixed_height = if max_col_height > 0 {
            Some(max_col_height)
        } else {
            None
        };
    }

    // Sort rows by the order they first appeared
    let ordered_row_ids: Vec<u16> = row_order.clone();
    let ordered_rows: Vec<&RowGroup> = ordered_row_ids
        .iter()
        .map(|id| rows.iter().find(|r| r.row_id == *id).unwrap())
        .collect();

    // Phase 3: Fill distribution
    let total_fixed: u16 = ordered_rows.iter().filter_map(|r| r.fixed_height).sum();
    let fill_count = ordered_rows.iter().filter(|r| r.fixed_height.is_none()).count() as u16;
    let remaining = terminal_height.saturating_sub(total_fixed);
    let fill_height = if fill_count > 0 { remaining / fill_count } else { 0 };

    // Phase 4: Position assignment
    let mut current_y: u16 = 0;
    let mut placement_order: Vec<(usize, ZoneLayout)> = Vec::new();

    for row in &ordered_rows {
        let row_height = row.fixed_height.unwrap_or(fill_height);
        let row_y = current_y;

        // Flex-like width distribution across columns
        let explicit_total: u16 = row.columns.iter().map(|c| c.width_pct).sum();
        let auto_count = row.columns.iter().filter(|c| c.width_pct == 0).count() as u16;
        let auto_width = if auto_count > 0 {
            100u16.saturating_sub(explicit_total) / auto_count
        } else {
            0
        };

        let mut current_x_pct: u16 = 0;
        for col in &row.columns {
            let col_w = if col.width_pct == 0 { auto_width } else { col.width_pct };

            if col.zones.len() == 1 {
                // Single zone — gets full row height
                let (zone_idx, _) = col.zones[0];
                let (min_w, min_h) = widgets[zone_idx].min_size();
                placement_order.push((zone_idx, ZoneLayout {
                    id: zones[zone_idx].id.clone(),
                    rect: ComputedRect::Fixed {
                        x: current_x_pct,
                        y: row_y,
                        w: col_w,
                        h: row_height,
                    },
                    min_w: zones[zone_idx].min_width.unwrap_or(min_w),
                    min_h: zones[zone_idx].min_height.unwrap_or(min_h),
                    row_y: Some(row_y),
                }));
            } else {
                // Multiple zones — distribute row_height proportionally
                let total_requested: u16 = col.zones.iter().map(|(idx, h)| {
                    if *h > 0 { *h } else { let (_, mh) = widgets[*idx].min_size(); mh }
                }).sum();

                let mut zone_y = row_y;
                for (j, (zone_idx, zone_h)) in col.zones.iter().enumerate() {
                    let requested = if *zone_h > 0 { *zone_h } else { let (_, mh) = widgets[*zone_idx].min_size(); mh };

                    let h = if j == col.zones.len() - 1 {
                        // Last zone gets remaining height to avoid rounding gaps
                        (row_y + row_height).saturating_sub(zone_y)
                    } else if total_requested > 0 {
                        (requested as u32 * row_height as u32 / total_requested as u32) as u16
                    } else {
                        row_height / col.zones.len() as u16
                    };

                    let (min_w, min_h) = widgets[*zone_idx].min_size();
                    placement_order.push((*zone_idx, ZoneLayout {
                        id: zones[*zone_idx].id.clone(),
                        rect: ComputedRect::Fixed {
                            x: current_x_pct,
                            y: zone_y,
                            w: col_w,
                            h,
                        },
                        min_w: zones[*zone_idx].min_width.unwrap_or(min_w),
                        min_h: zones[*zone_idx].min_height.unwrap_or(min_h),
                        row_y: Some(row_y),
                    }));

                    zone_y += h;
                }
            }

            current_x_pct += col_w;
        }

        current_y += row_height;
    }

    // Return in original zone order
    placement_order.sort_by_key(|(idx, _)| *idx);
    placement_order.into_iter().map(|(_, layout)| layout).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Frame;
    use crate::theme::Theme;

    struct MockWidget {
        min_w: u16,
        min_h: u16,
    }

    impl MockWidget {
        fn new(min_w: u16, min_h: u16) -> Self {
            Self { min_w, min_h }
        }
    }

    impl crate::widget::Widget for MockWidget {
        fn draw(&self, _frame: &mut Frame, _area: ratatui::layout::Rect, _theme: &Theme) {}
        fn min_size(&self) -> (u16, u16) {
            (self.min_w, self.min_h)
        }
    }

    fn zone(id: &str, x: u16, y: u16, w: u16, h: u16) -> ZoneConfig {
        ZoneConfig {
            id: id.to_string(),
            widget: "text".to_string(),
            x,
            y,
            width: w,
            height: h,
            row: None,
            col: None,
            min_width: None,
            min_height: None,
            config: None,
        }
    }

    fn mock_widgets(sizes: &[(u16, u16)]) -> Vec<Box<dyn crate::widget::Widget>> {
        sizes
            .iter()
            .map(|&(w, h)| -> Box<dyn crate::widget::Widget> { Box::new(MockWidget::new(w, h)) })
            .collect()
    }

    #[test]
    fn build_absolute_positioning() {
        let zones = vec![zone("a", 10, 20, 30, 40)];
        let widgets = mock_widgets(&[(12, 5)]);
        let layouts = build_absolute(&zones, &widgets);

        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0].id, "a");
        match &layouts[0].rect {
            ComputedRect::Percent { pct_x, pct_y, pct_w, pct_h } => {
                assert_eq!(*pct_x, 10);
                assert_eq!(*pct_y, 20);
                assert_eq!(*pct_w, 30);
                assert_eq!(*pct_h, 40);
            }
            _ => panic!("expected Percent rect"),
        }
    }

    #[test]
    fn build_rows_single_zone() {
        let mut z = zone("a", 0, 0, 100, 0);
        z.row = Some(1);
        let widgets = mock_widgets(&[(12, 10)]);
        let layouts = build_rows(&[z], &widgets, 40);

        assert_eq!(layouts.len(), 1);
        match &layouts[0].rect {
            ComputedRect::Fixed { x, w, .. } => {
                assert_eq!(*x, 0);
                assert_eq!(*w, 100);
            }
            _ => panic!("expected Fixed rect"),
        }
    }

    #[test]
    fn build_rows_two_columns() {
        let mut z1 = zone("a", 0, 0, 60, 10);
        z1.row = Some(1);
        z1.col = Some(1);
        let mut z2 = zone("b", 0, 0, 40, 10);
        z2.row = Some(1);
        z2.col = Some(2);
        let widgets = mock_widgets(&[(12, 5), (12, 5)]);
        let layouts = build_rows(&[z1, z2], &widgets, 40);

        assert_eq!(layouts.len(), 2);
        match &layouts[0].rect {
            ComputedRect::Fixed { x, w, .. } => {
                assert_eq!(*x, 0);
                assert_eq!(*w, 60);
            }
            _ => panic!("expected Fixed rect"),
        }
        match &layouts[1].rect {
            ComputedRect::Fixed { x, w, .. } => {
                assert_eq!(*x, 60);
                assert_eq!(*w, 40);
            }
            _ => panic!("expected Fixed rect"),
        }
    }

    #[test]
    fn build_rows_col_stacking() {
        let mut z1 = zone("a", 0, 0, 100, 5);
        z1.row = Some(1);
        z1.col = Some(1);
        let mut z2 = zone("b", 0, 0, 100, 5);
        z2.row = Some(1);
        z2.col = Some(1);
        let widgets = mock_widgets(&[(12, 5), (12, 5)]);
        let layouts = build_rows(&[z1, z2], &widgets, 40);

        assert_eq!(layouts.len(), 2);
        // Second zone should be stacked below first
        let y0 = match &layouts[0].rect { ComputedRect::Fixed { y, .. } => *y, _ => panic!() };
        let y1 = match &layouts[1].rect { ComputedRect::Fixed { y, .. } => *y, _ => panic!() };
        assert!(y1 > y0, "stacked zone should be below first");
    }

    #[test]
    fn to_rect_clamping() {
        let layout = ZoneLayout {
            id: "a".to_string(),
            rect: ComputedRect::Percent { pct_x: 90, pct_y: 90, pct_w: 50, pct_h: 50 },
            min_w: 0,
            min_h: 0,
            row_y: None,
        };
        let rect = layout.to_rect(100, 100);
        // x=90, w=50 -> clamped to 100-90=10
        assert!(rect.x + rect.width <= 100);
        assert!(rect.y + rect.height <= 100);
    }

    #[test]
    fn check_terminal_size_sufficient() {
        let layout = ZoneLayout {
            id: "a".to_string(),
            rect: ComputedRect::Percent { pct_x: 0, pct_y: 0, pct_w: 50, pct_h: 50 },
            min_w: 20,
            min_h: 10,
            row_y: None,
        };
        // min_w=20 at 50% -> need 40 cols. min_h=10 at 50% -> need 20 rows.
        assert!(check_terminal_size(&[&layout], 80, 40).is_none());
    }

    #[test]
    fn check_terminal_size_too_small() {
        let layout = ZoneLayout {
            id: "a".to_string(),
            rect: ComputedRect::Percent { pct_x: 0, pct_y: 0, pct_w: 50, pct_h: 50 },
            min_w: 20,
            min_h: 10,
            row_y: None,
        };
        // need 40 cols, only have 30
        let result = check_terminal_size(&[&layout], 30, 40);
        assert!(result.is_some());
        let (req_w, _) = result.unwrap();
        assert_eq!(req_w, 40);
    }

    #[test]
    fn check_terminal_size_fixed_layout() {
        let layout = ZoneLayout {
            id: "a".to_string(),
            rect: ComputedRect::Fixed { x: 0, y: 0, w: 100, h: 20 },
            min_w: 40,
            min_h: 10,
            row_y: Some(0),
        };
        // Fixed zones: required_h = y + h = 20
        let result = check_terminal_size(&[&layout], 50, 15);
        assert!(result.is_some());
    }
}

pub fn check_terminal_size(
    zones: &[&ZoneLayout],
    terminal_width: u16,
    terminal_height: u16,
) -> Option<(u16, u16)> {
    let mut required_w: u16 = 0;
    let mut required_h: u16 = 0;

    // Absolute mode: per-zone percentage formula
    for zone in zones {
        if let ComputedRect::Percent { pct_w, pct_h, .. } = &zone.rect {
            if *pct_w > 0 {
                let needed_w = (zone.min_w as u32 * 100 / *pct_w as u32) as u16;
                required_w = required_w.max(needed_w);
            }
            if *pct_h > 0 {
                let needed_h = (zone.min_h as u32 * 100 / *pct_h as u32) as u16;
                required_h = required_h.max(needed_h);
            }
        }
    }

    // Rows mode: group by (row_y, x) to identify unique columns.
    // Take max(min_w) per column (stacked zones share horizontal space).
    // Sum across distinct columns per row.
    let mut col_min_widths: std::collections::HashMap<(u16, u16), u16> =
        std::collections::HashMap::new();
    for zone in zones {
        if let ComputedRect::Fixed { x, .. } = &zone.rect {
            let ry = zone.row_y.unwrap_or_else(|| match &zone.rect {
                ComputedRect::Fixed { y, .. } => *y,
                _ => 0,
            });
            let entry = col_min_widths.entry((ry, *x)).or_insert(0);
            *entry = (*entry).max(zone.min_w);
        }
    }
    // Sum per row
    let mut row_min_widths: std::collections::HashMap<u16, u16> =
        std::collections::HashMap::new();
    for ((ry, _x), min_w) in &col_min_widths {
        *row_min_widths.entry(*ry).or_insert(0) += min_w;
    }
    for total_min_w in row_min_widths.values() {
        required_w = required_w.max(*total_min_w);
    }

    // Rows mode: required height is the bottom edge of the lowest zone
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
