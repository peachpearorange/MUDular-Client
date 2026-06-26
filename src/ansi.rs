use eframe::egui::Color32;

use crate::buffer::{Style, StyledLine, StyledSpan};

pub const DEFAULT_PALETTE: [Color32; 16] = [
  Color32::from_rgb(0, 0, 0),
  Color32::from_rgb(170, 0, 0),
  Color32::from_rgb(0, 170, 0),
  Color32::from_rgb(170, 85, 0),
  Color32::from_rgb(0, 0, 170),
  Color32::from_rgb(170, 0, 170),
  Color32::from_rgb(0, 170, 170),
  Color32::from_rgb(170, 170, 170),
  Color32::from_rgb(85, 85, 85),
  Color32::from_rgb(255, 85, 85),
  Color32::from_rgb(85, 255, 85),
  Color32::from_rgb(255, 255, 85),
  Color32::from_rgb(85, 85, 255),
  Color32::from_rgb(255, 85, 255),
  Color32::from_rgb(85, 255, 255),
  Color32::from_rgb(255, 255, 255)
];

fn color_256(n: u8, palette: &[Color32; 16]) -> Color32 {
  match n {
    0..=15 => palette[n as usize],
    16..=231 => {
      let n = n - 16;
      let b = (n % 6) * 51;
      let g = ((n / 6) % 6) * 51;
      let r = (n / 36) * 51;
      Color32::from_rgb(r, g, b)
    }
    _ => {
      let v = 8 + (n - 232) * 10;
      Color32::from_rgb(v, v, v)
    }
  }
}

pub fn strip_ansi(input: &str) -> String {
  let mut out = String::with_capacity(input.len());
  let mut chars = input.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch == '\x1b' {
      if chars.peek() == Some(&'[') {
        chars.next();
        while let Some(&c) = chars.peek() {
          if c.is_ascii_digit() || c == ';' {
            chars.next();
          } else {
            break;
          }
        }
        chars.next(); // consume finalizer
      }
    } else {
      out.push(ch);
    }
  }
  out
}

pub fn parse_ansi(input: &str, palette: Option<&[Color32; 16]>) -> Vec<StyledLine> {
  let pal = palette.unwrap_or(&DEFAULT_PALETTE);
  let mut lines = Vec::new();
  let mut current_spans: Vec<StyledSpan> = Vec::new();
  let mut current_text = String::new();
  let mut style = Style::default();
  let mut chars = input.chars().peekable();

  while let Some(ch) = chars.next() {
    if ch == '\x1b' {
      if chars.peek() == Some(&'[') {
        chars.next();
        let mut params = String::new();
        while let Some(&c) = chars.peek() {
          if c.is_ascii_digit() || c == ';' {
            params.push(c);
            chars.next();
          } else {
            break;
          }
        }
        let finalizer = chars.next().unwrap_or('m');
        if finalizer == 'm' {
          if !current_text.is_empty() {
            current_spans.push(StyledSpan {
              text: std::mem::take(&mut current_text),
              style: style.clone()
            });
          }
          apply_sgr(&params, &mut style, pal);
        }
      }
    } else if ch == '\n' {
      if !current_text.is_empty() {
        current_spans.push(StyledSpan {
          text: std::mem::take(&mut current_text),
          style: style.clone()
        });
      }
      lines.push(StyledLine { spans: std::mem::take(&mut current_spans) });
    } else if ch == '\r' {
      // skip
    } else {
      current_text.push(ch);
    }
  }

  if !current_text.is_empty() {
    current_spans
      .push(StyledSpan { text: std::mem::take(&mut current_text), style: style.clone() });
  }
  if !current_spans.is_empty() {
    lines.push(StyledLine { spans: current_spans });
  }

  if lines.is_empty() {
    lines.push(StyledLine { spans: Vec::new() });
  }

  lines
}

fn apply_sgr(params: &str, style: &mut Style, palette: &[Color32; 16]) {
  if params.is_empty() {
    *style = Style::default();
    return;
  }

  let mut codes = params.split(';').filter_map(|s| s.parse::<u8>().ok()).peekable();

  while let Some(code) = codes.next() {
    match code {
      0 => *style = Style::default(),
      1 => style.bold = true,
      3 => style.italic = true,
      4 => style.underline = true,
      9 => style.strikethrough = true,
      22 => style.bold = false,
      23 => style.italic = false,
      24 => style.underline = false,
      29 => style.strikethrough = false,
      30..=37 => {
        let idx = (code - 30) as usize;
        style.fg = Some(if style.bold { palette[idx + 8] } else { palette[idx] });
      }
      38 => match codes.next() {
        Some(5) => {
          if let Some(n) = codes.next() {
            style.fg = Some(color_256(n, palette));
          }
        }
        Some(2) => {
          let r = codes.next().unwrap_or(0);
          let g = codes.next().unwrap_or(0);
          let b = codes.next().unwrap_or(0);
          style.fg = Some(Color32::from_rgb(r, g, b));
        }
        _ => {}
      },
      39 => style.fg = None,
      40..=47 => style.bg = Some(palette[(code - 40) as usize]),
      48 => match codes.next() {
        Some(5) => {
          if let Some(n) = codes.next() {
            style.bg = Some(color_256(n, palette));
          }
        }
        Some(2) => {
          let r = codes.next().unwrap_or(0);
          let g = codes.next().unwrap_or(0);
          let b = codes.next().unwrap_or(0);
          style.bg = Some(Color32::from_rgb(r, g, b));
        }
        _ => {}
      },
      49 => style.bg = None,
      90..=97 => style.fg = Some(palette[(code - 90 + 8) as usize]),
      100..=107 => style.bg = Some(palette[(code - 100 + 8) as usize]),
      _ => {}
    }
  }
}
