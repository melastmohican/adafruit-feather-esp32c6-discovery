//! # I2C Bus Scanner Example for Adafruit Feather ESP32-C6
//!
//! This example scans the I2C bus for connected devices and prints their addresses.
//!
//! The ESP32-C6 chip technically has two I2C buses (I2C0 and LP_I2C0).
//! However, the Adafruit Feather ESP32-C6 exposes a single primary I2C bus (I2C0)
//! which is shared between:
//! - The main breakout pins (SDA: GPIO19, SCL: GPIO18)
//! - The STEMMA QT / Qwiic connector
//! - The onboard MAX17048 battery monitor (fixed at address 0x36)
//!
//! **Note:** The STEMMA QT connector and onboard NeoPixel share a power pin (GPIO20)
//! that must be pulled high for devices attached to it to power on. This example
//! pulls that pin high to ensure everything is visible on the scan.

#![no_std]
#![no_main]

use defmt::info;
use esp_hal::{
    i2c::master::{Config as I2cConfig, I2c},
    main,
};
use panic_rtt_target as _;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = esp_hal::delay::Delay::new();

    // Power on the I2C / NeoPixel port (GPIO 20)
    info!("Enabling I2C / NeoPixel Power (GPIO 20)");
    let _pwr = esp_hal::gpio::Output::new(
        peripherals.GPIO20,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );

    // Give the hardware a moment to boot up after receiving power
    delay.delay_millis(50);

    info!("Initializing I2C bus scanner...");

    // Configure I2C pins
    let sda = peripherals.GPIO19;
    let scl = peripherals.GPIO18;

    // Create I2C peripheral with default configuration (100kHz)
    let mut i2c = I2c::new(peripherals.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(sda)
        .with_scl(scl);

    info!("Scanning I2C bus (addresses 0x00 to 0x7F)...");
    info!("-------------------------------------------");

    let mut devices_found = 0;

    // Scan all possible 7-bit I2C addresses (0x00 to 0x7F)
    for address in 0x00..=0x7F {
        // Try a zero-length write first
        let write_ok = i2c.write(address, &[]).is_ok();

        // If write fails, try a 1-byte read. Some devices (or I2C peripherals)
        // don't ACK zero-length writes properly.
        let mut buf = [0u8; 1];
        let read_ok = i2c.read(address, &mut buf).is_ok();

        if write_ok || read_ok {
            info!("Device found at address 0x{:02X}", address);
            devices_found += 1;
        }
    }

    info!("-------------------------------------------");
    if devices_found == 0 {
        info!("No I2C devices found!");
    } else {
        info!("Scan complete! Found {} device(s)", devices_found);
    }

    // Halt after scanning
    loop {
        // Wait for interrupts (low power mode)
        unsafe { core::arch::asm!("wfi") };
    }
}
