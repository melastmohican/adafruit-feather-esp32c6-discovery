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
    primitives::{Circle, PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    main,
    rmt::Rmt,
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi},
    },
    time::Rate,
};
use esp_hal_smartled::{SmartLedsAdapter, smart_led_buffer};
use mipidsi::{
    Builder,
    models::ST7735s,
    options::{ColorInversion, ColorOrder, Orientation, Rotation},
};
use panic_rtt_target as _;
use smart_leds::{RGB8, SmartLedsWrite, brightness, gamma, hsv::Hsv, hsv::hsv2rgb};

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut delay = Delay::new();

    info!("Initializing...");

    // Power on the I2C / NeoPixel port (GPIO 20)
    let _pwr = esp_hal::gpio::Output::new(
        peripherals.GPIO20,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );

    // --- 1. SETUP RGB LED ---
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    let rmt_channel = rmt.channel0;
    let mut rmt_buffer = smart_led_buffer!(1);
    let mut smart_led = SmartLedsAdapter::new(rmt_channel, peripherals.GPIO8, &mut rmt_buffer);

    // DEBUG: Set LED to BLUE (Stage 1: Started)
    // We use a helper closure/macro to write color to allow easier swapping
    let mut set_color = |r, g, b| {
        let color = RGB8::new(r, g, b);
        let data = [color];
        smart_led
            .write(brightness(gamma(data.iter().cloned()), 20))
            .ok();
    };

    set_color(0, 0, 255); // BLUE
    info!("LED: BLUE (Started)");
    delay.delay_millis(1000);

    // --- 2. CONFIG SPI & GPIO ---
    info!("Configuring SPI/GPIO...");

    let sck = peripherals.GPIO21;
    let mosi = peripherals.GPIO22;
    let miso = peripherals.GPIO23;

    let cs = Output::new(peripherals.GPIO6, Level::High, OutputConfig::default());
    let dc = Output::new(peripherals.GPIO5, Level::Low, OutputConfig::default());
    let rst = Output::new(peripherals.GPIO9, Level::High, OutputConfig::default());

    // DEBUG: Set LED to ORANGE (Stage 2: GPIO Configured)
    set_color(255, 165, 0); // ORANGE
    delay.delay_millis(500); // Short delay to ensure color is seen before next step

    // SPI Init
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

    let spi_device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    let di = SPIInterface::new(spi_device, dc);

    // --- 3. DISPLAY INIT ---
    // If it crashes here, LED stays ORANGE
    let mut display = Builder::new(ST7735s, di)
        .display_size(160, 80)
        .reset_pin(rst)
        .invert_colors(ColorInversion::Inverted)
        .color_order(ColorOrder::Bgr)
        .orientation(Orientation::default().rotate(Rotation::Deg270))
        .init(&mut delay)
        .unwrap();

    // DEBUG: Set LED to GREEN (Stage 3: Success)
    set_color(0, 255, 0); // GREEN
    info!("Display Init Success. LED: GREEN");

    // Clear screen
    display.clear(Rgb565::BLACK).unwrap();

    // Draw something
    let style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
    Rectangle::new(Point::new(0, 0), Size::new(160, 80))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_DARK_SLATE_BLUE))
        .draw(&mut display)
        .unwrap();
    Text::with_alignment("Enviro+ Rust", Point::new(80, 20), style, Alignment::Center)
        .draw(&mut display)
        .unwrap();
    Circle::new(Point::new(15, 45), 20)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::YELLOW))
        .draw(&mut display)
        .unwrap();
    Text::new("Sensor: Active", Point::new(45, 60), style)
        .draw(&mut display)
        .unwrap();

    // --- 4. LOOP ---
    info!("Looping...");
    let mut hue = 0;
    loop {
        // Rainbow Cycle
        let color = hsv2rgb(Hsv {
            hue,
            sat: 255,
            val: 20,
        });
        let data = [color];
        smart_led
            .write(brightness(gamma(data.iter().cloned()), 10))
            .ok();

        hue = hue.wrapping_add(10);
        delay.delay_millis(100);
    }
}
