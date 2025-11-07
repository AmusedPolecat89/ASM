use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FigureConfig {
    pub width: u32,
    pub height: u32,
    pub bins: usize,
}

impl Default for FigureConfig {
    fn default() -> Self {
        Self {
            width: 320,
            height: 160,
            bins: 16,
        }
    }
}

pub fn render_histogram_svg(values: &[f64], config: &FigureConfig) -> String {
    if values.is_empty() {
        return format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{h}'></svg>",
            w = config.width,
            h = config.height
        );
    }
    let min = values
        .iter()
        .cloned()
        .fold(f64::INFINITY, |acc, val| acc.min(val));
    let max = values
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, |acc, val| acc.max(val));
    let bin_count = config.bins.max(1);
    let mut bins = vec![0usize; bin_count];
    let span = (max - min).max(1e-9);
    for value in values {
        let mut idx = ((value - min) / span * bin_count as f64).floor() as usize;
        if idx >= bin_count {
            idx = bin_count - 1;
        }
        bins[idx] += 1;
    }
    let max_bin = bins.iter().copied().max().unwrap_or(1) as f64;
    let bar_width = config.width as f64 / bin_count as f64;
    let mut parts = vec![format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{h}'>",
        w = config.width,
        h = config.height
    )];
    for (idx, count) in bins.iter().enumerate() {
        let height = if max_bin == 0.0 {
            0.0
        } else {
            (*count as f64 / max_bin) * config.height as f64
        };
        let x = bar_width * idx as f64;
        let y = config.height as f64 - height;
        parts.push(format!(
            "<rect x='{:.2}' y='{:.2}' width='{:.2}' height='{:.2}' fill='#3b82f6' />",
            x,
            y,
            bar_width.max(1.0),
            height
        ));
    }
    parts.push("</svg>".into());
    parts.join("")
}
