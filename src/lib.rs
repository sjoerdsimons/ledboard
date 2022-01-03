#![feature(type_alias_impl_trait)]
#![feature(generic_const_exprs)]
#![no_std]
use defmt::*;
use embassy::blocking_mutex::kind::CriticalSection;
use embassy::channel::mpsc::{self, Channel, Receiver, Sender, TryRecvError};
use embassy::executor::InterruptExecutor;
use embassy::interrupt::InterruptExt;
use embassy::time::Delay;
use embassy::util::Forever;
use embassy_stm32::adc::Adc;
use embassy_stm32::dma::NoDma;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Input, NoPin, Pull};
use embassy_stm32::interrupt;
use embassy_stm32::peripherals::{ADC1, DMA1_CH3, PA3, PA4, PB1, PB10, PB11, PB14, PB15, SPI1};
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::time::Hertz;
use embassy_stm32::Peripherals;
use futures::future::{select, Either};

pub mod rotary;
use rotary::*;

pub mod leds;
use leds::Leds;

#[derive(Copy, Clone, Format)]
pub enum RotorUpdate {
    Red(RotaryEvent),
    Yellow(RotaryEvent),
}

static INPUTS: Forever<Channel<CriticalSection, RotorUpdate, 3>> = Forever::new();

#[embassy::task]
async fn monitor_input(
    sender: Sender<'static, CriticalSection, RotorUpdate, 3>,
    mut red_rotor: RotaryButton<
        ExtiInput<'static, PA3>,
        ExtiInput<'static, PA4>,
        ExtiInput<'static, PB15>,
        19,
    >,
    mut yellow_rotor: RotaryButton<
        ExtiInput<'static, PB10>,
        ExtiInput<'static, PB11>,
        ExtiInput<'static, PB14>,
        19,
    >,
) {
    loop {
        let y_e = yellow_rotor.wait_for_event();
        let r_e = red_rotor.wait_for_event();
        futures::pin_mut!(y_e);
        futures::pin_mut!(r_e);
        match select(y_e, r_e).await {
            Either::Left(e) => {
                sender.try_send(RotorUpdate::Yellow(e.0));
            }
            Either::Right(e) => {
                sender.try_send(RotorUpdate::Red(e.0));
            }
        }
    }
}

pub struct LedBoard {
    /*
    yellow_rotor: RotaryButton<
        ExtiInput<'static, PB10>,
        ExtiInput<'static, PB11>,
        ExtiInput<'static, PB14>,
        20,
    >,
    red_rotor: RotaryButton<
        ExtiInput<'static, PA3>,
        ExtiInput<'static, PA4>,
        ExtiInput<'static, PB15>,
        20,
    >,
    */
    value_pin: PB1,
    adc: Adc<'static, ADC1>,
    pub leds: Leds<SPI1, DMA1_CH3, 144>,
    receiver: Receiver<'static, CriticalSection, RotorUpdate, 3>,
}

static INPUT_EXECUTOR: Forever<InterruptExecutor<interrupt::PVD>> = Forever::new();

impl LedBoard {
    pub fn new(p: Peripherals) -> Self {
        let pin_a = Input::new(p.PB10, Pull::Up);
        let pin_a = ExtiInput::new(pin_a, p.EXTI10);
        let pin_b = Input::new(p.PB11, Pull::Up);
        let pin_b = ExtiInput::new(pin_b, p.EXTI11);
        let button = Input::new(p.PB14, Pull::Up);
        let button = ExtiInput::new(button, p.EXTI14);
        let yellow_rotor = RotaryButton::new(pin_a, pin_b, button);

        let pin_a = Input::new(p.PA3, Pull::Up);
        let pin_a = ExtiInput::new(pin_a, p.EXTI3);
        let pin_b = Input::new(p.PA4, Pull::Up);
        let pin_b = ExtiInput::new(pin_b, p.EXTI4);
        let button = Input::new(p.PB15, Pull::Up);
        let button = ExtiInput::new(button, p.EXTI15);
        let red_rotor = RotaryButton::new(pin_a, pin_b, button);

        let channel = INPUTS.put(Channel::new());
        let (sender, mut receiver) = mpsc::split(channel);

        let spi = Spi::new(
            p.SPI1,
            p.PA5,
            p.PA7,
            NoPin,
            p.DMA1_CH3,
            NoDma,
            Hertz(3_000_000),
            spi::Config::default(),
        );

        let leds = Leds::new(spi);

        let irq = interrupt::take!(PVD);
        irq.set_priority(interrupt::Priority::P6);
        let executor = INPUT_EXECUTOR.put(InterruptExecutor::new(irq));
        executor.start(move |spawner| {
            unwrap!(spawner.spawn(monitor_input(sender, red_rotor, yellow_rotor)));
        });

        let mut adc = Adc::new(p.ADC1, &mut Delay);
        let mut vref = adc.enable_vref(&mut Delay);
        info!("{}", adc.calibrate(&mut vref));
        Self {
            value_pin: p.PB1,
            adc,
            leds,
            receiver,
        }
    }

    pub async fn monitor(&mut self) -> RotorUpdate {
        self.receiver.recv().await.unwrap()
    }

    pub fn get_pot(&mut self) -> u16 {
        self.adc.read(&mut self.value_pin)
    }
}
