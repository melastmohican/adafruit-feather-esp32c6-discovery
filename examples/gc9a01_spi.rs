//! # GC9A01 Round LCD Display SPI Example for Adafruit Feather ESP32-C6
//!
//! Draw images on a 240x240 round GC9A01 display over SPI.
//!
//! This example is for the Adafruit Feather ESP32-C6 using SPI2.
//!
//! ## Hardware: UNI128-240240-RGB-7-V1.0 Display Module
//!
//! This is a 240x240 round LCD display with GC9A01 controller.
//! **Note:** Despite having pins labeled SCL/SDA, this is an SPI display (not I2C).
//! The DC and CS pins confirm it's SPI - SCL=clock, SDA=MOSI.
//!
//! ## Wiring for Adafruit Feather ESP32-C6
//!
//! ```
//!      LCD Pin  ->  Feather ESP32-C6 (Hardware SPI)
//! --------------------------------------------------
//!        VCC    ->  3V
//!        GND    ->  GND
//!        SCL    ->  IO21 (SCK)
//!        SDA    ->  IO22 (MOSI)
//!        DC     ->  IO6  (A2)
//!        CS     ->  IO7  (D7)
//!        RST    ->  IO5  (A3)
//! ```
//!
//! ### Pin Details:
//! - **VCC**: Power supply (3.3V)
//! - **GND**: Ground
//! - **SCL**: SPI clock (IO21)
//! - **SDA**: SPI data (IO22)
//! - **DC**: Data/Command select (IO6)
//! - **CS**: Chip select (IO7)
//! - **RST**: Reset (IO5)
//!
//! **Note:** This 7-pin module has no separate backlight control pin - backlight is internally controlled.
//!
//! ### SPI Configuration:
//! - SPI Mode: 0 (CPOL=0, CPHA=0)
//! - Clock Speed: 60 MHz (GC9A01 supports up to 62.5 MHz)
//! - Display Resolution: 240x240 pixels (round)
//! - Color Format: RGB565 (16-bit color)
//!
//! Run with `cargo run --example gc9a01_spi`.

#![no_std]
#![no_main]

use defmt::info;
use display_interface_spi::SPIInterface;
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::Point,
    image::{Image, ImageRaw, ImageRawLE},
    pixelcolor::{Rgb565, RgbColor},
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
    models::GC9A01,
    options::{ColorInversion, ColorOrder},
};
use panic_rtt_target as _;
use tinybmp::Bmp;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    info!("Initializing GC9A01 round LCD display on Adafruit Feather ESP32-C6...");

    // Configure SPI pins for Adafruit Feather ESP32-C6
    let sck = peripherals.GPIO21; // SCK
    let mosi = peripherals.GPIO22; // MO (MOSI)
    let miso = peripherals.GPIO23; // MI (MISO) - Not used but needed for config

    // Control pins
    let cs = Output::new(peripherals.GPIO7, Level::High, OutputConfig::default()); // D7 - Chip Select
    let dc = Output::new(peripherals.GPIO6, Level::Low, OutputConfig::default()); // A2 - Data/Command
    let rst = Output::new(peripherals.GPIO5, Level::High, OutputConfig::default()); // A3 - Reset

    // Create SPI bus with 60 MHz clock speed
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(60))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sck)
    .with_mosi(mosi)
    .with_miso(miso);

    info!("SPI configured at 60 MHz");

    // Create exclusive SPI device with CS pin
    let spi_device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();

    // Create display interface
    let di = SPIInterface::new(spi_device, dc);

    let mut delay = Delay::new();

    info!("Initializing display with mipidsi...");

    // Create and initialize display using mipidsi
    // Note: Different UNI128-240240-RGB-7-V1.0 modules may need different settings
    // Try inverting colors and/or changing color order (RGB vs BGR) if colors are wrong
    let mut display = Builder::new(GC9A01, di)
        .reset_pin(rst)
        .invert_colors(ColorInversion::Inverted)
        .color_order(ColorOrder::Bgr) // Try Rgb if colors still wrong
        .display_size(240, 240)
        .init(&mut delay)
        .unwrap();

    info!("Display initialized!");

    // Clear screen to black
    display.clear(Rgb565::BLACK).unwrap();

    info!("Drawing images...");

    // Draw ferris (raw RGB565 image)
    let image_raw: ImageRawLE<Rgb565> = ImageRaw::new(include_bytes!("ferris.raw"), 86);
    let image: Image<_> = Image::new(&image_raw, Point::new(120, 80));
    image.draw(&mut display).unwrap();

    info!("Ferris drawn!");

    // Draw Rust logo (BMP format)
    let logo = Bmp::from_slice(include_bytes!("rust.bmp")).unwrap();
    let logo = Image::new(&logo, Point::new(40, 80));
    logo.draw(&mut display).unwrap();

    info!("Rust logo drawn!");
    info!("Display complete! (Backlight is always on with this 7-pin module)");

    // Main loop - display is now showing the images
    loop {
        // Use WFI (Wait For Interrupt) to save power
        unsafe { core::arch::asm!("wfi") };
    }
}
