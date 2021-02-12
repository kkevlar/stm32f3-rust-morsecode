#![no_std]

macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = heapless::FnvIndexMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Morse {
    Dot,
    Dash,
    Error,
    TinySpace,
    LetterSpace,
    WordSpace,
}

extern crate heapless;

use core::{cmp::min, convert::TryFrom};
use heapless::consts::*;
use heapless::FnvIndexMap;
use heapless::Vec;

pub type Time = i64;
pub type LightIntensity = u16;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum LightState {
    Light,
    Dark,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum MorseErr {
    TooFewTLEs,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct Scored<T> {
    pub item: T,
    pub score: i64,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct TimedLightEvent {
    pub light_state: LightState,
    pub duration: Time,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct MorseCandidate {
    pub light_state: LightState,
    pub units: Time,
}

const MORSE_CANDIDATES: [MorseCandidate; 5] = [
    MorseCandidate {
        light_state: LightState::Light,
        units: 1,
    },
    MorseCandidate {
        light_state: LightState::Light,
        units: 3,
    },
    MorseCandidate {
        light_state: LightState::Dark,
        units: 1,
    },
    MorseCandidate {
        light_state: LightState::Dark,
        units: 3,
    },
    MorseCandidate {
        light_state: LightState::Dark,
        units: 7,
    },
];

pub fn calc_error(
    event: &TimedLightEvent,
    candidate: &MorseCandidate,
    unit_millis: Time,
) -> Option<i64> {
    if event.light_state == candidate.light_state {
        Some((event.duration - candidate.units * unit_millis).abs())
    } else {
        None
    }
}

fn make_score(
    event: &TimedLightEvent,
    mc: &'static MorseCandidate,
    unit_millis: Time,
) -> Option<Scored<&'static MorseCandidate>> {
    Some(Scored {
        item: mc,
        score: calc_error(event, mc, unit_millis)?,
    })
}

fn poisoned_min<T>(
    min_so_far: Option<Result<Scored<T>, MorseErr>>,
    next: Result<Scored<T>, MorseErr>,
) -> Option<Result<Scored<T>, MorseErr>> {
    Some(match (min_so_far, next) {
        (None, next) => next,
        (Some(Err(prev_error)), _) => Err(prev_error),
        (Some(Ok(_)), Err(next_error)) => Err(next_error),
        (Some(Ok(msf)), Ok(next)) => {
            if msf.score < next.score {
                Ok(msf)
            } else {
                Ok(next)
            }
        }
    })
}

pub fn best_error(
    event: &TimedLightEvent,
    unit_millis: Time,
) -> Result<Scored<&MorseCandidate>, MorseErr> {
    let mut best = None;
    for mc in MORSE_CANDIDATES.iter() {
        match (calc_error(event, mc, unit_millis), best) {
            (None, _) => continue,
            (Some(curr), None) => {
                best = Some(Scored {
                    item: mc,
                    score: curr,
                })
            }
            (Some(curr), Some(Scored { score: b, .. })) if curr < b => {
                best = Some(Scored {
                    item: mc,
                    score: curr,
                })
            }
            _ => (),
        };
    }
    best.ok_or(MorseErr::TooFewTLEs)
}

pub fn score_possible_unit_millis(
    unit_millis: Time,
    timings: &[TimedLightEvent],
) -> Result<Scored<Time>, MorseErr> {
    let mut sum = 0;

    for event in timings {
        let score = best_error(event, unit_millis)?.score;
        sum += score;
    }

    let result = Ok(Scored {
        item: unit_millis,
        score: sum,
    });

    result
}

pub fn estimate_unit_time(
    timings: &[TimedLightEvent],
    min_millis: Time,
    max_millis: Time,
) -> Result<Scored<Time>, MorseErr> {
    // Iterate over possible unit times from 1 to 5000 ms
    (min_millis..max_millis)
        // For each time, score it by summing the scores of the best candidate for each event
        .map(|ratio| {
            // let ratio = ratio as f32;
            // let ratio = ratio / 100f32;
            // let plus = (max_millis - min_millis) as f32 * ratio;
            // let plus = plus as Time;
            // score_possible_unit_millis(min_millis + plus, timings)
            score_possible_unit_millis(ratio, timings)
        })
        // Converge on the minimum scoring unit time
        .fold(None, poisoned_min)
        // Ignore possible errors and pull out the best scoring unit time
        .unwrap_or(Err(MorseErr::TooFewTLEs))
}

fn fill_unit_time_possibilities() {
    for i in 1..100 {
        let i: f32 = i as f32;
    }
}

pub fn calc_digital_cutoffs(
    intensities: &[(Time, LightIntensity)],
) -> Result<(LightIntensity, LightIntensity), core::num::TryFromIntError> {
    let mut intensity_sum: u32 = 0; 

    for (_, li) in intensities
    {
        intensity_sum += *li as u32;
    }

    let intensity_avg: u32 = intensity_sum / (intensities.len() as u32);

        let mut lows = (0u32, 0u32);
        let mut highs = (0u32, 0u32);
    for (_ , li) in intensities
    {
        let li = *li as u32;
        if li > intensity_avg
        {
           highs = (highs.0+1, highs.1 +li);
        }
        else 
        {
           lows = (lows.0+1, lows.1 +li);
        }

    }

    let lows_avg = lows.1 / lows.0;
    let highs_avg = highs.1 / highs.0;

    let diff = highs_avg - lows_avg;
    let low_cut = lows_avg + (diff / 4);
    let high_cut = lows_avg + ((3 * diff) / 4);

    Ok((
        low_cut as u16,
        high_cut as u16,
    ))
}

pub fn convert<C>(
    intensities: &[(Time, LightIntensity)],
    light_states: &mut Vec<TimedLightEvent, C>,
    start_time: Time,
) -> Result<(), core::num::TryFromIntError>
where
    C: heapless::ArrayLength<TimedLightEvent>,
{
    use LightState::*;
    let mut curr_light_state = Dark;
    let mut start_time = start_time;

    let (low_cut, high_cut) = calc_digital_cutoffs(intensities)?;

    for (time, light) in intensities.iter() {
        let next_light_state = match (curr_light_state, light) {
            (Dark, x) if *x > high_cut => Some(Light),
            (Light, x) if *x < low_cut => Some(Dark),
            _ => None,
        };
        match next_light_state {
            Some(next_light_state) => {
                let tle = TimedLightEvent {
                    light_state: curr_light_state,
                    duration: *time - start_time,
                };

                light_states.push(tle);
                curr_light_state = next_light_state;
                start_time = *time;
            }
            _ => (),
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_error_spoton() {
        assert_eq!(
            0,
            calc_error(
                &TimedLightEvent {
                    light_state: LightState::Dark,
                    duration: 600,
                },
                &MorseCandidate {
                    light_state: LightState::Dark,
                    units: 3,
                },
                200
            )
            .unwrap()
        );
    }

    #[test]
    fn test_calc_error_confused() {
        assert_eq!(
            200,
            calc_error(
                &TimedLightEvent {
                    light_state: LightState::Light,
                    duration: 300,
                },
                &MorseCandidate {
                    light_state: LightState::Light,
                    units: 1,
                },
                100
            )
            .unwrap()
        );
    }

    fn best_error_helper(light_state: LightState, duration: i64, units: i64) -> i64 {
        best_error(
            &TimedLightEvent {
                light_state,
                duration,
            },
            units,
        )
        .unwrap()
        .score
    }

    #[test]
    fn test_best_error() {
        use super::LightState::*;

        assert_eq!(100, best_error_helper(Dark, 200, 100));
        assert_eq!(80, best_error_helper(Dark, 180, 100));
        assert_eq!(50, best_error_helper(Dark, 50, 100));
        assert_eq!(100, best_error_helper(Dark, 0, 100));
        assert_eq!(1, best_error_helper(Dark, 701, 100));
        assert_eq!(1, best_error_helper(Dark, 6, 1));

        assert_eq!(200, best_error_helper(Light, 800, 200));
        assert_eq!(400, best_error_helper(Light, 700, 100));
        assert_eq!(1000, best_error_helper(Light, 0, 1000));
        assert_eq!(100, best_error_helper(Light, 200, 100));
        assert_eq!(2, best_error_helper(Light, 1502, 500));
        assert_eq!(0, best_error_helper(Light, 75, 25));
    }

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

    #[test]
    fn test_estimate() {
        let test_durations = [
            700, 300, 100, 100, 100, 100, 100, 100, 300, 300, 100, 300, 100, 300, 300, 100, 100,
            100, 100, 300, 300, 300, 300, 300, 300, 100, 300, 300, 300, 100, 100, 700, 300, 100,
            300, 100, 300, 300, 300, 100, 300, 100, 300, 300, 100, 100, 100, 100, 300, 100, 100,
            700,
        ];
        let mut timed_light_events: Vec<TimedLightEvent, U128> = Vec::new();
        helper_fill_events_slice(&test_durations, &mut timed_light_events);
        assert_eq!(
            Scored {
                item: 100,
                score: 0
            },
            estimate_unit_time(&timed_light_events, 0, 10000).unwrap()
        );
    }
}

pub fn mc_to_morse(mc: &MorseCandidate) -> Morse {
    use Morse::*;
    match mc {
        MorseCandidate {
            light_state: LightState::Light,
            units: 1,
        } => Dot,
        MorseCandidate {
            light_state: LightState::Light,
            units: 3,
        } => Dash,
        MorseCandidate {
            light_state: LightState::Dark,
            units: 1,
        } => TinySpace,
        MorseCandidate {
            light_state: LightState::Dark,
            units: 3,
        } => LetterSpace,
        MorseCandidate {
            light_state: LightState::Dark,
            units: 7,
        } => WordSpace,
        _ => Morse::Error,
    }
}

// fn char_to_morse(c: char) -> Morse {
//     use Morse::*;
//     match c {
//         '0' => Dot,
//         '1' => Dash,
//         _ => Error,
//     }
// }

// fn string_to_code(code: &str) -> Vec<Morse, U8> {
//     code.chars().map(char_to_morse).collect()
// }

// fn ez(o: (&str, &char)) -> (Vec<Morse, U8>, char) {
//     match o {
//         (str, c) => (string_to_code(str), *c),
//     }
// }
