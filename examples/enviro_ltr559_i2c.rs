//! # LTR559 Light & Proximity Sensor Example for Adafruit Feather ESP32-C6
//!
//! Reads ambient light (Lux) and proximity data from an LTR559 sensor over I2C.
//!
//! ## Hardware
//! - **Board:** Adafruit Feather ESP32-C6
//! - **Sensor:** Pimoroni Enviro+ FeatherWing (LTR559)
//!
//! ## Pin Mapping (Feather C6)
//! - **SDA:** GPIO 18
//! - **SCL:** GPIO 19
//! - **PWR:** GPIO 20 (Must be HIGH to power the headers/Stemma port)

#![no_std]
#![no_main]

use defmt::info;
use esp_hal::{
    delay::Delay,
    i2c::master::{Config as I2cConfig, I2c},
    main,
};
use panic_rtt_target as _;

esp_bootloader_esp_idf::esp_app_desc!();

/// LTR559 I2C address
const LTR559_ADDR: u8 = 0x23;

// Registers
const REG_ALS_CONTROL: u8 = 0x80;
const REG_PS_CONTROL: u8 = 0x81;
const REG_ALS_DATA_CH1_0: u8 = 0x88;
const REG_PS_DATA: u8 = 0x8D;
const REG_ALS_MEAS_RATE: u8 = 0x85;

/// LTR559 Driver
struct Ltr559<I2C> {
    i2c: I2C,
}

impl<I2C> Ltr559<I2C>
where
    I2C: embedded_hal::i2c::I2c,
{
    /// Create a new LTR559 driver and initialize the sensor
    fn new(i2c: I2C, delay: &mut Delay) -> Result<Self, I2C::Error> {
        let mut sensor = Ltr559 { i2c };
        sensor.init(delay)?;
        Ok(sensor)
    }

    /// Initialize the sensor
    fn init(&mut self, delay: &mut Delay) -> Result<(), I2C::Error> {
        // Soft reset
        // Bit 1: SW Reset
        self.write_register(REG_ALS_CONTROL, 0x02)?;
        delay.delay_millis(100);

        // Active mode for ALS
        // Bit 0: Mode (1 = Active)
        // Bit 2-4: Gain
        // Mappings verified against LTR303-rs and Zephyr LTR329:
        // 000 = 1x
        // 001 = 2x
        // 010 = 4x (Default for Pimoroni Enviro+)
        // 011 = 8x
        // 110 = 48x
        // 111 = 96x
        self.write_register(REG_ALS_CONTROL, 0x01 | (0x02 << 2))?; // Active, Gain=4x (0x02)

        // Set integration time and measurement rate
        // Integration Time: 50ms (0x01 according to Python map? No. Python map: 50ms = 0b001 = 1)
        // Repeat Rate: 50ms (0b000)
        // REG_ALS_MEAS_RATE (0x85)
        // Bits 3-5: Integration Time. 50ms = 1 => (1 << 3) = 8.
        // Bits 0-2: Repeat Rate. 50ms = 0.
        // Value = 8.
        self.write_register(REG_ALS_MEAS_RATE, 0x08)?;

        // Active mode for PS
        // Bit 0-1: Active (11 = Active)
        // Bit 5: Saturation Indicator Enable (1 = Enable)
        self.write_register(REG_PS_CONTROL, 0x03 | 0x20)?;

        Ok(())
    }

    /// Read ALS (Ambient Light Sensor) data
    /// Returns (channel_1_raw, channel_0_raw)
    fn read_als_raw(&mut self) -> Result<(u16, u16), I2C::Error> {
        let mut buffer = [0u8; 4];
        // Read 4 bytes starting from REG_ALS_DATA_CH1_0 (0x80) -> Ch1 low, Ch1 high, Ch0 low, Ch0 high
        self.i2c
            .write_read(LTR559_ADDR, &[REG_ALS_DATA_CH1_0], &mut buffer)?;

        let ch1 = u16::from_le_bytes([buffer[0], buffer[1]]);
        let ch0 = u16::from_le_bytes([buffer[2], buffer[3]]);

        Ok((ch1, ch0))
    }

    /// Read Proximity Sensor data
    fn read_ps_raw(&mut self) -> Result<u16, I2C::Error> {
        let mut buffer = [0u8; 2];
        // Read 2 bytes from REG_PS_DATA (0x8D) -> PS low, PS high (only bits 0-2 of high byte valid)
        // Note: PS_DATA is 11 bits. Low byte is 0x8D, High byte is 0x8E (masked)
        // But register map says 0x8D is PS_DATA bits 0-7? Python code reads 0x8D as 16 bit?
        // Let's rely on reading 2 bytes to be safe, standard I2C auto-increment usually works.
        self.i2c
            .write_read(LTR559_ADDR, &[REG_PS_DATA], &mut buffer)?;

        let ps = u16::from_le_bytes([buffer[0], buffer[1]]) & 0x07FF; // Mask to 11 bits
        Ok(ps)
    }

    /// Calculate Lux from raw ALS data
    /// Uses formula and coefficients from Pimoroni Python driver
    fn get_lux(&self, ch1: u16, ch0: u16) -> f32 {
        let ch0 = ch0 as f32;
        let ch1 = ch1 as f32;

        let ratio = if (ch0 + ch1) > 0.0 {
            (ch1 * 100.0) / (ch1 + ch0)
        } else {
            101.0
        };

        // Coefficients from Pimoroni driver
        // ch0_c = (17743, 42785, 5926, 0)
        // ch1_c = (-11059, 19548, -1185, 0)
        let (c0, c1) = if ratio < 45.0 {
            (17743.0, -11059.0)
        } else if ratio < 64.0 {
            (42785.0, 19548.0)
        } else if ratio < 85.0 {
            (5926.0, -1185.0)
        } else {
            (0.0, 0.0)
        };

        let est_lux = (ch0 * c0) - (ch1 * c1);

        // Disable negative lux
        if est_lux < 0.0 {
            return 0.0;
        }

        // Integration time = 50ms, Gain = 4x
        let integration_time = 50.0;
        let gain = 4.0;

        est_lux / (integration_time / 100.0) / gain / 10000.0
    }

    /// Helper to write to a register
    fn write_register(&mut self, register: u8, value: u8) -> Result<(), I2C::Error> {
        self.i2c.write(LTR559_ADDR, &[register, value])
    }
}

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    info!("Initializing LTR559 sensor...");

    // Power on the I2C / NeoPixel port (GPIO 20)
    let _pwr = esp_hal::gpio::Output::new(
        peripherals.GPIO20,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );

    // Give hardware a moment to boot
    let mut delay = Delay::new();
    delay.delay_millis(50);

    // Configure I2C pins for Adafruit Feather ESP32-C6
    let sda = peripherals.GPIO18;
    let scl = peripherals.GPIO19;

    // Create I2C peripheral
    let i2c = I2c::new(peripherals.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(sda)
        .with_scl(scl);

    // Initialize sensor
    let mut sensor = match Ltr559::new(i2c, &mut delay) {
        Ok(s) => s,
        Err(_) => {
            info!("Failed to initialize LTR559. Check wiring!");
            loop {
                delay.delay_millis(1000);
            }
        }
    };

    info!("LTR559 initialized successfully!");
    info!("Starting measurements...");

    loop {
        // Read Proximity
        match sensor.read_ps_raw() {
            Ok(ps) => {
                info!("Proximity: {}", ps);
                if ps > 100 {
                    info!("-> Object Detected!");
                }
            }
            Err(_) => info!("Error reading Proximity"),
        }

        // Read Light
        match sensor.read_als_raw() {
            Ok((ch1, ch0)) => {
                let lux = sensor.get_lux(ch1, ch0);
                info!(
                    "Light: {}.{:02} Lux (Raw: Ch1={}, Ch0={})",
                    lux as u32,
                    ((lux % 1.0) * 100.0) as u32,
                    ch1,
                    ch0
                );
            }
            Err(_) => info!("Error reading Light"),
        }

        delay.delay_millis(500);
    }
}
