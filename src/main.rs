#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::i2c::{Config, I2c};
use embassy_stm32::time::Hertz;
use embassy_time::{block_for, Duration};
use {defmt_rtt as _, panic_probe as _};

const ADDRESS: u8 = 0b1111000;
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Hello world!");
    let p = embassy_stm32::init(Default::default());
    let mut config = Config::default();
    config.scl_pullup = true;
    config.sda_pullup = true;

    let mut i2c = I2c::new_blocking(p.I2C2, p.PB10, p.PB11, Hertz(100_000), config);

    let mut data = [0u8; 1];

    block_for(Duration::from_millis(100));
    match i2c.blocking_read(ADDRESS, &mut data) {
        Ok(()) => info!("Data read successfully: {:#x}", data),
        Err(e) => error!("Error reading data: {:?}", e),
    }
}
