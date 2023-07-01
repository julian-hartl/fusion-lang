use std::fmt::Display;

pub struct PerfMeasurement {
    pub name: String,
    pub start: std::time::Instant,
    pub end: std::time::Instant,
}

impl PerfMeasurement {
    pub fn new(name: String) -> PerfMeasurement {
        PerfMeasurement {
            name,
            start: std::time::Instant::now(),
            end: std::time::Instant::now(),
        }
    }

    pub fn start(&mut self) {
        self.start = std::time::Instant::now();
    }

    pub fn end(&mut self) {
        self.end = std::time::Instant::now();
    }

    pub fn duration(&self) -> std::time::Duration {
        self.end - self.start
    }
}

impl Display for PerfMeasurement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let duration = self.duration();
        write!(f, "{}: {}ms", self.name, duration.as_millis())
    }
}