//! # Enviro+ FeatherWing LCD Display Example for Adafruit Feather ESP32-C6
//!
//! This example drives the 0.96" 160x80 ST7735S display on the Enviro+ FeatherWing.
//!
//! ## Pin Mapping (Feather C6)
//! - **SCK**: GPIO 21
//! - **MOSI**: GPIO 22
//! - **LCD_CS (D6)**: GPIO 6
//! - **LCD_DC (D5)**: GPIO 5
//! - **LCD_BL (D9)**: GPIO 9 (Backlight)
//! - **PWR**: GPIO 20 (Required for Feather C6 headers)
//!
//! Note: LCD Reset is tied to the hardware RESET pin on this wing.

#![no_std]
#![no_main]

use defmt::info;
use display_interface_spi::SPIInterface;
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    main,
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi},
    },
    time::Rate,
};
use mipidsi::{
    Builder,
    models::ST7735s,
    options::{ColorInversion, ColorOrder, Orientation, Rotation},
};
use panic_rtt_target as _;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut delay = Delay::new();

    info!("Initializing Enviro+ Display...");

    // 1. Enable Power (GPIO 20)
    let _pwr = Output::new(peripherals.GPIO20, Level::High, OutputConfig::default());

    // 2. Setup Status LED (GPIO 15)
    let mut status_led = Output::new(peripherals.GPIO15, Level::Low, OutputConfig::default());
    status_led.set_high(); // ON while initializing

    // 3. Setup Backlight (GPIO 9)
    let mut _backlight = Output::new(peripherals.GPIO9, Level::High, OutputConfig::default());

    // 4. Configure SPI
    let sck = peripherals.GPIO21;
    let mosi = peripherals.GPIO22;
    let miso = peripherals.GPIO23; // Dummy
    let cs = Output::new(peripherals.GPIO6, Level::High, OutputConfig::default());
    let dc = Output::new(peripherals.GPIO5, Level::Low, OutputConfig::default());

    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(10))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sck)
    .with_mosi(mosi)
    .with_miso(miso);

    let spi_device = ExclusiveDevice::new(spi, cs, delay).unwrap();
    let di = SPIInterface::new(spi_device, dc);

    // 5. Initialize Display
    // The Enviro+ panel is physically 160x80.
    // The ST7735S driver often needs to be treated as 80x160 portrait and rotated.
    let mut display = Builder::new(ST7735s, di)
        .display_size(80, 160)
        .display_offset(26, 1)
        .invert_colors(ColorInversion::Inverted)
        .color_order(ColorOrder::Bgr)
        .orientation(Orientation::default().rotate(Rotation::Deg270))
        .init(&mut delay)
        .unwrap();

    status_led.set_low();
    info!("Display initialized successfully!");

    // 6. Draw Content
    display.clear(Rgb565::BLACK).unwrap();

    let style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
    
    // Background box
    Rectangle::new(Point::new(0, 0), Size::new(160, 80))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_DARK_SLATE_BLUE))
        .draw(&mut display)
        .unwrap();

    Text::with_alignment("ENVIRO+ RUST", Point::new(80, 30), style, Alignment::Center)
        .draw(&mut display)
        .unwrap();

    Text::with_alignment("DISPLAY OK", Point::new(80, 50), style, Alignment::Center)
        .draw(&mut display)
        .unwrap();

    // 7. Main Loop
    info!("Starting loop...");
    loop {
        status_led.toggle();
        delay.delay_millis(1000);
    }
}
