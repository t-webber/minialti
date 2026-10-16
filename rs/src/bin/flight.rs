#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::flash::Flash;
use embassy_rp::peripherals::{DMA_CH0, FLASH, I2C0, PIN_0, PIN_1};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Ticker, Timer};
use log::info;
use rpi::{
    Irqs, P, blink_task, get_accel_raw, log_err, logger_task, mem, parse_accel_values, setup_accel,
};

static CHANNEL: Channel<CriticalSectionRawMutex, [u8; 6], 1024> = Channel::new();

#[embassy_executor::task]
async fn measure_accel_task(peri: P<I2C0>, scl: P<PIN_1>, sda: P<PIN_0>) -> ! {
    let mut i2c = setup_accel(peri, scl, sda).await;
    let mut ticker = Ticker::every(Duration::from_millis(2));
    loop {
        ticker.next().await;
        let bytes = get_accel_raw(&mut i2c).await;
        CHANNEL.send(bytes).await;
    }
}

#[embassy_executor::task]
async fn save_data_task(flash: P<FLASH>, dma: P<DMA_CH0>) {
    let mut flash = Flash::<_, _, { mem::SIZE }>::new(flash, dma, Irqs);
    const CAP: usize = 6 * 512; // 3072
    let mut data = [0_u8; CAP];
    let mut data_cursor = 0;
    let mut from = mem::ADDR;
    info!("erasing flash");
    flash
        .blocking_erase(from, mem::SIZE as u32)
        .inspect_err(log_err)
        .ok();
    info!("done erasing flash");
    loop {
        let raw = &CHANNEL.receive().await;
        data[data_cursor..data_cursor + 6].copy_from_slice(raw);
        let (x, y, z) = parse_accel_values(*raw);
        info!("{x:+.2} {y:+.2} {z:+.2}");
        data_cursor += 6;
        if data_cursor == CAP {
            info!("storing data");
            flash.blocking_write(from, &data).inspect_err(log_err).ok();
            from += CAP as u32;
            data_cursor = 0;
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    spawner.spawn(blink_task(p.PIN_25).unwrap());
    spawner.spawn(logger_task(p.USB).unwrap());
    Timer::after_secs(1).await; // wait for flash.sh to find device
    spawner.spawn(save_data_task(p.FLASH, p.DMA_CH0).unwrap());
    info!("setup");
    spawner.spawn(measure_accel_task(p.I2C0, p.PIN_1, p.PIN_0).unwrap());
}
