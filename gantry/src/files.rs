use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use notify::Watcher;

use tokio::fs::File;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::{Mutex, RwLock};

use crate::gcode::GcodeFile;

lazy_static::lazy_static! {
    /// channel to send requests to file watching tokio runtime
    static ref RW: (UnboundedSender<PathBuf>, Mutex<UnboundedReceiver<anyhow::Result<Arc<GcodeFile>>>>) = init();
}

/// regestered handlers for watched paths
static HANDLERS: Mutex<
    Vec<(
        PathBuf,
        Box<dyn Fn(&notify::Event) -> Pin<Box<dyn Future<Output = bool>>> + Sync + Send>,
    )>,
> = Mutex::const_new(Vec::new());

/// initialise local thread tokio to monitor and parse gcode files
fn init() -> (
    UnboundedSender<PathBuf>,
    Mutex<UnboundedReceiver<anyhow::Result<Arc<GcodeFile>>>>,
) {
    use notify::EventKind;

    // channel for recieving path requests
    let (sender, mut recv) = unbounded_channel::<PathBuf>();
    // channel for sending results
    let (re_sender, re_recv) = unbounded_channel::<anyhow::Result<Arc<GcodeFile>>>();

    // create local thread tokio runtime
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    std::thread::spawn(move || {
        // create a local set of tasks
        let local = tokio::task::LocalSet::new();

        // cache to store gcode files
        let cache = Arc::new(RwLock::new(HashMap::<PathBuf, Arc<GcodeFile>>::new()));
        // create reference to cache
        let cache1 = cache.clone();

        // create channel for watcher
        let (no_sender, mut no_recv) = unbounded_channel();

        // create watcher
        let watcher = Arc::new(Mutex::new(
            notify::recommended_watcher(move |res| {
                let _ = no_sender.send(res);
            })
            .expect("failed to initialise file watcher"),
        ));

        // create reference to watcher
        let watcher1 = watcher.clone();

        // spawn task to handle file parsing
        local.spawn_local(async move {
            while let Some(filename) = recv.recv().await {
                if let Some(g) = cache1.read().await.get(&filename) {
                    let _ = re_sender.send(Ok(g.clone()));
                    continue;
                }

                let re = try_parse_file(&filename).await;

                if let Ok(g) = &re {
                    cache1.write().await.insert(filename.clone(), g.clone());
                }

                let watch_re = watcher1
                    .lock()
                    .await
                    .watch(filename.as_path(), notify::RecursiveMode::NonRecursive);

                if let Err(e) = watch_re {
                    log::warn!("Filsystem watcher: {}", e);
                }

                let _ = re_sender.send(re);
            }
        });

        // spawn task to handle file change
        local.spawn_local(async move {
            while let Some(res) = no_recv.recv().await {
                let event = match res {
                    Ok(event) => event,
                    Err(e) => {
                        log::warn!("Filsystem watcher: {}", e);
                        continue;
                    }
                };

                match event.kind {
                    EventKind::Modify(_) | EventKind::Remove(_) => {
                        // uncache gcode files if modified or removed
                        for path in &event.paths {
                            cache.write().await.remove(path);
                        }
                    }
                    _ => {}
                }
                // invoke handlers
                // acquire lock
                let mut handlers = HANDLERS.lock().await;
                let mut i = 0;

                // loop handlers
                'outer: while let Some((path, handler)) = handlers.get(i) {
                    // loop event paths
                    for p in &event.paths {
                        // call handler if path is ancesstor of event path
                        if p.starts_with(path) {
                            // create future from handler
                            let f = (handler)(&event);
                            // await for the callback
                            let retain_handler = f.await;

                            // remove handler
                            if !retain_handler {
                                let _ = watcher.lock().await.unwatch(&path);
                                let _ = handlers.swap_remove(i);
                                // continue to next handler without incrementing
                                continue 'outer;
                            }
                        }
                    }

                    i += 1;
                }
            }
        });

        // run the tokio runtime
        rt.block_on(local);
    });

    return (sender, Mutex::new(re_recv));
}

/// util function to parse gcode file
async fn try_parse_file(filename: &Path) -> anyhow::Result<Arc<GcodeFile>> {
    let file = File::open(filename).await?;

    let gcode = GcodeFile::async_parse(file).await?;

    return Ok(Arc::new(gcode));
}

/// open and parse a gcode file.
/// sends path request through channel to the file watching tokio runtime
pub async fn open_gcode_file(filename: PathBuf) -> anyhow::Result<Arc<GcodeFile>> {
    let path = filename.canonicalize()?;

    // acquire lock
    let mut recv = RW.1.lock().await;

    // request file
    let _ = RW.0.send(path);

    // recieve result
    recv.recv().await.unwrap()
}

pub async fn watch<F>(path: PathBuf, handler: F)
where
    F: Fn(&notify::Event) -> Pin<Box<dyn Future<Output = bool>>> + Sync + Send + 'static,
{
    let mut handlers = HANDLERS.lock().await;

    handlers.push((path, Box::new(handler)));
}
