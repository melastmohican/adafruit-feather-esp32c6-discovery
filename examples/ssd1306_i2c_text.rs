//! # SSD1306 OLED Display with Text Example for Adafruit Feather ESP32-C6
//!
//! This example demonstrates drawing text and shapes on a 128x64 SSD1306 display over I2C.
//!
//! Wiring connections (Stemma QT):
//!
//! ```
//!      Display -> Adafruit Feather ESP32-C6
//! (black)  GND -> GND (Stemma GND)
//! (red)    VCC -> 3.3V (Stemma V+)
//! (yellow) SCL -> GPIO 18 (Stemma SCL)
//! (blue)   SDA -> GPIO 19 (Stemma SDA)
//! ```
//!
//! Run with `cargo run --example ssd1306_i2c_text`.

#![no_std]
#![no_main]

use defmt::info;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};
use esp_hal::{
    delay::Delay,
    i2c::master::{Config as I2cConfig, I2c},
    main,
    time::Rate,
};
use panic_rtt_target as _;
use ssd1306::{I2CDisplayInterface, Ssd1306, prelude::*};

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    info!("Initializing SSD1306 OLED display...");

    // Power on the I2C / NeoPixel port (GPIO 20)
    info!("Enabling I2C / NeoPixel Power (GPIO 20)");
    let _pwr = esp_hal::gpio::Output::new(
        peripherals.GPIO20,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );

    // Give the hardware (especially OLED screens) a moment to boot up after receiving power
    delay.delay_millis(500);

    // Configure I2C pins
    let sda = peripherals.GPIO19;
    let scl = peripherals.GPIO18;

    // Create I2C peripheral
    let i2c_config = I2cConfig::default().with_frequency(Rate::from_khz(100));
    let i2c = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(sda)
        .with_scl(scl);

    // Create display interface
    let interface = I2CDisplayInterface::new(i2c);

    // Create display driver
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    // Initialize the display
    display.init().unwrap();
    info!("Display initialized!");

    let delay = Delay::new();

    // Create text style
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    // Clear the display buffer
    display.clear(BinaryColor::Off).unwrap();

    // Draw title text
    Text::with_baseline("Rust ESP", Point::new(30, 0), text_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();

    Text::with_baseline(
        "Rust ESP Board Demo",
        Point::new(25, 12),
        text_style,
        Baseline::Top,
    )
    .draw(&mut display)
    .unwrap();

    // Draw a separator line
    Line::new(Point::new(0, 24), Point::new(127, 24))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(&mut display)
        .unwrap();

    // Draw a rectangle
    Rectangle::new(Point::new(10, 30), Size::new(30, 20))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(&mut display)
        .unwrap();

    // Draw a filled circle
    Circle::new(Point::new(60, 35), 10)
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(&mut display)
        .unwrap();

    // Draw some text at bottom
    Text::with_baseline("Hello Rust!", Point::new(10, 54), text_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();

    // Flush to display
    display.flush().unwrap();

    info!("Display content rendered!");

    // Keep display showing
    loop {
        delay.delay_millis(1000);
    }
}
