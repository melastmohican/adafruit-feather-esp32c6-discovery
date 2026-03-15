//! # STHS34PF80 Full Feature Example for Adafruit Feather ESP32-C6
//!
//! This example provides "Full Feature" parity with official Arduino/C drivers.
//! It uses the `sths34pf80` crate for initialization and then performs manual
//! I2C register reads to access data not yet exposed by the crate's high-level API.
//!
//! ## Hardware
//!
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Sensor:** Adafruit STHS34PF80 IR Presence / Motion Sensor
//! - **Connection:** Qwiic/STEMMA QT cable (I2C)
//!
//! ## Wiring with Qwiic/STEMMA QT
//!
//! Simply connect the Qwiic/STEMMA QT cable between the board and the sensor.
//!
//! Run with `cargo run --example sths34pf80_full`.
//!
//! ## Expected Output
//!
//! ```text
//! [INFO ] Amb: 23.64°C | Pres: 1536 | Mot: 0 | Obj: 7849 | Comp: 7849
//! [INFO ] Amb: 23.68°C | Pres: 1536 | Mot: 0 | Obj: 7805 | Comp: 7805
//! [INFO ] Amb: 23.59°C | Pres: 1536 | Mot: 0 | Obj: 7912 | Comp: 7912
//! ```

#![no_std]
#![no_main]

use defmt::{error, info};
use esp_hal::{
    delay::Delay,
    i2c::master::{Config as I2cConfig, I2c},
    main,
    time::Rate,
};
use panic_rtt_target as _;
use sths34pf80::Sths34pf80;

esp_bootloader_esp_idf::esp_app_desc!();

// Register Map from Datasheet
const STHS34PF80_ADDR: u8 = 0x5A;
const REG_PRESENCE_L: u8 = 0x22;
const REG_MOTION_L: u8 = 0x24;
const REG_TOBJECT_L: u8 = 0x26;
const REG_TAMBIENT_L: u8 = 0x28;
const REG_TCOMP_L: u8 = 0x38;

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    info!("Initializing STHS34PF80 sensor (Full Example)...");

    // Power on the I2C / NeoPixel port (GPIO 20)
    let _pwr = esp_hal::gpio::Output::new(
        peripherals.GPIO20,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );

    // Give hardware time to boot
    delay.delay_millis(500);

    // Configure I2C
    let i2c_config = I2cConfig::default().with_frequency(Rate::from_khz(100));
    let i2c = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .with_sda(peripherals.GPIO19)
        .with_scl(peripherals.GPIO18);

    // 1. USE CRATE FOR INITIALIZATION
    info!("Phase 1: Initializing via sths34pf80 crate...");
    let mut sensor_driver = Sths34pf80::new(i2c, delay);

    if let Err(e) = sensor_driver.initialize() {
        error!(
            "Failed to initialize via crate: {:?}",
            defmt::Debug2Format(&e)
        );
        loop {
            delay.delay_millis(1000);
        }
    }
    info!("Sensor initialized successfully!");

    // 2. RELEASE I2C FOR MANUAL ACCESS
    // This gives us the I2c peripheral back so we can read any register we want.
    let mut i2c = sensor_driver.release();

    info!("Phase 2: Starting Manual Data Retrieval (Arduino Parity)...");

    loop {
        // Read buffer for 2-byte registers (Little Endian)
        let mut buf = [0u8; 2];

        // -- AMBIENT TEMPERATURE (0x28) --
        // Scale: 100 LSB/°C
        let amb_temp = if i2c
            .write_read(STHS34PF80_ADDR, &[REG_TAMBIENT_L], &mut buf)
            .is_ok()
        {
            let raw = i16::from_le_bytes(buf);
            raw as f32 / 100.0
        } else {
            0.0
        };

        // -- PRESENCE (0x22) --
        let presence = if i2c
            .write_read(STHS34PF80_ADDR, &[REG_PRESENCE_L], &mut buf)
            .is_ok()
        {
            i16::from_le_bytes(buf)
        } else {
            0
        };

        // -- MOTION (0x24) --
        let motion = if i2c
            .write_read(STHS34PF80_ADDR, &[REG_MOTION_L], &mut buf)
            .is_ok()
        {
            i16::from_le_bytes(buf)
        } else {
            0
        };

        // -- RAW OBJECT IR (0x26) --
        let obj_raw = if i2c
            .write_read(STHS34PF80_ADDR, &[REG_TOBJECT_L], &mut buf)
            .is_ok()
        {
            i16::from_le_bytes(buf)
        } else {
            0
        };

        // -- COMPENSATED OBJECT (0x38) --
        let obj_comp = if i2c
            .write_read(STHS34PF80_ADDR, &[REG_TCOMP_L], &mut buf)
            .is_ok()
        {
            i16::from_le_bytes(buf)
        } else {
            0
        };

        info!(
            "Amb: {}.{:02}°C | Pres: {} | Mot: {} | Obj: {} | Comp: {}",
            amb_temp as i32,
            ((amb_temp.abs() % 1.0) * 100.0) as u32,
            presence,
            motion,
            obj_raw,
            obj_comp,
        );

        delay.delay_millis(1000);
    }
}
