#![no_std]
#![no_main]

// ==============================================================================
// Adafruit Feather ESP32-C6 BME280 + SSD1306 Example
// ==============================================================================
// Hardware Used:
// - Adafruit ESP32-C6 Feather
// - Adafruit BME280 Breakout (I2C Address: 0x77 or 0x76)
// - Generic SSD1306 OLED Breakout (I2C Address: 0x3C, 128x64 pixels)
//
// Wiring (Stemma QT / I2C):
// - 3.3V  -> Sensor 3.3V / VIN
// - GND   -> Sensor GND
// - SCL   -> GPIO 18 (Stemma SCL)
// - SDA   -> GPIO 19 (Stemma SDA)
// ==============================================================================

use bme280::i2c::BME280;
use defmt::{error, info};
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle},
    text::Text,
};
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    i2c::master::{Config as I2cConfig, I2c},
    main,
    time::Rate,
};
use heapless::String;
use panic_rtt_target as _;
use ssd1306::{I2CDisplayInterface, Ssd1306, prelude::*};

esp_bootloader_esp_idf::esp_app_desc!();

#[allow(clippy::large_stack_frames)]
#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    info!("Initializing peripherals...");
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let mut delay = Delay::new();

    // Power on the I2C / NeoPixel port (GPIO 20)
    info!("Enabling I2C / NeoPixel Power (GPIO 20)");
    let mut _pwr = esp_hal::gpio::Output::new(
        peripherals.GPIO20,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );

    // Give the hardware (especially OLED screens) a moment to boot up after receiving power
    delay.delay_millis(500);

    info!("Initializing I2C0 (SDA: GPIO 19, SCL: GPIO 18)");
    let i2c_config = I2cConfig::default().with_frequency(Rate::from_khz(100));

    let mut i2c = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(peripherals.GPIO19)
        .with_scl(peripherals.GPIO18);

    // --- I2C Bus Scan ---
    info!("Scanning I2C bus...");
    for address in 1..127 {
        if i2c.write(address, &[]).is_ok() {
            info!("Found I2C device at address: 0x{:02x}", address);
        }
    }

    // Share I2C bus using embedded_hal_bus::i2c::RefCellDevice
    let i2c_bus = core::cell::RefCell::new(i2c);

    // --- Initialize SSD1306 ---
    info!("Initializing SSD1306...");
    let interface = I2CDisplayInterface::new(embedded_hal_bus::i2c::RefCellDevice::new(&i2c_bus));
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    if display.init().is_err() {
        error!("SSD1306 failed at 0x3C! Trying 0x3D...");
        let interface2 = I2CDisplayInterface::new_alternate_address(
            embedded_hal_bus::i2c::RefCellDevice::new(&i2c_bus),
        );
        display = Ssd1306::new(interface2, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        if display.init().is_err() {
            error!("SSD1306 failed at 0x3D! Continuing to BME280 without display...");
        } else {
            info!("SSD1306 ready at 0x3D.");
            let _ = display.clear(BinaryColor::Off);
            let text_style = MonoTextStyleBuilder::new()
                .font(&FONT_6X10)
                .text_color(BinaryColor::On)
                .build();
            let _ = Text::new("Display Ready!", Point::new(0, 10), text_style).draw(&mut display);
            let _ = display.flush();
        }
    } else {
        info!("SSD1306 ready at 0x3C.");
        let _ = display.clear(BinaryColor::Off);
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();
        let _ = Text::new("Display Ready!", Point::new(0, 10), text_style).draw(&mut display);
        let _ = display.flush();
    }

    // --- Initialize BME280 ---
    info!("Initializing BME280...");
    let mut bme280 = BME280::new_primary(embedded_hal_bus::i2c::RefCellDevice::new(&i2c_bus));
    if bme280.init(&mut delay).is_err() {
        error!("Could not find a valid BME280 sensor at 0x77!");
        // Try fallback
        bme280 = BME280::new_secondary(embedded_hal_bus::i2c::RefCellDevice::new(&i2c_bus));
        if bme280.init(&mut delay).is_err() {
            error!("Could not find BME280 at 0x76 either.");
        } else {
            info!("Found BME280 at 0x76.");
        }
    } else {
        info!("BME280 ready at 0x77.");
    }

    info!("--- BME280 + SSD1306 I2C Test Loop ---");

    loop {
        let measurements = match bme280.measure(&mut delay) {
            Ok(m) => Some(m),
            Err(_) => {
                error!("Failed to read from BME280 sensor!");
                None
            }
        };

        let _ = display.clear(BinaryColor::Off);
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

        // Header
        let _ = Text::new("BME280 Sensor Data", Point::new(0, 10), text_style).draw(&mut display);
        let _ = Line::new(Point::new(0, 14), Point::new(127, 14))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(&mut display);

        if let Some(m) = measurements {
            let temp = m.temperature;
            let pressure = m.pressure / 100.0; // Convert Pa to hPa
            let humidity = m.humidity;

            info!(
                "Temp: {} C, Hum: {} %, Pres: {} hPa",
                temp, humidity, pressure
            );

            // Display Temp
            let mut temp_str: String<32> = String::new();
            core::fmt::write(&mut temp_str, format_args!("{:.1} C", temp)).unwrap();
            let _ = Text::new(&temp_str, Point::new(0, 30), text_style).draw(&mut display);

            // Display Humidity
            let mut hum_str: String<32> = String::new();
            core::fmt::write(&mut hum_str, format_args!("Humidity: {:.1} %", humidity)).unwrap();
            let _ = Text::new(&hum_str, Point::new(0, 44), text_style).draw(&mut display);

            // Display Pressure
            let mut press_str: String<32> = String::new();
            core::fmt::write(
                &mut press_str,
                format_args!("Pressure: {:.0} hPa", pressure),
            )
            .unwrap();
            let _ = Text::new(&press_str, Point::new(0, 58), text_style).draw(&mut display);
        } else {
            let _ = Text::new("Sensor Error!", Point::new(0, 30), text_style).draw(&mut display);
            let _ = Text::new("Check I2C/Power", Point::new(0, 44), text_style).draw(&mut display);
        }

        let _ = display.flush();
        delay.delay_millis(2000);
    }
}
