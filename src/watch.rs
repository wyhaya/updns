use async_std::fs;
use async_std::io;
use async_std::prelude::*;
use async_std::stream;
use async_std::task;
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

    pub async fn for_each(&mut self, func: fn(path: &PathBuf)) {
        let mut repeat = stream::repeat(0);
        let mut before = match self.modified().await {
            Ok(time) => Some(time),
            Err(_) => None,
        };

        while let Some(_) = repeat.next().await {
            task::sleep(Duration::from_millis(self.interval)).await;

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
