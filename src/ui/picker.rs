//! Native file dialogs, run off the UI thread so the window keeps drawing while one is open.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use eframe::egui;

use crate::queue::MEDIA_EXT;

/// What the user chose in a dialog.
#[derive(Debug, Clone, PartialEq)]
pub enum Picked {
    Media(Vec<String>),
    Subtitle(String),
    SaveQueue(PathBuf),
    /// This many chosen files were left out: their names are not valid UTF-8, and mpv's API
    /// only takes UTF-8 paths.
    NotUtf8(usize),
}

/// Splits `paths` into the ones Lumen can pass to mpv and a count of those it cannot.
pub fn utf8_paths(paths: &[PathBuf]) -> (Vec<String>, usize) {
    let usable: Vec<String> = paths
        .iter()
        .filter_map(|p| p.to_str().map(str::to_owned))
        .collect();
    let skipped = paths.len() - usable.len();
    (usable, skipped)
}

pub struct Picker {
    tx: Sender<Picked>,
    rx: Receiver<Picked>,
    /// Only one dialog at a time, so a repeated shortcut does not stack them.
    busy: Arc<AtomicBool>,
}

impl Default for Picker {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            busy: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Picker {
    /// Where results arrive; tests use it to stand in for a dialog.
    pub fn sender(&self) -> Sender<Picked> {
        self.tx.clone()
    }

    pub fn poll(&self) -> Vec<Picked> {
        self.rx.try_iter().collect()
    }

    pub fn open_media(&self, ctx: &egui::Context, start_dir: &str) {
        let dir = start_dir.to_owned();
        self.run(ctx, move || {
            let mut extensions: Vec<&str> = MEDIA_EXT.to_vec();
            extensions.extend(["m3u", "m3u8"]);
            let paths = dialog(&dir)
                .add_filter("Media files", &extensions)
                .add_filter("All files", &["*"])
                .pick_files()?;
            let (usable, skipped) = utf8_paths(&paths);
            let mut picked = vec![Picked::Media(usable)];
            if skipped > 0 {
                picked.push(Picked::NotUtf8(skipped));
            }
            Some(picked)
        });
    }

    pub fn open_subtitle(&self, ctx: &egui::Context, start_dir: &str) {
        let dir = start_dir.to_owned();
        self.run(ctx, move || {
            let file = dialog(&dir)
                .add_filter("Subtitles", &["srt", "ass", "ssa", "vtt", "sub", "idx"])
                .add_filter("All files", &["*"])
                .pick_file()?;
            Some(vec![match file.to_str() {
                Some(path) => Picked::Subtitle(path.to_owned()),
                None => Picked::NotUtf8(1),
            }])
        });
    }

    pub fn save_queue(&self, ctx: &egui::Context, start_dir: &str) {
        let dir = start_dir.to_owned();
        self.run(ctx, move || {
            let file = dialog(&dir)
                .set_file_name("Lumen queue.m3u")
                .add_filter("Playlist", &["m3u"])
                .save_file()?;
            Some(vec![Picked::SaveQueue(file)])
        });
    }

    fn run(
        &self,
        ctx: &egui::Context,
        pick: impl FnOnce() -> Option<Vec<Picked>> + Send + 'static,
    ) {
        if self.busy.swap(true, Ordering::SeqCst) {
            return;
        }
        let (tx, busy, ctx) = (self.tx.clone(), self.busy.clone(), ctx.clone());
        thread::spawn(move || {
            for picked in pick().unwrap_or_default() {
                let _ = tx.send(picked);
            }
            busy.store(false, Ordering::SeqCst);
            ctx.request_repaint();
        });
    }
}

fn dialog(start_dir: &str) -> rfd::FileDialog {
    let dialog = rfd::FileDialog::new();
    if start_dir.is_empty() || !std::path::Path::new(start_dir).is_dir() {
        dialog
    } else {
        dialog.set_directory(start_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn names_that_are_not_utf8_are_counted_not_mangled() {
        use std::os::unix::ffi::OsStrExt;
        let latin1 = PathBuf::from(std::ffi::OsStr::from_bytes(b"/m/caf\xe9.mkv"));
        let (usable, skipped) = utf8_paths(&[PathBuf::from("/m/ok.mkv"), latin1]);
        assert_eq!(usable, ["/m/ok.mkv"]);
        assert_eq!(skipped, 1);
    }
}
