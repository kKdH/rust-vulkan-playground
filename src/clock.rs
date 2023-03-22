use std::time::{Duration, Instant};

pub type Time = f32;
pub type DeltaTime = f32;

#[derive(Debug, Copy, Clone)]
pub struct Tick {
    pub time: Time,
    pub delta: Time,
}

impl Tick {

    pub fn new(time: Time, delta: Time) -> Self {
        Tick {
            time,
            delta,
        }
    }
}

#[derive(Debug)]
pub struct Clock {
    time: Time,
    delta_time: DeltaTime,
    current_time: Instant,
    frame_time: Duration,
    accumulator: Time,
    last_tick: Tick
}

impl Clock {

    pub fn new(delta_time: DeltaTime) -> Self {
        Clock {
            time: 0.0,
            delta_time,
            current_time: Instant::now(),
            frame_time: Duration::default(),
            accumulator: 0.0,
            last_tick: Tick::new(0.0, 0.0),
        }
    }

    pub fn produce(&mut self) {
        let now = Instant::now();
        self.frame_time = (now - self.current_time);
        self.accumulator = self.frame_time.as_secs_f32();
        self.current_time = now;
    }

    pub fn consume(&mut self) -> Option<Tick> {
        if self.accumulator >= self.delta_time {
            let delta = self.time - self.last_tick.time;
            let tick = Tick::new(
                self.time,
                delta,
            );
            self.accumulator -= self.delta_time;
            self.time += self.delta_time;
            self.last_tick = tick;
            Some(tick)
        }
        else {
            None
        }
    }

    pub fn delta_time(&self) -> DeltaTime {
        self.delta_time
    }

    pub fn current_time(&self) -> &Instant {
        &self.current_time
    }

    pub fn frame_time(&self) -> &Duration {
        &self.frame_time
    }
}
