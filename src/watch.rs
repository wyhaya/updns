use async_std::fs;
use async_std::io;
use async_std::prelude::*;
use async_std::stream;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub struct Watch {
    path: PathBuf,
    interval: u64,
}

impl Watch {
    pub fn new(path: PathBuf, interval: u64) -> Watch {
        Watch { interval, path }
    }

    async fn modified(&self) -> io::Result<SystemTime> {
        let file = fs::File::open(&self.path).await?;
        let modified = file.metadata().await?.modified()?;
        Ok(modified)
    }

    // todo
    // use Stream
    pub async fn for_each(&mut self, func: fn(path: &PathBuf)) {
        let mut before = match self.modified().await {
            Ok(time) => Some(time),
            Err(_) => None,
        };

        let mut interval = stream::interval(Duration::from_millis(self.interval));
        while let Some(_) = interval.next().await {
            let after = match self.modified().await {
                Ok(time) => Some(time),
                Err(_) => None,
            };

            if before != after {
                before = after;
                func(&self.path);
            }
        }
    }
}
