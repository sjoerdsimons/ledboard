use embassy_stm32::dma::NoDma;
use embassy_stm32::spi::{Instance as SpiInstance, Spi, TxDmaChannel};
use embassy_traits::spi::Write;

//use embedded_hal::digital::v2::OutputPin;
//use embassy_stm32::adc::Adc;

use core::convert::TryInto;

#[derive(Default, Clone, Copy)]
pub struct Led {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub white: u8,
}

struct LedByte([u8; 4]);

impl LedByte {
    fn from_byte(b: u8) -> LedByte {
        let mut l = [0; 4];
        for i in 0..4 {
            l[i] = match b >> (6 - (2 * i)) & 0x3 {
                0x0 => 0x88,
                0x1 => 0x8c,
                0x2 => 0xc8,
                0x3 => 0xcc,
                _ => defmt::unreachable!("bager"),
            };
        }
        LedByte(l)
    }
}

pub struct Leds<T: SpiInstance, Tx, const N_LEDS: usize> {
    spi: Spi<'static, T, Tx, NoDma>,
}

const RESET_BYTES: usize = 64;
const BYTES_PER_LED: usize = 16;
impl<T, Tx, const N_LEDS: usize> Leds<T, Tx, N_LEDS>
where
    T: SpiInstance,
    Tx: TxDmaChannel<T>,
{
    pub fn new(spi: Spi<'static, T, Tx, NoDma>) -> Self {
        Self { spi }
    }

    pub async fn update<I>(&mut self, iter: I)
    where
        I: Iterator<Item = Led>,
        [(); 2 * RESET_BYTES + (BYTES_PER_LED * N_LEDS)]: ,
    {
        let mut data = [0x0u8; 2 * RESET_BYTES + (BYTES_PER_LED * N_LEDS)];
        for (chunk, led) in data[RESET_BYTES..RESET_BYTES + (BYTES_PER_LED * N_LEDS)]
            .chunks_exact_mut(16)
            .zip(iter.chain(core::iter::repeat(Led::default())))
        {
            let green: &mut [u8; 4] = (&mut chunk[0..4]).try_into().unwrap();
            *green = LedByte::from_byte(led.green).0;

            let red: &mut [u8; 4] = (&mut chunk[4..8]).try_into().unwrap();
            *red = LedByte::from_byte(led.red).0;

            let blue: &mut [u8; 4] = (&mut chunk[8..12]).try_into().unwrap();
            *blue = LedByte::from_byte(led.blue).0;

            let white: &mut [u8; 4] = (&mut chunk[12..16]).try_into().unwrap();
            *white = LedByte::from_byte(led.white).0;
        }

        self.spi.write(&data).await.ok();
    }
}
