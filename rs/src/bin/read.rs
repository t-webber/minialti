#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::flash::Flash;
use embassy_rp::peripherals::{DMA_CH0, FLASH};
use embassy_time::Timer;
use log::info;
use rpi::{Irqs, P, blink_task, logger_task, mem, parse_accel_values};

#[embassy_executor::task]
async fn display_saved_data(flash: P<FLASH>, dma: P<DMA_CH0>) {
    let mut flash = Flash::<_, _, { mem::SIZE }>::new(flash, dma, Irqs);
    let mut from = mem::ADDR;
    let mut buf = [0; 252];
    while from + buf.len() as u32 + 6 < mem::SIZE as u32 {
        flash.blocking_read(from, &mut buf).unwrap();
        from += buf.len() as u32;
        let mut cursor = 0;
        while cursor <= 250 {
            let (x, y, z) = parse_accel_values(&buf[cursor..cursor + 6]);
            info!("{x:+.2} {y:+.2} {z:+.2}");
            cursor += 6;
            Timer::after_millis(1).await;
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    spawner.spawn(blink_task(p.PIN_25, 0).unwrap());
    spawner.spawn(logger_task(p.USB).unwrap());
    Timer::after_secs(1).await; // wait for flash.sh to find device
    info!("setup");
    spawner.spawn(display_saved_data(p.FLASH, p.DMA_CH0).unwrap());
}
