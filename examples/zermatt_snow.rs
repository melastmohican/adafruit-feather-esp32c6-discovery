//! # Zermatt Image Display with Falling Snow Effect
//!
//! Display a 320x240 image of Zermatt on the ILI9341 2.2" TFT LCD display with animated falling snow.
//!
//! This example is adapted for the Adafruit Feather ESP32-C6 (Product 5933).
//!
//! ## Hardware: Adafruit 2.2" TFT SPI 240x320 Display (Product 1480)
//!
//! ## Wiring for Adafruit Feather ESP32-C6
//!
//! ```
//!      LCD Pin     ->  Feather ESP32-C6
//! -----------------------------------------------
//!        VCC       ->  3.3V
//!        GND       ->  GND
//!        SCK       ->  IO21 (SCK)
//!        MOSI      ->  IO22 (MOSI)
//!        MISO      ->  IO23 (MISO)
//!        CS        ->  IO7  (D7)
//!        DC        ->  IO6  (A2)
//!        RESET     ->  IO5  (A3)
//!        LITE      ->  IO4  (A0) (Backlight)
//! ```
//!
//! Run with `cargo run --example zermatt_snow`.

#![no_std]
#![no_main]

use defmt::info;
use display_interface_spi::SPIInterface;
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::Point,
    image::{GetPixel, Image},
    pixelcolor::{Rgb565, RgbColor},
};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    main,
    rng::Rng,
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi},
    },
    time::Rate,
};
use mipidsi::{
    Builder,
    models::ILI9341Rgb565,
    options::{ColorOrder, Orientation, Rotation},
};
use panic_rtt_target as _;
use tinybmp::Bmp;

esp_bootloader_esp_idf::esp_app_desc!();

// Display dimensions in landscape mode
const DISPLAY_WIDTH: usize = 320;
const DISPLAY_HEIGHT: usize = 240;

// Physics engine grid size
const PHY_DISP_RATIO: usize = 2; // Physical cell size in pixels
const PHY_WIDTH: usize = DISPLAY_WIDTH / PHY_DISP_RATIO;
const PHY_HEIGHT: usize = DISPLAY_HEIGHT / PHY_DISP_RATIO;

// Grid storage (1 bit per cell)
const BITS_PER_CELL: usize = 1;
const CELLS_PER_BYTE: usize = 8 / BITS_PER_CELL;
const GRID_TOTAL_CELLS: usize = PHY_WIDTH * PHY_HEIGHT;
const GRID_SIZE_BYTES: usize = GRID_TOTAL_CELLS / CELLS_PER_BYTE;

const FLAKE_SIZE: i32 = 2; // Small 2x2 pixel snowflakes
const SNOW_COLOR: Rgb565 = Rgb565::WHITE;

struct SnowGrid {
    grid: [u8; GRID_SIZE_BYTES],
}

impl SnowGrid {
    fn new() -> Self {
        Self {
            grid: [0u8; GRID_SIZE_BYTES],
        }
    }

    fn get_cell(&self, row: usize, col: usize) -> bool {
        let cell_index = row * PHY_WIDTH + col;
        let byte_index = cell_index / CELLS_PER_BYTE;
        let bit_index = cell_index % CELLS_PER_BYTE;
        (self.grid[byte_index] >> bit_index) & 1 == 1
    }

    fn set_cell(&mut self, row: usize, col: usize, value: bool) {
        let cell_index = row * PHY_WIDTH + col;
        let byte_index = cell_index / CELLS_PER_BYTE;
        let bit_index = cell_index % CELLS_PER_BYTE;

        if value {
            self.grid[byte_index] |= 1 << bit_index;
        } else {
            self.grid[byte_index] &= !(1 << bit_index);
        }
    }
}

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    info!("Initializing ILI9341 TFT LCD display for Mocha snow effect...");

    // Configure SPI pins for Adafruit Feather ESP32-C6
    let sck = peripherals.GPIO21;
    let mosi = peripherals.GPIO22;
    let miso = peripherals.GPIO23;

    // Control pins
    let cs = Output::new(peripherals.GPIO7, Level::High, OutputConfig::default());
    let dc = Output::new(peripherals.GPIO6, Level::Low, OutputConfig::default());
    let rst = Output::new(peripherals.GPIO5, Level::High, OutputConfig::default());

    // Backlight control - turn on
    let _backlight = Output::new(peripherals.GPIO4, Level::High, OutputConfig::default());

    // Hardware RNG for snow generation
    let rng = Rng::new();

    // Create SPI bus with 40 MHz clock speed
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(40))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(sck)
    .with_mosi(mosi)
    .with_miso(miso);

    info!("SPI configured at 40 MHz");

    // Create exclusive SPI device with CS pin
    let spi_device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();

    // Create display interface
    let di = SPIInterface::new(spi_device, dc);

    let mut delay = Delay::new();

    info!("Initializing display in landscape mode...");

    // Create and initialize display using mipidsi
    let mut display = Builder::new(ILI9341Rgb565, di)
        .reset_pin(rst)
        .display_size(240, 320)
        .orientation(Orientation::new().rotate(Rotation::Deg90).flip_horizontal())
        .color_order(ColorOrder::Bgr)
        .init(&mut delay)
        .unwrap();

    info!("Display initialized in landscape mode (320x240)!");

    // Clear screen to black
    display.clear(Rgb565::BLACK).unwrap();

    info!("Loading Zermatt image (320x240 BMP)...");

    // Load and display the BMP image
    let bmp_data = include_bytes!("zermatt_320x240.bmp");
    let bmp = Bmp::<Rgb565>::from_slice(bmp_data).expect("Failed to load BMP image");

    info!("Drawing Zermatt image...");
    let image = Image::new(&bmp, Point::new(0, 0));
    image.draw(&mut display).unwrap();

    info!("Image displayed! Starting snow animation...");

    // Initialize snow grid
    let mut snow_grid = SnowGrid::new();

    let mut frame_count = 0u32;

    // Main animation loop
    loop {
        // Simulate falling snow (iterate from bottom to top)
        for row in (0..PHY_HEIGHT - 1).rev() {
            for col in 0..PHY_WIDTH {
                if snow_grid.get_cell(row, col) {
                    // Calculate future column with slight randomness
                    // We need a value from -1 to 1.
                    let rand_val = rng.random() % 3;
                    let offset = (rand_val as i32) - 1; // 0->-1, 1->0, 2->1

                    let future_col =
                        (col as i32 + offset).max(0).min(PHY_WIDTH as i32 - 1) as usize;

                    // Check if future cell is empty
                    if !snow_grid.get_cell(row + 1, future_col) {
                        // Move snowflake down
                        snow_grid.set_cell(row + 1, future_col, true);
                        render_flake(&mut display, row + 1, future_col);
                    }

                    // Clear current position
                    snow_grid.set_cell(row, col, false);
                    render_void(&mut display, bmp_data, row, col);
                }
            }
        }

        // Clear snowflakes that reached the bottom
        for col in 0..PHY_WIDTH {
            if snow_grid.get_cell(PHY_HEIGHT - 1, col) {
                snow_grid.set_cell(PHY_HEIGHT - 1, col, false);
                render_void(&mut display, bmp_data, PHY_HEIGHT - 1, col);
            }
        }

        // Create new snow at the top
        for col in 0..PHY_WIDTH {
            if rng.random().is_multiple_of(25) {
                snow_grid.set_cell(0, col, true);
                render_flake(&mut display, 0, col);
            }
        }

        // Delay between frames
        delay.delay_millis(20);

        frame_count += 1;
        if frame_count.is_multiple_of(50) {
            info!("Frame: {}", frame_count);
        }
    }
}

// Render a snowflake at the given grid position
fn render_flake(display: &mut impl DrawTarget<Color = Rgb565>, row: usize, col: usize) {
    let x = (col * PHY_DISP_RATIO) as i32;
    let y = (row * PHY_DISP_RATIO) as i32;

    // Draw a small 2x2 white square for each snowflake
    for dy in 0..FLAKE_SIZE {
        for dx in 0..FLAKE_SIZE {
            display
                .draw_iter(core::iter::once(embedded_graphics::Pixel(
                    Point::new(x + dx, y + dy),
                    SNOW_COLOR,
                )))
                .ok();
        }
    }
}

// Restore the background image at the given grid position
fn render_void(
    display: &mut impl DrawTarget<Color = Rgb565>,
    bmp_data: &[u8],
    row: usize,
    col: usize,
) {
    let x = (col * PHY_DISP_RATIO) as i32;
    let y = (row * PHY_DISP_RATIO) as i32;

    // Load the BMP and extract the pixel region
    if let Ok(bmp) = Bmp::<Rgb565>::from_slice(bmp_data) {
        // For simplicity, redraw pixels one by one
        for dy in 0..FLAKE_SIZE {
            for dx in 0..FLAKE_SIZE {
                let px = x + dx;
                let py = y + dy;

                if px >= 0 && px < DISPLAY_WIDTH as i32 && py >= 0 && py < DISPLAY_HEIGHT as i32 {
                    // Get pixel from BMP
                    if let Some(pixel_color) = bmp.pixel(Point::new(px, py)) {
                        display
                            .draw_iter(core::iter::once(embedded_graphics::Pixel(
                                Point::new(px, py),
                                pixel_color,
                            )))
                            .ok();
                    }
                }
            }
        }
    }
}
