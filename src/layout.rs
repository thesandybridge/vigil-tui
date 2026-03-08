use ratatui::layout::Rect;

use crate::config::ZoneConfig;

pub struct ZoneLayout {
    pub id: String,
    pub pct_x: u16,
    pub pct_y: u16,
    pub pct_w: u16,
    pub pct_h: u16,
    pub min_w: u16,
    pub min_h: u16,
}

impl ZoneLayout {
    /// Build a ZoneLayout from config + the widget's own min_size.
    /// Config min_width/min_height override the widget's defaults.
    pub fn from_config(zone: &ZoneConfig, widget_min: (u16, u16)) -> Self {
        Self {
            id: zone.id.clone(),
            pct_x: zone.x,
            pct_y: zone.y,
            pct_w: zone.width,
            pct_h: zone.height,
            min_w: zone.min_width.unwrap_or(widget_min.0),
            min_h: zone.min_height.unwrap_or(widget_min.1),
        }
    }

    pub fn to_rect(&self, terminal_width: u16, terminal_height: u16) -> Rect {
        let x = (self.pct_x as u32 * terminal_width as u32 / 100) as u16;
        let y = (self.pct_y as u32 * terminal_height as u32 / 100) as u16;
        let w = (self.pct_w as u32 * terminal_width as u32 / 100) as u16;
        let h = (self.pct_h as u32 * terminal_height as u32 / 100) as u16;
        Rect::new(x, y, w, h)
    }
}

pub fn check_terminal_size(
    zones: &[&ZoneLayout],
    terminal_width: u16,
    terminal_height: u16,
) -> Option<(u16, u16)> {
    let mut required_w: u16 = 0;
    let mut required_h: u16 = 0;
    for zone in zones {
        if zone.pct_w > 0 {
            let needed_w = (zone.min_w as u32 * 100 / zone.pct_w as u32) as u16;
            required_w = required_w.max(needed_w);
        }
        if zone.pct_h > 0 {
            let needed_h = (zone.min_h as u32 * 100 / zone.pct_h as u32) as u16;
            required_h = required_h.max(needed_h);
        }
    }
    if terminal_width < required_w || terminal_height < required_h {
        Some((required_w, required_h))
    } else {
        None
    }
}
