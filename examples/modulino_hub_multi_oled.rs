//! # Modulino Hub Multi-OLED Example for Adafruit Feather ESP32-C6
//!
//! Demonstrates how to use multiple SSD1306 OLED displays connected to a Modulino Hub
//! (TCA9548A I2C multiplexer) with the Adafruit Feather ESP32-C6.
//!
//! ## Hardware
//!
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Hub:** Arduino Modulino Hub
//! - **OLEDs:** Two SSD1306 128x32 OLED displays (connected to Port 0 and Port 1 of the Hub)
//!
//! ## Wiring Diagram
//!
//! ```
//!      Adafruit Feather ESP32-C6          Modulino Hub
//!    +----------------------------+      +----------------------+
//!    |                            |      |                      |
//!    |  3.3V (Stemma V+) ---------+------+-> VCC                |
//!    |  GND (Stemma GND) ---------+------+-> GND                |
//!    |  GPIO19 (Stemma SDA) ------+------+-> SDA                |
//!    |  GPIO18 (Stemma SCL) ------+------+-> SCL                |
//!    |                            |      |                      |
//!    +----------------------------+      +----------+-----------+
//!                                                   |
//!                                           +-------+-------+
//!                                           |               |
//!                                        Port 0          Port 1
//!                                           |               |
//!                                           v               v
//!                                    +------------+  +------------+
//!                                    | OLED Disp A|  | OLED Disp B|
//!                                    +------------+  +------------+
//! ```
//!
//! ## Run
//!
//! ```bash
//! cargo run --example modulino_hub_multi_oled
//! ```

#![no_std]
#![no_main]

use core::cell::RefCell;
use defmt::{Debug2Format, error, info};
use embedded_graphics::{
    mono_font::{
        MonoTextStyleBuilder,
        ascii::{FONT_6X10, FONT_9X15},
    },
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use esp_hal::{
    delay::Delay,
    i2c::master::{Config as I2cConfig, I2c},
    main,
    time::Rate,
};
use heapless::String;
use modulino::Hub;
use panic_rtt_target as _;
use ssd1306::{I2CDisplayInterface, Ssd1306, prelude::*};

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    info!("Initializing Modulino Hub and OLED displays...");

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

    // Share the I2C bus using a RefCell
    let i2c_bus = RefCell::new(i2c);

    // Initialize Hub using RefCellDevice
    let mut hub = Hub::new(embedded_hal_bus::i2c::RefCellDevice::new(&i2c_bus));

    // Initialize OLED A on Port 0
    info!("Initializing OLED A (Port 0)...");
    if let Err(e) = hub.select(0) {
        error!("Failed to select Port 0: {:?}", Debug2Format(&e));
    }

    let interface_a = I2CDisplayInterface::new(embedded_hal_bus::i2c::RefCellDevice::new(&i2c_bus));
    let mut display_a = Ssd1306::new(interface_a, DisplaySize128x32, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    if let Err(e) = display_a.init() {
        error!("Failed to initialize Display A: {:?}", Debug2Format(&e));
    }

    let _ = display_a.clear(BinaryColor::Off);
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();
    let _ = Text::with_baseline(
        "Screen A Ready",
        Point::new(0, 0),
        text_style,
        Baseline::Top,
    )
    .draw(&mut display_a);
    let _ = display_a.flush();
    let _ = hub.clear();

    // Initialize OLED B on Port 1
    info!("Initializing OLED B (Port 1)...");
    if let Err(e) = hub.select(1) {
        error!("Failed to select Port 1: {:?}", Debug2Format(&e));
    }

    let interface_b = I2CDisplayInterface::new(embedded_hal_bus::i2c::RefCellDevice::new(&i2c_bus));
    let mut display_b = Ssd1306::new(interface_b, DisplaySize128x32, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    if let Err(e) = display_b.init() {
        error!("Failed to initialize Display B: {:?}", Debug2Format(&e));
    }

    let _ = display_b.clear(BinaryColor::Off);
    let _ = Text::with_baseline(
        "Screen B Ready",
        Point::new(0, 0),
        text_style,
        Baseline::Top,
    )
    .draw(&mut display_b);
    let _ = display_b.flush();
    let _ = hub.clear();

    delay.delay_millis(1000);

    let mut counter = 0;
    let large_text_style = MonoTextStyleBuilder::new()
        .font(&FONT_9X15)
        .text_color(BinaryColor::On)
        .build();

    info!("Starting update loop...");
    loop {
        counter += 1;

        // Update Screen A
        if let Err(e) = hub.select(0) {
            error!("Failed to select Port 0: {:?}", Debug2Format(&e));
        }
        let _ = display_a.clear(BinaryColor::Off);
        let _ = Text::with_baseline("SCREEN A", Point::new(0, 0), text_style, Baseline::Top)
            .draw(&mut display_a);

        let mut val_str_a: String<32> = String::new();
        let _ = core::fmt::write(&mut val_str_a, format_args!("Val: {}", counter));
        let _ = Text::with_baseline(
            &val_str_a,
            Point::new(0, 16),
            large_text_style,
            Baseline::Top,
        )
        .draw(&mut display_a);
        let _ = display_a.flush();
        let _ = hub.clear();

        // Update Screen B
        if let Err(e) = hub.select(1) {
            error!("Failed to select Port 1: {:?}", Debug2Format(&e));
        }
        let _ = display_b.clear(BinaryColor::Off);
        let _ = Text::with_baseline("SCREEN B", Point::new(0, 0), text_style, Baseline::Top)
            .draw(&mut display_b);

        let mut val_str_b: String<32> = String::new();
        let _ = core::fmt::write(&mut val_str_b, format_args!("Val: {}", counter * 2));
        let _ = Text::with_baseline(
            &val_str_b,
            Point::new(0, 16),
            large_text_style,
            Baseline::Top,
        )
        .draw(&mut display_b);
        let _ = display_b.flush();
        let _ = hub.clear();

        delay.delay_millis(1000);
    }
}
