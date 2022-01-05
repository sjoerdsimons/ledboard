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

mod conway;
use conway::Conway;

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
        let l = if self.offset < 72 {
            if self.status.yellow * 2 > self.offset {
                Some(Led {
                    red: self.status.level,
                    green: (self.status.level / 3).max(1),
                    ..Default::default()
                })
            } else {
                Some(Led::default())
            }
        } else if self.offset < 72 + 20 * 2 + 1 {
            if self.offset >= 72 && self.status.red * 2 > (self.offset - 72) {
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

async fn step<const W: usize, const H: usize>(ledboard: &mut LedBoard, conway: &mut Conway<W, H>) {
    let v = ledboard.get_pot();
    let level = (255.min((v + 15) / 16)) as u8;

    if conway.all_dead() {
        ledboard
            .leds
            .update(core::iter::repeat(Led {
                red: 1.max(level / 2),
                ..Default::default()
            }))
            .await;
        conway.reset(ledboard.get_random());
    } else {
        ledboard
            .leds
            .update(conway.iter_linear().map(|on| {
                if on {
                    Led {
                        white: level,
                        ..Default::default()
                    }
                } else {
                    Led::default()
                }
            }))
            .await;
    }
    if !conway.step() {
        ledboard
            .leds
            .update(core::iter::repeat(Led {
                green: 1.max(level / 2),
                ..Default::default()
            }))
            .await;
        conway.reset(ledboard.get_random());
    }
}

#[embassy::main(config = "config()")]
async fn main(_spawner: Spawner, p: Peripherals) {
    let mut ledboard = LedBoard::new(p).await;
    let mut conway: Conway<12, 12> = Conway::new(ledboard.get_random());
    let mut d = Duration::from_millis(500);
    loop {
        step(&mut ledboard, &mut conway).await;
        let mut after = Timer::after(d);
        loop {
            let (a, reset) = {
                let monitor = ledboard.monitor();
                pin_mut!(monitor);
                match select(after, monitor).await {
                    Either::Left(_) => break,
                    Either::Right((update, after)) => {
                        let reset = if let RotorUpdate::Red(event) = update {
                            match event {
                                RotaryEvent::Up => true,
                                RotaryEvent::CW(_) => {
                                    let td = d / 2;
                                    if td.as_ticks() > 0 {
                                        d = td;
                                    }
                                    info!("Duration: {}", d);
                                    break;
                                }
                                RotaryEvent::CCW(_) => {
                                    d *= 2;
                                    info!("Duration: {}", d);
                                    false
                                }
                                _ => false,
                            }
                        } else {
                            false
                        };
                        (after, reset)
                    }
                }
            };
            if reset {
                conway.reset(ledboard.get_random());
                break;
            } else {
                after = a;
            }
        }
    }
}
