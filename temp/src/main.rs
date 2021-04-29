#![no_main]
#![no_std]

use aux9::{entry, gpioa, tim6, Leds};
use lcd::Delay;

mod lcd;

#[inline(never)]
fn delay(tim6: &tim6::RegisterBlock, us: u16) {

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
        { delay(self.tim, ms ) }
    }

    fn delay_us(&self, ms: u16) -> () {
        delay(self.tim, ms )
    }
}

#[entry]
fn main() -> ! {
    let (mut leds, gpioa, mut gpioc, rcc, tim6) = aux9::init();

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

    let mut bruh = gpioc
        .pc0
        .into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper)
        .downgrade();
    let mut pc0pin = lcd::LcdPin::new(&mut bruh);
    let mut bruh = gpioc
        .pc1
        .into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper)
        .downgrade();
    let mut pc1pin = lcd::LcdPin::new(&mut bruh);
    let mut bruh = gpioc
        .pc2
        .into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper)
        .downgrade();
    let mut pc2pin = lcd::LcdPin::new(&mut bruh);
    let mut bruh = gpioc
        .pc3
        .into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper)
        .downgrade();
    let mut pc3pin = lcd::LcdPin::new(&mut bruh);
    let mut bruh = gpioc
        .pc4
        .into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper)
        .downgrade();
    let mut pc4pin = lcd::LcdPin::new(&mut bruh);
    let mut bruh = gpioc
        .pc6
        .into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper)
        .downgrade();
    let mut pc6pin = lcd::LcdPin::new(&mut bruh);
    let mut bruh = gpioc
        .pc7
        .into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper)
        .downgrade();
    let mut pc7pin = lcd::LcdPin::new(&mut bruh);

    let mut mydelay = MyDelay { tim: tim6 };

    let mut lcd_obj = lcd::LcdObject::new(
        lcd::DataPinCollection::Four([pc0pin, pc1pin, pc2pin, pc3pin]),
        pc7pin,
        pc4pin,
        pc6pin,
        &mydelay,
    );

    leds[0].on();

    mydelay.delay_ms(2000);
    leds[0].off();
    leds[1].on();

    

    lcd_obj.initialize();

    for c in "Hello World!".chars() {
        lcd_obj.send_char(c);
    }


    let mut buster = false;

    let mut i = 0u32;
    let ms = 50;
    loop {
        for curr in 0..8 {
            let next = (curr + 1) % 8;

            leds[next].on();
            delay(tim6, ms);
            leds[curr].off();
            delay(tim6, ms);

            if i > 1000 {
                leds[0].on();
                i = 0;
            }
            i += 1;
        }
    }
}
