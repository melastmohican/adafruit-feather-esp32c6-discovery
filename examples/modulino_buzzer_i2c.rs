//! # Arduino Modulino Buzzer Example for Adafruit Feather ESP32-C6
//!
//! Plays a simple melody on the Arduino Modulino Buzzer module over I2C.
//!
//! ## Hardware
//!
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Module:** Arduino Modulino Buzzer
//!
//! ## Wiring with Qwiic/STEMMA QT
//!
//! Simply connect the Qwiic/STEMMA QT cable between the board and the Modulino Buzzer.
//! The cable provides:
//! ```
//!      Modulino Buzzer -> Adafruit Feather ESP32-C6
//! (black)  GND -> GND (Stemma GND)
//! (red)    VCC -> 3.3V (Stemma V+)
//! (yellow) SCL -> GPIO 18 (Stemma SCL)
//! (blue)   SDA -> GPIO 19 (Stemma SDA)
//! ```
//!
//! ## Run
//!
//! ```bash
//! cargo run --example modulino_buzzer_i2c
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
use modulino::{Buzzer, Note};

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    info!("Initializing Arduino Modulino Buzzer...");

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

    // Create Modulino Buzzer driver
    // Automatically uses default address 0x1E
    let mut buzzer = match Buzzer::new(i2c) {
        Ok(b) => b,
        Err(e) => {
            error!(
                "Failed to initialize Modulino Buzzer: {:?}",
                Debug2Format(&e)
            );
            loop {
                delay.delay_millis(1000);
            }
        }
    };

    info!(
        "Modulino Buzzer initialized at address 0x{:02X}!",
        buzzer.address()
    );
    info!("Playing melody...");

    // Simple Melody: Super Mario Theme (Intro)
    let melody = [
        (Note::E5, 100),
        (Note::E5, 100),
        (Note::Rest, 100),
        (Note::E5, 100),
        (Note::Rest, 100),
        (Note::C5, 100),
        (Note::E5, 100),
        (Note::Rest, 100),
        (Note::G5, 100),
        (Note::Rest, 300),
        (Note::G4, 100),
        (Note::Rest, 300),
    ];

    loop {
        for (note, duration) in melody.iter() {
            if *note == Note::Rest {
                // For rest, ensure no tone is playing and wait
                buzzer.no_tone().ok();
            } else {
                // Play the note
                // Note: The buzzer module handles the duration internally for the tone generation,
                // but we also need to wait here so we don't immediately send the next command.
                // We add a small gap between notes for articulation.
                if let Err(e) = buzzer.play_note(*note, *duration) {
                    error!("Failed to play note: {:?}", Debug2Format(&e));
                }
            }

            // Wait for the note duration plus a little gap
            delay.delay_millis(*duration as u32);

            // Small gap between notes (articulation)
            delay.delay_millis(50);
        }

        // Wait before repeating
        delay.delay_millis(2000);
    }
}
