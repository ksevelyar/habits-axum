use jiff::Timestamp;

pub trait Clock: std::fmt::Debug + Send + Sync {
    fn read_current_time(&self) -> Timestamp;
}

#[derive(Debug)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn read_current_time(&self) -> Timestamp {
        Timestamp::now()
    }
}
