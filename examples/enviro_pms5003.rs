//! # PMS5003 PM2.5 Air Quality Sensor (UART) Example for Adafruit Feather ESP32-C6
//!
//! Reads particulate matter data from a PMS5003 sensor over UART (9600 8N1)
//! and prints PM1.0 / PM2.5 / PM10 concentrations via RTT/defmt.
//!
//! ## Hardware
//!
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Sensor:** Adafruit PM2.5 Air Quality Sensor (PMS5003)
//! - **FeatherWing:** Pimoroni Enviro+ FeatherWing
//!
//! ## Wiring
//!
//! On the Adafruit Feather ESP32-C6, the Enviro+ Wing connects the PMS5003 to:
//!
//! ```text
//! PMS5003 TXD -> MCU GPIO 17 (RX)
//! PMS5003 RXD -> MCU GPIO 16 (TX)
//! ```
//!
//! ## Notes
//!
//! - PMS5003 default UART settings: 9600 baud, 8 data bits, no parity, 1 stop bit.
//! - Requires 5V VCC (VBUS) to be connected for the sensor fan and laser.
//!
//! ## Run
//!
//! ```bash
//! cargo run --example enviro_pms5003
//! ```

#![no_std]
#![no_main]

use defmt::println;
use esp_hal::{
    Config as HalConfig,
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    main,
    uart::{Config as UartConfig, Uart},
};
use panic_rtt_target as _;
use pmsx003::PmsX003Sensor;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(HalConfig::default());


    // Power on the I2C / NeoPixel port (GPIO 20)
    // This is required on the Feather C6 to power the Stemma QT port and headers
    let _pwr = Output::new(
        peripherals.GPIO20,
        Level::High,
        OutputConfig::default(),
    );

    // Give hardware (especially I2C sensors) a moment to boot up after receiving power
    let delay = Delay::new();
    delay.delay_millis(500);

    let config = UartConfig::default().with_baudrate(9_600);

    // Physical slots labeled TX (16) and RX (17) on the Right Header
    let tx = peripherals.GPIO16; // MCU TX -> sensor RX
    let rx = peripherals.GPIO17; // MCU RX <- sensor TX

    println!("Using MCU TX=GPIO16 and MCU RX=GPIO17 (Native C6 Slots)");

    // Create UART and bind pins
    let mut uart = Uart::new(peripherals.UART1, config)
        .unwrap()
        .with_tx(tx)
        .with_rx(rx);

    println!("PMS5003 sensor initialized");
    println!("Starting continuous readings...");

    loop {
        // 1. Drain stale backlog from MCU UART buffer to ensure we catch the LATEST frame
        while uart.read_ready() {
            let mut discard = [0u8; 1];
            let _ = uart.read(&mut discard);
        }

        // 2. Initialize sensor and start "Deep Hunting" for a valid, non-zero frame
        let mut sensor = PmsX003Sensor::new(&mut uart);
        let mut found = false;
        
        for _ in 0..20 {
            match sensor.read() {
                Ok(frame) => {
                    // We hunt for non-zero frames to ensure the sensor is active and responsive
                    if frame.pm2_5 > 0 || frame.pm10 > 0 {
                        println!("PM1.0: {} μg/m³", frame.pm1_0);
                        println!("PM2.5: {} μg/m³", frame.pm2_5);
                        println!("PM10:  {} μg/m³", frame.pm10);
                        println!("---");
                        found = true;
                        break;
                    }
                }
                Err(_e) => {
                    // Stale data or mid-packet sync, retry after a short delay
                    delay.delay_millis(50);
                }
            }
        }

        if !found {
            println!("Wait: Catching fresh sync with PMS5003...");
        }

        delay.delay_millis(1500u32); // Pulse every 1.5s to match dashboard cadence
    }
}
