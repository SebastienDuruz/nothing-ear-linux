//! The app logo: a pair of earbuds drawn on a dot grid, like the dot-matrix
//! style of Nothing products, with a red dot on the right earbud.
//!
//! One pattern gives every version of the logo:
//! - [`rgba`]: the tray icon and the window icon (white dots on a black
//!   rounded square);
//! - [`paint`]: the dots alone, drawn in the window with the theme colours;
//! - `svg` (tests only): the application menu icon, saved as
//!   `packaging/nothing-ear.svg`. A test checks the file still matches the
//!   pattern.

use eframe::egui::{self, Color32, Vec2};

/// `#` is a white dot, `r` the red dot, `.` no dot.
const PATTERN: [&str; 7] = [
    ".##...##.",
    "###...###",
    "###...###",
    ".##...##.",
    "..#...#..",
    "..#...#..",
    "..#...r..",
];
const COLUMNS: usize = PATTERN[0].len();
const ROWS: usize = PATTERN.len();

/// Proportions, as a fraction of the icon size.
const MARGIN: f32 = 0.16;
const CORNER_RADIUS: f32 = 0.22;
/// Dot radius, as a fraction of a grid cell.
const DOT_RADIUS: f32 = 0.40;

const BACKGROUND: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const RED: [u8; 3] = [215, 25, 33];
/// Dots of the "not connected" tray icon.
const DIMMED: [u8; 3] = [110, 110, 110];

#[derive(Clone, Copy, PartialEq)]
enum Dot {
    White,
    Red,
}

/// Every dot of the pattern: column, row and kind.
fn dots() -> impl Iterator<Item = (usize, usize, Dot)> {
    PATTERN.iter().enumerate().flat_map(|(row, line)| {
        line.chars().enumerate().filter_map(move |(column, c)| match c {
            '#' => Some((column, row, Dot::White)),
            'r' => Some((column, row, Dot::Red)),
            _ => None,
        })
    })
}

/// Places the grid in a square of side `size`, leaving `margin` (a fraction
/// of the size) on each side: returns the cell size and the centre of each
/// dot.
fn layout(size: f32, margin: f32) -> (f32, impl Iterator<Item = (f32, f32, Dot)>) {
    let cell = size * (1.0 - 2.0 * margin) / COLUMNS.max(ROWS) as f32;
    let left = (size - cell * COLUMNS as f32) / 2.0;
    let top = (size - cell * ROWS as f32) / 2.0;
    let centres = dots().map(move |(column, row, dot)| {
        (
            left + (column as f32 + 0.5) * cell,
            top + (row as f32 + 0.5) * cell,
            dot,
        )
    });
    (cell, centres)
}

/// The logo as RGBA pixels (straight alpha), `size` × `size`. When
/// `connected` is false the dots are grey.
pub fn rgba(size: u32, connected: bool) -> Vec<u8> {
    let side = size as f32;
    let (cell, centres) = layout(side, MARGIN);
    let dot_radius = cell * DOT_RADIUS;
    let centres: Vec<_> = centres.collect();

    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            // Sample at the pixel centre; coverage gives smooth edges.
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let alpha = coverage(-rounded_square_distance(px, py, side));
            let mut colour = BACKGROUND;
            for &(cx, cy, dot) in &centres {
                let inside = coverage(dot_radius - ((px - cx).powi(2) + (py - cy).powi(2)).sqrt());
                if inside > 0.0 {
                    let dot_colour = match (connected, dot) {
                        (false, _) => DIMMED,
                        (true, Dot::White) => WHITE,
                        (true, Dot::Red) => RED,
                    };
                    colour = mix(colour, dot_colour, inside);
                }
            }
            pixels.extend_from_slice(&[colour[0], colour[1], colour[2], (alpha * 255.0).round() as u8]);
        }
    }
    pixels
}

/// Signed distance from a point to the edge of the background square
/// (negative inside).
fn rounded_square_distance(x: f32, y: f32, side: f32) -> f32 {
    let radius = side * CORNER_RADIUS;
    let half = side / 2.0;
    let qx = (x - half).abs() - (half - radius);
    let qy = (y - half).abs() - (half - radius);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - radius
}

/// How much of a pixel is covered, from the distance (in pixels) between
/// its centre and a shape's edge, positive inside.
fn coverage(distance_inside: f32) -> f32 {
    (distance_inside + 0.5).clamp(0.0, 1.0)
}

fn mix(from: [u8; 3], to: [u8; 3], amount: f32) -> [u8; 3] {
    std::array::from_fn(|i| (from[i] as f32 + (to[i] as f32 - from[i] as f32) * amount).round() as u8)
}

/// Draws the dots alone (no background), `size` points high, in the theme's
/// text colour with the red dot.
pub fn paint(ui: &mut egui::Ui, size: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let colour = ui.visuals().strong_text_color();
        let (cell, centres) = layout(size, 0.0);
        for (x, y, dot) in centres {
            let fill = match dot {
                Dot::White => colour,
                Dot::Red => Color32::from_rgb(RED[0], RED[1], RED[2]),
            };
            ui.painter()
                .circle_filled(rect.min + Vec2::new(x, y), cell * DOT_RADIUS, fill);
        }
    }
    response
}

/// The logo as an SVG document, for the application menu.
#[cfg(test)]
fn svg() -> String {
    const SIZE: f32 = 256.0;
    let (cell, centres) = layout(SIZE, MARGIN);
    let hex = |[r, g, b]: [u8; 3]| format!("#{r:02x}{g:02x}{b:02x}");
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {SIZE} {SIZE}\" width=\"{SIZE}\" height=\"{SIZE}\">\n\
         <rect width=\"{SIZE}\" height=\"{SIZE}\" rx=\"{:.2}\" fill=\"{}\"/>\n",
        SIZE * CORNER_RADIUS,
        hex(BACKGROUND),
    );
    for (x, y, dot) in centres {
        let fill = match dot {
            Dot::White => WHITE,
            Dot::Red => RED,
        };
        svg += &format!(
            "<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"{:.2}\" fill=\"{}\"/>\n",
            cell * DOT_RADIUS,
            hex(fill)
        );
    }
    svg += "</svg>\n";
    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../packaging/nothing-ear.svg");

    /// After changing the pattern, regenerate the file with:
    /// `NOTHING_EAR_WRITE_ICON=1 cargo test -p nothing-tray`
    #[test]
    fn packaged_svg_matches_the_pattern() {
        if std::env::var_os("NOTHING_EAR_WRITE_ICON").is_some() {
            std::fs::write(SVG_PATH, svg()).unwrap();
        }
        let saved = std::fs::read_to_string(SVG_PATH).unwrap_or_default();
        assert!(
            saved == svg(),
            "packaging/nothing-ear.svg is out of date: run NOTHING_EAR_WRITE_ICON=1 cargo test -p nothing-tray"
        );
    }

    #[test]
    fn pattern_rows_have_the_same_width() {
        assert!(PATTERN.iter().all(|row| row.len() == COLUMNS));
    }

    #[test]
    fn corners_are_transparent_and_centre_is_opaque() {
        let size = 32;
        let pixels = rgba(size, true);
        let alpha = |x: u32, y: u32| pixels[((y * size + x) * 4 + 3) as usize];
        assert_eq!(alpha(0, 0), 0);
        assert_eq!(alpha(size / 2, size / 2), 255);
    }
}
