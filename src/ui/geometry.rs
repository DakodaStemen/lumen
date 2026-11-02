//! Where named widgets were drawn on the last frame, so tools that script the interface
//! (the demo recorder) can aim at them by name instead of by pixel position.
//!
//! Only the current and the previous frame are kept, so names that stop being drawn (queue
//! rows scrolled away, files long gone) do not pile up.

use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui::mutex::Mutex;
use eframe::egui::{Context, Id, Rect};

#[derive(Default)]
struct Frames {
    pass: u64,
    current: HashMap<String, Rect>,
    previous: HashMap<String, Rect>,
}

fn frames(ctx: &Context) -> Arc<Mutex<Frames>> {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Arc<Mutex<Frames>>>(Id::new("lumen-geometry"))
            .clone()
    })
}

pub fn note(ctx: &Context, label: &str, rect: Rect) {
    let pass = ctx.cumulative_pass_nr();
    let frames = frames(ctx);
    let mut frames = frames.lock();
    if frames.pass != pass {
        frames.previous = std::mem::take(&mut frames.current);
        frames.pass = pass;
    }
    frames.current.insert(label.to_owned(), rect);
}

pub fn find(ctx: &Context, label: &str) -> Option<Rect> {
    let frames = frames(ctx);
    let frames = frames.lock();
    frames
        .current
        .get(label)
        .or_else(|| frames.previous.get(label))
        .copied()
}
