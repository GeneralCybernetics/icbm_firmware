use embassy_stm32::gpio::{Level, Output, Pin, Speed};
use embassy_stm32::Peripheral;
use embassy_time::Timer;

#[derive(Debug)]
pub enum HeaterState {
    Off,
    Heating,
}

pub struct Heater<'d> {
    pin: Output<'d>,
    state: HeaterState,
}

impl<'d> Heater<'d> {
    pub fn new(pin: impl Peripheral<P = impl Pin> + 'd, level: Level, speed: Speed) -> Self {
        Self {
            pin: Output::new(pin, level, speed),
            state: HeaterState::Off,
        }
    }

    // call repeatedly in an async loop with SCD41 readings
    // recommended measure time 60 secs - to allow heat to diffuse
    pub async fn heat(&mut self) {
        const INTERVAL_MS: u64 = 1000;
        self.state = HeaterState::Heating;
        self.pin.set_high();
        Timer::after_millis(INTERVAL_MS).await;
        self.pin.set_low();
    }

    pub fn stop(&mut self) {
        self.pin.set_low();
        self.state = HeaterState::Off;
    }

    pub fn state(&self) -> &HeaterState {
        &self.state
    }
}
