//! Settings and per-file memory (resume point, tracks, sync offsets, bookmarks).
//!
//! Lumen keeps its own profile in its own settings directory.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const RECENT_LIMIT: usize = 15;
/// Most remembered files kept; the least recently saved are evicted first.
const FILES_LIMIT: usize = 2000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub volume: u32,
    /// Seconds moved by the seek keys.
    pub seek_step: u32,
    pub resume: bool,
    pub remember_tracks: bool,
    pub auto_next_folder: bool,
    /// Streamlink quality: `best`, or a name such as `720p60`.
    pub stream_quality: String,
    pub low_latency: bool,
    pub sponsorblock: bool,
    /// Also skip intros and outros, not just sponsor reads.
    pub sponsorblock_intros: bool,
    pub show_queue: bool,
    pub always_on_top: bool,
    /// The folder the file picker opens in.
    pub last_dir: String,
    /// Key bindings by action id. An empty string unbinds the action.
    pub keys: BTreeMap<String, String>,
    pub recent: Vec<String>,
    /// Hardware decoding: `auto-safe`, `vaapi`, `nvdec` or `no`.
    pub hwdec: String,
    /// HDR to SDR tone-mapping algorithm for mpv's `gpu` renderer; `auto` leaves mpv's default.
    pub tone_mapping: String,
    pub subtitles: SubtitleStyle,
    pub picture: Picture,
    pub audio: AudioPrefs,
    /// Shader file names (inside the shaders folder) that are switched on.
    pub shaders: Vec<String>,
}

/// How subtitles are drawn. An empty font keeps mpv's default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SubtitleStyle {
    pub font: String,
    /// Font size in mpv's scaled pixels (mpv's default is 55).
    pub size: u32,
    /// Text colour as `#RRGGBB`.
    pub color: String,
    /// Outline colour as `#RRGGBB`.
    pub border_color: String,
    /// Outline width (mpv's default is 3).
    pub border: f32,
    /// Vertical position, 0 (top) to 100 (bottom).
    pub pos: u32,
}

impl Default for SubtitleStyle {
    fn default() -> Self {
        Self {
            font: String::new(),
            size: 55,
            color: "#FFFFFF".to_owned(),
            border_color: "#000000".to_owned(),
            border: 3.0,
            pos: 100,
        }
    }
}

/// Colour adjustments, each from -100 to 100 and 0 when untouched.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Picture {
    pub brightness: i32,
    pub contrast: i32,
    pub saturation: i32,
    pub gamma: i32,
    pub hue: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioPrefs {
    /// Night mode: loudness normalisation (EBU R128) with a narrow range.
    pub night_mode: bool,
    /// `no`, `track` or `album`.
    pub replaygain: String,
    /// An mpv audio device name, or `auto`.
    pub device: String,
}

impl Default for AudioPrefs {
    fn default() -> Self {
        Self {
            night_mode: false,
            replaygain: "no".to_owned(),
            device: "auto".to_owned(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            volume: 80,
            seek_step: 10,
            resume: true,
            remember_tracks: true,
            auto_next_folder: true,
            stream_quality: "best".to_owned(),
            low_latency: true,
            sponsorblock: true,
            sponsorblock_intros: false,
            show_queue: true,
            always_on_top: false,
            last_dir: String::new(),
            keys: BTreeMap::new(),
            recent: Vec::new(),
            hwdec: "auto-safe".to_owned(),
            tone_mapping: "auto".to_owned(),
            subtitles: SubtitleStyle::default(),
            picture: Picture::default(),
            audio: AudioPrefs::default(),
            shaders: Vec::new(),
        }
    }
}

/// What Lumen remembers about one file. Times are milliseconds.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FileState {
    pub position: i64,
    pub length: i64,
    pub audio: Option<i64>,
    /// Subtitle track id, or -1 when subtitles were switched off.
    pub spu: Option<i64>,
    pub audio_delay: i64,
    pub spu_delay: i64,
    pub bookmarks: Vec<i64>,
    pub subtitles: Vec<String>,
    /// Save sequence number used for eviction. Old files.json lacks it (0 = oldest).
    pub saved: u64,
}

pub struct Store {
    pub settings: Settings,
    files: BTreeMap<String, FileState>,
    /// Files saved by this process. Only these are written over what is on disk, so a second
    /// Lumen window does not erase what the first one remembered.
    touched: BTreeSet<String>,
    writer: Option<Writer>,
}

impl Store {
    /// A store that is never written to disk.
    pub fn in_memory() -> Self {
        Self {
            settings: Settings::default(),
            files: BTreeMap::new(),
            touched: BTreeSet::new(),
            writer: None,
        }
    }

    /// Opens the profile in `dir`. Unreadable entries are skipped; a file that cannot be read
    /// at all is kept beside it as `*.broken` instead of being overwritten by the next save.
    pub fn open(dir: PathBuf) -> Self {
        let mut settings: Settings = load_lenient(&dir.join("settings.json"));
        settings.volume = settings.volume.min(MAX_VOLUME);
        let files = load_files(&dir.join("files.json"));
        Self {
            settings,
            files,
            touched: BTreeSet::new(),
            writer: Some(Writer::spawn(dir)),
        }
    }

    /// Waits until everything saved so far is on disk.
    pub fn flush(&self) {
        if let Some(writer) = &self.writer {
            writer.flush();
        }
    }

    pub fn default_dir() -> anyhow::Result<PathBuf> {
        crate::platform::config_dir()
    }

    fn file_key(source: &str) -> String {
        let digest = Sha256::digest(source.as_bytes());
        digest.iter().take(10).map(|b| format!("{b:02x}")).collect()
    }

    pub fn file_state(&self, source: &str) -> FileState {
        let mut state = self
            .files
            .get(&Self::file_key(source))
            .cloned()
            .unwrap_or_default();
        state.saved = 0;
        state
    }

    pub fn save_file_state(&mut self, source: &str, mut state: FileState) {
        state.saved = self.files.values().map(|f| f.saved).max().unwrap_or(0) + 1;
        let key = Self::file_key(source);
        self.files.insert(key.clone(), state);
        evict_oldest(&mut self.files, FILES_LIMIT);
        self.touched.insert(key);
        self.touched.retain(|k| self.files.contains_key(k));
        if let Some(writer) = &self.writer {
            let touched = self
                .touched
                .iter()
                .filter_map(|k| Some((k.clone(), self.files.get(k)?.clone())))
                .collect();
            writer.send(Job::Files(touched));
        }
    }

    /// Saves the settings on the writer thread; repeated saves in quick succession (a volume
    /// drag, say) collapse into one write.
    pub fn save_settings(&self) {
        if let Some(writer) = &self.writer {
            writer.send(Job::Settings(Box::new(self.settings.clone())));
        }
    }

    pub fn remember_recent(&mut self, source: &str) {
        let source = redact_source(source);
        self.settings.recent.retain(|s| *s != source);
        self.settings.recent.insert(0, source);
        self.settings.recent.truncate(RECENT_LIMIT);
        self.save_settings();
    }
}

/// Drops userinfo, query and fragment from URLs of any scheme except `file://` (`srt://`,
/// `ftp://` and `smb://` carry passwords too) so secrets never reach settings.json. Local paths
/// and other strings are returned unchanged.
fn redact_source(source: &str) -> String {
    let Some(at) = source.find("://") else {
        return source.to_owned();
    };
    let scheme = &source[..at];
    let is_scheme = scheme
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if !is_scheme || scheme.eq_ignore_ascii_case("file") {
        return source.to_owned();
    }
    let (head, rest) = source.split_at(at + 3);
    let end = rest.find(['?', '#']).unwrap_or(rest.len());
    let rest = &rest[..end];
    let (authority, path) = match rest.find('/') {
        Some(i) => rest.split_at(i),
        None => (rest, ""),
    };
    let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    format!("{head}{host}{path}")
}

/// Keeps at most `limit` entries, removing those with the lowest `saved`.
fn evict_oldest(files: &mut BTreeMap<String, FileState>, limit: usize) {
    if files.len() <= limit {
        return;
    }
    let mut order: Vec<(u64, String)> = files.iter().map(|(k, v)| (v.saved, k.clone())).collect();
    order.sort();
    for (_, key) in order.into_iter().take(files.len() - limit) {
        files.remove(&key);
    }
}

/// The highest volume Lumen sets (percent).
pub const MAX_VOLUME: u32 = 200;

/// Reads `path` as JSON. A missing file is `None`; a file that is not JSON is set aside as
/// `<name>.broken` (so the next save cannot destroy what may be recoverable) and is also `None`.
fn read_value(path: &Path) -> Option<serde_json::Value> {
    let bytes = fs::read(path).ok()?;
    match serde_json::from_slice(&bytes) {
        Ok(value) => Some(value),
        Err(e) => {
            set_aside(path, &e.to_string());
            None
        }
    }
}

fn set_aside(path: &Path, why: &str) {
    let mut broken = path.as_os_str().to_owned();
    broken.push(".broken");
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    eprintln!("lumen: {name} could not be read ({why}); kept as {name}.broken");
    let _ = fs::rename(path, broken);
}

/// Loads a settings-like struct field by field: a field with the wrong type falls back to its
/// default instead of discarding every other setting.
fn load_lenient<T: Default + Serialize + serde::de::DeserializeOwned>(path: &Path) -> T {
    let Some(value) = read_value(path) else {
        return T::default();
    };
    if let Ok(parsed) = serde_json::from_value::<T>(value.clone()) {
        return parsed;
    }
    let serde_json::Value::Object(stored) = value else {
        set_aside(path, "not a JSON object");
        return T::default();
    };
    let Ok(serde_json::Value::Object(mut merged)) = serde_json::to_value(T::default()) else {
        return T::default();
    };
    for (key, field) in stored {
        let previous = merged.insert(key.clone(), field);
        if serde_json::from_value::<T>(serde_json::Value::Object(merged.clone())).is_err() {
            match previous {
                Some(previous) => merged.insert(key, previous),
                None => merged.remove(&key),
            };
        }
    }
    serde_json::from_value(serde_json::Value::Object(merged)).unwrap_or_default()
}

/// Loads `files.json`, skipping any entry that does not parse.
fn load_files(path: &Path) -> BTreeMap<String, FileState> {
    match read_value(path) {
        Some(serde_json::Value::Object(entries)) => entries
            .into_iter()
            .filter_map(|(k, v)| Some((k, serde_json::from_value(v).ok()?)))
            .collect(),
        Some(_) => {
            set_aside(path, "not a JSON object");
            BTreeMap::new()
        }
        None => BTreeMap::new(),
    }
}

enum Job {
    Settings(Box<Settings>),
    /// Every entry this process has saved, to lay over what is on disk.
    Files(BTreeMap<String, FileState>),
    Flush(Sender<()>),
}

/// Writes the profile off the UI thread, in order, collapsing a burst of saves into one write.
struct Writer {
    jobs: Option<Sender<Job>>,
    thread: Option<JoinHandle<()>>,
}

impl Writer {
    fn spawn(dir: PathBuf) -> Self {
        let (jobs, rx) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("profile-writer".into())
            .spawn(move || write_jobs(&dir, &rx))
            .ok();
        Self {
            jobs: Some(jobs),
            thread,
        }
    }

    fn send(&self, job: Job) {
        if let Some(jobs) = &self.jobs {
            let _ = jobs.send(job);
        }
    }

    fn flush(&self) {
        let (done, wait) = mpsc::channel();
        self.send(Job::Flush(done));
        let _ = wait.recv();
    }
}

impl Drop for Writer {
    /// Everything saved reaches the disk before Lumen exits.
    fn drop(&mut self) {
        self.jobs = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn write_jobs(dir: &Path, jobs: &Receiver<Job>) {
    while let Ok(first) = jobs.recv() {
        let (mut settings, mut files, mut flushed) = (None, None, Vec::new());
        for job in std::iter::once(first).chain(jobs.try_iter()) {
            match job {
                Job::Settings(s) => settings = Some(s),
                Job::Files(f) => files = Some(f),
                Job::Flush(done) => flushed.push(done),
            }
        }
        if let Some(settings) = settings {
            write_json(&dir.join("settings.json"), &settings, true);
        }
        if let Some(touched) = files {
            merge_files(dir, touched);
        }
        for done in flushed {
            let _ = done.send(());
        }
    }
}

/// Lays this process's entries over the current `files.json` and writes the result, holding
/// the profile lock so two Lumen windows never interleave a read and a write.
fn merge_files(dir: &Path, touched: BTreeMap<String, FileState>) {
    create_private_dir(dir);
    let _lock = ProfileLock::take(dir);
    let path = dir.join("files.json");
    let mut files = load_files(&path);
    let newest = files.values().map(|f| f.saved).max().unwrap_or(0);
    let mut order: Vec<_> = touched.into_iter().collect();
    order.sort_by_key(|(_, state)| state.saved);
    for (n, (key, mut state)) in (1..).zip(order) {
        // Ours were saved after anything another window wrote before this merge.
        state.saved = newest + n;
        files.insert(key, state);
    }
    evict_oldest(&mut files, FILES_LIMIT);
    write_json(&path, &files, false);
}

/// An advisory lock on the profile folder, released when dropped. Unix only; elsewhere the
/// merge still keeps another window's entries but two writes can race.
struct ProfileLock {
    #[cfg(unix)]
    _file: Option<fs::File>,
}

impl ProfileLock {
    fn take(dir: &Path) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            use std::os::unix::io::AsRawFd;
            let file = fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .mode(0o600)
                .open(dir.join(".lock"))
                .ok()
                // SAFETY: flock on a descriptor this function owns; it is released on close.
                .filter(|f| unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX) } == 0);
            Self { _file: file }
        }
        #[cfg(not(unix))]
        {
            let _ = dir;
            Self {}
        }
    }
}

/// Creates `dir` (0700 on Unix) and tightens it to 0700 if it already exists with looser
/// permissions. A symlink is left alone rather than changing whatever it points at.
fn create_private_dir(dir: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        let _ = fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir);
        if let Ok(meta) = fs::symlink_metadata(dir)
            && meta.is_dir()
            && meta.permissions().mode() & 0o077 != 0
        {
            let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
        }
    }
    #[cfg(not(unix))]
    let _ = fs::create_dir_all(dir);
}

/// Writes through a private (0600), freshly created temporary file with a random name,
/// fsynced before the rename, so a crash never leaves half a profile and a planted
/// `settings.json.tmp` symlink cannot redirect the write. The rename replaces `path` itself,
/// never a file a symlink there points at.
fn write_json<T: Serialize>(path: &Path, value: &T, pretty: bool) {
    use std::io::Write;
    let bytes = if pretty {
        serde_json::to_vec_pretty(value)
    } else {
        serde_json::to_vec(value)
    };
    let Ok(bytes) = bytes else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    create_private_dir(parent);
    let written = tempfile::Builder::new()
        .prefix(".lumen-")
        .suffix(".tmp")
        .tempfile_in(parent)
        .and_then(|mut temp| {
            temp.write_all(&bytes)?;
            temp.as_file().sync_all()?;
            temp.persist(path).map_err(|e| e.error)
        });
    if let Err(e) = written {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        eprintln!("lumen: could not save {name}: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_state_round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path().to_path_buf());
        let state = FileState {
            position: 12_000,
            length: 30_000,
            spu: Some(-1),
            bookmarks: vec![5_000],
            ..Default::default()
        };
        store.save_file_state("/a.mkv", state.clone());
        store.settings.volume = 120;
        store.remember_recent("/a.mkv");
        store.flush();

        let reopened = Store::open(dir.path().to_path_buf());
        assert_eq!(reopened.file_state("/a.mkv"), state);
        assert_eq!(reopened.file_state("/other.mkv"), FileState::default());
        assert_eq!(reopened.settings.volume, 120);
        assert_eq!(reopened.settings.recent, ["/a.mkv"]);
    }

    #[test]
    fn recent_list_is_deduplicated_and_capped() {
        let mut store = Store::in_memory();
        for n in 0..20 {
            store.remember_recent(&format!("/{n}"));
        }
        store.remember_recent("/10");
        assert_eq!(store.settings.recent.len(), RECENT_LIMIT);
        assert_eq!(store.settings.recent[0], "/10");
        assert_eq!(
            store.settings.recent.iter().filter(|s| *s == "/10").count(),
            1
        );
    }

    #[test]
    fn corrupt_profile_falls_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("settings.json"), "{ not json").unwrap();
        fs::write(dir.path().join("files.json"), "[]").unwrap();
        let store = Store::open(dir.path().to_path_buf());
        assert_eq!(store.settings.volume, 80);
        // The unreadable file is kept for the user rather than overwritten.
        assert_eq!(
            fs::read_to_string(dir.path().join("settings.json.broken")).unwrap(),
            "{ not json"
        );
    }

    #[test]
    fn one_bad_setting_keeps_the_others() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("settings.json"),
            r#"{"volume": 55, "seek_step": "ten", "resume": false}"#,
        )
        .unwrap();
        let store = Store::open(dir.path().to_path_buf());
        assert_eq!(store.settings.volume, 55);
        assert!(!store.settings.resume);
        assert_eq!(store.settings.seek_step, Settings::default().seek_step);
    }

    #[test]
    fn a_stored_volume_out_of_range_is_clamped() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("settings.json"),
            r#"{"volume": 4294967295}"#,
        )
        .unwrap();
        assert_eq!(
            Store::open(dir.path().to_path_buf()).settings.volume,
            MAX_VOLUME
        );
    }

    #[test]
    fn one_bad_file_entry_keeps_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let (good, bad) = (Store::file_key("/good.mkv"), Store::file_key("/bad.mkv"));
        fs::write(
            dir.path().join("files.json"),
            format!(r#"{{"{good}": {{"position": 7}}, "{bad}": {{"position": "x"}}}}"#),
        )
        .unwrap();
        let mut store = Store::open(dir.path().to_path_buf());
        assert_eq!(store.file_state("/good.mkv").position, 7);
        store.save_file_state("/new.mkv", FileState::default());
        store.flush();
        let reopened = Store::open(dir.path().to_path_buf());
        assert_eq!(reopened.file_state("/good.mkv").position, 7);
    }

    #[test]
    fn two_windows_keep_each_others_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut first = Store::open(dir.path().to_path_buf());
        let mut second = Store::open(dir.path().to_path_buf());
        let at = |position| FileState {
            position,
            ..Default::default()
        };
        first.save_file_state("/a.mkv", at(1_000));
        first.flush();
        second.save_file_state("/b.mkv", at(2_000));
        second.flush();
        first.save_file_state("/a.mkv", at(3_000));
        first.flush();
        let reopened = Store::open(dir.path().to_path_buf());
        assert_eq!(reopened.file_state("/a.mkv").position, 3_000);
        assert_eq!(reopened.file_state("/b.mkv").position, 2_000);
    }

    #[test]
    fn recent_redacts_network_secrets() {
        let mut store = Store::in_memory();
        store.remember_recent("rtsp://user:pass@cam.local:554/live?token=abc#x");
        store.remember_recent("https://u:p@example.com/v.mp4?sig=SECRET");
        store.remember_recent("/home/me/a.mkv?keep");
        store.remember_recent("srt://cam.local:9000?passphrase=hunter22");
        store.remember_recent("ftp://me:pw@files.local/a.mkv");
        store.remember_recent("file:///home/me/b.mkv?keep");
        assert_eq!(
            store.settings.recent,
            [
                "file:///home/me/b.mkv?keep",
                "ftp://files.local/a.mkv",
                "srt://cam.local:9000",
                "/home/me/a.mkv?keep",
                "https://example.com/v.mp4",
                "rtsp://cam.local:554/live"
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn profile_files_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let base = tempfile::tempdir().unwrap();
        let dir = base.path().join("profile");
        let mut store = Store::open(dir.clone());
        store.save_file_state("/a", FileState::default());
        store.save_settings();
        store.flush();
        let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&dir.join("settings.json")), 0o600);
        assert_eq!(mode(&dir.join("files.json")), 0o600);
        assert_eq!(mode(&dir), 0o700);
    }

    #[cfg(unix)]
    #[test]
    fn saving_never_writes_through_planted_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let victim = dir.path().join("victim");
        fs::write(&victim, "untouched").unwrap();
        let profile = dir.path().join("profile");
        fs::create_dir(&profile).unwrap();
        // The old fixed temporary name, and the target itself, both pointing elsewhere.
        std::os::unix::fs::symlink(&victim, profile.join("settings.json.tmp")).unwrap();
        std::os::unix::fs::symlink(&victim, profile.join("settings.json")).unwrap();
        let mut store = Store::open(profile.clone());
        store.settings.volume = 42;
        store.save_settings();
        store.flush();
        assert_eq!(fs::read_to_string(&victim).unwrap(), "untouched");
        let saved = fs::symlink_metadata(profile.join("settings.json")).unwrap();
        assert!(saved.is_file(), "the link is replaced by a real file");
        assert_eq!(Store::open(profile).settings.volume, 42);
    }

    #[cfg(unix)]
    #[test]
    fn an_existing_profile_folder_is_made_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let profile = dir.path().join("profile");
        fs::create_dir(&profile).unwrap();
        fs::set_permissions(&profile, fs::Permissions::from_mode(0o755)).unwrap();
        Store::open(profile.clone()).save_settings();
        let mode = fs::metadata(&profile).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
        let leftovers: Vec<_> = fs::read_dir(&profile)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "no temporary files are left behind");
    }

    #[test]
    fn remembered_files_are_capped_evicting_oldest() {
        let mut store = Store::in_memory();
        for n in 0..FILES_LIMIT + 5 {
            store.save_file_state(
                &format!("/{n}"),
                FileState {
                    position: n as i64 + 1,
                    ..Default::default()
                },
            );
        }
        assert_eq!(store.files.len(), FILES_LIMIT);
        assert_eq!(store.file_state("/0"), FileState::default());
        assert_eq!(store.file_state("/4"), FileState::default());
        assert_eq!(store.file_state("/5").position, 6);
        // Re-saving refreshes recency.
        store.save_file_state("/5", FileState::default());
        store.save_file_state("/new", FileState::default());
        assert!(store.files.contains_key(&Store::file_key("/5")));
        assert!(!store.files.contains_key(&Store::file_key("/6")));
    }

    #[test]
    fn old_files_json_without_saved_field_loads() {
        let dir = tempfile::tempdir().unwrap();
        let key = Store::file_key("/old.mkv");
        fs::write(
            dir.path().join("files.json"),
            format!(r#"{{"{key}": {{"position": 7, "length": 9}}}}"#),
        )
        .unwrap();
        let store = Store::open(dir.path().to_path_buf());
        let state = store.file_state("/old.mkv");
        assert_eq!((state.position, state.length), (7, 9));
    }
}
