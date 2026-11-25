use std::fmt::Write;

use ratatui::buffer::{Buffer, Cell};
use ratatui::style::{Color, Modifier};

pub const CELL_W: f32 = 8.5;
pub const CELL_H: f32 = 18.0;
const FONT_SIZE: f32 = 14.0;
const PAD: f32 = 12.0;

pub const FG: (u8, u8, u8) = (0xF8, 0xF8, 0xF2);
pub const BG: (u8, u8, u8) = (0x27, 0x28, 0x22);

/// Renders every cell of `buf` onto a fixed grid so columns line up in any monospace font.
pub fn render(buf: &Buffer) -> String {
    let area = buf.area;
    let width = area.width as f32 * CELL_W + PAD * 2.0;
    let height = area.height as f32 * CELL_H + PAD * 2.0;
    let mut out = String::new();
    let _ = write!(
        out,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" font-family="'JetBrains Mono','Cascadia Mono','SFMono-Regular',Menlo,Consolas,monospace" font-size="{FONT_SIZE}">"##
    );
    let _ = write!(
        out,
        r#"<rect width="100%" height="100%" rx="10" fill="{}"/>"#,
        hex(BG)
    );
    let _ = write!(out, r#"<g transform="translate({PAD} {PAD})">"#);

    for y in area.top()..area.bottom() {
        let row: Vec<&Cell> = (area.left()..area.right()).map(|x| &buf[(x, y)]).collect();
        render_backgrounds(&mut out, &row, y - area.top());
        render_text(&mut out, &row, y - area.top());
    }

    out.push_str("</g></svg>");
    out
}

fn render_backgrounds(out: &mut String, row: &[&Cell], y: u16) {
    let mut x = 0;
    while x < row.len() {
        let bg = row[x].bg;
        let start = x;
        while x < row.len() && row[x].bg == bg {
            x += 1;
        }
        if let Some(rgb) = to_rgb(bg) {
            let _ = write!(
                out,
                r#"<rect x="{}" y="{}" width="{}" height="{CELL_H}" fill="{}"/>"#,
                start as f32 * CELL_W,
                y as f32 * CELL_H,
                (x - start) as f32 * CELL_W,
                hex(rgb)
            );
        }
    }
}

fn render_text(out: &mut String, row: &[&Cell], y: u16) {
    let baseline = y as f32 * CELL_H + 13.0;
    let mut x = 0;
    while x < row.len() {
        let (fg, modifier) = (row[x].fg, row[x].modifier);
        let start = x;
        while x < row.len() && row[x].fg == fg && row[x].modifier == modifier {
            x += 1;
        }
        let color = hex(to_rgb(fg).unwrap_or(FG));
        let mut xs = String::new();
        let mut text = String::new();
        for (i, cell) in row[start..x].iter().enumerate() {
            let symbol = cell.symbol();
            let cx = (start + i) as f32 * CELL_W;
            if let Some((fx, fy, fw, fh)) = block_fraction(symbol) {
                let _ = write!(
                    out,
                    r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{color}"/>"#,
                    cx + fx * CELL_W,
                    y as f32 * CELL_H + fy * CELL_H,
                    fw * CELL_W,
                    fh * CELL_H
                );
            } else if let Some(path) = box_path(symbol, cx, y as f32 * CELL_H) {
                let _ = write!(out, r#"<path d="{path}" stroke="{color}" fill="none"/>"#);
            } else if !symbol.trim().is_empty() {
                let _ = write!(xs, "{cx} ");
                escape_into(&mut text, symbol);
            }
        }
        if text.is_empty() {
            continue;
        }
        let weight = if modifier.contains(Modifier::BOLD) {
            r#" font-weight="bold""#
        } else {
            ""
        };
        let _ = write!(
            out,
            r#"<text x="{}" y="{baseline}" fill="{color}"{weight}>{text}</text>"#,
            xs.trim_end()
        );
    }
}

/// Light box-drawing lines become strokes through the cell centre so they join across rows.
fn box_path(symbol: &str, x: f32, y: f32) -> Option<String> {
    let (cx, cy) = (x + CELL_W / 2.0, y + CELL_H / 2.0);
    let (r, b) = (x + CELL_W, y + CELL_H);
    let k = CELL_W / 2.0;
    Some(match symbol {
        "│" => format!("M{cx} {y}V{b}"),
        "─" => format!("M{x} {cy}H{r}"),
        "╭" => format!("M{r} {cy}H{}Q{cx} {cy} {cx} {}V{b}", cx + k, cy + k),
        "╮" => format!("M{x} {cy}H{}Q{cx} {cy} {cx} {}V{b}", cx - k, cy + k),
        "╰" => format!("M{r} {cy}H{}Q{cx} {cy} {cx} {}V{y}", cx + k, cy - k),
        "╯" => format!("M{x} {cy}H{}Q{cx} {cy} {cx} {}V{y}", cx - k, cy - k),
        _ => return None,
    })
}

/// Block elements become solid rects (x, y, w, h as cell fractions), since glyphs leave gaps.
fn block_fraction(symbol: &str) -> Option<(f32, f32, f32, f32)> {
    let mut chars = symbol.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    Some(match c {
        '█' => (0.0, 0.0, 1.0, 1.0),
        '▀' => (0.0, 0.0, 1.0, 0.5),
        '▐' => (0.5, 0.0, 0.5, 1.0),
        '▁'..='▇' => {
            let h = (c as u32 - '▁' as u32 + 1) as f32 / 8.0;
            (0.0, 1.0 - h, 1.0, h)
        }
        '▉'..='▏' => (0.0, 0.0, ('▏' as u32 - c as u32 + 1) as f32 / 8.0, 1.0),
        _ => return None,
    })
}

fn escape_into(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            _ => out.push(c),
        }
    }
}

fn hex((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

fn to_rgb(color: Color) -> Option<(u8, u8, u8)> {
    Some(match color {
        Color::Reset => return None,
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0x27, 0x28, 0x22),
        Color::Red => (0xF9, 0x26, 0x72),
        Color::Green => (0xA6, 0xE2, 0x2E),
        Color::Yellow => (0xE6, 0xDB, 0x74),
        Color::Blue => (0x66, 0xD9, 0xEF),
        Color::Magenta => (0xAE, 0x81, 0xFF),
        Color::Cyan => (0xA1, 0xEF, 0xE4),
        Color::Gray => (0xCF, 0xCF, 0xC2),
        Color::DarkGray => (0x75, 0x71, 0x5E),
        Color::White => FG,
        _ => FG,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;
    use ratatui::style::Style;

    #[test]
    fn places_each_glyph_on_the_grid_and_escapes_markup() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 6, 1));
        buf.set_string(1, 0, "a<b", Style::default().fg(Color::Rgb(1, 2, 3)));
        let svg = render(&buf);
        assert!(
            svg.contains(r##"<text x="8.5 17 25.5" y="13" fill="#010203">a&lt;b</text>"##),
            "{svg}"
        );
    }

    #[test]
    fn draws_backgrounds_for_styled_runs_only() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
        buf.set_string(2, 0, "  ", Style::default().bg(Color::Rgb(9, 9, 9)));
        let svg = render(&buf);
        assert_eq!(svg.matches("<rect").count(), 2, "{svg}");
        assert!(
            svg.contains(r##"<rect x="17" y="0" width="17" height="18" fill="#090909"/>"##),
            "{svg}"
        );
        assert!(!svg.contains("<text"), "{svg}");
    }
}
