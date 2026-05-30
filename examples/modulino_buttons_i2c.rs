//! # Arduino Modulino Buttons Example for Adafruit Feather ESP32-C6
//!
//! Reads button states from the Arduino Modulino Buttons module and controls the LEDs.
//!
//! ## Hardware
//!
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Module:** Arduino Modulino Buttons
//!
//! ## Wiring with Qwiic/STEMMA QT
//!
//! Simply connect the Qwiic/STEMMA QT cable between the board and the Modulino Buttons.
//! The cable provides:
//! ```
//!      Modulino Buttons -> Adafruit Feather ESP32-C6
//! (black)  GND -> GND (Stemma GND)
//! (red)    VCC -> 3.3V (Stemma V+)
//! (yellow) SCL -> GPIO 18 (Stemma SCL)
//! (blue)   SDA -> GPIO 19 (Stemma SDA)
//! ```
//!
//! ## Run
//!
//! ```bash
//! cargo run --example modulino_buttons_i2c
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

// Import the Modulino library
use modulino::Buttons;

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    info!("Initializing Arduino Modulino Buttons...");

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

    // Create Modulino Buttons driver
    let mut buttons = match Buttons::new(i2c) {
        Ok(b) => b,
        Err(e) => {
            error!(
                "Failed to initialize Modulino Buttons: {:?}",
                Debug2Format(&e)
            );
            loop {
                delay.delay_millis(1000);
            }
        }
    };

    info!(
        "Modulino Buttons initialized at address 0x{:02X}",
        buttons.address()
    );

    // Startup Test: Blink all LEDs to confirm connection
    info!("Testing LEDs...");
    if let Err(e) = buttons.all_leds_on() {
        error!("Failed to turn on LEDs: {:?}", Debug2Format(&e));
    }
    delay.delay_millis(500);
    if let Err(e) = buttons.all_leds_off() {
        error!("Failed to turn off LEDs: {:?}", Debug2Format(&e));
    }
    info!("LED Test Complete.");

    info!("Press buttons to toggle LEDs!");

    let mut prev_state = modulino::ButtonState::default();

    loop {
        // Read button states
        match buttons.read() {
            Ok(state) => {
                let mut need_update = false;

                // Button A: Rising Edge Detection
                if state.a && !prev_state.a {
                    info!("Button A pressed - Toggling LED A");
                    buttons.led_a.toggle();
                    need_update = true;
                }

                // Button B: Rising Edge Detection
                if state.b && !prev_state.b {
                    info!("Button B pressed - Toggling LED B");
                    buttons.led_b.toggle();
                    need_update = true;
                }

                // Button C: Rising Edge Detection
                if state.c && !prev_state.c {
                    info!("Button C pressed - Toggling LED C");
                    buttons.led_c.toggle();
                    need_update = true;
                }

                // Update LEDs only if state changed
                if need_update {
                    let update_res = buttons.update_leds();
                    if let Err(e) = update_res {
                        error!("Failed to update LEDs: {:?}", Debug2Format(&e));
                    }
                }

                // Save state for next iteration
                prev_state = state;
            }
            Err(e) => {
                error!("Failed to read buttons: {:?}", Debug2Format(&e));
            }
        }

        // Poll every 20ms for better responsiveness
        delay.delay_millis(20);
    }
}
