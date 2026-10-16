#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::peripherals::{I2C0, PIN_0, PIN_1};
use embassy_time::Timer;
use log::info;
use rpi::{P, blink_task, get_accel_raw, logger_task, setup_accel};

#[embassy_executor::task]
async fn save_accel_task(peri: P<I2C0>, scl: P<PIN_1>, sda: P<PIN_0>) -> ! {
    let mut i2c = setup_accel(peri, scl, sda).await;

    loop {
        Timer::after_millis(100).await;
        let _bytes = get_accel_raw(&mut i2c).await;
    }
}

#[embassy_executor::task]
async fn save_data_task() {}

#[embassy_executor::task]
async fn log_data_task() {}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    spawner.spawn(blink_task(p.PIN_25).unwrap());
    spawner.spawn(logger_task(p.USB).unwrap());
    Timer::after_secs(1).await; // wait for flash.sh to find device
    info!("setup");
    spawner.spawn(save_data_task().unwrap());
    spawner.spawn(log_data_task().unwrap());
}
