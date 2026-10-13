use chrono::{DateTime, Utc};

pub trait Clock: std::fmt::Debug + Send + Sync {
    fn read_current_time(&self) -> DateTime<Utc>;
}

#[derive(Debug)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn read_current_time(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
