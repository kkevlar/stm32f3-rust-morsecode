
use heapless::consts::*;
use heapless::Vec;

use morse_utils::*;

macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

// fn split_slice<'a, T>(sl: &'a [T], on: &T) -> std::vec::Vec<std::vec::Vec<&'a T>>
// where
//     T: core::fmt::Debug + std::cmp::PartialEq,
// {
//     let mut v = std::vec::Vec::new();

//     v.push(std::vec::Vec::new());
//     let mut count = 0;

//     for item in sl.iter() {
//         if item == on {
//             v.push(std::vec::Vec::new());
//             count += 1;
//         } else {
//             v[count].push(item);
//         }
//     }

//     println!(" returning {:?}", v);
//     v

// }

const test_durations : [i64; 52] = [
        700, 300, 100, 100, 100, 100, 100, 100, 300, 300, 100, 300, 100, 300, 300, 100, 100, 100,
        100, 300, 300, 300, 300, 300, 300, 100, 300, 300, 300, 100, 100, 700, 300, 100, 300, 100,
        300, 300, 300, 100, 300, 100, 300, 300, 100, 100, 100, 100, 300, 100, 100, 700,
    ];
 const myint: [(Time, LightIntensity); 9] = [
        (5, 50),
        (10, 50),
        (15, 500),
        (20, 50),
        (25, 500),
        (30, 50),
        (35, 500),
        (40, 50),
        (60, 51),
    ];


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




fn main() -> () {
    use morse_utils::*;

let mut timed_light_events: Vec<TimedLightEvent, U64> = Vec::new();
    helper_fill_events_slice(&test_durations, &mut timed_light_events);


    let expected: Scored<i64> = Scored {
        item: 100,
        score: 0,
    };
    match estimate_unit_time(&timed_light_events, 100, 110) {
        Ok(actual) if expected == actual => {
        },
        Err(_) => loop {
        },
        _ => 
        loop {
        },
    };

 
    let mut ttt: Vec<TimedLightEvent, U32> = Vec::new();
    match convert(&myint[0..], &mut ttt, 0) {
        Err(_) => loop {
        },
        _ => (),
    };

    let r = estimate_unit_time(&ttt, 5, 6);
    let mut unwr;
    match r {
        Err(_) => loop {
        },
        Ok(r) => unwr = r.item,
    }

println!  ("{:?}",timed_light_events)   ;
}
