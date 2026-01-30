//! OLED display module for SSD1306
//!
//! Following pins are used:
//! SDA     GPIO5
//! SCL     GPIO6
//!
//! I2C address: 0x3c

use anyhow::Result;
use esp_idf_hal::{
    delay::FreeRtos,
    i2c::{I2C0, *},
    gpio::{Gpio5, Gpio6},
    units::*,
};
use log::*;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use ssd1306::{prelude::*, Ssd1306, mode::BufferedGraphicsMode};
use std::sync::Mutex;

const SSD1306_ADDRESS: u8 = 0x3c;

/// OLED display wrapper for thread-safe access
/// Supports both 128x64 and 72x40 displays
pub struct OledDisplay {
    display: Mutex<DisplayType>,
}

enum DisplayType {
    #[allow(dead_code)] // Available for future use with 128x64 displays
    Size128x64(Ssd1306<I2CInterface<&'static mut I2cDriver<'static>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>),
    Size72x40(Ssd1306<I2CInterface<&'static mut I2cDriver<'static>>, DisplaySize72x40, BufferedGraphicsMode<DisplaySize72x40>>),
}

impl OledDisplay {
    /// Initialize the OLED display
    pub fn init(i2c: I2C0, sda: Gpio5, scl: Gpio6) -> Result<Self> {
        info!("Starting I2C SSD1306 initialization");

        info!("I2C address: 0x{:02x}", SSD1306_ADDRESS);

        let config = I2cConfig::new().baudrate(100.kHz().into());
        let i2c_driver = I2cDriver::new(i2c, sda, scl, &config)?;

        info!("Creating I2C display interface...");
        // I2CInterface::new takes (i2c, address, data_byte)
        // data_byte is typically 0x40 for data commands
        // Make the driver static by leaking it (it will live for the lifetime of the program)
        let i2c_driver = Box::leak(Box::new(i2c_driver));
        let interface = I2CInterface::new(i2c_driver, SSD1306_ADDRESS, 0x40);

        // Initialize for 72x40 display
        info!("Initializing SSD1306 display (72x40)...");
        let mut display = Ssd1306::new(
            interface,
            DisplaySize72x40,
            DisplayRotation::Rotate0,
        )
        .into_buffered_graphics_mode();

        info!("Calling display.init()...");
        display.init().map_err(|e| {
            error!("Display init failed: {:?}", e);
            error!("Check I2C wiring (SDA=GPIO5, SCL=GPIO6) and address (try 0x3d if 0x3c doesn't work)");
            anyhow::anyhow!("Display init error: {:?}", e)
        })?;
        
        info!("Display initialized successfully with 72x40!");
        
        // Give display time to stabilize
        FreeRtos::delay_ms(200);

        // Test the display with a simple pattern
        info!("Testing display with pattern...");
        display.clear(BinaryColor::Off).map_err(|e| anyhow::anyhow!("Clear error: {:?}", e))?;
        
        // Draw a test pattern to verify display works
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();
        
        Text::with_baseline("OLED Test", Point::new(0, 5), text_style, Baseline::Top)
            .draw(&mut display)
            .map_err(|_| anyhow::anyhow!("Test draw error"))?;
        
        display.flush().map_err(|e| anyhow::anyhow!("Flush error: {:?}", e))?;
        info!("Display test pattern drawn successfully");
        
        // Clear after test and show ready message
        FreeRtos::delay_ms(1000);
        display.clear(BinaryColor::Off).map_err(|e| anyhow::anyhow!("Clear error: {:?}", e))?;
        
        // Show initial ready message (adjusted for 72x40 - max 4 lines, ~12 chars per line)
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();
        
        Text::with_baseline("Server", Point::new(0, 5), text_style, Baseline::Top)
            .draw(&mut display)
            .map_err(|_| anyhow::anyhow!("Initial message draw error"))?;
        Text::with_baseline("Ready!", Point::new(0, 15), text_style, Baseline::Top)
            .draw(&mut display)
            .map_err(|_| anyhow::anyhow!("Initial message draw error"))?;
        Text::with_baseline("Waiting...", Point::new(0, 25), text_style, Baseline::Top)
            .draw(&mut display)
            .map_err(|_| anyhow::anyhow!("Initial message draw error"))?;
        
        display.flush().map_err(|e| anyhow::anyhow!("Flush error: {:?}", e))?;
        info!("Initial ready message displayed");
        
        Ok(Self {
            display: Mutex::new(DisplayType::Size72x40(display)),
        })
    }

    /// Display a message on the OLED screen
    /// Messages are wrapped to fit on multiple lines if needed
    pub fn display_message(&self, message: &str) -> Result<()> {
        let mut display_guard = self.display.lock().unwrap();

        // Create text style
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

        // Clear and draw based on display type
        match *display_guard {
            DisplayType::Size128x64(ref mut display) => {
                info!("Clearing 128x64 display...");
                display.clear(BinaryColor::Off).map_err(|e| anyhow::anyhow!("Clear error: {:?}", e))?;
                info!("Drawing text on 128x64 display...");
                self.draw_text_128x64(display, message, &text_style)?;
                info!("Flushing 128x64 display...");
                display.flush().map_err(|e| anyhow::anyhow!("Flush error: {:?}", e))?;
                info!("128x64 display flushed successfully");
            }
            DisplayType::Size72x40(ref mut display) => {
                info!("Clearing 72x40 display...");
                display.clear(BinaryColor::Off).map_err(|e| anyhow::anyhow!("Clear error: {:?}", e))?;
                info!("Drawing text on 72x40 display...");
                self.draw_text_72x40(display, message, &text_style)?;
                info!("Flushing 72x40 display...");
                display.flush().map_err(|e| anyhow::anyhow!("Flush error: {:?}", e))?;
                info!("72x40 display flushed successfully");
            }
        }

        info!("Display updated with message: {}", message);
        Ok(())
    }
    
    fn draw_text_128x64<D: DrawTarget<Color = BinaryColor>>(
        &self,
        display: &mut D,
        message: &str,
        text_style: &MonoTextStyle<'_, BinaryColor>,
    ) -> Result<()> {
        const CHARS_PER_LINE: usize = 20;
        const MAX_LINES: usize = 6;
        const LINE_HEIGHT: i32 = 10;

        let lines = self.wrap_text(message, CHARS_PER_LINE, MAX_LINES);

        for (i, line) in lines.iter().enumerate() {
            let y_pos = (i as i32 * LINE_HEIGHT) + 5;
            if y_pos + LINE_HEIGHT <= 64 {
                Text::with_baseline(line, Point::new(0, y_pos), *text_style, Baseline::Top)
                    .draw(display)
                    .map_err(|_| anyhow::anyhow!("Text draw error"))?;
            }
        }
        Ok(())
    }
    
    fn draw_text_72x40<D: DrawTarget<Color = BinaryColor>>(
        &self,
        display: &mut D,
        message: &str,
        text_style: &MonoTextStyle<'_, BinaryColor>,
    ) -> Result<()> {
        const CHARS_PER_LINE: usize = 12;
        const MAX_LINES: usize = 4;
        const LINE_HEIGHT: i32 = 10;

        let lines = self.wrap_text(message, CHARS_PER_LINE, MAX_LINES);

        for (i, line) in lines.iter().enumerate() {
            let y_pos = (i as i32 * LINE_HEIGHT) + 5;
            if y_pos + LINE_HEIGHT <= 40 {
                Text::with_baseline(line, Point::new(0, y_pos), *text_style, Baseline::Top)
                    .draw(display)
                    .map_err(|_| anyhow::anyhow!("Text draw error"))?;
            }
        }
        Ok(())
    }
    
    fn wrap_text(&self, message: &str, chars_per_line: usize, max_lines: usize) -> Vec<String> {
        let words: Vec<&str> = message.split_whitespace().collect();
        let mut lines: Vec<String> = Vec::new();
        let mut current_line = String::new();

        for word in words {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if (current_line.len() + word.len() + 1) <= chars_per_line {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.len() > max_lines {
            lines.truncate(max_lines);
        }
        lines

    }

    /// Display a welcome message
    /// This can be called to refresh the welcome message if needed
    pub fn display_welcome(&self) -> Result<()> {
        // The initial message is already shown during init, but we can update it
        self.display_message("Server Ready Waiting...")
    }
}

