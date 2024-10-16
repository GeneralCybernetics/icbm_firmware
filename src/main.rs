#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::Output;
use embassy_stm32::i2c::{Config, I2c};
use embassy_stm32::{bind_interrupts, i2c, peripherals};

use embassy_stm32::time::Hertz;
use embassy_time::{block_for, Duration, Timer};
use {defmt_rtt as _, panic_probe as _};
mod drivers;
use drivers::scd41::SCD41;

bind_interrupts!(struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    let mut config = Config::default();
    config.scl_pullup = true;
    config.sda_pullup = true;

    let mut i2c = I2c::new(
        p.I2C1,
        p.PB6,
        p.PB7,
        Irqs,
        p.DMA1_CH6,
        p.DMA1_CH0,
        Hertz(100_000),
        config,
    );

    let mut scd41sensor = SCD41::new(i2c);

    match scd41sensor.stop_periodic_measurement().await {
        Ok(()) => {
            info!("stop_periodic_measurement successful");
        }
        Err(e) => {
            error!("error: {}", e)
        }
    }

    match scd41sensor.get_serial_number().await {
        Ok(abc) => {
            info!("we got: {:#x}", abc)
        }
        Err(e) => {
            error!("error: {}", e)
        }
    }
}
