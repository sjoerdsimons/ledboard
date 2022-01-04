#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt_rtt as _;
// global logger
use panic_probe as _;

pub use defmt::*;
use embassy::executor::Spawner;
use embassy::time::{Duration, Timer};
use embassy_stm32::time::Hertz;
use embassy_stm32::Config;
use embassy_stm32::Peripherals;
use futures::future::{select, Either};
use futures::pin_mut;

use ledboard::leds::Led;
use ledboard::rotary::RotaryEvent;
use ledboard::{LedBoard, RotorUpdate};

struct Status {
    yellow: u8,
    red: u8,
    level: u8,
}

impl Status {
    fn new() -> Status {
        Status {
            yellow: 0,
            red: 0,
            level: 0x10,
        }
    }

    fn level(&mut self, level: u8) {
        self.level = level;
    }

    fn filler(&self) -> StatusIter<'_> {
        StatusIter {
            status: self,
            offset: 0,
        }
    }
}

struct StatusIter<'a> {
    status: &'a Status,
    offset: u8,
}

impl Iterator for StatusIter<'_> {
    type Item = Led;

    fn next(&mut self) -> Option<Self::Item> {
        if self.status.level == 0 {
            return None;
        }
        let l = if self.offset < 24 {
            if self.status.yellow > self.offset {
                Some(Led {
                    red: self.status.level,
                    green: (self.status.level / 3).max(1),
                    ..Default::default()
                })
            } else {
                Some(Led::default())
            }
        } else if self.offset < 72 {
            if self.offset >= 48 && self.status.red > (self.offset - 48) {
                Some(Led {
                    red: self.status.level,
                    ..Default::default()
                })
            } else {
                Some(Led::default())
            }
        } else {
            None
        };
        self.offset = self.offset.wrapping_add(1);
        l
    }
}

fn config() -> Config {
    let mut config = Config::default();
    // Needed to allow SPI to run at 3mhz
    config.rcc.sys_ck = Some(Hertz(48_000_000));
    // Avoid violating the requirements
    config.rcc.pclk1 = Some(Hertz(24_000_000));
    config
}

#[embassy::main(config = "config()")]
async fn main(_spawner: Spawner, p: Peripherals) {
    let mut ledboard = LedBoard::new(p).await;
    let mut status = Status::new();
    loop {
        let mut after = Timer::after(Duration::from_millis(33));
        loop {
            let m = ledboard.monitor();
            pin_mut!(m);
            after = match select(after, m).await {
                Either::Left(_) => break,
                Either::Right((update, after)) => {
                    match update {
                        RotorUpdate::Red(event) => match event {
                            RotaryEvent::CW(v) | RotaryEvent::CCW(v) => status.red = v,
                            _ => status.red = 0,
                        },
                        RotorUpdate::Yellow(event) => match event {
                            RotaryEvent::CW(v) | RotaryEvent::CCW(v) => status.yellow = v,
                            _ => status.yellow = 0,
                        },
                    }
                    info!("{:?}", update);
                    after
                }
            };
        }
        let v = ledboard.get_pot();
        status.level(255.min((v + 15) / 16) as u8);
        ledboard.leds.update(status.filler()).await;
    }
}
