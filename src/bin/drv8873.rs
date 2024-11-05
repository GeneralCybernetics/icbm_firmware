#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::OutputType;
use embassy_stm32::timer::simple_pwm::PwmPin;
use embassy_time::Timer;
use icbm_firmware::drivers::drv8873::{PeltierController, ThermalState};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let ch2 = PwmPin::new_ch2(p.PA9, OutputType::PushPull);
    let ch3 = PwmPin::new_ch3(p.PA10, OutputType::PushPull);

    let mut controller = PeltierController::new(p.TIM1, Some(ch2), Some(ch3));

    // Basic sanity tests
    match controller.state() {
        ThermalState::Off => info!("Init: Off"),
        _ => error!("Init: Not in Off state"),
    }

    info!("Starting heating test cycles");
    for i in 1..=5 {
        info!("Heat cycle {}/5", i);
        controller.heat().await;
        match controller.state() {
            ThermalState::Off => info!("Heat cycle completed"),
            _ => error!("Unexpected state after heating"),
        }
        info!("Waiting 2 seconds...");
        Timer::after_secs(2).await;
    }

    info!("Starting cooling test cycles");
    for i in 1..=5 {
        info!("Cool cycle {}/5", i);
        controller.cool().await;
        match controller.state() {
            ThermalState::Off => info!("Cool cycle completed"),
            _ => error!("Unexpected state after cooling"),
        }
        info!("Waiting 2 seconds...");
        Timer::after_secs(2).await;
    }

    controller.stop();
    match controller.state() {
        ThermalState::Off => info!("Final state: Off (Success)"),
        _ => error!("Final state: Not Off (Failed)"),
    }

    info!("All tests completed!");
}
