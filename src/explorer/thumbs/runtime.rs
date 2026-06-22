use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};

use eframe::egui::{self, Vec2};

use super::db::ThumbnailDb;
use super::generate::{display_size, generate_thumbnail, mtime_ns, png_to_color_image};
use super::media::{MediaKind, media_kind};
use crate::explorer::fs::FileEntry;

const MAX_CONCURRENT: usize = 4;

struct CachedThumb {
    handle: egui::TextureHandle,
    pixel_size: [u32; 2],
    is_video: bool,
}

struct PendingJob {
    path: PathBuf,
    generation: u64,
    receiver: mpsc::Receiver<WorkerResult>,
}

struct WorkerResult {
    path: PathBuf,
    generation: u64,
    image: Option<(egui::ColorImage, [u32; 2])>,
}

pub struct ThumbnailRuntime {
    db: Arc<Mutex<ThumbnailDb>>,
    directory: Option<PathBuf>,
    generation: u64,
    job_queue: VecDeque<(PathBuf, MediaKind, u128)>,
    pending: Vec<PendingJob>,
    ready: Vec<(PathBuf, egui::ColorImage, [u32; 2])>,
    textures: HashMap<PathBuf, CachedThumb>,
    in_flight: HashSet<PathBuf>,
    total_jobs: usize,
    finished_jobs: usize,
}

impl ThumbnailRuntime {
    pub fn new() -> Self {
        Self {
            db: Arc::new(Mutex::new(ThumbnailDb::open().unwrap_or_default())),
            directory: None,
            generation: 0,
            job_queue: VecDeque::new(),
            pending: Vec::new(),
            ready: Vec::new(),
            textures: HashMap::new(),
            in_flight: HashSet::new(),
            total_jobs: 0,
            finished_jobs: 0,
        }
    }

    pub fn on_directory_changing(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.directory = None;
        self.job_queue.clear();
        self.pending.clear();
        self.in_flight.clear();
        self.total_jobs = 0;
        self.finished_jobs = 0;
        self.textures.clear();
        self.ready.clear();
    }

    /// Process 2: queue thumbnail work after directory metadata is complete.
    pub fn begin_directory_load(&mut self, directory: PathBuf, entries: &[FileEntry]) {
        self.generation = self.generation.wrapping_add(1);
        self.directory = Some(directory);
        self.job_queue.clear();
        self.pending.clear();
        self.in_flight.clear();
        self.ready.clear();
        self.textures.clear();
        self.total_jobs = 0;
        self.finished_jobs = 0;

        for entry in entries {
            if entry.is_dir {
                continue;
            }
            if let Some(kind) = media_kind(&entry.path) {
                self.total_jobs += 1;
                self.job_queue
                    .push_back((entry.path.clone(), kind, mtime_ns(entry.modified)));
            }
        }

        self.dispatch();
    }

    pub fn directory(&self) -> Option<&Path> {
        self.directory.as_deref()
    }

    pub fn is_loading(&self, directory: &Path) -> bool {
        if self.directory.as_deref() != Some(directory) {
            return false;
        }
        if self.total_jobs == 0 {
            return false;
        }
        self.finished_jobs < self.total_jobs
            || !self.job_queue.is_empty()
            || !self.pending.is_empty()
    }

    pub fn progress(&self, directory: &Path) -> f32 {
        if self.directory.as_deref() != Some(directory) || self.total_jobs == 0 {
            return 1.0;
        }
        (self.finished_jobs as f32 / self.total_jobs as f32).clamp(0.0, 1.0)
    }

    fn dispatch(&mut self) {
        while self.pending.len() < MAX_CONCURRENT {
            let Some((path, kind, modified_ns)) = self.job_queue.pop_front() else {
                break;
            };

            if self.textures.contains_key(&path) {
                self.finished_jobs += 1;
                continue;
            }

            self.in_flight.insert(path.clone());
            let (tx, rx) = mpsc::channel();
            let db = Arc::clone(&self.db);
            let path_for_thread = path.clone();
            let generation = self.generation;

            std::thread::spawn(move || {
                let image = (|| {
                    let db = db.lock().ok()?;
                    let record =
                        generate_thumbnail(&db, &path_for_thread, modified_ns, kind).ok()??;
                    let pixel_size = [record.width, record.height];
                    let image = png_to_color_image(&record.png).ok()?;
                    Some((image, pixel_size))
                })();
                let _ = tx.send(WorkerResult {
                    path: path_for_thread,
                    generation,
                    image,
                });
            });

            self.pending.push(PendingJob {
                path,
                generation,
                receiver: rx,
            });
        }
    }

    pub fn poll(&mut self, ctx: &egui::Context) -> bool {
        let mut changed = false;

        self.pending.retain_mut(|job| {
            match job.receiver.try_recv() {
                Ok(result) => {
                    if result.generation == self.generation {
                        if let Some((image, pixel_size)) = result.image {
                            self.ready.push((result.path.clone(), image, pixel_size));
                        }
                        self.finished_jobs += 1;
                    }
                    self.in_flight.remove(&result.path);
                    false
                }
                Err(mpsc::TryRecvError::Empty) => true,
                Err(mpsc::TryRecvError::Disconnected) => {
                    if job.generation == self.generation {
                        self.finished_jobs += 1;
                    }
                    self.in_flight.remove(&job.path);
                    false
                }
            }
        });

        if !self.pending.is_empty() || !self.job_queue.is_empty() {
            self.dispatch();
        }

        for (path, image, pixel_size) in self.ready.drain(..) {
            if self.textures.contains_key(&path) {
                continue;
            }
            let name = path.to_string_lossy().into_owned();
            let handle = ctx.load_texture(name, image, egui::TextureOptions::LINEAR);
            let is_video = matches!(media_kind(&path), Some(MediaKind::Video));
            self.textures.insert(
                path,
                CachedThumb {
                    handle,
                    pixel_size,
                    is_video,
                },
            );
            changed = true;
        }

        changed
    }

    pub fn texture(&self, path: &Path) -> Option<egui::TextureId> {
        self.textures
            .get(path)
            .map(|cached| cached.handle.id())
    }

    pub fn display_size(&self, path: &Path, max_side: f32) -> Option<Vec2> {
        let cached = self.textures.get(path)?;
        Some(display_size(
            cached.pixel_size[0],
            cached.pixel_size[1],
            max_side,
        ))
    }

    pub fn is_video_thumbnail(&self, path: &Path) -> bool {
        self.textures
            .get(path)
            .is_some_and(|cached| cached.is_video)
    }
}

impl Default for ThumbnailRuntime {
    fn default() -> Self {
        Self::new()
    }
}
