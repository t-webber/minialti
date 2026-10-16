#![no_std]

use core::fmt;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::i2c::I2c;
use embassy_rp::peripherals::{DMA_CH0, I2C0, PIN_0, PIN_1, PIN_25, USB};
use embassy_rp::{Peri, bind_interrupts, dma, i2c, usb};
use embassy_time::Timer;
use log::error;
use panic_halt as _;

pub type P<T> = Peri<'static, T>;

bind_interrupts!(pub struct Irqs {
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
});

pub fn log_err<T: fmt::Debug>(err: &T) {
    error!("\x1b[31merror: {err:?}\x1b[0m");
}

#[embassy_executor::task]
pub async fn blink_task(pin: P<PIN_25>) -> ! {
    let mut led = Output::new(pin, Level::Low);
    loop {
        // 0.1s ON, 1.9s OFF pour préserver la diode
        // utilisée comme témoin de fonctionnement
        led.toggle();
        Timer::after_millis(100).await;
        led.toggle();
        Timer::after_millis(1900).await;
    }
}

#[embassy_executor::task]
pub async fn logger_task(pin: P<USB>) -> ! {
    let driver = usb::Driver::new(pin, Irqs);
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

pub async fn get_accel_raw(i2c: &mut I2c<'_, I2C0, i2c::Async>) -> [u8; 6] {
    let mut raw = [0; 6];
    i2c.write_read_async(acc::ACCEL_ADDR, [acc::OUT_X_L], &mut raw)
        .await
        .inspect_err(log_err)
        .ok();
    raw
}

pub async fn setup_accel(
    peri: P<I2C0>,
    scl: P<PIN_1>,
    sda: P<PIN_0>,
) -> I2c<'static, I2C0, i2c::Async> {
    let mut i2c = i2c::I2c::new_async(peri, scl, sda, Irqs, i2c::Config::default());
    i2c.write_async(acc::ACCEL_ADDR, [acc::CTRL_REG1, acc::CTRL_REG1_VAL])
        .await
        .inspect_err(log_err)
        .ok();
    i2c.write_async(acc::ACCEL_ADDR, [acc::CTRL_REG4, acc::CTRL_REG4_VAL])
        .await
        .inspect_err(log_err)
        .ok();
    i2c
}

pub fn parse_accel_values(raw: [u8; 6]) -> (f32, f32, f32) {
    let dec = |bytes: [u8; 2]| i16::from_le_bytes(bytes) as f32 / acc::SCALING_FACTOR;
    (
        dec([raw[0], raw[1]]),
        dec([raw[2], raw[3]]),
        dec([raw[4], raw[5]]),
    )
}

pub mod acc {
    pub const ACCEL_ADDR: u16 = 0x1E; // SA0 low
    pub const WHO_AM_I: u8 = 0x0F;
    pub const CTRL_REG1: u8 = 0x20;
    pub const CTRL_REG1_VAL: u8 = 0b00011111; // 10Hz, Normal Mode, XYZ enabled
    pub const CTRL_REG4: u8 = 0x23;
    pub const CTRL_REG4_VAL: u8 = 0b00110100; // 8G, ya pas 16G sur cet accéléromètre
    pub const OUT_X_L: u8 = 0x28;
    pub const SCALING_FACTOR: f32 = 4096.; // Valeur pour 8G
}

pub mod mem {
    pub const ADDR: u32 = 256 * 1024; // 256KB for firmware
    pub const SIZE: usize = 2 * 1024 * 1024; // 16 MB = 2 Mb
}
