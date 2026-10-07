#![no_std]
#![no_main]

use core::fmt;

use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::i2c::I2c;
use embassy_rp::peripherals::{I2C0, PIN_0, PIN_1, PIN_25, USB};
use embassy_rp::{Peri, bind_interrupts, i2c, usb};

use embassy_time::Timer;
use log::info;
use panic_halt as _;

type P<T> = Peri<'static, T>;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
});

#[embassy_executor::task]
pub async fn blink(pin: P<PIN_25>) -> ! {
    let mut led = Output::new(pin, Level::Low);
    loop {
        led.toggle();
        Timer::after_millis(100).await;
        led.toggle();
        Timer::after_millis(900).await;
    }
}

#[embassy_executor::task]
async fn logger_task(pin: P<USB>) -> ! {
    let driver = usb::Driver::new(pin, Irqs);
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

fn log_err<T: fmt::Debug>(res: Result<(), T>) {
    if let Err(err) = res {
        info!("error: {err:?}");
    }
}

async fn get_byte(i2c: &mut I2c<'_, I2C0, i2c::Async>, address: u8) -> u8 {
    let mut byte = [0; 1];
    log_err(
        i2c.write_read_async(acc::ACCEL_ADDR, [address], &mut byte)
            .await,
    );
    byte[0]
}

async fn get_accel(i2c: &mut I2c<'_, I2C0, i2c::Async>) -> (f32, f32, f32) {
    let mut raw = [0; 6];
    log_err(
        i2c.write_read_async(acc::ACCEL_ADDR, [acc::OUT_X_L], &mut raw)
            .await,
    );
    (
        i16::from_le_bytes([raw[0], raw[1]]) as f32 / acc::ACCELEROMETER_SCALING_FACTOR,
        i16::from_le_bytes([raw[2], raw[3]]) as f32 / acc::ACCELEROMETER_SCALING_FACTOR,
        i16::from_le_bytes([raw[4], raw[5]]) as f32 / acc::ACCELEROMETER_SCALING_FACTOR,
    )
}

fn col(x: f32) -> &'static str {
    let x = x.abs();
    if x > 1. {
        "\x1b[31m"
    } else if x > 0.5 {
        "\x1b[32m"
    } else {
        "\x1b[0m"
    }
}

#[embassy_executor::task]
async fn accelerometer(peri: P<I2C0>, scl: P<PIN_1>, sda: P<PIN_0>) -> ! {
    let mut i2c = i2c::I2c::new_async(peri, scl, sda, Irqs, i2c::Config::default());
    let who = get_byte(&mut i2c, acc::WHO_AM_I).await;
    info!("who: 0x{:02x}", who);
    assert!(who == 0x41);

    log_err(
        i2c.write_async(acc::ACCEL_ADDR, [acc::CTRL_REG1, acc::CTRL_REG1_VAL])
            .await,
    );
    log_err(
        i2c.write_async(acc::ACCEL_ADDR, [acc::CTRL_REG4, acc::CTRL_REG4_VAL])
            .await,
    );

    let reg1 = get_byte(&mut i2c, acc::CTRL_REG1).await;
    info!("reg1: 0x{:02x}", reg1);
    assert!(reg1 == acc::CTRL_REG1_VAL);

    let reg4 = get_byte(&mut i2c, acc::CTRL_REG4).await;
    info!("reg4: 0x{:02x}", reg4);
    assert!(reg4 == acc::CTRL_REG4_VAL);

    let (mut x, mut y, mut z) = get_accel(&mut i2c).await;

    loop {
        Timer::after_millis(100).await;
        let (new_x, new_y, new_z) = get_accel(&mut i2c).await;
        info!(
            "{}{:+.2} {}{:+.2} {}{:+.2}\r",
            col(new_x - x),
            new_x,
            col(new_y - y),
            new_y,
            col(new_z - z),
            new_z
        );
        (x, y, z) = (new_x, new_y, new_z);
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    spawner.spawn(blink(p.PIN_25).unwrap());
    spawner.spawn(logger_task(p.USB).unwrap());
    Timer::after_secs(1).await; // wait for flash.sh to find device
    info!("setup");
    spawner.spawn(accelerometer(p.I2C0, p.PIN_1, p.PIN_0).unwrap());
}

mod acc {
    pub const ACCEL_ADDR: u16 = 0x1E; // SA0 low
    pub const WHO_AM_I: u8 = 0x0F;
    pub const CTRL_REG1: u8 = 0x20;
    pub const CTRL_REG1_VAL: u8 = 0b00011111; // 10Hz, Normal Mode, XYZ enabled
    pub const CTRL_REG4: u8 = 0x23;
    pub const CTRL_REG4_VAL: u8 = 0b00110100; // 8G, ya pas 16G sur cet accéléromètre
    pub const OUT_X_L: u8 = 0x28;
    pub const ACCELEROMETER_SCALING_FACTOR: f32 = 4096.; // Valeur pour 8G
}
