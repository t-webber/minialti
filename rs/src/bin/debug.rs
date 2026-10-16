#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::flash::{self, Flash};
use embassy_rp::i2c;
use embassy_rp::i2c::I2c;
use embassy_rp::peripherals::{DMA_CH0, FLASH, I2C0, PIN_0, PIN_1};
use embassy_time::Timer;
use log::info;
use rpi::{
    Irqs, P, acc, blink_task, get_accel_raw, log_err, logger_task, mem, parse_accel_values,
    setup_accel,
};

async fn get_byte(i2c: &mut I2c<'_, I2C0, i2c::Async>, address: u8) -> u8 {
    let mut byte = [0; 1];
    i2c.write_read_async(acc::ACCEL_ADDR, [address], &mut byte)
        .await
        .inspect_err(log_err)
        .ok();
    byte[0]
}

async fn get_accel(i2c: &mut I2c<'_, I2C0, i2c::Async>) -> (f32, f32, f32) {
    let raw = get_accel_raw(i2c).await;
    parse_accel_values(&raw)
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
async fn log_accel_task(peri: P<I2C0>, scl: P<PIN_1>, sda: P<PIN_0>) -> ! {
    let mut i2c = setup_accel(peri, scl, sda).await;

    let who = get_byte(&mut i2c, acc::WHO_AM_I).await;
    info!("who: 0x{:02x}", who);
    assert!(who == 0x41);

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

#[embassy_executor::task]
async fn flash_saver(flash: P<FLASH>, dma: P<DMA_CH0>) {
    let mut flash = Flash::<_, _, { mem::SIZE }>::new(flash, dma, Irqs);

    info!("jedec id: 0x{:x}", flash.blocking_jedec_id().unwrap());
    info!("capacity: 0x{:x}", flash.capacity());

    for (from, to) in [
        (mem::ADDR, mem::ADDR + flash::ERASE_SIZE as u32),
        ((mem::SIZE - flash::ERASE_SIZE) as u32, mem::SIZE as u32),
    ] {
        flash.blocking_erase(from, to).inspect_err(log_err).ok();
    }
    info!("blocks erased!");

    const SIZE: usize = 1024;
    const DATA: [u8; SIZE] = [51; SIZE];
    for from in [mem::ADDR, mem::SIZE as u32 - SIZE as u32] {
        flash.blocking_write(from, &DATA).inspect_err(log_err).ok();
    }
    info!("blocks written!");

    for from in [mem::ADDR, mem::SIZE as u32 - SIZE as u32] {
        let mut buf = [0; SIZE];
        flash.blocking_read(from, &mut buf).unwrap();
        info!("@read: {:?}..{:?}", &buf[..5], &buf[SIZE - 5..]);
        info!("@read valid: {}", buf == DATA);
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    spawner.spawn(blink_task(p.PIN_25).unwrap());
    spawner.spawn(logger_task(p.USB).unwrap());
    Timer::after_secs(1).await; // wait for flash.sh to find device
    info!("setup");
    spawner.spawn(log_accel_task(p.I2C0, p.PIN_1, p.PIN_0).unwrap());
    spawner.spawn(flash_saver(p.FLASH, p.DMA_CH0).unwrap());
}
