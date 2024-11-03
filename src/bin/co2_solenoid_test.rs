#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Speed};
use embassy_time::Timer;
use icbm_firmware::drivers::co2_solenoid::{Co2Solenoid, Co2State};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let mut co2_solenoid = Co2Solenoid::new(p.PA0, Level::Low, Speed::High);

    Timer::after_secs(10).await;

    //basic sanity tests
    match co2_solenoid.state() {
        Co2State::Idle => info!("Init: Idle"),
        _ => error!("Init: Not Idle"),
    }

    info!("Burst seq start");
    for i in 1..=10 {
        info!("Burst {}/10", i);
        co2_solenoid.execute_burst(1000).await;
    }
    info!("Burst seq end");

    match co2_solenoid.state() {
        Co2State::Idle => info!("Post-burst: Idle"),
        _ => error!("Post-burst: Not Idle"),
    }

    info!("Continuous start");
    co2_solenoid.start_continuous();

    match co2_solenoid.state() {
        Co2State::Continuous => info!("Continuous: Active"),
        _ => error!("Continuous: Not Active"),
    }

    Timer::after_secs(5).await;

    co2_solenoid.stop_continuous();
    Timer::after_secs(5).await;

    match co2_solenoid.state() {
        Co2State::Idle => info!("Post-continuous: Idle"),
        _ => error!("Post-continuous: Not Idle"),
    }

    info!("Continuous end");

    match co2_solenoid.state() {
        Co2State::Idle => info!("Final: Idle"),
        _ => error!("Final: Not Idle"),
    }

    info!("Test complete");
}
