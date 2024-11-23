#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Speed};
use embassy_stm32::usart::{Config, DataBits, Parity, StopBits, Uart};
use embassy_stm32::{bind_interrupts, peripherals, usart};
use embassy_time::Timer;
use icbm_firmware::drivers::co2_solenoid::Co2Solenoid;
use icbm_firmware::drivers::explorir_m_e_100::ExplorIrME100;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USART3 => usart::InterruptHandler<peripherals::USART3>;
});

const CO2_THRESHOLD: i32 = 50000; // ppm
const MEASUREMENT_INTERVAL: u64 = 30; // secs
const BURST_DURATION: u64 = 1; // milliseconds

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Starting CO2 homeostatic control system");

    let mut config = Config::default();
    config.baudrate = 9600;
    config.parity = Parity::ParityNone;
    config.stop_bits = StopBits::STOP1;
    config.data_bits = DataBits::DataBits8;

    let usart = Uart::new(p.USART3, p.PD9, p.PD8, Irqs, p.DMA1_CH3, p.DMA1_CH1, config).unwrap();

    Timer::after_secs(2).await;
    let mut co2_solenoid = Co2Solenoid::new(p.PA0, Level::Low, Speed::High);
    let mut co2_sensor = ExplorIrME100::new(usart);

    Timer::after_secs(2).await;

    match co2_sensor.read_serial_no().await {
        Ok(msg) => {
            info!("Sensor initialized successfully. Serial: {}", msg);
        }
        Err(error_msg) => {
            info!("Failed to initialize sensor: {}", error_msg);
        }
    }

    Timer::after_secs(2).await;

    loop {
        match co2_sensor.get_filtered_co2().await {
            Ok(co2_level) => {
                info!("CO2 level: {} ppm", co2_level);

                if co2_level < CO2_THRESHOLD {
                    info!("CO2 level below threshold. Executing burst.");
                    co2_solenoid.execute_burst(BURST_DURATION).await;
                    info!("Burst completed");
                } else {
                    info!("CO2 level above threshold. No action needed.");
                }
            }
            Err(error_msg) => {
                error!("Failed to get CO2 reading: {}", error_msg);
            }
        }

        Timer::after_secs(MEASUREMENT_INTERVAL).await;
    }
}
