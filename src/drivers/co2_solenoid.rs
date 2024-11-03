use embassy_stm32::gpio::{Level, Output, Pin, Speed};
use embassy_stm32::Peripheral;
use embassy_time::Timer;

pub enum Co2State {
    Idle,
    Burst,
    Continuous,
}

pub struct Co2Solenoid<'d> {
    output: Output<'d>,
    state: Co2State,
}

impl<'d> Co2Solenoid<'d> {
    pub fn new(pin: impl Peripheral<P = impl Pin> + 'd, level: Level, speed: Speed) -> Self {
        let output = Output::new(pin, level, speed);
        let state = match level {
            Level::Low => Co2State::Idle,
            Level::High => Co2State::Continuous,
        };

        Co2Solenoid { output, state }
    }

    // call repeatedly in an async loop with ExplorIR M E 100 readings
    // recommended measure time 30 secs - to allow Co2 to diffuse
    pub async fn execute_burst(&mut self, interval: u64) {
        match self.state {
            Co2State::Idle => {}
            _ => {
                self.output.set_low();
                self.state = Co2State::Idle;
            }
        }

        self.state = Co2State::Burst;
        self.output.set_high();
        Timer::after_millis(interval).await;
        self.output.set_low();
        self.state = Co2State::Idle;
    }

    pub fn start_continuous(&mut self) {
        self.state = Co2State::Continuous;
        self.output.set_high();
    }

    pub fn stop_continuous(&mut self) {
        if matches!(self.state, Co2State::Continuous) {
            self.output.set_low();
            self.state = Co2State::Idle;
        }
    }

    pub fn state(&self) -> &Co2State {
        &self.state
    }
}
