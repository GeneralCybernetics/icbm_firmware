#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::adc::Adc;
use embassy_time::Timer;
use icbm_firmware::drivers::thermistor::Thermistor;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    let adc = Adc::new(p.ADC1);
    let adc_pin = p.PA7;
    let mut thermistor = Thermistor::new(adc_pin, adc);

    loop {
        let temp = thermistor.get_temperature_celsius();
        info!("temp: {}", temp);
        Timer::after_secs(10).await;
    }
}
