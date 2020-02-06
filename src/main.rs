//#![deny(warnings)]
#![no_std]
#![no_main]

use core::convert::TryInto;
use panic_itm as _;
use cortex_m::iprintln;
use rtfm::cyccnt::{Instant, U32Ext as _ };
use stm32f4xx_hal::{prelude::*, i2c::I2c, gpio::{Alternate, AF4, gpiob::{PB8, PB9}}, stm32};
use embedded_graphics::{prelude::*, pixelcolor::BinaryColor, image::Image};
use ssd1306::{prelude::*, Builder as SSD1306Builder};

const PERIOD: u32 = 1_000_000;

#[rtfm::app(device = stm32f4::stm32f446, peripherals = true, monotonic = rtfm::cyccnt::CYCCNT)]
const APP: () = {

    struct Resources {
        #[init(0)]
        n: u32,
        itm: cortex_m::peripheral::ITM,
        disp: GraphicsMode<ssd1306::interface::i2c::I2cInterface<stm32f4xx_hal::i2c::I2c<stm32f4::stm32f446::I2C1, (stm32f4xx_hal::gpio::gpiob::PB8<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF4>>, stm32f4xx_hal::gpio::gpiob::PB9<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF4>>)>>>,
        im: Image<'static, BinaryColor>,
    }

    #[init(schedule = [tick])]
    fn init(mut cx: init::Context) -> init::LateResources {

        // Initialize (enable) the monotonic timer (CYCCNT)
        cx.core.DCB.enable_trace();
        // required on devices that software lock the DWT (e.g. STM32F7)
        unsafe { cx.core.DWT.lar.write(0xC5ACCE55) }
        cx.core.DWT.enable_cycle_counter();

        let mut itm = cx.core.ITM;

        // semantically, the monotonic timer is frozen at time "zero" during `init`
        let now = cx.start; // the start time of the system

        // Schedule `tick` to run 8e6 cycles (clock cycles) in the future
        cx.schedule.tick(now + PERIOD.cycles()).unwrap();

        //iprintln!(&mut itm.stim[0], "init @ {:?}", now);

        let rcc = cx.device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();
        let gpiob = cx.device.GPIOB.split();

        let scl = gpiob.pb8.into_alternate_af4().set_open_drain();
        let sda = gpiob.pb9.into_alternate_af4().set_open_drain();

        let i2c = I2c::i2c1(
            cx.device.I2C1,
            (scl, sda),
            100.khz(),
            clocks
        );

        let mut disp: GraphicsMode<_> = SSD1306Builder::new().connect_i2c(i2c).into();

        let im: Image<BinaryColor> = Image::new(include_bytes!("./rust.raw"), 64, 64);

        disp.init().unwrap();
        disp.flush().unwrap();
        //disp.clear();
        //disp.flush().unwrap();
        disp.draw(im.into_iter());
        disp.flush().unwrap();

        init::LateResources {
            itm: itm,
            disp: disp,
            im: im,
        }
    }

    #[task(schedule = [tick], resources = [itm, n, im, disp])]
    fn tick(cx: tick::Context) {

        let now = Instant::now();

        *cx.resources.n += 1;
        let x_coord: i32 = (*cx.resources.n % 128).try_into().unwrap();

        cx.resources.disp.draw(cx.resources.im.translate(Point::new(x_coord, 0)).into_iter());
        cx.resources.disp.flush().unwrap();

        cx.schedule.tick(cx.scheduled + PERIOD.cycles()).unwrap();
        //iprintln!(&mut cx.resources.itm.stim[0], "tick(scheduled = {:?}, now = {:?})", cx.scheduled, now);
    }

    extern "C" {
        fn UART4();
    }
};
