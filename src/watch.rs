use futures::{prelude::*, ready};
use std::{
    path::{Path, PathBuf},
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, SystemTime},
};
use tokio::{
    fs::File,
    io::Result,
    time::{interval, Interval},
};

pub struct Watch {
    path: PathBuf,
    state: Option<Pin<Box<dyn Future<Output = Result<SystemTime>>>>>,
    modified: Result<SystemTime>,
    timer: Interval,
}

impl Watch {
    pub async fn new<P: AsRef<Path>>(path: P, duration: u64) -> Watch {
        let path = path.as_ref().to_path_buf();
        Watch {
            path: path.clone(),
            state: None,
            modified: Self::modified(path).await,
            timer: interval(Duration::from_millis(duration)),
        }
    }

    async fn modified(p: PathBuf) -> Result<SystemTime> {
        let file = File::open(p).await?;
        file.metadata().await?.modified()
    }

    fn eq(a: &Result<SystemTime>, b: &Result<SystemTime>) -> bool {
        if a.is_ok() && b.is_ok() {
            if a.as_ref().ok() == b.as_ref().ok() {
                return true;
            }
        } else if a.is_err() && b.is_err() {
            let left = a.as_ref().err().unwrap();
            let right = b.as_ref().err().unwrap();
            if left.kind() == right.kind() && left.raw_os_error() == right.raw_os_error() {
                return true;
            }
        }
        false
    }
}

impl Stream for Watch {
    type Item = ();
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if let Some(state) = &mut self.state {
                let modified: Result<SystemTime> = ready!(Pin::new(state).poll(cx));
                self.state = None;

                if !Self::eq(&self.modified, &modified) {
                    self.modified = modified;
                    return Poll::Ready(Some(()));
                }
            } else {
                ready!(self.timer.poll_next_unpin(cx));

                self.state = Some(Box::pin(Self::modified(self.path.clone())));
            }
        }
    }
}
