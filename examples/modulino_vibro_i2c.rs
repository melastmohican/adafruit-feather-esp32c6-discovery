//! # Arduino Modulino Vibro Example for Adafruit Feather ESP32-C6
//!
//! Demonstrates various vibration patterns on the Arduino Modulino Vibro module over I2C.
//!
//! ## Hardware
//!
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Module:** Arduino Modulino Vibro
//!
//! ## Wiring with Qwiic/STEMMA QT
//!
//! Simply connect the Qwiic/STEMMA QT cable between the board and the Modulino Vibro.
//! The cable provides:
//! ```
//!      Modulino Vibro -> Adafruit Feather ESP32-C6
//! (black)  GND -> GND (Stemma GND)
//! (red)    VCC -> 3.3V (Stemma V+)
//! (yellow) SCL -> GPIO 18 (Stemma SCL)
//! (blue)   SDA -> GPIO 19 (Stemma SDA)
//! ```
//!
//! ## Run
//!
//! ```bash
//! cargo run --example modulino_vibro_i2c
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
use modulino::{PowerLevel, Vibro};

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    info!("Initializing Arduino Modulino Vibro...");

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

    // Create I2C peripheral
    let i2c_config = I2cConfig::default().with_frequency(Rate::from_khz(100));
    let i2c = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(sda)
        .with_scl(scl);

    // Create Modulino Vibro driver
    // Automatically uses default address 0x24
    let mut vibro = match Vibro::new(i2c) {
        Ok(v) => v,
        Err(e) => {
            error!(
                "Failed to initialize Modulino Vibro: {:?}",
                Debug2Format(&e)
            );
            loop {
                delay.delay_millis(1000);
            }
        }
    };

    info!(
        "Modulino Vibro initialized at address 0x{:02X}!",
        vibro.address()
    );

    loop {
        // 1. Gentle Pulses
        info!("Pattern 1: Gentle Pulses");
        for _ in 0..3 {
            vibro.pulse(100, PowerLevel::Gentle).ok();
            delay.delay_millis(500);
        }
        delay.delay_millis(1000);

        // 2. Medium Vibration
        info!("Pattern 2: Medium Vibration");
        vibro.on(1000, PowerLevel::Medium).ok();
        delay.delay_millis(2000);

        // 3. Intense Double Pulse
        info!("Pattern 3: Intense Double Pulse");
        vibro.pulse(200, PowerLevel::Intense).ok();
        delay.delay_millis(300);
        vibro.pulse(200, PowerLevel::Intense).ok();
        delay.delay_millis(2000);

        // 4. Power Sweep
        info!("Pattern 4: Power Sweep");
        let power_levels = [
            PowerLevel::Gentle,
            PowerLevel::Moderate,
            PowerLevel::Medium,
            PowerLevel::Intense,
            PowerLevel::Powerful,
            PowerLevel::Maximum,
        ];

        for &level in power_levels.iter() {
            info!("Power Level: {:?}", Debug2Format(&level));
            vibro.on(500, level).ok();
            delay.delay_millis(1000);
        }

        info!("Pattern complete. Waiting 3 seconds...");
        delay.delay_millis(3000);
    }
}
