//! # Arduino Modulino Latch Relay Example for Adafruit Feather ESP32-C6
//!
//! Demonstrates how to control the Arduino Modulino Latch Relay module over I2C.
//! A latching relay maintains its state even when power is removed.
//!
//! ## Hardware
//!
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Module:** Arduino Modulino Latch Relay
//!
//! ## Wiring with Qwiic/STEMMA QT
//!
//! Simply connect the Qwiic/STEMMA QT cable between the board and the Modulino Latch Relay.
//! The cable provides:
//! ```
//!      Modulino Latch Relay -> Adafruit Feather ESP32-C6
//! (black)  GND -> GND (Stemma GND)
//! (red)    VCC -> 3.3V (Stemma V+)
//! (yellow) SCL -> GPIO 18 (Stemma SCL)
//! (blue)   SDA -> GPIO 19 (Stemma SDA)
//! ```
//!
//! ## Run
//!
//! ```bash
//! cargo run --example modulino_latch_relay_i2c
//! ```

#![no_std]
#![no_main]

use defmt::{Debug2Format, error, info};
use esp_hal::{
    delay::Delay,
    i2c::master::{Config as I2cConfig, I2c},
    main,
    time::Rate,
};
use panic_rtt_target as _;

esp_bootloader_esp_idf::esp_app_desc!();

// Import from modulino library
use modulino::LatchRelay;

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    info!("Initializing Arduino Modulino Latch Relay...");

    // Power on the I2C / NeoPixel port (GPIO 20)
    info!("Enabling I2C / NeoPixel Power (GPIO 20)");
    let _pwr = esp_hal::gpio::Output::new(
        peripherals.GPIO20,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );

    // Give the hardware a moment to boot up after receiving power
    delay.delay_millis(50);

    // Configure I2C pins
    let sda = peripherals.GPIO19;
    let scl = peripherals.GPIO18;

    // Create I2C peripheral (400kHz speed matches original)
    let i2c_config = I2cConfig::default().with_frequency(Rate::from_khz(400));
    let i2c = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(sda)
        .with_scl(scl);

    // Create Modulino Latch Relay driver
    // Automatically uses default address 0x02
    let mut relay = match LatchRelay::new(i2c) {
        Ok(r) => r,
        Err(e) => {
            error!(
                "Failed to initialize Modulino Latch Relay: {:?}",
                Debug2Format(&e)
            );
            loop {
                delay.delay_millis(1000);
            }
        }
    };

    info!(
        "Modulino Latch Relay initialized at address 0x{:02X}!",
        relay.address()
    );

    // Wait a bit for the device to be ready
    delay.delay_millis(100);

    loop {
        // 1. Check current state
        match relay.is_on() {
            Ok(Some(true)) => info!("Relay is currently ON"),
            Ok(Some(false)) => info!("Relay is currently OFF"),
            Ok(None) => info!("Relay state is unknown (first power-up)"),
            Err(e) => error!("Failed to read relay state: {:?}", Debug2Format(&e)),
        }

        // 2. Turn ON
        info!("Turning relay ON...");
        if let Err(e) = relay.on() {
            error!("Failed to turn relay ON: {:?}", Debug2Format(&e));
        }
        delay.delay_millis(2000);

        // 3. Turn OFF
        info!("Turning relay OFF...");
        if let Err(e) = relay.off() {
            error!("Failed to turn relay OFF: {:?}", Debug2Format(&e));
        }
        delay.delay_millis(2000);

        // 4. Toggle
        info!("Toggling relay...");
        if let Err(e) = relay.toggle() {
            error!("Failed to toggle relay: {:?}", Debug2Format(&e));
        }
        delay.delay_millis(2000);

        // 5. Toggle again
        info!("Toggling relay back...");
        if let Err(e) = relay.toggle() {
            error!("Failed to toggle relay back: {:?}", Debug2Format(&e));
        }

        info!("Cycle complete. Waiting 5 seconds...");
        delay.delay_millis(5000);
    }
}
