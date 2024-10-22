#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::OutputType;
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::simple_pwm::PwmPin;
use embassy_time::Timer;
use icbm_firmware::drivers::bsz070::{Heater, HeaterState};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let ch1 = PwmPin::new_ch1(p.PE9, OutputType::PushPull);
    let mut heater = Heater::new(p.TIM1, Some(ch1), Hertz(25_000));

    Timer::after_secs(10).await;

    match heater.state() {
        HeaterState::Off => info!("Init: Off"),
        _ => error!("Init: Not Off"),
    }

    info!("Testing duty cycles");

    let duties = [0, 25, 50, 75, 100];
    for duty in duties {
        info!("Setting duty cycle to {}%", duty);
        heater.heat(duty);

        match heater.state() {
            HeaterState::Off if duty == 0 => info!("State: Off at {}%", duty),
            HeaterState::Heating if duty > 0 => info!("State: Heating at {}%", duty),
            _ => error!("Unexpected state at {}%", duty),
        }

        Timer::after_secs(10).await;
    }

    info!("Testing stop()");
    heater.stop();

    match heater.state() {
        HeaterState::Off => info!("Post-stop: Off"),
        _ => error!("Post-stop: Not Off"),
    }

    Timer::after_secs(10).await;

    info!("Testing heating after stop");
    heater.heat(50);

    match heater.state() {
        HeaterState::Heating => info!("Post-stop heating: Active"),
        _ => error!("Post-stop heating: Not Active"),
    }

    Timer::after_secs(10).await;

    heater.stop();

    match heater.state() {
        HeaterState::Off => info!("Final: Off"),
        _ => error!("Final: Not Off"),
    }

    info!("Test complete");
}
