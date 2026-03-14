//! # GC9A01 Round LCD Display SPI Text Example for Adafruit Feather ESP32-C6
//!
//! This example demonstrates drawing text and shapes on a 240x240 round GC9A01 display over SPI.
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
//! Run with `cargo run --example gc9a01_spi_text`.

#![no_std]
#![no_main]

use defmt::info;
use display_interface_spi::SPIInterface;
use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10, ascii::FONT_9X15_BOLD, ascii::FONT_10X20},
    pixelcolor::{Rgb565, RgbColor},
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle},
    text::{Baseline, Text},
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

    info!("Drawing text and shapes...");

    // Create text styles
    let title_style = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(Rgb565::WHITE)
        .build();

    let subtitle_style = MonoTextStyleBuilder::new()
        .font(&FONT_9X15_BOLD)
        .text_color(Rgb565::YELLOW)
        .build();

    let small_text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(Rgb565::CYAN)
        .build();

    // Draw title text centered at top (accounting for round display shape)
    // Position slightly lower to avoid being cut off by circular edge
    Text::with_baseline("GC9A01", Point::new(75, 30), title_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();

    // Draw subtitle on second line
    Text::with_baseline(
        "Round Display",
        Point::new(60, 55),
        subtitle_style,
        Baseline::Top,
    )
    .draw(&mut display)
    .unwrap();

    // Draw small text below
    Text::with_baseline(
        "240x240 RGB",
        Point::new(78, 75),
        small_text_style,
        Baseline::Top,
    )
    .draw(&mut display)
    .unwrap();

    // Draw a large circle outline in the center (emphasizes round shape)
    Circle::new(Point::new(50, 100), 90)
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::BLUE, 2))
        .draw(&mut display)
        .unwrap();

    // Draw smaller concentric circle
    Circle::new(Point::new(80, 130), 30)
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::GREEN, 2))
        .draw(&mut display)
        .unwrap();

    // Draw filled circles at strategic positions
    Circle::new(Point::new(95, 115), 15)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
        .draw(&mut display)
        .unwrap();

    Circle::new(Point::new(130, 135), 12)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::MAGENTA))
        .draw(&mut display)
        .unwrap();

    Circle::new(Point::new(105, 150), 10)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_ORANGE))
        .draw(&mut display)
        .unwrap();

    // Draw lines radiating from center (like a clock face)
    let center = Point::new(120, 120);

    Line::new(center, Point::new(120, 40))
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::WHITE, 1))
        .draw(&mut display)
        .unwrap();

    Line::new(center, Point::new(200, 120))
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::WHITE, 1))
        .draw(&mut display)
        .unwrap();

    Line::new(center, Point::new(120, 200))
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::WHITE, 1))
        .draw(&mut display)
        .unwrap();

    Line::new(center, Point::new(40, 120))
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::WHITE, 1))
        .draw(&mut display)
        .unwrap();

    // Draw bottom text (positioned to fit within circular bounds)
    Text::with_baseline(
        "ESP32-C6",
        Point::new(85, 205),
        small_text_style,
        Baseline::Top,
    )
    .draw(&mut display)
    .unwrap();

    info!("Display complete! (Backlight is always on with this 7-pin module)");

    // Keep display showing
    loop {
        unsafe { core::arch::asm!("wfi") };
    }
}
