#![no_std]
#![no_main]

use core::cell::RefCell;
use defmt::*;
use display_interface_spi::SPIInterface;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::{Level, Output, Speed},
    i2c::{self, Config as I2cConfig, I2c},
    mode::Blocking,
    spi::{self, Spi},
    time::{hz, Hertz},
    usart::{Config as UartConfig, DataBits, Parity, StopBits, Uart},
    {bind_interrupts, peripherals, usart},
};
use embassy_sync::blocking_mutex::{raw::NoopRawMutex, Mutex};
use embassy_time::{Delay, Timer};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::Rectangle,
    text::{Alignment, Text},
};
use heapless::String;
use icbm_firmware::drivers::{
    bsz070::Heater, co2_solenoid::Co2Solenoid, explorir_m_e_100::ExplorIrME100, scd41::SCD41,
};
use ili9341::{DisplaySize240x320, Ili9341, Orientation};
use itoa;
use libm::fabsf;
use {defmt_rtt as _, panic_probe as _};

// Constants for control system
const TARGET_CO2_PPM: f32 = 50_000.0; // 5%
const TARGET_TEMP_C: f32 = 37.0; // Body temperature
const CO2_TOLERANCE_PPM: f32 = 2000.0; // ±0.2%
const TEMP_TOLERANCE_C: f32 = 1.0; // ±1°C

bind_interrupts!(struct UartIrqs {
    USART3 => usart::InterruptHandler<peripherals::USART3>;
});

bind_interrupts!(struct I2cIrqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting Ion Concentration Bio-Modulator");
    let p = embassy_stm32::init(Default::default());

    let mut co2_valve = Co2Solenoid::new(p.PA0, Level::Low, Speed::VeryHigh);
    let mut heater = Heater::new(p.PA1, Level::Low, Speed::VeryHigh);
    heater.stop();
    co2_valve.stop_continuous();

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
        Hertz(10_000),
        i2c_config,
    );

    let clk = p.PC10;
    let miso = p.PC11;
    let mosi = p.PC12;
    let lcd_cs = p.PC13;
    let lcd_dc = p.PC14;
    let lcd_reset = p.PC15;
    let mut lcd_spi_config = spi::Config::default();
    lcd_spi_config.frequency = hz(8_000_000); // 8MHz
    let spi = Spi::new_blocking(p.SPI3, clk, mosi, miso, lcd_spi_config.clone());

    let spi_bus: Mutex<NoopRawMutex, RefCell<Spi<'static, Blocking>>> =
        Mutex::new(RefCell::new(spi));

    let lcd_spi = SpiDeviceWithConfig::new(
        &spi_bus,
        Output::new(lcd_cs, Level::High, Speed::Medium),
        lcd_spi_config,
    );

    let lcd_reset = Output::new(lcd_reset, Level::Low, Speed::Medium);
    let lcd_dc = Output::new(lcd_dc, Level::Low, Speed::Medium);

    let spi_iface = SPIInterface::new(lcd_spi, lcd_dc);

    let mut lcd = Ili9341::new(
        spi_iface,
        lcd_reset,
        &mut Delay,
        Orientation::Landscape,
        DisplaySize240x320,
    )
    .unwrap();

    lcd.clear(Rgb565::BLACK).unwrap();
    let style = MonoTextStyle::new(&FONT_10X20, Rgb565::GREEN);

    Text::with_alignment(
        "ION CONCENTRATION BIO-MODULATOR",
        Point::new(320 / 2, 40),
        style,
        Alignment::Center,
    )
    .draw(&mut lcd)
    .unwrap();

    Text::with_alignment(
        "INITIALIZING...",
        Point::new(320 / 2, 120),
        style,
        Alignment::Center,
    )
    .draw(&mut lcd)
    .unwrap();

    Text::with_alignment(
        "GENERAL CYBERNETICS CORPORATION",
        Point::new(320 / 2, 230),
        style,
        Alignment::Center,
    )
    .draw(&mut lcd)
    .unwrap();

    Timer::after_secs(2).await;

    info!("Starting SCD41 initialization");
    let mut scd41sensor = SCD41::new(i2c);
    match scd41sensor.init(None).await {
        Ok(()) => info!("SCD41 base initialization successful"),
        Err(e) => error!("SCD41 initialization error: {}", e),
    }

    Timer::after_secs(2).await;

    info!("Starting ExplorIR-M-E-100 CO2 sensor initialization");
    let mut co2_sensor = ExplorIrME100::new(usart);
    match co2_sensor.init().await {
        Ok(_) => info!("CO2 sensor initialization successful"),
        Err(e) => error!("CO2 sensor initialization failed: {}", e),
    }

    let mut co2_str: String<32> = String::new();
    let mut temp_str: String<32> = String::new();
    let mut co2_buf = itoa::Buffer::new();
    let mut temp_buf = itoa::Buffer::new();
    let mut fract_buf = itoa::Buffer::new();

    loop {
        Timer::after_secs(30).await;

        co2_str.clear();
        temp_str.clear();

        lcd.fill_solid(
            &Rectangle::new(Point::new(0, 80), Size::new(320, 100)),
            Rgb565::BLACK,
        )
        .unwrap();

        Text::with_alignment(
            "ION CONCENTRATION BIO-MODULATOR",
            Point::new(320 / 2, 40),
            style,
            Alignment::Center,
        )
        .draw(&mut lcd)
        .unwrap();

        let current_temp = match scd41sensor.read_measurement().await {
            Ok((_, temp, _)) => {
                info!("Temperature reading: {} C", temp);
                temp
            }
            Err(e) => {
                error!("SCD41 measurement error: {}", e);
                Text::with_alignment(
                    "TEMP SENSOR ERROR",
                    Point::new(320 / 2, 100),
                    style,
                    Alignment::Center,
                )
                .draw(&mut lcd)
                .unwrap();
                continue;
            }
        };

        let current_co2 = match co2_sensor.get_filtered_co2().await {
            Ok(ppm) => {
                info!("CO2 reading: {} ppm", ppm);
                ppm as f32
            }
            Err(e) => {
                error!("CO2 sensor error: {}", e);
                Text::with_alignment(
                    "CO2 SENSOR ERROR",
                    Point::new(320 / 2, 120),
                    style,
                    Alignment::Center,
                )
                .draw(&mut lcd)
                .unwrap();
                continue;
            }
        };

        Timer::after_secs(1).await;
        let temp_error = TARGET_TEMP_C - current_temp;
        if temp_error > TEMP_TOLERANCE_C {
            info!("Activating heater: temp diff {}", temp_error);
            heater.heat().await; //3000ms heat cycle
        } else {
            heater.stop();
        }
        Timer::after_secs(2).await;

        let co2_error = TARGET_CO2_PPM - current_co2;
        if co2_error > CO2_TOLERANCE_PPM {
            info!("Activating CO2: diff {}", co2_error);
            co2_valve.execute_burst(1000).await;
        }

        let co2_num = co2_buf.format(current_co2 as i32);
        co2_str.push_str("CO2: ").unwrap();
        co2_str.push_str(co2_num).unwrap();
        co2_str.push_str(" PPM").unwrap();

        let temp_whole = temp_buf.format(current_temp as i32);
        let temp_fract = ((libm::fmodf(current_temp, 1.0) * 10.0) as i32).abs();
        let temp_fract_str = fract_buf.format(temp_fract);
        temp_str.push_str("TEMP: ").unwrap();
        temp_str.push_str(temp_whole).unwrap();
        temp_str.push_str(".").unwrap();
        temp_str.push_str(temp_fract_str).unwrap();
        temp_str.push_str(" C").unwrap();

        Text::with_alignment(&co2_str, Point::new(320 / 2, 100), style, Alignment::Center)
            .draw(&mut lcd)
            .unwrap();

        Text::with_alignment(
            &temp_str,
            Point::new(320 / 2, 140),
            style,
            Alignment::Center,
        )
        .draw(&mut lcd)
        .unwrap();

        let temp_stable = fabsf(TARGET_TEMP_C - current_temp) <= TEMP_TOLERANCE_C;
        let co2_stable = fabsf(TARGET_CO2_PPM - current_co2) <= CO2_TOLERANCE_PPM;

        lcd.fill_solid(
            &Rectangle::new(Point::new(0, 160), Size::new(320, 20)),
            Rgb565::BLACK,
        )
        .unwrap();

        Text::with_alignment(
            if temp_stable && co2_stable {
                "STABLE"
            } else {
                "ADJUSTING"
            },
            Point::new(320 / 2, 180),
            style,
            Alignment::Center,
        )
        .draw(&mut lcd)
        .unwrap();

        Text::with_alignment(
            "GENERAL CYBERNETICS CORPORATION",
            Point::new(320 / 2, 230),
            style,
            Alignment::Center,
        )
        .draw(&mut lcd)
        .unwrap();

        info!("Loop iteration complete");
    }
}
