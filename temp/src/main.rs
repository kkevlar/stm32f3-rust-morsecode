#![no_main]
#![no_std]

use aux9::{entry, tim6};

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

mod buster {
    use core::convert::TryFrom;
    use heapless::consts::*;
    use heapless::FnvIndexMap;
    use heapless::Vec;
    use morse_utils::*;

    fn helper_fill_events_slice<T>(durations: &[i64], vec: &mut Vec<TimedLightEvent, T>)
    where
        T: heapless::ArrayLength<TimedLightEvent>,
    {
        for (i, duration) in durations.iter().enumerate() {
            vec.push(TimedLightEvent {
                light_state: {
                    if i % 2 == 0 {
                        LightState::Dark
                    } else {
                        LightState::Dark
                    }
                },
                duration: *duration,
            })
            .unwrap();
        }
    }

    fn best_error_helper(light_state: LightState, duration: i64, units: i64) -> i64 {
        match best_error(
            &TimedLightEvent {
                light_state,
                duration,
            },
            units,
        ) {
            Ok(s) => s.score,
            _ => 200000,
        }
    }

    macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::heapless::FnvIndexMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

    fn split_slice<'a, T>(sl: &'a [T], on: &T) -> Vec<Vec<&'a T, U32>, U32>
    where
        T: core::fmt::Debug + core::cmp::PartialEq,
    {
        let mut v = Vec::new();

        v.push(Vec::new());
        let mut count = 0;

        for item in sl.iter() {
            if item == on {
                v.push(Vec::new());
                count += 1;
            } else {
                v[count].push(item);
            }
        }
        v
    }

    // const myint: [(Time, LightIntensity); 9] = [
    //     (5, 50),
    //     (10, 50),
    //     (15, 500),
    //     (20, 50),
    //     (25, 500),
    //     (30, 50),
    //     (35, 500),
    //     (40, 50),
    //     (55, 51),
    //     (60, 500),
    //     (75, 50),
    //     (90, 500),
    // ];

    const test_durations: [i64; 52] = [
        700, 300, 100, 100, 100, 100, 100, 100, 300, 300, 100, 300, 100, 300, 300, 100, 100, 100,
        100, 300, 300, 300, 300, 300, 300, 100, 300, 300, 300, 100, 100, 700, 300, 100, 300, 100,
        300, 300, 300, 100, 300, 100, 300, 300, 100, 100, 100, 100, 300, 100, 100, 700,
    ];

    pub fn buster2() -> bool {
        let mut vb: Vec<u8, U8> = Vec::new();
        let mut vc: Vec<u8, U64> = Vec::new();

        vb.push(90);
        vc.push(90);
        vc.push(0);

        let morse_key: FnvIndexMap<&str, char, U64> = hashmap![
        "01" => 'a',
        "1000" => 'b',
        "1010" => 'c',
        "100" => 'd',
        "0" => 'e',
        "0010" => 'f',
        "110" => 'g',
        "0000" => 'h',
        "00" => 'i',
        "0111" => 'j',
        "101" => 'k',
        "0100" => 'l',
        "11" => 'm',
        "10" => 'n',
        "111" => 'o',
        "0110" => 'p',
        "1101" => 'q',
        "010" => 'r',
        "000" => 's',
        "1" => 't',
        "001" => 'u',
        "0001" => 'v',
        "011" => 'w',
        "1001" => 'x',
        "1011" => 'y',
        "1100" => 'z'
        ];

        if vb[0] == vc[0] {
            if vc.len() >= 2 {
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    const MORSE_KEY : [(&str, char); 26] = 
    [
( "01" , 'a'),
(        "1000" , 'b'),
(        "1010" , 'c'),
(        "100" , 'd'),
(        "0" , 'e'),
(        "0010" , 'f'),
(        "110" , 'g'),
(        "0000" , 'h'),
(        "00" , 'i'),
(        "0111" , 'j'),
(        "101" , 'k'),
(        "0100" , 'l'),
(        "11" , 'm'),
(        "10" , 'n'),
(        "111" , 'o'),
(        "0110" , 'p'),
(        "1101" , 'q'),
(        "010" , 'r'),
(        "000" , 's'),
(        "1" , 't'),
(        "001" , 'u'),
(        "0001" , 'v'),
(        "011" , 'w'),
(        "1001" , 'x'),
(        "1011" , 'y'),
(        "1100" , 'z'),
    ];

    pub fn lookup(s: &str) -> char
    {
        let mut ret = '?';
        for (seq, c) in MORSE_KEY.iter()
        {
            if &s == seq
            {
                ret = *c;
            }
        }
        ret
    }

    fn letterify<S>(morse : & mut Vec<morse_utils::Morse, S>) -> char
    where
        S: heapless::ArrayLength<morse_utils::Morse>,
    {
        let mut curr_str : heapless::String<U64> = heapless::String::new();
        let mut did_error = false;
        let mut is_space = false;
            let mut expect_tiny_space = false;

        loop
        {
            use morse_utils::Morse::*;
            let m = morse.pop();
is_space=false;
            match m 
            {
                None | Some(LetterSpace) => break, 
                Some(WordSpace) => { is_space = true; break}
                Some(TinySpace) if expect_tiny_space => expect_tiny_space = false,
                Some(Dot) if !expect_tiny_space => { curr_str.push('0').unwrap(); expect_tiny_space = true },
                Some(Dash) if !expect_tiny_space => { curr_str.push('1').unwrap(); expect_tiny_space = true },
                _ => 
                {
did_error = true;
break;
                },
            };
        }

        match (did_error, is_space)
        {
            (true, _) => '?',
            (false, true) => ' ',
            (false, false) => {
                lookup(curr_str.as_str())
            }
        }
    }

    fn heapless_reverse<T,S> (mut input : Vec<T,S>) -> Vec<T,S>
    where
        S: heapless::ArrayLength<T>,
    {
        let mut output : Vec<T,S> = Vec::new();
        loop
        {
            match input.pop()
            {
                Some(x) => output.push(x),
                _ => break
            };
        }
        output
    }


    pub fn buster(leds: &mut aux9::Leds) -> bool {
        let mut vb: Vec<u8, U8> = Vec::new();
        let mut vc: Vec<u8, U64> = Vec::new();

        let mut ttt: Vec<TimedLightEvent, U8> = Vec::new();

        convert(&myint[..], &mut ttt, 0).unwrap();

        let r = estimate_unit_time(&ttt, 5, 6);
        let mut unwr = r.unwrap().item;

        let r: Vec<Scored<&MorseCandidate>, U16> = ttt
            .iter()
            .map(|tle| morse_utils::best_error(tle, unwr))
            .filter_map(Result::ok)
            .collect();

        let r: Vec<morse_utils::Morse, U256> = r
            .into_iter()
            .map(|s| morse_utils::mc_to_morse(s.item))
            .collect();

        let mut r = heapless_reverse(r);

        for (i, b) in r.iter().enumerate() {
            if *b != Morse::Error {
                if i % 2 == 0 {
                    leds[0].on();
                } else {
                    leds[0].off();
                }
            } else {
                if i % 2 == 0 {
                    leds[1].on();
                } else {
                    leds[1].off();
                }
            }
        }

        loop
        {
let c = letterify(&mut r);
                    leds[0].off();
if c == '?'
                    { leds[1].on(); }

        }

        vb.push(90);
        vc.push(90);
        vc.push(0);


                if lookup("01") == 'a' {
                    leds[0].on();
                } else {
                    leds[0].off();
                }
          


        //         if vb[0] == vc[0]
        //         {
        //         if v.len() > 2
        //         {
        //             false
        //         }
        //         else
        //         {
        //            true
        //         }
        //     }
        //     else
        //     {
        // false
        //     }
        // }

        true
    }
}

#[entry]
fn main() -> ! {
    let (mut leds, rcc, tim6) = aux9::init();

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

    buster::buster(&mut leds);

    let ms = 50;
    loop {
        for curr in 0..8 {
            let next = (curr + 1) % 8;

            leds[next].on();
            delay(tim6, ms);
            leds[curr].off();
            delay(tim6, ms);
        }
    }
}
