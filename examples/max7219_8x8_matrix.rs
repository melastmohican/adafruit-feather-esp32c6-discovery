//! Wiring Diagram (Adafruit Feather ESP32-C6 -> MAX7219 Matrix):
//! ------------------------------------------------
//! 5V (USB)  -> VCC
//! GND       -> GND
//! IO7       -> DIN   (SPI MOSI)
//! IO5       -> CS    (SPI CS)
//! IO6       -> CLK   (SPI SCK)
//!
//! Run with `cargo run --example max7219_8x8_matrix --release`.

#![no_std]
#![no_main]

use embedded_hal::delay::DelayNs;
use embedded_hal::spi::SpiDevice;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    main,
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi},
    },
    time::{Instant, Rate},
};
use panic_rtt_target as _;

esp_bootloader_esp_idf::esp_app_desc!();

// --------------------------------------
// Minimal MAX7219 Driver
// --------------------------------------
pub struct Max7219<SPI> {
    spi: SPI,
}

impl<SPI: SpiDevice> Max7219<SPI> {
    pub fn new(spi: SPI) -> Self {
        Self { spi }
    }

    pub fn init(&mut self) -> Result<(), SPI::Error> {
        self.write_reg(0x09, 0x00)?; // Decode mode: None
        self.write_reg(0x0A, 0x01)?; // Intensity: 1/32
        self.write_reg(0x0B, 0x07)?; // Scan limit: 8 digits
        self.write_reg(0x0C, 0x01)?; // Shutdown: Normal operation
        self.write_reg(0x0F, 0x00)?; // Display test: Off
        self.clear()?;
        Ok(())
    }

    pub fn write_reg(&mut self, reg: u8, data: u8) -> Result<(), SPI::Error> {
        self.spi.write(&[reg, data])
    }

    pub fn clear(&mut self) -> Result<(), SPI::Error> {
        for i in 1..=8 {
            self.write_reg(i, 0x00)?;
        }
        Ok(())
    }

    pub fn draw_raw(&mut self, buffer: &[u8; 8]) -> Result<(), SPI::Error> {
        for (i, &row) in buffer.iter().enumerate() {
            self.write_reg((i + 1) as u8, row)?;
        }
        Ok(())
    }

    pub fn set_intensity(&mut self, intensity: u8) -> Result<(), SPI::Error> {
        self.write_reg(0x0A, intensity & 0x0F)
    }
}

// --------------------------------------
// Pseudo-random number generator
// --------------------------------------
struct Lcg {
    state: u32,
}
impl Lcg {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }
    fn next(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }
    fn random_range(&mut self, min: i32, max: i32) -> i32 {
        let range = max - min + 1;
        let val = (self.next() >> 16) as i32;
        min + val.rem_euclid(range)
    }
}

// --------------------------------------
// Drawing Primitives and Buffer
// --------------------------------------
struct Drawing {
    buffer: [u8; 8],
}

impl Drawing {
    fn new() -> Self {
        Self { buffer: [0; 8] }
    }

    fn clear_screen(&mut self, on: bool) {
        self.buffer = if on { [255; 8] } else { [0; 8] };
    }

    fn set_pixel(&mut self, r: u8, c: u8) {
        if r < 8 && c < 8 {
            self.buffer[r as usize] |= 1 << (7 - c);
        }
    }

    fn unset_pixel(&mut self, r: u8, c: u8) {
        if r < 8 && c < 8 {
            self.buffer[r as usize] &= !(1 << (7 - c));
        }
    }

    fn draw_pixel(&mut self, r: u8, c: u8, on: bool) {
        if on {
            self.set_pixel(r, c);
        } else {
            self.unset_pixel(r, c);
        }
    }

    fn draw_line(&mut self, r1: i8, c1: i8, r2: i8, c2: i8, on: bool) {
        let dr = (r2 - r1).abs() as f32;
        let dc = (c2 - c1).abs() as f32;
        let steps = if dr >= dc { dr } else { dc };

        if steps == 0.0 {
            self.draw_pixel(r1 as u8, c1 as u8, on);
            return;
        }

        let rstep = (r2 - r1) as f32 / steps;
        let cstep = (c2 - c1) as f32 / steps;

        let mut r = r1 as f32;
        let mut c = c1 as f32;

        for _ in 0..=(steps as i32) {
            self.draw_pixel(libm::roundf(r) as u8, libm::roundf(c) as u8, on);
            r += rstep;
            c += cstep;
        }
    }

    fn draw_rectangle(&mut self, r1: u8, c1: u8, r2: u8, c2: u8, on: bool) {
        self.draw_line(r1 as i8, c1 as i8, r1 as i8, c2 as i8, on);
        self.draw_line(r2 as i8, c1 as i8, r2 as i8, c2 as i8, on);
        self.draw_line(r1 as i8, c1 as i8, r2 as i8, c1 as i8, on);
        self.draw_line(r1 as i8, c2 as i8, r2 as i8, c2 as i8, on);
    }

    fn draw_rectangle_filled(&mut self, r1: u8, c1: u8, r2: u8, c2: u8, on: bool) {
        for r in r1..=r2 {
            for c in c1..=c2 {
                self.draw_pixel(r, c, on);
            }
        }
    }

    fn draw_row(&mut self, r: u8, on: bool) {
        if r < 8 {
            self.buffer[r as usize] = if on { 255 } else { 0 };
        }
    }

    fn draw_column(&mut self, c: u8, on: bool) {
        for r in 0..8 {
            self.draw_pixel(r, c, on);
        }
    }

    fn shift_down(&mut self, on: bool) {
        for r in (1..8).rev() {
            self.buffer[r] = self.buffer[r - 1];
        }
        self.draw_row(0, on);
    }

    fn shift_up(&mut self, on: bool) {
        for r in 0..7 {
            self.buffer[r] = self.buffer[r + 1];
        }
        self.draw_row(7, on);
    }

    fn shift_left(&mut self, on: bool) {
        for r in 0..8 {
            self.buffer[r] <<= 1;
        }
        if on {
            self.draw_column(7, true);
        }
    }

    fn shift_right(&mut self, on: bool) {
        for r in 0..8 {
            self.buffer[r] >>= 1;
        }
        if on {
            self.draw_column(0, true);
        }
    }

    fn update_display<SPI: SpiDevice>(&self, display: &mut Max7219<SPI>) -> Result<(), SPI::Error> {
        display.draw_raw(&self.buffer)
    }
}

// --------------------------------------
// Animations
// --------------------------------------

fn run_blinking_smile<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: BlinkingSmile");
    let mut draw = Drawing::new();
    let frames = [
        [
            0b00000000, 0b01100110, 0b01100110, 0b00000000, 0b10000001, 0b01000010, 0b00111100,
            0b00000000,
        ],
        [
            0b00000000, 0b00000000, 0b01100110, 0b00000000, 0b10000001, 0b01000010, 0b00111100,
            0b00000000,
        ],
    ];
    let start = Instant::now();
    let mut current = 0;
    while start.elapsed().as_secs() < 5 {
        draw.buffer = frames[current];
        draw.update_display(display)?;
        current = (current + 1) % 2;
        delay.delay_ms(if current == 1 { 150 } else { 1200 });
    }
    Ok(())
}

fn run_bouncing_ball<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
    rng: &mut Lcg,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: BouncingBall");
    let mut draw = Drawing::new();
    let mut r: f32 = 3.5;
    let mut c: f32 = 3.5;
    let mut dr = (rng.random_range(5, 12) as f32) / 10.0;
    if rng.random_range(0, 1) == 0 {
        dr *= -1.0;
    }
    let mut dc = (rng.random_range(5, 12) as f32) / 10.0;
    if rng.random_range(0, 1) == 0 {
        dc *= -1.0;
    }

    let start = Instant::now();
    while start.elapsed().as_secs() < 5 {
        draw.clear_screen(false);
        draw.draw_pixel(libm::roundf(r) as u8 % 8, libm::roundf(c) as u8 % 8, true);
        draw.update_display(display)?;
        r += dr;
        c += dc;
        if r <= 0.0 || r >= 7.0 {
            dr *= -1.0;
            dr += (rng.random_range(-2, 2) as f32) / 10.0;
            if libm::fabsf(dr) < 0.5 {
                dr = if dr > 0.0 { 0.5 } else { -0.5 };
            }
            if libm::fabsf(dr) > 1.2 {
                dr = if dr > 0.0 { 1.2 } else { -1.2 };
            }
        }
        if c <= 0.0 || c >= 7.0 {
            dc *= -1.0;
            dc += (rng.random_range(-2, 2) as f32) / 10.0;
            if libm::fabsf(dc) < 0.5 {
                dc = if dc > 0.0 { 0.5 } else { -0.5 };
            }
            if libm::fabsf(dc) > 1.2 {
                dc = if dc > 0.0 { 1.2 } else { -1.2 };
            }
        }
        delay.delay_ms(80);
    }
    Ok(())
}

fn run_countdown<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: Countdown");
    let mut draw = Drawing::new();
    let digits = [
        [
            0b00111100, 0b01100110, 0b01100110, 0b01100110, 0b01100110, 0b01100110, 0b01100110,
            0b00111100,
        ], // 0
        [
            0b00011000, 0b00111000, 0b01011000, 0b00011000, 0b00011000, 0b00011000, 0b00011000,
            0b00111100,
        ], // 1
        [
            0b00111100, 0b01100110, 0b00000110, 0b00001100, 0b00011000, 0b00110000, 0b01100000,
            0b01111110,
        ], // 2
        [
            0b00111100, 0b01100110, 0b00000110, 0b00011100, 0b00000110, 0b00000110, 0b01100110,
            0b00111100,
        ], // 3
        [
            0b00001100, 0b00011100, 0b00101100, 0b01001100, 0b01111110, 0b00001100, 0b00001100,
            0b00001100,
        ], // 4
        [
            0b01111110, 0b01100000, 0b01111100, 0b00000110, 0b00000110, 0b00000110, 0b01100110,
            0b00111100,
        ], // 5
        [
            0b00111100, 0b01100110, 0b01100000, 0b01111100, 0b01100110, 0b01100110, 0b01100110,
            0b00111100,
        ], // 6
        [
            0b01111110, 0b01100110, 0b00000110, 0b00001100, 0b00011000, 0b00110000, 0b01100000,
            0b01100000,
        ], // 7
        [
            0b00111100, 0b01100110, 0b01100110, 0b00111100, 0b01100110, 0b01100110, 0b01100110,
            0b00111100,
        ], // 8
        [
            0b00111100, 0b01100110, 0b01100110, 0b01100110, 0b00111110, 0b00000110, 0b01100110,
            0b00111100,
        ], // 9
    ];
    let start = Instant::now();
    let mut current = 9;
    while start.elapsed().as_secs() < 5 {
        draw.buffer = digits[current as usize];
        draw.update_display(display)?;
        if current > 0 {
            current -= 1;
        } else {
            current = 9;
        }
        delay.delay_ms(500);
    }
    Ok(())
}

fn run_filled_rectangles<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
    rng: &mut Lcg,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: FilledRectangles");
    let mut draw = Drawing::new();
    let start = Instant::now();
    while start.elapsed().as_secs() < 5 {
        let r1 = rng.random_range(0, 7) as u8;
        let c1 = rng.random_range(0, 7) as u8;
        let r2 = rng.random_range(0, 7) as u8;
        let c2 = rng.random_range(0, 7) as u8;
        draw.clear_screen(false);
        draw.draw_rectangle_filled(r1.min(r2), c1.min(c2), r1.max(r2), c1.max(c2), true);
        draw.update_display(display)?;
        delay.delay_ms(500);
    }
    Ok(())
}

fn run_game_of_life<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
    rng: &mut Lcg,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: GameOfLife");
    let mut draw = Drawing::new();
    for r in &mut draw.buffer {
        *r = rng.next() as u8;
    }
    let start = Instant::now();
    while start.elapsed().as_secs() < 5 {
        draw.update_display(display)?;
        delay.delay_ms(200);
        let mut next = [0u8; 8];
        for (r, row_val) in next.iter_mut().enumerate() {
            for c in 0..8 {
                let mut nb = 0;
                for dr in -1..=1 {
                    for dc in -1..=1 {
                        if dr == 0 && dc == 0 {
                            continue;
                        }
                        let nr = ((r as i8 + dr + 8) % 8) as usize;
                        let nc = ((c as i8 + dc + 8) % 8) as usize;
                        if (draw.buffer[nr] & (1 << (7 - nc))) != 0 {
                            nb += 1;
                        }
                    }
                }
                let alive = (draw.buffer[r] & (1 << (7 - c))) != 0;
                if nb == 3 || (alive && nb == 2) {
                    *row_val |= 1 << (7 - c);
                }
            }
        }
        if next == draw.buffer || next == [0; 8] {
            for r in &mut next {
                *r = rng.next() as u8;
            }
        }
        draw.buffer = next;
    }
    Ok(())
}

fn run_heartbeat<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: Heartbeat");
    let mut draw = Drawing::new();
    let frames = [
        [
            0b00000000, 0b00000000, 0b00100100, 0b01111110, 0b00111100, 0b00011000, 0b00000000,
            0b00000000,
        ],
        [
            0b00000000, 0b01100110, 0b11111111, 0b11111111, 0b01111110, 0b00111100, 0b00011000,
            0b00000000,
        ],
    ];
    let start = Instant::now();
    let mut current = 0;
    while start.elapsed().as_secs() < 5 {
        draw.buffer = frames[current];
        draw.update_display(display)?;
        current = (current + 1) % 2;
        delay.delay_ms(if current == 1 { 150 } else { 600 });
    }
    Ok(())
}

fn run_human_walk<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: HumanWalk");
    let mut draw = Drawing::new();
    let frames = [
        [
            0b00011000, 0b00011000, 0b00001000, 0b00111100, 0b01001010, 0b00001000, 0b00010100,
            0b00100010,
        ],
        [
            0b00011000, 0b00011000, 0b00001000, 0b00011100, 0b00001000, 0b00001000, 0b00001000,
            0b00010100,
        ],
        [
            0b00011000, 0b00011000, 0b00001000, 0b00111100, 0b00101010, 0b00001000, 0b00010100,
            0b00001010,
        ],
        [
            0b00011000, 0b00011000, 0b00001000, 0b00011100, 0b00001000, 0b00001000, 0b00001000,
            0b00010100,
        ],
    ];
    let start = Instant::now();
    let mut current = 0;
    while start.elapsed().as_secs() < 5 {
        draw.buffer = frames[current];
        draw.update_display(display)?;
        current = (current + 1) % 4;
        delay.delay_ms(250);
    }
    Ok(())
}

fn run_line_path<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
    rng: &mut Lcg,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: LinePath");
    let mut draw = Drawing::new();
    let mut r = 3;
    let mut c = 3;
    let mut count = 0;
    let start = Instant::now();
    while start.elapsed().as_secs() < 5 {
        let nr = rng.random_range(0, 7) as i8;
        let nc = rng.random_range(0, 7) as i8;
        draw.draw_line(r, c, nr, nc, true);
        draw.update_display(display)?;
        r = nr;
        c = nc;
        count += 1;
        if count == 4 {
            count = 0;
            delay.delay_ms(500);
            draw.clear_screen(false);
        } else {
            delay.delay_ms(500);
        }
    }
    Ok(())
}

fn run_matrix_waterfall<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
    rng: &mut Lcg,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: MatrixWaterfall");
    let mut draw = Drawing::new();
    let start = Instant::now();
    while start.elapsed().as_secs() < 5 {
        draw.shift_down(false);
        draw.set_pixel(0, rng.random_range(0, 7) as u8);
        draw.update_display(display)?;
        delay.delay_ms(100);
    }
    Ok(())
}

fn run_matrix_waterfall_reversing<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
    rng: &mut Lcg,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: MatrixWaterfallReversing");
    let mut draw = Drawing::new();
    let mut state: f32 = 1.0;
    let mut step: f32 = -1.0 / 64.0;
    let w = core::f32::consts::PI / 2.0;
    let start = Instant::now();
    while start.elapsed().as_secs() < 5 {
        let row = if state >= 0.0 {
            draw.shift_down(false);
            0
        } else {
            draw.shift_up(false);
            7
        };
        for _ in 0..2 {
            draw.set_pixel(row, rng.random_range(0, 7) as u8);
        }
        draw.update_display(display)?;
        state += step;
        if state >= 1.0 || state <= -1.0 {
            step *= -1.0;
        }
        let s = libm::sinf(w * state);
        delay.delay_ms((50.0 + 300.0 * (1.0 - s * s)) as u32);
    }
    Ok(())
}

fn run_moving_columns<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: MovingColumns");
    let mut draw = Drawing::new();
    let mut col: i8 = 0;
    let mut dir: i8 = 1;
    let start = Instant::now();
    while start.elapsed().as_secs() < 5 {
        draw.clear_screen(false);
        draw.draw_column(col as u8, true);
        draw.update_display(display)?;
        if col == 0 {
            dir = 1;
        } else if col == 7 {
            dir = -1;
        }
        col += dir;
        delay.delay_ms(50);
    }
    Ok(())
}

fn run_pacman<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: PacMan");
    let mut draw = Drawing::new();
    let frames = [
        [
            0b00111100, 0b01111110, 0b11100111, 0b11111000, 0b11111000, 0b11111111, 0b01111110,
            0b00111100,
        ],
        [
            0b00111100, 0b01111110, 0b11100111, 0b11111111, 0b11111111, 0b11111111, 0b01111110,
            0b00111100,
        ],
    ];
    let start = Instant::now();
    let mut current = 0;
    while start.elapsed().as_secs() < 5 {
        draw.buffer = frames[current];
        draw.update_display(display)?;
        current = (current + 1) % 2;
        delay.delay_ms(200);
    }
    Ok(())
}

fn run_pong_game<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
    rng: &mut Lcg,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: PongGame");
    let mut draw = Drawing::new();
    let mut br: f32 = 3.0;
    let mut bc: f32 = 3.0;
    let mut dr: f32 = 0.5;
    let mut dc: f32 = 1.0;
    let mut p1r: i8 = 3;
    let mut p2r: i8 = 4;
    let start = Instant::now();
    while start.elapsed().as_secs() < 5 {
        draw.clear_screen(false);
        draw.draw_pixel(p1r as u8, 0, true);
        draw.draw_pixel((p1r + 1) as u8, 0, true);
        draw.draw_pixel(p2r as u8, 7, true);
        draw.draw_pixel((p2r + 1) as u8, 7, true);
        let cr = libm::roundf(br) as i8;
        let cc = libm::roundf(bc) as i8;
        if cr >= 0 && cc >= 0 {
            draw.draw_pixel(cr as u8, cc as u8, true);
        }
        br += dr;
        bc += dc;
        if br <= 0.0 || br >= 7.0 {
            dr *= -1.0;
            br = br.clamp(0.0, 7.0);
        }
        let abs_dr = libm::fabsf(dr);
        if bc <= 1.0 {
            dc *= -1.0;
            bc = 1.0;
            if dr > 0.0 && p1r < 6 {
                p1r += 1;
            }
            if dr < 0.0 && p1r > 0 {
                p1r -= 1;
            }
            dr = (dr + (rng.random_range(-10, 10) as f32) / 20.0).clamp(-1.0, 1.0);
            if abs_dr < 0.2 {
                dr = if dr > 0.0 { 0.3 } else { -0.3 };
            }
        } else if bc >= 6.0 {
            dc *= -1.0;
            bc = 6.0;
            if dr > 0.0 && p2r < 6 {
                p2r += 1;
            }
            if dr < 0.0 && p2r > 0 {
                p2r -= 1;
            }
            dr = (dr + (rng.random_range(-10, 10) as f32) / 20.0).clamp(-1.0, 1.0);
            if abs_dr < 0.2 {
                dr = if dr > 0.0 { 0.3 } else { -0.3 };
            }
        } else {
            if dc < 0.0 && rng.random_range(0, 10) > 5 {
                if cr > p1r + 1 && p1r < 6 {
                    p1r += 1;
                }
                if cr < p1r && p1r > 0 {
                    p1r -= 1;
                }
            }
            if dc > 0.0 && rng.random_range(0, 10) > 5 {
                if cr > p2r + 1 && p2r < 6 {
                    p2r += 1;
                }
                if cr < p2r && p2r > 0 {
                    p2r -= 1;
                }
            }
        }
        draw.update_display(display)?;
        delay.delay_ms(80);
    }
    Ok(())
}

fn run_rectangles<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
    rng: &mut Lcg,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: Rectangles");
    let mut draw = Drawing::new();
    let start = Instant::now();
    while start.elapsed().as_secs() < 5 {
        let r1 = rng.random_range(0, 7) as u8;
        let c1 = rng.random_range(0, 7) as u8;
        let r2 = rng.random_range(0, 7) as u8;
        let c2 = rng.random_range(0, 7) as u8;
        draw.clear_screen(false);
        draw.draw_rectangle(r1.min(r2), c1.min(c2), r1.max(r2), c1.max(c2), true);
        draw.update_display(display)?;
        delay.delay_ms(500);
    }
    Ok(())
}

fn run_ripple_drop<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: RippleDrop");
    let mut draw = Drawing::new();
    let frames = [
        [
            0b00000000, 0b00000000, 0b00010000, 0b00010000, 0b00000000, 0b00000000, 0b00000000,
            0b00010000,
        ],
        [
            0b00000000, 0b00000000, 0b00000000, 0b00010000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000,
        ],
        [
            0b00000000, 0b00000000, 0b00000000, 0b00011000, 0b00011000, 0b00000000, 0b00000000,
            0b00000000,
        ],
        [
            0b00000000, 0b00000000, 0b00111100, 0b00100100, 0b00100100, 0b00111100, 0b00000000,
            0b00000000,
        ],
        [
            0b00000000, 0b01111110, 0b01000010, 0b01000010, 0b01000010, 0b01000010, 0b01111110,
            0b00000000,
        ],
        [
            0b11111111, 0b10000001, 0b10000001, 0b10000001, 0b10000001, 0b10000001, 0b10000001,
            0b11111111,
        ],
    ];
    let start = Instant::now();
    let mut current = 0;
    while start.elapsed().as_secs() < 5 {
        draw.buffer = frames[current];
        draw.update_display(display)?;
        current = (current + 1) % 6;
        let interval = if current == 0 { 600 } else { 120 };
        delay.delay_ms(interval);
    }
    Ok(())
}

fn run_sine_wave<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: SineWave");
    let mut draw = Drawing::new();
    let mut offset: f32 = 0.0;
    let start = Instant::now();
    while start.elapsed().as_secs() < 5 {
        draw.clear_screen(false);
        for c in 0..8 {
            let angle = (c as f32 + offset) * 0.785;
            let r = (3.0 + libm::roundf(libm::sinf(angle) * 3.0)) as i8;
            draw.draw_pixel(r.clamp(0, 7) as u8, c as u8, true);
        }
        draw.update_display(display)?;
        offset += 1.0;
        delay.delay_ms(100);
    }
    Ok(())
}

fn run_single_pixel_scanner<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: SinglePixelScanner");
    let mut draw = Drawing::new();
    let rstep: [[i8; 8]; 8] = [
        [0, 0, 0, 0, 0, 0, 0, 1],
        [0, 0, 0, 0, 0, 0, 1, 1],
        [-1, 0, 0, 0, 0, 1, 1, 1],
        [-1, -1, 0, 0, 1, 1, 1, 1],
        [-1, -1, -1, -4, 0, 1, 1, 1],
        [-1, -1, -1, 0, 0, 0, 1, 1],
        [-1, -1, 0, 0, 0, 0, 0, 1],
        [-1, 0, 0, 0, 0, 0, 0, 0],
    ];
    let cstep: [[i8; 8]; 8] = [
        [1, 1, 1, 1, 1, 1, 1, 0],
        [1, 1, 1, 1, 1, 1, 0, 0],
        [0, 1, 1, 1, 1, 0, 0, 0],
        [0, 0, 1, 1, 0, 0, 0, 0],
        [0, 0, 0, -3, -1, 0, 0, 0],
        [0, 0, 0, -1, -1, -1, 0, 0],
        [0, 0, -1, -1, -1, -1, -1, 0],
        [0, -1, -1, -1, -1, -1, -1, -1],
    ];
    let toggle: [[u8; 8]; 8] = [
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 1, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
    ];
    let mut row: i8 = 0;
    let mut col: i8 = 0;
    let mut color = true;
    let start = Instant::now();
    while start.elapsed().as_secs() < 5 {
        draw.draw_pixel(row as u8, col as u8, color);
        draw.update_display(display)?;
        let dr = rstep[row as usize][col as usize];
        let dc = cstep[row as usize][col as usize];
        if toggle[row as usize][col as usize] != 0 {
            color = !color;
        }
        row += dr;
        col += dc;
        delay.delay_ms(16);
    }
    Ok(())
}

fn run_space_invader<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: SpaceInvader");
    let mut draw = Drawing::new();
    let frames = [
        [
            0b00011000, 0b00111100, 0b01111110, 0b11011011, 0b11111111, 0b00100100, 0b01011010,
            0b10000001,
        ],
        [
            0b00011000, 0b00111100, 0b01111110, 0b11011011, 0b11111111, 0b00100100, 0b01000010,
            0b00100100,
        ],
    ];
    let start = Instant::now();
    let mut current = 0;
    while start.elapsed().as_secs() < 5 {
        draw.buffer = frames[current];
        draw.update_display(display)?;
        current = (current + 1) % 2;
        delay.delay_ms(500);
    }
    Ok(())
}

fn run_spinning_lines<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: SpinningLines");
    let mut draw = Drawing::new();
    let mut j = 0;
    let start = Instant::now();
    while start.elapsed().as_secs() < 5 {
        draw.clear_screen(false);
        let i = j % 8;
        if j < 8 {
            draw.draw_line(i as i8, 0, 7 - i as i8, 7, true);
        } else {
            draw.draw_line(0, 7 - i as i8, 7, i as i8, true);
        }
        draw.update_display(display)?;
        j = (j + 1) % 16;
        delay.delay_ms(50);
    }
    Ok(())
}

fn run_windy_particles<SPI: SpiDevice>(
    delay: &mut Delay,
    display: &mut Max7219<SPI>,
    rng: &mut Lcg,
) -> Result<(), SPI::Error> {
    defmt::info!("Running Demo: WindyParticles");
    let mut draw = Drawing::new();
    let mut dir = true;
    let mut period = 0;
    let start = Instant::now();
    while start.elapsed().as_secs() < 5 {
        if dir {
            draw.shift_left(false);
            draw.set_pixel(rng.random_range(0, 7) as u8, 7);
            draw.set_pixel(rng.random_range(0, 7) as u8, 7);
        } else {
            draw.shift_right(false);
            draw.set_pixel(rng.random_range(0, 7) as u8, 0);
            draw.set_pixel(rng.random_range(0, 7) as u8, 0);
        }
        draw.update_display(display)?;
        period = (period + 1) % 32;
        if period == 0 {
            dir = !dir;
        }
        delay.delay_ms(100);
    }
    Ok(())
}

#[main]
fn main() -> ! {
    rtt_target::rtt_init_defmt!();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let cs = Output::new(peripherals.GPIO5, Level::High, OutputConfig::default());
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(10))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO6)
    .with_mosi(peripherals.GPIO7);

    let spi_device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    let mut display = Max7219::new(spi_device);
    display.init().unwrap();
    display.set_intensity(0x07).unwrap();

    let mut delay = Delay::new();
    let mut rng = Lcg::new(42);

    loop {
        let _ = run_blinking_smile(&mut delay, &mut display);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_bouncing_ball(&mut delay, &mut display, &mut rng);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_countdown(&mut delay, &mut display);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_filled_rectangles(&mut delay, &mut display, &mut rng);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_game_of_life(&mut delay, &mut display, &mut rng);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_heartbeat(&mut delay, &mut display);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_human_walk(&mut delay, &mut display);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_line_path(&mut delay, &mut display, &mut rng);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_matrix_waterfall(&mut delay, &mut display, &mut rng);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_matrix_waterfall_reversing(&mut delay, &mut display, &mut rng);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_moving_columns(&mut delay, &mut display);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_pacman(&mut delay, &mut display);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_pong_game(&mut delay, &mut display, &mut rng);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_rectangles(&mut delay, &mut display, &mut rng);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_ripple_drop(&mut delay, &mut display);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_sine_wave(&mut delay, &mut display);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_single_pixel_scanner(&mut delay, &mut display);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_space_invader(&mut delay, &mut display);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_spinning_lines(&mut delay, &mut display);
        display.clear().unwrap();
        delay.delay_ms(2000);

        let _ = run_windy_particles(&mut delay, &mut display, &mut rng);
        display.clear().unwrap();
        delay.delay_ms(2000);
    }
}
