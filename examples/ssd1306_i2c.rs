//! # SSD1306 OLED Display Example for Adafruit Feather ESP32-C6
//!
//! Draw a 1 bit per pixel black and white image on a 128x64 SSD1306 display over I2C.
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
//! Run with `cargo run --example ssd1306_i2c`.

#![no_std]
#![no_main]

use defmt::info;
use embedded_graphics::{
    image::{Image, ImageRaw},
    pixelcolor::BinaryColor,
    prelude::*,
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

    // Load the raw image data (64x64 pixels, 1 bit per pixel)
    let raw: ImageRaw<BinaryColor> = ImageRaw::new(include_bytes!("./rust.raw"), 64);

    // Create an image positioned at x=32, y=0 to center it horizontally
    let image = Image::new(&raw, Point::new(32, 0));

    // Draw the image to the display buffer
    image.draw(&mut display).unwrap();

    // Flush the buffer to the display
    display.flush().unwrap();
    info!("Image displayed!");

    // Halt - image is now displayed
    loop {
        unsafe { core::arch::asm!("wfi") };
    }
}
