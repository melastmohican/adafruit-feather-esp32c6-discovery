//! # Enviro+ FeatherWing LCD Display Example for Adafruit Feather ESP32-C6
//!
//! This example drives the ST7735s display on the Enviro+ FeatherWing.
//!
//! ## Pin Mapping (Feather C6)
//! - **SCK**: GPIO 21
//! - **MOSI**: GPIO 22
//! - **LCD_CS (D6)**: GPIO 6 (Shared with A2/OX Gas)
//! - **LCD_DC (D5)**: GPIO 5
//! - **LCD_RST/BL (D9)**: GPIO 9 (Shared with onboard NeoPixel)

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

    info!("Initializing...");

    // --- 1. SETUP STATUS LED (GPIO 15) ---
    let mut status_led = esp_hal::gpio::Output::new(
        peripherals.GPIO15,
        esp_hal::gpio::Level::Low,
        esp_hal::gpio::OutputConfig::default(),
    );

    // --- 2. POWER & BACKLIGHT ---
    // Power on the I2C / NeoPixel port (GPIO 20)
    let mut _pwr = esp_hal::gpio::Output::new(
        peripherals.GPIO20,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );

    // Heater Enable (GPIO 4) - Sometimes used for display power/backlight on some wings
    let mut _heater = esp_hal::gpio::Output::new(
        peripherals.GPIO4,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );

    // Backlight / Reset (GPIO 9)
    let mut bl_rst = Output::new(peripherals.GPIO9, Level::Low, OutputConfig::default());
    delay.delay_millis(100);
    bl_rst.set_high(); // ON
    delay.delay_millis(100);

    status_led.set_high();
    info!("Power and Backlight enabled. LED: ON");

    // --- 3. CONFIG SPI ---
    let sck = peripherals.GPIO21;
    let mosi = peripherals.GPIO22;
    let miso = peripherals.GPIO23;
    let cs = Output::new(peripherals.GPIO6, Level::High, OutputConfig::default());
    let dc = Output::new(peripherals.GPIO5, Level::Low, OutputConfig::default());

    // SPI at 26MHz (Standard)
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(26))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sck)
    .with_mosi(mosi)
    .with_miso(miso);

    let spi_device = ExclusiveDevice::new(spi, cs, delay).unwrap();
    let di = SPIInterface::new(spi_device, dc);

    // --- 4. DISPLAY INIT ---
    // Try the most standard 0.96" ST7735S configuration
    let mut display = Builder::new(ST7735s, di)
        .display_size(80, 160)
        .display_offset(26, 1)
        .invert_colors(ColorInversion::Inverted)
        .color_order(ColorOrder::Bgr)
        .orientation(Orientation::default().rotate(Rotation::Deg270))
        .init(&mut delay)
        .unwrap();

    status_led.set_low();
    info!("Display Init Success. LED: OFF");

    // Clear screen to BLUE (If colors are swapped, this helps identify)
    display.clear(Rgb565::BLUE).unwrap();

    // Draw something bold
    let style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
    Rectangle::new(Point::new(0, 0), Size::new(160, 80))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_DARK_MAGENTA))
        .draw(&mut display)
        .unwrap();
    Text::with_alignment("ENVIRO+ TEST", Point::new(80, 40), style, Alignment::Center)
        .draw(&mut display)
        .unwrap();

    // --- 4. LOOP ---
    info!("Looping...");
    loop {
        // Blink status LED every second to show life
        status_led.toggle();
        delay.delay_millis(1000);
    }
}
