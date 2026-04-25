use chrono::{DateTime, Datelike, Local, Timelike};
use std::fmt::{Display, Formatter};
use std::time::SystemTime;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct VFileTime {
    time: SystemTime,
}

impl Display for VFileTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let local_time: DateTime<Local> = self.time.into();
        write!(
            f,
            "{}-{:02}-{:02} {:02}:{:02}:{:02}",
            local_time.year(),
            local_time.month(),
            local_time.day(),
            local_time.hour(),
            local_time.minute(),
            local_time.second()
        )
    }
}

impl VFileTime {
    pub fn new(time: SystemTime) -> VFileTime {
        Self { time }
    }
}
