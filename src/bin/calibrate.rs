#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Speed};
use embassy_stm32::i2c::Config as I2cConfig;
use embassy_stm32::i2c::I2c;
use embassy_stm32::time::Hertz;
use embassy_stm32::usart::{Config as UartConfig, DataBits, Parity, StopBits, Uart};
use embassy_stm32::{bind_interrupts, i2c, peripherals, usart};
use embassy_time::Timer;
use icbm_firmware::drivers::bsz070::Heater;
use icbm_firmware::drivers::explorir_m_e_100::ExplorIrME100;
use icbm_firmware::drivers::scd41::{SensorSettings, SCD41};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct UartIrqs {
    USART3 => usart::InterruptHandler<peripherals::USART3>;
});

bind_interrupts!(struct I2cIrqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting sensor initialization sequence");
    let p = embassy_stm32::init(Default::default());

    let mut heater = Heater::new(p.PA1, Level::Low, Speed::VeryHigh);
    heater.stop();

    let mut uart_config = UartConfig::default();
    uart_config.baudrate = 9600;
    uart_config.parity = Parity::ParityNone;
    uart_config.stop_bits = StopBits::STOP1;
    uart_config.data_bits = DataBits::DataBits8;

    let usart = Uart::new(
        p.USART3,
        p.PD9,
        p.PD8,
        UartIrqs,
        p.DMA1_CH3,
        p.DMA1_CH1,
        uart_config,
    )
    .unwrap();

    let mut i2c_config = I2cConfig::default();
    i2c_config.scl_pullup = true;
    i2c_config.sda_pullup = true;

    let i2c = I2c::new(
        p.I2C1,
        p.PB6,
        p.PB7,
        I2cIrqs,
        p.DMA1_CH6,
        p.DMA1_CH0,
        Hertz(100_000),
        i2c_config,
    );

    Timer::after_secs(2).await;

    info!("Starting SCD41 initialization");
    let mut scd41sensor = SCD41::new(i2c);
    match scd41sensor.init(None).await {
        Ok(()) => info!("SCD41 base initialization successful"),
        Err(e) => error!("SCD41 initialization error: {}", e),
    }

    Timer::after_secs(60).await;

    info!("Taking initial SCD41 measurements");

    let (temp, _) = match scd41sensor.read_measurement().await {
        Ok((_, temp, humidity)) => {
            info!(
                "Initial readings - Temperature: {}°C, Humidity: {}%",
                temp, humidity
            );
            (temp, humidity)
        }
        Err(e) => {
            error!(
                "SCD41 measurement error: {}, defaulting to fallback values",
                e
            );
            (37.1, 50.0)
        }
    };

    info!("Applying SCD41 calibration");
    match scd41sensor
        .init(Some(SensorSettings::Custom {
            current_temp: temp,   // Current temperature in Celsius
            reference_temp: 37.1, // Target/reference temperature for the incubator
            pressure: 102_133,    // Atmospheric pressure in Pascals
            altitude: 142,        // Altitude in meters above sea level
        }))
        .await
    {
        Ok(()) => info!("SCD41 environmental calibration successful"),
        Err(e) => error!("SCD41 calibration error: {}", e),
    }

    match scd41sensor.stop_periodic_measurement().await {
        Ok(()) => info!("Stopped periodic measurement"),
        Err(e) => error!("Failed to stop periodic measurement: {}", e),
    }

    match scd41sensor.persist().await {
        Ok(()) => info!("Settings persisted successfully"),
        Err(e) => error!("Failed to persist settings: {}", e),
    }

    match scd41sensor.start_periodic_measurement().await {
        Ok(()) => info!("Started periodic measurement"),
        Err(e) => error!("Failed to start periodic measurement: {}", e),
    }

    Timer::after_secs(60).await;
    info!("Taking post-calibration SCD41 measurements");
    match scd41sensor.read_measurement().await {
        Ok((_, temp, humidity)) => {
            info!(
                "Calibrated readings - Temperature: {}°C, Humidity: {}%",
                temp, humidity
            )
        }
        Err(e) => error!("SCD41 measurement error: {}", e),
    }

    Timer::after_secs(10).await;

    info!("Starting ExplorIR-M-E-100 CO2 sensor initialization");
    let mut co2_sensor = ExplorIrME100::new(usart);
    match co2_sensor.init().await {
        Ok(_) => info!("CO2 sensor initialization successful"),
        Err(e) => error!("CO2 sensor initialization failed: {}", e),
    }

    Timer::after_secs(10).await;

    info!("Setting CO2 sensor pressure compensation");

    //In millibars (mBar), range 300-1100
    match co2_sensor.set_pressure_and_concentration(1016.9325).await {
        Ok(()) => info!("CO2 sensor pressure compensation set"),
        Err(e) => error!("CO2 sensor pressure compensation error: {}", e),
    }

    Timer::after_secs(60).await;

    let ppm = co2_sensor.get_filtered_co2().await.unwrap();
    info!("Initial CO2 reading: {} ppm", ppm);

    Timer::after_secs(10).await;

    info!("Starting CO2 sensor fine tuning");
    // Target/reference Co2 for the incubator
    match co2_sensor.fine_tune(51_000, ppm as u32).await {
        Ok(()) => info!("CO2 sensor fine tuning successful"),
        Err(error_msg) => error!("CO2 sensor fine tuning failed: {}", error_msg),
    }

    Timer::after_secs(30).await;
    let ppm = co2_sensor.get_filtered_co2().await.unwrap();
    info!("Final calibrated CO2 reading: {} ppm", ppm);

    info!("Calibration Successful");
}
