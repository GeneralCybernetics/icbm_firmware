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

    pub async fn heat(&mut self) {
        const PERIOD_MS: u64 = 10_000; // 0.1Hz
        const ON_TIME: u64 = PERIOD_MS / 10; // 10% duty cycle
        const OFF_TIME: u64 = PERIOD_MS - ON_TIME;

        self.state = HeaterState::Heating;
        self.pin.set_high();
        Timer::after_millis(ON_TIME).await;
        self.pin.set_low();
        Timer::after_millis(OFF_TIME).await;
    }

    pub fn stop(&mut self) {
        self.pin.set_low();
        self.state = HeaterState::Off;
    }

    pub fn state(&self) -> &HeaterState {
        &self.state
    }
}
