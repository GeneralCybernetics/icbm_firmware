#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::usart::{Config, DataBits, Parity, StopBits, Uart};
use embassy_stm32::{bind_interrupts, peripherals, usart};
use embassy_time::Timer;
use icbm_firmware::drivers::explorir_m_e_100::ExplorIrME100;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USART3 => usart::InterruptHandler<peripherals::USART3>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Hello World!");

    let mut config = Config::default();
    config.baudrate = 9600;
    config.parity = Parity::ParityNone;
    config.stop_bits = StopBits::STOP1;
    config.data_bits = DataBits::DataBits8;

    let usart = Uart::new(p.USART3, p.PD9, p.PD8, Irqs, p.DMA1_CH3, p.DMA1_CH1, config).unwrap();

    Timer::after_secs(2).await;

    let mut co2_sensor = ExplorIrME100::new(usart);

    match co2_sensor.read_serial_no().await {
        Ok(msg) => {
            info!("{}", msg);
        }
        Err(error_msg) => {
            info!("Failed: {}", error_msg);
        }
    }

    Timer::after_secs(2).await;

    match co2_sensor.get_pressure_and_concentration().await {
        Ok(val) => info!("the value is now {}", val),
        Err(e) => info!("{}", e),
    }

    Timer::after_secs(2).await;
    loop {
        match co2_sensor.get_filtered_co2().await {
            Ok(co2_level) => {
                info!("CO2 level: {} ppm", co2_level);
            }
            Err(error_msg) => {
                info!("Failed to get CO2 reading: {}", error_msg);
            }
        }
        Timer::after_secs(30).await;
    }
}
