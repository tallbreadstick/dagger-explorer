use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver, Sender};
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
    Done {
        invalidate: Vec<PathBuf>,
    },
    Error(String),
}

const KIO_COMPLETION_TIMEOUT: Duration = Duration::from_secs(300);
const KIO_COMPLETION_POLL_INTERVAL: Duration = Duration::from_millis(120);

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
        self.conflict_state = None;
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
            label: "Starting transfer…".to_string(),
            total_files: 0,
            done_files: 0,
            total_bytes: 0,
            done_bytes: 0,
            error: None,
        };

        let count_sources = sources.clone();
        let count_tx = event_tx.clone();
        thread::spawn(move || {
            let (total_files, total_bytes) = count_transfer(count_sources.as_slice());
            let _ = count_tx.send(TransferEvent::Totals {
                total_files,
                total_bytes,
            });
        });

        thread::spawn(move || {
            let result = run_transfer(sources, dest_dir, mode, &event_tx);
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
                    if total_files > 0 {
                        self.progress.total_files = total_files;
                    }
                    if total_bytes > 0 {
                        self.progress.total_bytes = total_bytes;
                    }
                }
                TransferEvent::Progress {
                    done_files,
                    done_bytes,
                    current,
                } => {
                    self.progress.counting = false;
                    self.progress.done_files = done_files;
                    self.progress.done_bytes = done_bytes;
                    self.progress.label = current;
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
) -> Result<(), String> {
    let Some(kioclient) = detect_kioclient_binary() else {
        return Err(
            "Could not find `kioclient6`, `kioclient5`, or `kioclient` in PATH".to_string(),
        );
    };

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

        let (source_files, source_bytes) = transfer_stats(&source);
        let dest = destination_path_for(&source, &dest_dir)?;
        run_kioclient_command(kioclient, mode, &source, &dest_dir)?;
        wait_for_transfer_completion(mode, &source, &dest, source_files, source_bytes)?;

        done_files += source_files;
        done_bytes += source_bytes;
        invalidate.push(source.clone());
        invalidate.push(dest.clone());
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

fn run_kioclient_command(
    kioclient: &str,
    mode: TransferMode,
    source: &Path,
    dest_dir: &Path,
) -> Result<(), String> {
    let action = match mode {
        TransferMode::Copy => "copy",
        TransferMode::Move => "move",
    };

    let output = Command::new(kioclient)
        .arg(action)
        .arg(source.as_os_str())
        .arg(dest_dir.as_os_str())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("Failed to run {kioclient} {action}: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        Err(format!(
            "{kioclient} {action} failed for {}",
            source.display()
        ))
    } else {
        Err(stderr)
    }
}

fn detect_kioclient_binary() -> Option<&'static str> {
    for candidate in ["kioclient6", "kioclient5", "kioclient"] {
        if Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            return Some(candidate);
        }
    }
    None
}

fn destination_path_for(source: &Path, dest_dir: &Path) -> Result<PathBuf, String> {
    let file_name = source
        .file_name()
        .ok_or_else(|| format!("Invalid source path: {}", source.display()))?;
    Ok(dest_dir.join(file_name))
}

fn transfer_stats(path: &Path) -> (usize, u64) {
    let mut files = 0usize;
    let mut bytes = 0u64;
    count_path(path, &mut files, &mut bytes);
    (files.max(1), bytes)
}

fn wait_for_transfer_completion(
    mode: TransferMode,
    source: &Path,
    dest: &Path,
    expected_files: usize,
    expected_bytes: u64,
) -> Result<(), String> {
    let started = Instant::now();
    while started.elapsed() <= KIO_COMPLETION_TIMEOUT {
        let completed = match mode {
            TransferMode::Copy => {
                if !dest.exists() {
                    false
                } else {
                    let (dest_files, dest_bytes) = transfer_stats(dest);
                    dest_files >= expected_files && dest_bytes >= expected_bytes
                }
            }
            TransferMode::Move => {
                if source.exists() || !dest.exists() {
                    false
                } else {
                    let (dest_files, dest_bytes) = transfer_stats(dest);
                    dest_files >= expected_files && dest_bytes >= expected_bytes
                }
            }
        };

        if completed {
            return Ok(());
        }
        thread::sleep(KIO_COMPLETION_POLL_INTERVAL);
    }

    Err(format!(
        "Timed out waiting for {} operation to finish for {}",
        match mode {
            TransferMode::Copy => "copy",
            TransferMode::Move => "move",
        },
        source.display()
    ))
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
