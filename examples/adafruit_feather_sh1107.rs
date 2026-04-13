//! # Adafruit 128x64 OLED FeatherWing (SH1107) Image Example
//!
//! This example implements a local driver for the SH1107 OLED display using I2C.
//! Product: <https://www.adafruit.com/product/4650>
//!
//! It displays a raw image (rust.raw) on the screen.
//!
//! ## Hardware
//! - Board: Adafruit Feather ESP32-C6
//! - Display: Adafruit 128x64 OLED FeatherWing (SH1107)
//!
//! ## Wiring
//! - SDA: GPIO 19
//! - SCL: GPIO 18
//! - VCC: 3.3V
//! - GND: GND
//!

#![no_std]
#![no_main]

use defmt::info;
use embedded_graphics::{
    image::{Image, ImageRaw},
    pixelcolor::BinaryColor,
    prelude::*,
};
use esp_hal::{
    delay::Delay,
    i2c::master::{Config as I2cConfig, I2c},
    main,
};
use panic_rtt_target as _;
use tinybmp::Bmp;

esp_bootloader_esp_idf::esp_app_desc!();

/// Minimal SH1107 Driver (128x64 I2C)
pub struct Sh1107<I2C> {
    i2c: I2C,
    addr: u8,
    buffer: [u8; 128 * 64 / 8], // 1024 bytes for 128x64 monochrome
}

impl<I2C> Sh1107<I2C>
where
    I2C: embedded_hal::i2c::I2c,
{
    pub fn new(i2c: I2C, addr: u8) -> Self {
        Self {
            i2c,
            addr,
            buffer: [0; 128 * 64 / 8],
        }
    }

    pub fn init(&mut self) -> Result<(), I2C::Error> {
        let cmds = [
            0xAE, // Display OFF
            0xD5, 0x51, // Set Clock Divide Ratio
            0x20, // Set Memory Addressing Mode
            0x81, 0x4F, // Set Contrast Control
            0xAD, 0x8A, // DC-DC ON
            0xA0, // Segment Remap (A0 = Seg0 -> Col0) - Un-mirrored
            0xC8, // COM Scan (C8 = COM[N-1] -> COM0)  - Hardware Flip X
            0xDC, 0x00, // Display Start Line 0
            0xD3, 0x60, // Display Offset 0x60 (96)
            0xD9, 0x22, // Pre-charge
            0xDB, 0x35, // VCOMH
            0xA8, 0x7F, // Multiplex Ratio 128
            0xA4, // Output follows RAM
            0xA6, // Normal Display
            0x2E, // Deactivate Scroll
            0xAF, // Display ON
        ];

        for cmd in cmds {
            self.write_command(cmd)?;
        }
        Ok(())
    }

    fn write_command(&mut self, cmd: u8) -> Result<(), I2C::Error> {
        self.i2c.write(self.addr, &[0x00, cmd])
    }

    pub fn flush(&mut self) -> Result<(), I2C::Error> {
        // Physical Layout: 64 Columns x 128 Rows (16 Pages)
        for page in 0..16 {
            self.write_command(0xB0 + page as u8)?; // Page Address
            self.write_command(0x00)?; // Col Low
            self.write_command(0x10)?; // Col High

            let mut data_buf = [0u8; 65]; // Control + 64 bytes
            data_buf[0] = 0x40;

            // Calculate buffer slice
            // Buffer is packed: Page 0 (Cols 0..63), Page 1 ...
            let start = page * 64;
            let end = start + 64;
            data_buf[1..65].copy_from_slice(&self.buffer[start..end]);

            self.i2c.write(self.addr, &data_buf)?;
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.buffer.fill(0);
    }
}

impl<I2C> OriginDimensions for Sh1107<I2C> {
    fn size(&self) -> Size {
        Size::new(128, 64)
    }
}

impl<I2C> DrawTarget for Sh1107<I2C>
where
    I2C: embedded_hal::i2c::I2c,
{
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels.into_iter() {
            if point.x >= 0 && point.x < 128 && point.y >= 0 && point.y < 64 {
                // ROTATION 90 deg (Logical 128x64 -> Physical 64x128)
                // Hardware flips handles Direction. We just map Coordinate Space.
                // Row = X, Col = Y.

                let row = point.x as usize;
                let col = point.y as usize;

                // Physical Memory Mapping (64x128)
                let page = row / 8;
                let bit = row % 8;
                let idx = (page * 64) + col;

                match color {
                    BinaryColor::On => self.buffer[idx] |= 1 << bit,
                    BinaryColor::Off => self.buffer[idx] &= !(1 << bit),
                }
            }
        }
        Ok(())
    }
}

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    info!("Initializing SH1107 OLED...");

    // Configure I2C pins
    let sda = peripherals.GPIO19;
    let scl = peripherals.GPIO18;

    // Create I2C peripheral
    let i2c = I2c::new(peripherals.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(sda)
        .with_scl(scl);

    // Initialize SH1107 driver
    let mut display = Sh1107::new(i2c, 0x3C);

    display.init().unwrap();
    display.clear();
    display.flush().unwrap();

    info!("Display initialized!");

    // Load Images
    // rust.raw is 64x64 monochrome (Raw bits)
    // ferris64x64bw.bmp is 64x64 monochrome (BMP format)

    let rust_raw: ImageRaw<BinaryColor> = ImageRaw::new(include_bytes!("./rust.raw"), 64);

    // Load BMP
    let bmp_data = include_bytes!("./ferris64x64bw.bmp");
    let ferris_bmp: Bmp<BinaryColor> = Bmp::from_slice(bmp_data).unwrap();

    // Display Side-by-Side
    // Ferris on Left (0,0)
    let img_ferris = Image::new(&ferris_bmp, Point::new(0, 0));

    // Rust on Right (64,0)
    let img_rust = Image::new(&rust_raw, Point::new(64, 0));

    img_ferris.draw(&mut display).unwrap();
    img_rust.draw(&mut display).unwrap();

    // Flush buffer to screen
    display.flush().unwrap();
    info!("Drawing complete. Loop forever...");

    loop {
        delay.delay_millis(1000);
    }
}
