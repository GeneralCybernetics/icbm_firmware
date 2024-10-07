#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::i2c::{Config, I2c};
use embassy_stm32::time::Hertz;
use embassy_stm32::{bind_interrupts, i2c, peripherals};
use embassy_time::{Duration, Instant, Timer};
use {defmt_rtt as _, panic_probe as _};
mod drivers;
use drivers::slf3s::SLF3S;

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

    let i2c = I2c::new(
        p.I2C2,
        p.PB10,
        p.PB11,
        Irqs,
        p.DMA1_CH7,
        p.DMA1_CH2,
        Hertz(100_000),
        config,
    );

    Timer::after_millis(100).await;

    let mut flow_sensor = SLF3S::new(i2c);

    match flow_sensor.start_measurement().await {
        Ok(()) => info!("start_measurement command sent successfully"),
        Err(e) => error!("Error sending start_measurement command: {:?}", e),
    };

    let start_time = Instant::now();
    let sample_duration = Duration::from_secs(1);
    let sample_interval = Duration::from_millis(20); // 50 Hz = 20 ms interval

    loop {
        let loop_start = Instant::now();

        match flow_sensor.read_sample().await {
            Ok((a, b)) => info!("{}, {}", a, b),
            Err(e) => error!("Error reading sample: {:?}", e),
        }

        if Instant::now() - start_time >= sample_duration {
            break;
        }

        let elapsed = Instant::now() - loop_start;
        if elapsed < sample_interval {
            Timer::after(sample_interval - elapsed).await;
        }
    }

    match flow_sensor.stop_measurement().await {
        Ok(()) => info!("stop_measurement command sent successfully"),
        Err(e) => error!("Error stop_measurement first command: {:?}", e),
    };

    match flow_sensor.reset().await {
        Ok(()) => info!("reset command sent successfully"),
        Err(e) => error!("Error sending reset command: {:?}", e),
    };
}
