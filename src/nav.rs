use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

/// Returns the index of the nearest zone in the given direction from `current`.
/// If `current` is None (no focus), picks the first zone.
pub fn find_neighbor(zones: &[Rect], current: Option<usize>, dir: Dir) -> Option<usize> {
    if zones.is_empty() {
        return None;
    }

    let idx = match current {
        Some(i) if i < zones.len() => i,
        _ => return Some(0),
    };

    let source = zones[idx];
    let src_cx = source.x as i32 + source.width as i32 / 2;
    let src_cy = source.y as i32 + source.height as i32 / 2;

    let mut best_idx: Option<usize> = None;
    let mut best_dist = i64::MAX;

    for (i, rect) in zones.iter().enumerate() {
        if i == idx {
            continue;
        }

        let cx = rect.x as i32 + rect.width as i32 / 2;
        let cy = rect.y as i32 + rect.height as i32 / 2;

        let qualifies = match dir {
            Dir::Left => cx < src_cx,
            Dir::Right => cx > src_cx,
            Dir::Up => cy < src_cy,
            Dir::Down => cy > src_cy,
        };
        if !qualifies {
            continue;
        }

        // Primarily axial distance, secondarily lateral (avoids diagonal drift)
        let (axial, lateral) = match dir {
            Dir::Left | Dir::Right => (
                (src_cx - cx).unsigned_abs() as i64,
                (src_cy - cy).unsigned_abs() as i64,
            ),
            Dir::Up | Dir::Down => (
                (src_cy - cy).unsigned_abs() as i64,
                (src_cx - cx).unsigned_abs() as i64,
            ),
        };
        let dist = axial * 10 + lateral;

        if dist < best_dist {
            best_dist = dist;
            best_idx = Some(i);
        }
    }

    best_idx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: u16, y: u16, w: u16, h: u16) -> Rect {
        Rect::new(x, y, w, h)
    }

    #[test]
    fn navigate_right() {
        let zones = vec![r(0, 0, 10, 10), r(20, 0, 10, 10)];
        assert_eq!(find_neighbor(&zones, Some(0), Dir::Right), Some(1));
    }

    #[test]
    fn navigate_left() {
        let zones = vec![r(0, 0, 10, 10), r(20, 0, 10, 10)];
        assert_eq!(find_neighbor(&zones, Some(1), Dir::Left), Some(0));
    }

    #[test]
    fn navigate_down() {
        let zones = vec![r(0, 0, 10, 10), r(0, 20, 10, 10)];
        assert_eq!(find_neighbor(&zones, Some(0), Dir::Down), Some(1));
    }

    #[test]
    fn navigate_up() {
        let zones = vec![r(0, 0, 10, 10), r(0, 20, 10, 10)];
        assert_eq!(find_neighbor(&zones, Some(1), Dir::Up), Some(0));
    }

    #[test]
    fn no_neighbor_in_direction() {
        let zones = vec![r(0, 0, 10, 10), r(20, 0, 10, 10)];
        assert_eq!(find_neighbor(&zones, Some(1), Dir::Right), None);
    }

    #[test]
    fn no_focus_returns_first() {
        let zones = vec![r(0, 0, 10, 10), r(20, 0, 10, 10)];
        assert_eq!(find_neighbor(&zones, None, Dir::Right), Some(0));
    }

    #[test]
    fn prefers_closer_zone() {
        let zones = vec![
            r(0, 0, 10, 10),  // source
            r(20, 0, 10, 10), // close right
            r(50, 0, 10, 10), // far right
        ];
        assert_eq!(find_neighbor(&zones, Some(0), Dir::Right), Some(1));
    }

    #[test]
    fn prefers_aligned_zone() {
        let zones = vec![
            r(0, 10, 10, 10),  // source at center (5, 15)
            r(20, 0, 10, 10),  // right, high (25, 5)
            r(20, 10, 10, 10), // right, aligned (25, 15)
        ];
        assert_eq!(find_neighbor(&zones, Some(0), Dir::Right), Some(2));
    }

    #[test]
    fn empty_zones() {
        assert_eq!(find_neighbor(&[], Some(0), Dir::Right), None);
    }
}
