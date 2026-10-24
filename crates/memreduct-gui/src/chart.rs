use gtk4::cairo::Context;
use gtk4::prelude::*;
use gtk4::DrawingArea;
use std::cell::RefCell;
use std::rc::Rc;

use crate::history::HistoryBuffer;

pub fn create_chart_widget(history: Rc<RefCell<HistoryBuffer>>) -> DrawingArea {
    let area = DrawingArea::builder()
        .content_height(100)
        .content_width(260)
        .hexpand(true)
        .build();

    let hist_clone = history.clone();

    area.set_draw_func(move |_area, cr, width, height| {
        draw_chart(&hist_clone.borrow(), cr, width as f64, height as f64);
    });

    area
}

fn draw_chart(history: &HistoryBuffer, cr: &Context, w: f64, h: f64) {
    // Background
    cr.set_source_rgba(0.08, 0.09, 0.11, 0.4);
    let _ = cr.paint();

    // Subtle horizontal grid lines at 25%, 50%, 75%
    cr.set_line_width(0.7);
    cr.set_source_rgba(0.5, 0.5, 0.5, 0.2);

    for pct in [0.25, 0.50, 0.75] {
        let y = h * (1.0 - pct);
        cr.move_to(0.0, y);
        cr.line_to(w, y);
    }
    let _ = cr.stroke();

    if history.len() < 2 {
        return;
    }

    let points: Vec<f64> = history.points().iter().copied().collect();
    let num_points = points.len();
    let dx = w / (num_points - 1) as f64;

    let latest = *points.last().unwrap_or(&0.0);

    // Color based on latest value
    let (r, g, b) = if latest >= 90.0 {
        (0.92, 0.25, 0.25) // Red
    } else if latest >= 75.0 {
        (0.95, 0.65, 0.15) // Orange/Yellow
    } else {
        (0.20, 0.75, 0.45) // Green
    };

    // Draw area fill
    cr.move_to(0.0, h);
    for (i, &val) in points.iter().enumerate() {
        let clamped = val.clamp(0.0, 100.0);
        let y = h - (clamped / 100.0 * h);
        cr.line_to(i as f64 * dx, y);
    }
    cr.line_to((num_points - 1) as f64 * dx, h);
    cr.close_path();

    cr.set_source_rgba(r, g, b, 0.20);
    let _ = cr.fill();

    // Draw top line stroke
    cr.set_line_width(2.0);
    cr.set_source_rgba(r, g, b, 0.95);

    for (i, &val) in points.iter().enumerate() {
        let clamped = val.clamp(0.0, 100.0);
        let y = h - (clamped / 100.0 * h);
        if i == 0 {
            cr.move_to(0.0, y);
        } else {
            cr.line_to(i as f64 * dx, y);
        }
    }
    let _ = cr.stroke();
}
