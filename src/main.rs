#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::i2c::{Config, I2c};
use embassy_stm32::time::Hertz;
use embassy_stm32::{bind_interrupts, i2c, peripherals};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

const ADDRESS: u8 = 0x08;
const PROD_IDENTIFY_1: [u8; 2] = [0x36, 0x08];

bind_interrupts!(struct Irqs {
    I2C2_EV => i2c::EventInterruptHandler<peripherals::I2C2>;
    I2C2_ER => i2c::ErrorInterruptHandler<peripherals::I2C2>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Hello world!");

    let p = embassy_stm32::init(Default::default());

    let mut config = Config::default();
    config.scl_pullup = true;
    config.sda_pullup = true;

    let mut i2c = I2c::new(
        p.I2C2,
        p.PB10,
        p.PB11,
        Irqs,
        p.DMA1_CH7,
        p.DMA1_CH2,
        Hertz(100_000),
        config,
    );

    let mut data = [0u8; 18];

    Timer::after_millis(100).await;

    match i2c.write(ADDRESS, &PROD_IDENTIFY_1).await {
        Ok(()) => info!("First command sent successfully"),
        Err(e) => error!("Error sending first command: {:?}", e),
    }

    Timer::after_millis(100).await;

    match i2c.read(ADDRESS, &mut data).await {
        Ok(()) => info!("Data read successfully: {:?}", data),
        Err(e) => error!("Error reading data: {:?}", e),
    }
}
