#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Speed};
use embassy_time::Timer;
use icbm_firmware::drivers::bsz070::{Heater, HeaterState};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let mut heater = Heater::new(p.PA1, Level::Low, Speed::VeryHigh);

    // Check initial state
    match heater.state() {
        HeaterState::Off => info!("Init: Off"),
        _ => error!("Init: Not Off"),
    }

    info!("Testing heat cycles");

    // Test heating cycles
    info!("Starting heat cycles");
    for i in 1..=5 {
        info!("Heat cycle {}/5", i);
        heater.heat().await;

        match heater.state() {
            HeaterState::Heating => info!("State: Heating during cycle {}", i),
            _ => error!("Unexpected state during cycle {}", i),
        }
    }

    info!("Testing stop()");
    heater.stop();

    match heater.state() {
        HeaterState::Off => info!("Post-stop: Off"),
        _ => error!("Post-stop: Not Off"),
    }

    Timer::after_secs(10).await;

    info!("Testing heating after stop");
    heater.heat().await;

    match heater.state() {
        HeaterState::Heating => info!("Post-stop heating: Active"),
        _ => error!("Post-stop heating: Not Active"),
    }

    heater.stop();

    match heater.state() {
        HeaterState::Off => info!("Final: Off"),
        _ => error!("Final: Not Off"),
    }

    info!("Test complete");
}
