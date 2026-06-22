use std::collections::HashSet;
use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use jwalk::WalkDir;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferMode {
    Copy,
    Move,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictChoice {
    Skip,
    Replace,
    Rename,
    Cancel,
}

#[derive(Clone, Debug, Default)]
pub struct TransferProgress {
    pub active: bool,
    pub counting: bool,
    pub operation: String,
    pub label: String,
    pub total_files: usize,
    pub done_files: usize,
    pub total_bytes: u64,
    pub done_bytes: u64,
    pub error: Option<String>,
}

pub struct PendingConflict {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub reply: Sender<ConflictChoice>,
}

struct ConflictState {
    apply_to_all: Arc<Mutex<Option<ConflictChoice>>>,
}

enum TransferEvent {
    Totals {
        total_files: usize,
        total_bytes: u64,
    },
    Progress {
        done_files: usize,
        done_bytes: u64,
        current: String,
    },
    Conflict {
        source: PathBuf,
        destination: PathBuf,
        reply: Sender<ConflictChoice>,
    },
    Done {
        invalidate: Vec<PathBuf>,
    },
    Error(String),
}

const COPY_BUFFER_SIZE: usize = 4 * 1024 * 1024;
const PROGRESS_UPDATE_INTERVAL: Duration = Duration::from_millis(100);

pub struct TransferManager {
    event_rx: Option<Receiver<TransferEvent>>,
    pub progress: TransferProgress,
    pub pending_conflict: Option<PendingConflict>,
    pub apply_to_all: bool,
    conflict_state: Option<ConflictState>,
    invalidation: Vec<PathBuf>,
}

impl TransferManager {
    pub fn new() -> Self {
        Self {
            event_rx: None,
            progress: TransferProgress::default(),
            pending_conflict: None,
            apply_to_all: false,
            conflict_state: None,
            invalidation: Vec::new(),
        }
    }

    pub fn take_invalidation(&mut self) -> Vec<PathBuf> {
        let mut unique = HashSet::new();
        std::mem::take(&mut self.invalidation)
            .into_iter()
            .filter(|path| unique.insert(path.clone()))
            .collect()
    }

    pub fn is_active(&self) -> bool {
        self.progress.active
    }

    pub fn has_conflict(&self) -> bool {
        self.pending_conflict.is_some()
    }

    pub fn start(&mut self, sources: Vec<PathBuf>, dest_dir: PathBuf, mode: TransferMode) {
        if sources.is_empty() || self.progress.active {
            return;
        }

        let (event_tx, event_rx) = mpsc::channel();
        let apply_to_all = Arc::new(Mutex::new(None));
        self.conflict_state = Some(ConflictState {
            apply_to_all: Arc::clone(&apply_to_all),
        });
        self.apply_to_all = false;
        self.pending_conflict = None;
        self.event_rx = Some(event_rx);

        let operation = match mode {
            TransferMode::Copy => "Copying".to_string(),
            TransferMode::Move => "Moving".to_string(),
        };
        self.progress = TransferProgress {
            active: true,
            counting: true,
            operation,
            label: "Calculating size…".to_string(),
            total_files: 0,
            done_files: 0,
            total_bytes: 0,
            done_bytes: 0,
            error: None,
        };

        thread::spawn(move || {
            // Process 1: traverse all staged inputs and only calculate totals.
            let (total_files, total_bytes) = count_transfer(sources.as_slice());
            let _ = event_tx.send(TransferEvent::Totals {
                total_files,
                total_bytes,
            });

            // Process 2: perform the actual transfer using the precalculated totals.
            let result = run_transfer(sources, dest_dir, mode, &event_tx, apply_to_all);
            if let Err(message) = result {
                let _ = event_tx.send(TransferEvent::Error(message));
            }
        });
    }

    pub fn poll(&mut self) -> bool {
        let events: Vec<TransferEvent> = if let Some(rx) = self.event_rx.as_ref() {
            rx.try_iter().collect()
        } else {
            return false;
        };

        if events.is_empty() {
            return false;
        }

        for event in events {
            match event {
                TransferEvent::Totals {
                    total_files,
                    total_bytes,
                } => {
                    self.progress.total_files = total_files;
                    self.progress.total_bytes = total_bytes;
                    self.progress.counting = false;
                    self.progress.label = "Starting transfer…".to_string();
                }
                TransferEvent::Progress {
                    done_files,
                    done_bytes,
                    current,
                } => {
                    self.progress.counting = false;
                    self.progress.done_files = done_files;
                    self.progress.done_bytes = done_bytes;
                    if self.progress.total_bytes < self.progress.done_bytes {
                        self.progress.total_bytes = self.progress.done_bytes;
                    }
                    self.progress.label = current;
                }
                TransferEvent::Conflict {
                    source,
                    destination,
                    reply,
                } => {
                    if let Some(state) = &self.conflict_state {
                        if let Ok(locked) = state.apply_to_all.lock() {
                            if let Some(choice) = *locked {
                                let _ = reply.send(choice);
                                continue;
                            }
                        }
                    }
                    self.pending_conflict = Some(PendingConflict {
                        source,
                        destination,
                        reply,
                    });
                }
                TransferEvent::Done { invalidate } => {
                    self.invalidation = invalidate;
                    self.finish();
                }
                TransferEvent::Error(message) => {
                    self.progress.error = Some(message);
                    self.progress.active = false;
                    self.event_rx = None;
                    self.conflict_state = None;
                }
            }
        }
        true
    }

    pub fn resolve_conflict(&mut self, choice: ConflictChoice) {
        let Some(conflict) = self.pending_conflict.take() else {
            return;
        };

        if self.apply_to_all && choice != ConflictChoice::Cancel {
            if let Some(state) = &self.conflict_state {
                if let Ok(mut locked) = state.apply_to_all.lock() {
                    *locked = Some(choice);
                }
            }
        }

        let _ = conflict.reply.send(choice);

        if choice == ConflictChoice::Cancel {
            self.progress.error = Some("Transfer cancelled".into());
            self.progress.active = false;
            self.event_rx = None;
            self.conflict_state = None;
        }
    }

    fn finish(&mut self) {
        self.progress.active = false;
        self.progress.label = "Done".into();
        self.event_rx = None;
        self.conflict_state = None;
        self.pending_conflict = None;
    }
}

impl Default for TransferManager {
    fn default() -> Self {
        Self::new()
    }
}

fn count_transfer(sources: &[PathBuf]) -> (usize, u64) {
    let mut files = 0usize;
    let mut bytes = 0u64;
    for source in sources {
        count_path(source, &mut files, &mut bytes);
    }
    (files.max(1), bytes)
}

fn count_path(path: &Path, files: &mut usize, bytes: &mut u64) {
    if path.is_file() {
        *files += 1;
        if let Ok(meta) = path.metadata() {
            *bytes += meta.len();
        }
        return;
    }
    if path.is_dir() {
        for entry in WalkDir::new(path).into_iter().flatten() {
            if entry.file_type().is_file() {
                *files += 1;
                if let Ok(meta) = entry.metadata() {
                    *bytes += meta.len();
                }
            }
        }
    }
}

fn run_transfer(
    sources: Vec<PathBuf>,
    dest_dir: PathBuf,
    mode: TransferMode,
    event_tx: &Sender<TransferEvent>,
    apply_to_all: Arc<Mutex<Option<ConflictChoice>>>,
) -> Result<(), String> {
    let mut done_files = 0usize;
    let mut done_bytes = 0u64;
    let mut invalidate = vec![dest_dir.clone()];

    for source in sources {
        if !source.exists() {
            continue;
        }

        let file_name = source
            .file_name()
            .ok_or_else(|| format!("Invalid source path: {}", source.display()))?;
        if source.parent() == Some(dest_dir.as_path()) && mode == TransferMode::Move {
            continue;
        }

        let label = file_name.to_string_lossy().into_owned();
        let _ = event_tx.send(TransferEvent::Progress {
            done_files,
            done_bytes,
            current: label.clone(),
        });

        let (files, bytes) = transfer_entry(
            &source,
            &dest_dir.join(file_name),
            mode,
            event_tx,
            &apply_to_all,
            &mut done_files,
            &mut done_bytes,
        )?;

        let _ = (files, bytes);
        invalidate.push(source.clone());
        if source.parent().is_some() {
            invalidate.push(source.parent().unwrap().to_path_buf());
        }
        let _ = event_tx.send(TransferEvent::Progress {
            done_files,
            done_bytes,
            current: label,
        });
    }

    let _ = event_tx.send(TransferEvent::Done { invalidate });
    Ok(())
}

fn transfer_entry(
    source: &Path,
    dest: &Path,
    mode: TransferMode,
    event_tx: &Sender<TransferEvent>,
    apply_to_all: &Arc<Mutex<Option<ConflictChoice>>>,
    done_files: &mut usize,
    done_bytes: &mut u64,
) -> Result<(usize, u64), String> {
    if source.is_dir() {
        return transfer_directory(
            source,
            dest,
            mode,
            event_tx,
            apply_to_all,
            done_files,
            done_bytes,
        );
    }

    let actual_dest = resolve_destination(source, dest, event_tx, apply_to_all)?;
    let Some(actual_dest) = actual_dest else {
        return Ok((0, 0));
    };

    if mode == TransferMode::Move && !actual_dest.exists() {
        let bytes = source.metadata().map(|meta| meta.len()).unwrap_or(0);
        if fs::rename(source, &actual_dest).is_ok() {
            *done_files += 1;
            *done_bytes += bytes;
            let _ = event_tx.send(TransferEvent::Progress {
                done_files: *done_files,
                done_bytes: *done_bytes,
                current: actual_dest
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("")
                    .to_string(),
            });
            return Ok((1, bytes));
        }
    }

    let bytes = copy_file_with_progress(source, &actual_dest, event_tx, *done_files, *done_bytes)?;
    *done_files += 1;
    *done_bytes += bytes;
    let _ = event_tx.send(TransferEvent::Progress {
        done_files: *done_files,
        done_bytes: *done_bytes,
        current: actual_dest
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string(),
    });

    if mode == TransferMode::Move {
        fs::remove_file(source).map_err(|e| e.to_string())?;
    }

    Ok((1, bytes))
}

fn transfer_directory(
    source: &Path,
    dest: &Path,
    mode: TransferMode,
    event_tx: &Sender<TransferEvent>,
    apply_to_all: &Arc<Mutex<Option<ConflictChoice>>>,
    done_files: &mut usize,
    done_bytes: &mut u64,
) -> Result<(usize, u64), String> {
    let actual_dest = resolve_destination(source, dest, event_tx, apply_to_all)?;
    let Some(actual_dest) = actual_dest else {
        return Ok((0, 0));
    };

    fs::create_dir_all(&actual_dest).map_err(|e| e.to_string())?;

    let mut added_files = 0usize;
    let mut added_bytes = 0u64;

    for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let child_source = entry.path();
        let child_dest = actual_dest.join(entry.file_name());
        let (files, bytes) = transfer_entry(
            &child_source,
            &child_dest,
            mode,
            event_tx,
            apply_to_all,
            done_files,
            done_bytes,
        )?;
        added_files += files;
        added_bytes += bytes;
    }

    if mode == TransferMode::Move {
        let _ = fs::remove_dir_all(source);
    }

    Ok((added_files, added_bytes))
}

fn resolve_destination(
    source: &Path,
    dest: &Path,
    event_tx: &Sender<TransferEvent>,
    apply_to_all: &Arc<Mutex<Option<ConflictChoice>>>,
) -> Result<Option<PathBuf>, String> {
    if !dest.exists() {
        return Ok(Some(dest.to_path_buf()));
    }

    if let Ok(locked) = apply_to_all.lock() {
        if let Some(choice) = *locked {
            return Ok(apply_conflict_choice(dest, choice));
        }
    }

    let (reply_tx, reply_rx) = mpsc::channel();
    event_tx
        .send(TransferEvent::Conflict {
            source: source.to_path_buf(),
            destination: dest.to_path_buf(),
            reply: reply_tx,
        })
        .map_err(|e| e.to_string())?;

    let choice = reply_rx
        .recv()
        .map_err(|_| "Conflict dialog closed".to_string())?;

    if choice == ConflictChoice::Cancel {
        return Err("Transfer cancelled".into());
    }

    Ok(apply_conflict_choice(dest, choice))
}

fn apply_conflict_choice(dest: &Path, choice: ConflictChoice) -> Option<PathBuf> {
    match choice {
        ConflictChoice::Skip => None,
        ConflictChoice::Replace => {
            if dest.is_dir() {
                let _ = fs::remove_dir_all(dest);
            } else if dest.is_file() {
                let _ = fs::remove_file(dest);
            }
            Some(dest.to_path_buf())
        }
        ConflictChoice::Rename => Some(unique_path(dest)),
        ConflictChoice::Cancel => None,
    }
}

fn unique_path(dest: &Path) -> PathBuf {
    if !dest.exists() {
        return dest.to_path_buf();
    }

    let stem = dest
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let extension = dest
        .extension()
        .and_then(|s| s.to_str())
        .map(|ext| format!(".{ext}"))
        .unwrap_or_default();
    let parent = dest.parent().unwrap_or_else(|| Path::new("."));

    for index in 1..10_000 {
        let candidate = parent.join(format!("{stem} ({index}){extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    dest.to_path_buf()
}

fn copy_file_with_progress(
    source: &Path,
    dest: &Path,
    event_tx: &Sender<TransferEvent>,
    done_files: usize,
    done_bytes_base: u64,
) -> Result<u64, String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut input = fs::File::open(source).map_err(|e| e.to_string())?;
    let output = fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut output = BufWriter::with_capacity(COPY_BUFFER_SIZE, output);

    let mut buffer = vec![0u8; COPY_BUFFER_SIZE];
    let mut copied = 0u64;
    let mut last_emit = Instant::now();
    let mut emitted_once = false;
    let label = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_string();

    loop {
        let read = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read]).map_err(|e| e.to_string())?;
        copied += read as u64;

        if !emitted_once || last_emit.elapsed() >= PROGRESS_UPDATE_INTERVAL {
            let _ = event_tx.send(TransferEvent::Progress {
                done_files,
                done_bytes: done_bytes_base + copied,
                current: label.clone(),
            });
            last_emit = Instant::now();
            emitted_once = true;
        }
    }

    output.flush().map_err(|e| e.to_string())?;
    Ok(copied)
}

pub fn path_disk_size(path: &Path) -> u64 {
    if path.is_file() {
        return path.metadata().map(|meta| meta.len()).unwrap_or(0);
    }
    if path.is_dir() {
        return WalkDir::new(path)
            .into_iter()
            .flatten()
            .filter(|entry| entry.file_type().is_file())
            .filter_map(|entry| entry.metadata().ok())
            .map(|meta| meta.len())
            .sum();
    }
    0
}
