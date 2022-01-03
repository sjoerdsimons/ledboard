use defmt::*;
use embassy_traits::gpio::WaitForAnyEdge;
use embedded_hal::digital::v2::InputPin;
use futures::future::{select, Either};
use rotary_encoder_hal::{Direction, Rotary};

#[derive(Format, Clone, Copy)]
pub enum RotaryEvent {
    CW(u8),
    CCW(u8),
    Button,
}

pub struct RotaryButton<A, B, C, const N: u8> {
    pos: u8,
    encoder: Rotary<A, B>,
    button: C,
}

impl<A, B, C, const N: u8> RotaryButton<A, B, C, N>
where
    A: WaitForAnyEdge + InputPin,
    B: WaitForAnyEdge + InputPin,
    C: WaitForAnyEdge + InputPin,
{
    pub fn new(pin_a: A, pin_b: B, button: C) -> Self {
        let encoder = Rotary::new(pin_a, pin_b);

        Self {
            pos: 0,
            encoder,
            button,
        }
    }

    pub async fn wait_for_event(&mut self) -> RotaryEvent
    where
        for<'a> <A as WaitForAnyEdge>::Future<'a>: Unpin,
        for<'b> <B as WaitForAnyEdge>::Future<'b>: Unpin,
        for<'c> <C as WaitForAnyEdge>::Future<'c>: Unpin,
    {
        loop {
            let (a, b): (&mut A, &mut B) = self.encoder.pins();
            //select(a.wait_for_any_edge(), b.wait_for_any_edge()).await;
            let e = select(a.wait_for_any_edge(), b.wait_for_any_edge());
            match select(self.button.wait_for_any_edge(), e).await {
                Either::Left(_) => return RotaryEvent::Button,
                Either::Right(_) => (),
            }

            if let Ok(direction) = self.encoder.update() {
                match direction {
                    Direction::Clockwise => {
                        if self.pos == N {
                            self.pos = 0
                        } else {
                            self.pos += 1
                        }
                        return RotaryEvent::CW(self.pos);
                    }
                    Direction::CounterClockwise => {
                        if self.pos == 0 {
                            self.pos = N
                        } else {
                            self.pos -= 1;
                        }
                        return RotaryEvent::CCW(self.pos);
                    }
                    _ => (),
                }
            }
        }
    }
}
