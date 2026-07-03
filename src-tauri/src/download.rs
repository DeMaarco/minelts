use serde::Serialize;
use sha1_smol::Sha1;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

const MAX_RETRIES: u32 = 3;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const READ_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub url: String,
    pub dest: PathBuf,
    pub expected_sha1: Option<String>,
    pub expected_size: Option<u64>,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub enum DownloadProgress {
    Started { label: String },
    Completed,
    Failed { label: String, error: String },
    Progress { completed: usize, total: usize },
}

pub struct ParallelDownloader {
    workers: usize,
}

fn http_agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_connect(CONNECT_TIMEOUT)
            .timeout_read(READ_TIMEOUT)
            .max_idle_connections_per_host(4)
            .build()
    })
}

impl ParallelDownloader {
    pub fn new(workers: usize) -> Self {
        Self {
            workers: workers.max(1),
        }
    }

    pub fn run(
        &self,
        tasks: Vec<DownloadTask>,
        progress_tx: Option<std::sync::mpsc::Sender<DownloadProgress>>,
    ) -> Result<(), String> {
        if tasks.is_empty() {
            return Ok(());
        }

        let total = tasks.len();
        let completed = Arc::new(AtomicUsize::new(0));
        let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let queue = Arc::new(Mutex::new(tasks));

        if let Some(tx) = &progress_tx {
            let _ = tx.send(DownloadProgress::Progress {
                completed: 0,
                total,
            });
        }

        let mut handles = Vec::new();

        for _ in 0..self.workers {
            let queue = Arc::clone(&queue);
            let completed = Arc::clone(&completed);
            let errors = Arc::clone(&errors);
            let progress_tx = progress_tx.clone();

            handles.push(thread::spawn(move || {
                loop {
                    let task = {
                        let mut q = queue.lock().unwrap();
                        q.pop()
                    };

                    let Some(task) = task else {
                        break;
                    };

                    if let Some(tx) = &progress_tx {
                        let _ = tx.send(DownloadProgress::Started {
                            label: task.label.clone(),
                        });
                    }

                    let result = download_one_with_retries(&task);

                    match result {
                        Ok(()) => {
                            let done = completed.fetch_add(1, Ordering::SeqCst) + 1;
                            if let Some(tx) = &progress_tx {
                                let _ = tx.send(DownloadProgress::Completed);
                                let _ = tx.send(DownloadProgress::Progress { completed: done, total });
                            }
                        }
                        Err(e) => {
                            if let Some(tx) = &progress_tx {
                                let _ = tx.send(DownloadProgress::Failed {
                                    label: task.label.clone(),
                                    error: e.clone(),
                                });
                            }
                            errors.lock().unwrap().push(format!("{}: {e}", task.label));
                            let done = completed.fetch_add(1, Ordering::SeqCst) + 1;
                            if let Some(tx) = &progress_tx {
                                let _ = tx.send(DownloadProgress::Progress { completed: done, total });
                            }
                        }
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().map_err(|_| "Hilo de descarga falló".to_string())?;
        }

        let errs = errors.lock().unwrap();
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs.join("\n"))
        }
    }
}

fn download_one_with_retries(task: &DownloadTask) -> Result<(), String> {
    let mut last_err = String::new();
    for attempt in 1..=MAX_RETRIES {
        match download_one(task) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = if attempt < MAX_RETRIES {
                    format!("{e} (reintento {attempt}/{MAX_RETRIES})")
                } else {
                    e
                };
                if attempt < MAX_RETRIES {
                    thread::sleep(Duration::from_millis(500 * attempt as u64));
                }
            }
        }
    }
    Err(last_err)
}

fn file_matches(task: &DownloadTask) -> bool {
    let Ok(meta) = fs::metadata(&task.dest) else {
        return false;
    };

    if let Some(expected_size) = task.expected_size {
        if meta.len() != expected_size {
            return false;
        }
    }

    if let Some(expected) = &task.expected_sha1 {
        if let Ok(actual) = sha1_file(&task.dest) {
            return actual.eq_ignore_ascii_case(expected);
        }
        return false;
    }

    true
}

fn download_one(task: &DownloadTask) -> Result<(), String> {
    if file_matches(task) {
        return Ok(());
    }

    if task.dest.exists() {
        fs::remove_file(&task.dest).ok();
    }

    if let Some(parent) = task.dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear directorio {}: {e}", parent.display()))?;
    }

    let response = http_agent()
        .get(&task.url)
        .call()
        .map_err(|e| format!("HTTP {}: {e}", task.url))?;

    let mut reader = response.into_reader();
    let mut data = Vec::new();
    reader
        .read_to_end(&mut data)
        .map_err(|e| format!("Error leyendo {}: {e}", task.url))?;

    if let Some(expected_size) = task.expected_size {
        if data.len() as u64 != expected_size {
            return Err(format!(
                "Tamaño incorrecto para {}: esperado {expected_size}, obtenido {}",
                task.dest.display(),
                data.len()
            ));
        }
    }

    if let Some(expected) = &task.expected_sha1 {
        let actual = sha1_bytes(&data);
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(format!(
                "SHA1 incorrecto para {}: esperado {expected}, obtenido {actual}",
                task.dest.display()
            ));
        }
    }

    let mut file = fs::File::create(&task.dest)
        .map_err(|e| format!("No se pudo crear {}: {e}", task.dest.display()))?;
    file.write_all(&data)
        .map_err(|e| format!("No se pudo escribir {}: {e}", task.dest.display()))?;

    Ok(())
}

pub fn sha1_file(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.digest().to_string())
}

pub fn sha1_bytes(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    hasher.digest().to_string()
}
