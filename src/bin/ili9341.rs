#![no_std]
#![no_main]

use core::cell::RefCell;
use display_interface_spi::SPIInterface;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::time::hz;
use embassy_sync::blocking_mutex::{raw::NoopRawMutex, Mutex};
use embassy_time::Delay;
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    text::{Alignment, Text},
};
use ili9341::{DisplaySize240x320, Ili9341, Orientation};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    // SPI config
    let clk = p.PC10;
    let miso = p.PC11;
    let mosi = p.PC12;
    let lcd_cs = p.PC13;
    let lcd_dc = p.PC14;
    let lcd_reset = p.PC15;
    let mut lcd_spi_config = spi::Config::default();
    lcd_spi_config.frequency = hz(8_000_000); // 26MHz
    let spi = Spi::new_blocking(p.SPI3, clk, mosi, miso, lcd_spi_config.clone());

    // Create shared SPI bus
    let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));

    // Create SPI device for LCD
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
        "CO2: 51402 PPM",
        Point::new(320 / 2, 100),
        style,
        Alignment::Center,
    )
    .draw(&mut lcd)
    .unwrap();

    Text::with_alignment(
        "TEMP: 37.1 C",
        Point::new(320 / 2, 140),
        style,
        Alignment::Center,
    )
    .draw(&mut lcd)
    .unwrap();

    Text::with_alignment(
        "GENERAL CYBERNETICS CORPORATION",
        Point::new(320 / 2, 200),
        style,
        Alignment::Center,
    )
    .draw(&mut lcd)
    .unwrap();

    loop {
        embassy_time::Timer::after_secs(1).await;
    }
}
