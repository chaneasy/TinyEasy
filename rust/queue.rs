use crate::{
    credentials::Account,
    error::Error,
    files::Destination,
    model::{Output, SaveOptions},
    tinify::{LIMIT, Tinify, Usage},
};
use futures_util::{StreamExt, stream};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

#[derive(Debug)]
struct KeySlot {
    account: Account,
    usage: Usage,
    in_flight: u32,
}
#[derive(Debug, Default)]
struct PoolState {
    slots: Vec<KeySlot>,
    next: usize,
}
#[derive(Debug, Clone, Default)]
pub struct KeyPool(Arc<Mutex<PoolState>>);
impl KeyPool {
    pub fn new(accounts: Vec<Account>, usage: Vec<Usage>) -> Self {
        Self(Arc::new(Mutex::new(PoolState {
            slots: accounts
                .into_iter()
                .zip(usage)
                .map(|(account, usage)| KeySlot {
                    account,
                    usage,
                    in_flight: 0,
                })
                .collect(),
            next: 0,
        })))
    }
    fn reserve(&self) -> Result<(usize, Account), Error> {
        let mut state = self.0.lock().map_err(|_| Error::Service)?;
        for offset in 0..state.slots.len() {
            let index = (state.next + offset) % state.slots.len();
            let slot = &mut state.slots[index];
            if slot.usage.remaining() > slot.in_flight {
                slot.in_flight += 1;
                let account = slot.account.clone();
                state.next = (index + 1) % state.slots.len();
                return Ok((index, account));
            }
        }
        Err(Error::Quota)
    }
    fn observe(&self, index: usize, used: u32) {
        if let Ok(mut s) = self.0.lock() {
            let slot = &mut s.slots[index];
            slot.usage.used = slot.usage.used.max(used);
        }
    }
    fn finish(&self, index: usize, result: &Result<Output, Error>) {
        if let Ok(mut s) = self.0.lock() {
            let slot = &mut s.slots[index];
            slot.in_flight = slot.in_flight.saturating_sub(1);
            match result {
                Err(Error::InvalidKey) => {
                    slot.usage.valid = false;
                    slot.usage.error = Some(Error::InvalidKey);
                }
                Err(Error::Quota) => {
                    slot.usage.used = LIMIT;
                    slot.usage.error = Some(Error::Quota);
                }
                _ => {}
            }
        }
    }
    pub fn usage(&self) -> Vec<Usage> {
        self.0
            .lock()
            .map(|s| s.slots.iter().map(|s| s.usage.clone()).collect())
            .unwrap_or_default()
    }
}
#[derive(Debug, Clone)]
pub enum Event {
    Started(u64),
    Progress(u64, f32),
    Finished(u64, Result<Output, Error>),
    Complete,
}
pub async fn validate(client: Tinify, accounts: Vec<Account>) -> Vec<Usage> {
    let mut result = stream::iter(accounts.into_iter().enumerate().map(|(i, a)| {
        let client = client.clone();
        async move { (i, client.validate(&a).await) }
    }))
    .buffer_unordered(3)
    .collect::<Vec<_>>()
    .await;
    result.sort_by_key(|(i, _)| *i);
    result.into_iter().map(|(_, usage)| usage).collect()
}
pub async fn run(
    client: Tinify,
    pool: KeyPool,
    items: Vec<(u64, PathBuf)>,
    options: SaveOptions,
    tx: mpsc::Sender<Event>,
) {
    stream::iter(items.into_iter().map(|(id, input)| {
        let (client, pool, options, tx) =
            (client.clone(), pool.clone(), options.clone(), tx.clone());
        async move {
            if tx.send(Event::Started(id)).await.is_err() {
                return;
            }
            let result = match pool.reserve() {
                Ok((index, account)) => {
                    let result = process(
                        client,
                        pool.clone(),
                        index,
                        account,
                        input,
                        options,
                        id,
                        tx.clone(),
                    )
                    .await;
                    pool.finish(index, &result);
                    result
                }
                Err(e) => Err(e),
            };
            // Terminal events use awaited send and cannot be lost to progress throttling.
            let _ = tx.send(Event::Finished(id, result)).await;
        }
    }))
    .buffer_unordered(3)
    .collect::<Vec<_>>()
    .await;
    let _ = tx.send(Event::Complete).await;
}
#[expect(
    clippy::too_many_arguments,
    reason = "worker context keeps credentials and filesystem operations out of UI state"
)]
async fn process(
    client: Tinify,
    pool: KeyPool,
    index: usize,
    account: Account,
    input: PathBuf,
    options: SaveOptions,
    id: u64,
    tx: mpsc::Sender<Event>,
) -> Result<Output, Error> {
    let prepare_input = input.clone();
    let destination =
        tokio::task::spawn_blocking(move || Destination::prepare(&prepare_input, &options))
            .await
            .map_err(|_| Error::File)??;
    let before = tokio::fs::metadata(&input).await?.len();
    let last = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1)));
    client
        .compress(
            &account,
            &input,
            destination.path(),
            move |progress| {
                if let Ok(mut last) = last.lock()
                    && last.elapsed() >= Duration::from_millis(50)
                {
                    let _ = tx.try_send(Event::Progress(id, progress));
                    *last = Instant::now();
                }
            },
            move |used| pool.observe(index, used),
        )
        .await?;
    let after = tokio::fs::metadata(destination.path()).await?.len();
    let path = tokio::task::spawn_blocking(move || destination.commit())
        .await
        .map_err(|_| Error::File)??;
    Ok(Output {
        path,
        before,
        after,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tinify::tests::random_account;
    #[test]
    fn rotation_and_reservations_prevent_quota_oversubscription() {
        let pool = KeyPool::new(
            vec![random_account(), random_account()],
            vec![
                Usage {
                    valid: true,
                    used: 499,
                    error: None
                };
                2
            ],
        );
        assert_eq!(pool.reserve().unwrap().0, 0);
        assert_eq!(pool.reserve().unwrap().0, 1);
        assert_eq!(pool.reserve().unwrap_err(), Error::Quota);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{
        model::SaveMode,
        tinify::{mock, tests::random_account},
    };
    use std::sync::atomic::Ordering;
    fn options() -> SaveOptions {
        SaveOptions {
            mode: SaveMode::Overwrite,
            directory: None,
            common_root: None,
            preserve: false,
        }
    }
    fn pool() -> KeyPool {
        KeyPool::new(
            vec![random_account(), random_account()],
            vec![
                Usage {
                    valid: true,
                    used: 0,
                    error: None
                };
                2
            ],
        )
    }
    async fn execute(client: Tinify, paths: Vec<PathBuf>) -> Vec<Event> {
        let (tx, mut rx) = mpsc::channel(64);
        let worker = tokio::spawn(run(
            client,
            pool(),
            paths
                .into_iter()
                .enumerate()
                .map(|(id, p)| (id as u64, p))
                .collect(),
            options(),
            tx,
        ));
        let mut events = vec![];
        while let Some(e) = rx.recv().await {
            events.push(e);
        }
        worker.await.unwrap();
        events
    }
    #[tokio::test]
    async fn streaming_queue_limits_concurrency_and_delivers_all_terminal_events() {
        let server = mock::start(201, false, false).await;
        let dir = tempfile::tempdir().unwrap();
        let mut paths = vec![];
        for i in 0..7 {
            let path = dir.path().join(format!("{i}.png"));
            std::fs::write(&path, vec![42; 100_000]).unwrap();
            paths.push(path);
        }
        let events = execute(server.client.clone(), paths.clone()).await;
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, Event::Finished(_, Ok(_))))
                .count(),
            7
        );
        assert!(matches!(events.last(), Some(Event::Complete)));
        assert!(server.peak.load(Ordering::SeqCst) <= 3);
        assert_eq!(server.peak.load(Ordering::SeqCst), 3);
        assert_eq!(server.uploads.load(Ordering::SeqCst), 7);
        assert_eq!(server.active.load(Ordering::SeqCst), 0);
        assert!(
            events
                .iter()
                .any(|e| matches!(e,Event::Progress(_,p)if *p>0.))
        );
        for path in paths {
            assert_eq!(std::fs::read(path).unwrap(), b"compressed image");
        }
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 7);
    }
    #[tokio::test]
    async fn interrupted_download_preserves_original_and_removes_partial_file() {
        let server = mock::start(201, true, false).await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("original.png");
        std::fs::write(&path, b"original").unwrap();
        let events = execute(server.client.clone(), vec![path.clone()]).await;
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Finished(_, Err(Error::Network))))
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    #[tokio::test]
    async fn service_failure_is_reported_without_aborting_batch() {
        let server = mock::start(503, false, false).await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("original.png");
        std::fs::write(&path, b"original").unwrap();
        let events = execute(server.client.clone(), vec![path.clone()]).await;
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Finished(_, Err(Error::Service))))
        );
        assert!(matches!(events.last(), Some(Event::Complete)));
        assert_eq!(std::fs::read(path).unwrap(), b"original");
    }
    #[tokio::test]
    async fn foreign_download_location_does_not_receive_credentials() {
        let server = mock::start(201, false, true).await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("original.png");
        std::fs::write(&path, b"original").unwrap();
        let events = execute(server.client.clone(), vec![path.clone()]).await;
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Finished(_, Err(Error::Service))))
        );
        assert_eq!(std::fs::read(path).unwrap(), b"original");
    }
}
