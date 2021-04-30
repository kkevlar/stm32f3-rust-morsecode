#![no_main]
#![no_std]

use aux9::{entry, gpioa, tim6, Leds};
use lcd::{Delay, LcdObject};

mod lcd;

#[inline(never)]
fn delayus(tim6: &tim6::RegisterBlock, us: u16) {

    let us = if us < 10 {10} else{  us };
    let us = us/10;

    // Set the timer to go off in `ms` ticks
    // 1 tick = 1 ms
    tim6.arr.write(|w| w.arr().bits(us));

    // CEN: Enable the counter
    tim6.cr1.modify(|_, w| w.cen().set_bit());

    // Wait until the alarm goes off (until the update event occurs)
    while !tim6.sr.read().uif().bit_is_set() {}

    // Clear the update event flag
    tim6.sr.modify(|_, w| w.uif().clear_bit());
}

fn setup_output(gpioa: &aux9::gpioa::RegisterBlock) {
    gpioa.moder.write(|w| unsafe { w.bits(0x0000u32) });
}

fn setup_input(rcc: &aux9::rcc::RegisterBlock, gpioa: &aux9::gpioa::RegisterBlock) {
    // Allow GPIOA
    rcc.ahbenr.modify(|_, w| w.iopaen().set_bit());

    gpioa.moder.modify(|_, w| w.moder0().input());

    unsafe {
        gpioa.pupdr.modify(|_, w| w.pupdr0().bits(2));
    }
}

struct MyDelay<'a> {
    tim: &'a tim6::RegisterBlock,
}

impl<'a> lcd::Delay for MyDelay<'a> {
    fn delay_ms(&self, ms: u16) -> () {
        for _ in 0..1000
        { delayus(self.tim, ms ) }
    }

    fn delay_us(&self, ms: u16) -> () {
        delayus(self.tim, ms )
    }

}

use aux9::gpioc::PCx;
use aux9::hal::gpio::Output;
use aux9::hal::gpio::PushPull;

struct PreLcdInfo<'a>
{
   c0 :PCx<Output<PushPull>> ,
   c1 :PCx<Output<PushPull>> ,
   c2 :PCx<Output<PushPull>> ,
   c3 :PCx<Output<PushPull>> ,
   c4 :PCx<Output<PushPull>> ,
   c6 :PCx<Output<PushPull>> ,
   c7 :PCx<Output<PushPull>> ,
   mydelay: MyDelay<'a>,
}

fn construct_lcd<'a>(info: &'a mut PreLcdInfo) -> Result<LcdObject<'a,'a,'a,'a,'a>, ()>
{
   let mut lcd_obj = LcdObject::new(
lcd::DataPinCollection::Four(
       [
       lcd::LcdPin::new(&mut info.c0),
       lcd::LcdPin::new(&mut info.c1),
       lcd::LcdPin::new(&mut info.c2),
       lcd::LcdPin::new(&mut info.c3),
       ]),

       lcd::LcdPin::new(&mut info.c7),
       lcd::LcdPin::new(&mut info.c4),
       lcd::LcdPin::new(&mut info.c6),
      &info.mydelay, 

   ) ;
    info.mydelay.delay_ms(100);
  lcd_obj.initialize()?;


    for c in "test".chars()
    {
       lcd_obj.send_char(c) ?;
    }
    lcd_obj.send_command(lcd::LcdCommand::ClearDisplay)?;
    lcd_obj.send_command(lcd::LcdCommand::ReturnHome)?;

    for c in "Hello Lcd!".chars() {
        lcd_obj.send_char(c)?;
    }
    lcd_obj.set_cursor(1, 0)?;
    for c in "LIQUID CRYSTAL".chars() {
        lcd_obj.send_char(c)?;
    }

    Ok(lcd_obj)
}

fn prep_lcd_construction<'a>(mut gpioc: aux9::gpioc::Parts, tim6: &'a tim6::RegisterBlock,
) ->  PreLcdInfo<'a>
{
 let pp0 = gpioc
        .pc0
        .into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper)
        .downgrade();
    let pp1 = gpioc
        .pc1
        .into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper)
        .downgrade();
    let pp2 = gpioc
        .pc2
        .into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper)
        .downgrade();
    let pp3 = gpioc
        .pc3
        .into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper)
        .downgrade();
    let pp4 = gpioc
        .pc4
        .into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper)
        .downgrade();
    let pp6 = gpioc
        .pc6
        .into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper)
        .downgrade();
    let pp7 = gpioc
        .pc7
        .into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper)
        .downgrade();

    let mydelay = MyDelay { tim: tim6 };

    PreLcdInfo{
        c0: pp0,
        c1: pp1,
        c2: pp2,
        c3: pp3,
        c4: pp4,
        c6: pp6,
        c7: pp7,
        mydelay: mydelay,
    }
}

#[entry]
fn main() -> ! {
    let (mut leds, gpioa, gpioc, rcc, tim6) = aux9::init();

    // Power on the TIM6 timer
    rcc.apb1enr.modify(|_, w| w.tim6en().set_bit());

    // OPM Select one pulse mode
    // CEN Keep the counter disabled for now
    tim6.cr1.write(|w| w.opm().set_bit().cen().clear_bit());

    // Configure the prescaler to have the counter operate at 1 KHz
    // APB1_CLOCK = 8 MHz
    // PSC = 7999
    // 8 MHz / (7999 + 1) = 1 KHz
    // The counter (CNT) will increase on every millisecond
    tim6.psc.write(|w| w.psc().bits(79));

    setup_input(rcc, gpioa);

let mut pre = prep_lcd_construction( gpioc, tim6);
let mut lcd = construct_lcd(&mut pre).unwrap();

for _ in 0..1000
{ delayus(tim6, 10000); }

lcd.send_command(lcd::LcdCommand::ClearDisplay);


    let mut i = 0u32;
    let ms = 50;
    loop {
        for curr in 0..8 {
            let next = (curr + 1) % 8;

            leds[next].on();
            delayus(tim6, ms);
            leds[curr].off();
            delayus(tim6, ms);

            if i > 1000 {
                leds[0].on();
                i = 0;
            }
            i += 1;
        }
    }
}
