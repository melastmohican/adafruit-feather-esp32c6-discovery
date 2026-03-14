//! # ILI9341 TFT LCD Display SPI Text Example for Adafruit Feather ESP32-C6
//!
//! This example demonstrates drawing text and shapes on a 240x320 ILI9341 display over SPI.
//!
//! This example is adapted for the Adafruit Feather ESP32-C6 (Product 5933).
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
//! Run with `cargo run --example ili9341_spi_text`.

#![no_std]
#![no_main]

use defmt::info;
use display_interface_spi::SPIInterface;
use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10, ascii::FONT_9X15_BOLD, ascii::FONT_10X20},
    pixelcolor::{Rgb565, RgbColor},
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, Rectangle},
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
    models::ILI9341Rgb565,
    options::{ColorOrder, Orientation},
};
use panic_rtt_target as _;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    info!("Initializing ILI9341 TFT LCD display (2.2 inch 240x320)...");

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

    info!("Initializing display with mipidsi...");

    // Create and initialize display using mipidsi
    let mut display = Builder::new(ILI9341Rgb565, di)
        .reset_pin(rst)
        .display_size(240, 320)
        .orientation(Orientation::default().flip_horizontal())
        .color_order(ColorOrder::Bgr)
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
        .background_color(Rgb565::BLUE)
        .build();

    let subtitle_style = MonoTextStyleBuilder::new()
        .font(&FONT_9X15_BOLD)
        .text_color(Rgb565::YELLOW)
        .build();

    let small_text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(Rgb565::CYAN)
        .build();

    // Draw title bar at top
    Rectangle::new(Point::new(0, 0), Size::new(240, 25))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
        .draw(&mut display)
        .unwrap();

    // Draw title text
    Text::with_baseline(
        "ILI9341 Display",
        Point::new(10, 5),
        title_style,
        Baseline::Top,
    )
    .draw(&mut display)
    .unwrap();

    // Draw subtitle
    Text::with_baseline(
        "Feather ESP32-C6",
        Point::new(45, 35),
        subtitle_style,
        Baseline::Top,
    )
    .draw(&mut display)
    .unwrap();

    // Draw resolution info
    Text::with_baseline(
        "240x320 TFT LCD",
        Point::new(65, 55),
        small_text_style,
        Baseline::Top,
    )
    .draw(&mut display)
    .unwrap();

    // Draw separator line
    Line::new(Point::new(0, 75), Point::new(239, 75))
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::GREEN, 2))
        .draw(&mut display)
        .unwrap();

    // Draw a large rectangle
    Rectangle::new(Point::new(20, 90), Size::new(200, 80))
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::RED, 3))
        .draw(&mut display)
        .unwrap();

    // Draw filled rectangles
    Rectangle::new(Point::new(30, 100), Size::new(80, 30))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
        .draw(&mut display)
        .unwrap();

    Rectangle::new(Point::new(130, 100), Size::new(80, 30))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
        .draw(&mut display)
        .unwrap();

    Rectangle::new(Point::new(30, 135), Size::new(80, 30))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
        .draw(&mut display)
        .unwrap();

    Rectangle::new(Point::new(130, 135), Size::new(80, 30))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::YELLOW))
        .draw(&mut display)
        .unwrap();

    // Draw circles
    Circle::new(Point::new(50, 190), 40)
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::MAGENTA, 2))
        .draw(&mut display)
        .unwrap();

    Circle::new(Point::new(150, 190), 40)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::CYAN))
        .draw(&mut display)
        .unwrap();

    // Draw some diagonal lines
    Line::new(Point::new(20, 250), Point::new(220, 280))
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::WHITE, 2))
        .draw(&mut display)
        .unwrap();

    Line::new(Point::new(220, 250), Point::new(20, 280))
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::WHITE, 2))
        .draw(&mut display)
        .unwrap();

    // Draw bottom text
    Text::with_baseline(
        "Rust Embedded Graphics",
        Point::new(25, 295),
        small_text_style,
        Baseline::Top,
    )
    .draw(&mut display)
    .unwrap();

    info!("Display complete!");

    // Keep display showing
    loop {
        unsafe { core::arch::asm!("wfi") };
    }
}
