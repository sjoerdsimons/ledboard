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
        for (i, item) in l.iter_mut().enumerate() {
            *item = match b >> (6 - (2 * i)) & 0x3 {
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
struct ZigZag<'a, const N_LEDS: usize> {
    data: &'a mut [u8],
    offset: usize,
}

impl<'a, const N_LEDS: usize> ZigZag<'a, N_LEDS> {
    fn new(data: &'a mut [u8]) -> Self {
        ZigZag { data, offset: 0 }
    }
}

impl<'a, const N_LEDS: usize> Iterator for ZigZag<'a, N_LEDS> {
    type Item = &'a mut [u8; BYTES_PER_LED];

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset < N_LEDS {
            let mut o = match (self.offset % 12, self.offset / 12) {
                (x, y) if x % 2 == 0 => x * 12 + y,
                (x, y) => (x + 1) * 12 - y - 1,
            };
            o *= BYTES_PER_LED;
            o += RESET_BYTES;
            let slice = &mut self.data[o..o + BYTES_PER_LED];

            let ptr = slice.as_mut_ptr() as *mut [u8; BYTES_PER_LED];
            self.offset += 1;
            unsafe { Some(&mut *ptr) }
        } else {
            None
        }
    }
}

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
        let zigzag = ZigZag::<N_LEDS>::new(&mut data);
        for (chunk, led) in zigzag.zip(iter.chain(core::iter::repeat(Led::default()))) {
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
