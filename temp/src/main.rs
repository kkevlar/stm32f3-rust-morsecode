#![no_main]
#![no_std]

use aux9::{entry, gpioa, tim6, Leds};
use morse_utils::{construct_key, MorseKey};

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

    let mut chars_so_far: Vec<char, U16> = Vec::new();
    let mut mm: MorseManager<U60, U90> = MorseManager::new(
        400,
        MorseUnitTimeDecision::EstimateToBeDetermined(DeriveUnitTimeConfig {
            guess_after_this_many_tles: 5,
            max_guess_ms: 1000,
            min_guess_ms: 100,
        }),
    );

    let mut time: Time = 0;
    let mut err = None;

    while err.is_none() {
        delay(tim6, 50);
        time += 50;

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
        }
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

fn test_manager() -> bool {
    use heapless::consts::*;
    use heapless::spsc::*;
    use heapless::Vec;
    use morse_utils::Morse::*;
    use morse_utils::*;

    let my_intensities = [
        (100, 0),
        (100, 20),
        (100, 40),
        (900, 60),
        (100, 120),
        (900, 140),
        (100, 160),
        (900, 180),
        (100, 200),
        (900, 220),
        (100, 240),
        (100, 500),
        (900, 520),
        (100, 540),
        (100, 600),
        (900, 601),
        (900, 660),
        (100, 661),
        (100, 680),
        (900, 681),
        (900, 700),
        (100, 720),
        (900, 740),
        (100, 760),
        (900, 820), //o
        (100, 880),
        (900, 900),
        (100, 960),
        (900, 980),
        (100, 1040),
        (900, 1100), //g
        (100, 1160),
        (900, 1180),
        (100, 1240),
        (900, 1260),
        (100, 1280),
        (100, 1600),
        (900, 1620),
        (100, 1640),
    ];

    let mut converter: MorseManager<U64, U64> = MorseManager::new(
        500,
        MorseUnitTimeDecision::EstimateToBeDetermined(DeriveUnitTimeConfig {
            guess_after_this_many_tles: 7,
            max_guess_ms: 40,
            min_guess_ms: 10,
        }),
    );

    for (light, time) in my_intensities.iter() {
        match converter
            .add_sample(SampledLightIntensity {
                intensity: *light,
                sample_time: *time,
            })
            {
                Ok(_) => (),
                Err(_) => return false,
            }
    }

    let vec: Vec<_, U32> = match converter.produce_chars()
    {
        Ok(v) => v,
        _ => return false,
    };

    &['b', ' ', 'e', 'd', 'o', 'g', ' '] == &vec[..]
}

#[entry]
fn main() -> ! {
    let (mut leds, gpioa, rcc, tim6) = aux9::init();

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

    let himom = test_manager();

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
