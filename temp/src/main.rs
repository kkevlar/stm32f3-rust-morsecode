#![no_main]
#![no_std]

use aux9::{entry, gpioa, tim6, Leds};
use morse_utils::{construct_key, MorseKey};

mod lcd;

#[inline(never)]
fn delay(tim6: &tim6::RegisterBlock, ms: u16) {
    // Set the timer to go off in `ms` ticks
    // 1 tick = 1 ms
    tim6.arr.write(|w| w.arr().bits(ms));

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

fn ima_key(leds: &mut Leds) -> morse_utils::MorseKey {
    morse_utils::construct_key().unwrap()
}

fn oofus() {}

fn test_do_it(
    gpioa: &'static gpioa::RegisterBlock,
    tim6: &'static tim6::RegisterBlock,
) -> morse_utils::MorseErr {
    use heapless::consts::*;
    use heapless::spsc::*;
    use heapless::Vec;
    use morse_utils::Morse::*;
    use morse_utils::*;

    let mut memory = MorseConverterMemoryStruct::new().map_err(|_| MorseErr::QueueBug).unwrap();

    let mut chars_so_far: Vec<char, U16> = Vec::new();
    let mut mm: MorseManager<U120, U100> = MorseManager::new(
        &mut memory,
        400,
        MorseUnitTimeDecision::EstimateToBeDetermined(DeriveUnitTimeConfig {
            guess_after_this_many_tles: 3,
            max_guess_ms: 1000,
            min_guess_ms: 100,
        }),
    );

    let mut time: Time = 0;
    let mut err = None;

    while err.is_none() {
        delay(tim6, 100);
        time += 100;

        let bit: bool = gpioa.idr.read().idr0().bit();
        let sample = SampledLightIntensity {
            sample_time: time,
            intensity: match bit {
                false => 100,
                true => 1000,
            },
        };

        match mm.add_sample(sample) {
            Ok(_) => (),
            Err(me) => err = Some(me),
        };

        let new_chars: Vec<char, U8> = match mm.produce_chars() {
            Ok(vec) => vec,
            Err(me) => {
                err = Some(me);
                Vec::new()
            }
        };
        for c in new_chars.iter() {
            chars_so_far.push(*c);
        }
        if chars_so_far.len() > 1 {
            loop {
                oofus()
            }
        }
    }

    return err.unwrap();
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
    tim6.psc.write(|w| w.psc().bits(7_999));

    setup_input(rcc, gpioa);

    let mut bruh = gpioc
        .pc0
        .into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper)
        .downgrade();
    let mut mypin = lcd::LcdPin::new(&mut bruh);

    let mut buster = false;

    //    let ret =  test_do_it(gpioa, tim6);

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

        if buster {
            mypin.set_low();
        } else {
            mypin.set_high();
        }
        buster = !buster;
    }
}
