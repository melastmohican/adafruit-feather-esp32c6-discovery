//! # Mocha Image Display Example for ILI9341 TFT LCD
//!
//! Display a 320x240 image of Mocha on the ILI9341 2.2" TFT LCD display (Product 1480).
//!
//! This example is adapted for the Adafruit Feather ESP32-C6 (Product 5933) and includes
//! backlight control.
//!
//! ## Hardware: Adafruit 2.2" TFT SPI 240x320 Display (Product 1480)
//!
//! ## Wiring for Adafruit Feather ESP32-C6
//!
//! ```
//!      LCD Pin     ->  Feather ESP32-C6
//! -----------------------------------------------
//!        VCC       ->  3.3V
//!        GND       ->  GND
//!        SCK       ->  IO21 (SCK)
//!        MOSI      ->  IO22 (MOSI)
//!        MISO      ->  IO23 (MISO)
//!        CS        ->  IO7  (D7)
//!        DC        ->  IO6  (A2)
//!        RESET     ->  IO5  (A3)
//!        LITE      ->  IO4  (A0) (Backlight)
//! ```
//!
//! Run with `cargo run --example ili9341_spi`.

#![no_std]
#![no_main]

use defmt::info;
use display_interface_spi::SPIInterface;
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::Point,
    image::Image,
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
    models::ILI9341Rgb565,
    options::{ColorOrder, Orientation, Rotation},
};
use panic_rtt_target as _;
use tinybmp::Bmp;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    info!("Initializing ILI9341 TFT LCD display for Mocha image...");

    // Configure SPI pins for Adafruit Feather ESP32-C6
    let sck = peripherals.GPIO21;
    let mosi = peripherals.GPIO22;
    let miso = peripherals.GPIO23;

    // Control pins
    let cs = Output::new(peripherals.GPIO7, Level::High, OutputConfig::default());
    let dc = Output::new(peripherals.GPIO6, Level::Low, OutputConfig::default());
    let rst = Output::new(peripherals.GPIO5, Level::High, OutputConfig::default());

    // Backlight control - turn on
    let _backlight = Output::new(peripherals.GPIO4, Level::High, OutputConfig::default());

    // Create SPI bus with 40 MHz clock speed
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(40))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sck)
    .with_mosi(mosi)
    .with_miso(miso);

    info!("SPI configured at 40 MHz");

    // Create exclusive SPI device with CS pin
    let spi_device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();

    // Create display interface
    let di = SPIInterface::new(spi_device, dc);

    let mut delay = Delay::new();

    info!("Initializing display in landscape mode...");

    // Create and initialize display using mipidsi
    // Physical display is 240x320 (portrait), we rotate to landscape (320x240)
    let mut display = Builder::new(ILI9341Rgb565, di)
        .reset_pin(rst)
        .display_size(240, 320) // Physical dimensions
        .orientation(Orientation::new().rotate(Rotation::Deg90).flip_horizontal())
        .color_order(ColorOrder::Bgr)
        .init(&mut delay)
        .unwrap();

    info!("Display initialized in landscape mode (320x240)!");

    // Clear screen to black
    display.clear(Rgb565::BLACK).unwrap();

    info!("Loading Mocha image (320x240 BMP)...");

    // Load the BMP image data using tinybmp
    let bmp = Bmp::<Rgb565>::from_slice(include_bytes!("mocha_320x240.bmp"))
        .expect("Failed to load BMP image");

    info!("Drawing Mocha image...");

    // Draw the image at origin (0, 0) to fill the entire screen
    let image = Image::new(&bmp, Point::new(0, 0));
    image.draw(&mut display).unwrap();

    info!("Mocha image displayed!");

    // Main loop - image is now showing
    loop {
        // Use WFI (Wait For Interrupt) to save power
        unsafe { core::arch::asm!("wfi") };
    }
}
