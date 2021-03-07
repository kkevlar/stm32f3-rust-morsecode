#![no_std]

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Morse {
    Dot,
    Dash,
    TinySpace,
    LetterSpace,
    WordSpace,
}

extern crate heapless;

use core::convert::TryFrom;

use heapless::consts::*;
use heapless::spsc::Queue;
use heapless::Vec;
use heapless::{spsc::Consumer, ArrayLength};
use heapless::{spsc::Producer, FnvIndexMap};

pub type Time = i64;
pub type LightIntensity = u16;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum LightState {
    Light,
    Dark,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum MorseErr {
    BestErrorBug,
    InputTooLarge,
    MorseInputCrossesLetterBound(Morse),
    QueueBug,
    UnknownChar((u8, u8)),
    EmptyInput,
    ConsumeLogicBug,
    InvalidMorseCandidate(MorseCandidate),
    FailedTLEConversion(ConvertErrs),
    InvalidLetterTinySpacing,
    CalcDigitalFailed(CalcDigitalCutoffsErrs),
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct Scored<T> {
    pub item: T,
    pub score: i64,
}
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct SampledLightIntensity {
    pub intensity: LightIntensity,
    pub sample_time: Time,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct TimedLightEvent {
    pub light_state: LightState,
    pub duration: Time,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ConsumeSamplesInfo<C: heapless::ArrayLength<TimedLightEvent>> {
    pub tles: Vec<TimedLightEvent, C>,
    pub state: (Time, LightState),
}
#[derive(PartialEq, Eq, Clone, Debug, Copy)]
pub struct DeriveUnitTimeConfig {
    guess_after_this_many_tles: u32,
    min_guess_ms: Time,
    max_guess_ms: Time,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum MorseUnitTimeDecision {
    // How many tles should we wait for until we guess?j
    EstimateToBeDetermined(DeriveUnitTimeConfig),
    EstimateProvided(Time),
}

#[derive(PartialEq, Eq, Debug)]
pub struct MorseManager<C, D>
where
    C: ArrayLength<SampledLightIntensity> + ArrayLength<TimedLightEvent> + ArrayLength<Morse>,
    D: ArrayLength<SampledLightIntensity>,
{
    converter: Option<MorseConverter<C>>,
    sample_buf: Vec<SampledLightIntensity, D>,
    span_count: u32,
    likely_middle: LightIntensity,
    likely_last_light_state: LightState,
    unit_time: MorseUnitTimeDecision,
}

impl<C, D> MorseManager<C, D>
where
    C: ArrayLength<SampledLightIntensity> + ArrayLength<TimedLightEvent> + ArrayLength<Morse>,
    D: ArrayLength<SampledLightIntensity>,
{
    pub fn new(
        likely_middle: LightIntensity,
        unit_time: MorseUnitTimeDecision,
    ) -> MorseManager<C, D> {
        MorseManager {
            converter: None,
            sample_buf: Vec::new(),
            span_count: 0,
            likely_middle,
            likely_last_light_state: LightState::Dark,
            unit_time,
        }
    }

    pub fn add_sample(&mut self, sli: SampledLightIntensity) -> Result<(), MorseErr> {
        match &mut self.converter {
            None => {
                if sli.intensity < self.likely_middle
                    && self.likely_last_light_state == LightState::Light
                {
                    self.span_count += 1;
                    self.likely_last_light_state = LightState::Dark;
                } else if sli.intensity > self.likely_middle
                    && self.likely_last_light_state == LightState::Dark
                {
                    self.span_count += 1;
                    self.likely_last_light_state = LightState::Light;
                }

                match self.sample_buf.push(sli) {
                    Ok(_) => Ok(()),
                    Err(_) => Err(MorseErr::InputTooLarge),
                }
            }
            Some(converter) => converter.add_sample(sli),
        }
    }

    pub fn produce_chars<E>(&mut self) -> Result<Vec<char, E>, MorseErr>
    where
        E: ArrayLength<char>,
    {
        match &mut self.converter {
            None if self.span_count > 7 => {
                let cuts = calc_digital_cutoffs(&self.sample_buf[..]);
                let cuts = cuts.map_err(|e| MorseErr::CalcDigitalFailed(e))?;
                let mut converter =
                    MorseConverter::new(self.sample_buf[0].sample_time, self.unit_time, cuts, None)
                        .map_err(|_| MorseErr::InputTooLarge)?;
                for sli in self.sample_buf.iter() {
                    converter.add_sample(*sli)?;
                }
                self.converter = Some(converter);
                self.produce_chars()
            }
            None => Ok(Vec::new()),
            Some(converter) => converter.produce_chars(),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct MorseConverter<C>
where
    C: ArrayLength<SampledLightIntensity> + ArrayLength<TimedLightEvent> + ArrayLength<Morse>,
{
    samples: Queue<SampledLightIntensity, C, usize>,
    tles: Queue<TimedLightEvent, C, usize>,
    morses: Queue<Morse, C, usize>,
    hold_word: Queue<Morse, C, usize>,
    to_tles_init: (Time, LightState),
    cuts: IntensityCutoffs,
    morse_key: MorseKey,
    dark_push_time: Option<Time>,
    unit_time: MorseUnitTimeDecision,
}

fn queue_fill_vec<T, C>(mut q: Queue<T, C, usize>) -> (Queue<T, C, usize>, Vec<T, C>)
where
    T: Clone + core::fmt::Debug,
    C: ArrayLength<T>,
{
    let mut v = Vec::new();
    let mut newq = Queue::new();
    while !q.is_empty() {
        let item = q.dequeue().unwrap();
        v.push(item.clone()).unwrap();
        newq.enqueue(item).unwrap();
    }
    (newq, v)
}

impl<C> MorseConverter<C>
where
    C: ArrayLength<SampledLightIntensity> + ArrayLength<TimedLightEvent> + ArrayLength<Morse>,
{
    pub fn new(
        start_time: Time,
        unit_time: MorseUnitTimeDecision,
        cuts: IntensityCutoffs,
        dark_push_time: Option<Time>,
    ) -> Result<MorseConverter<C>, ()> {
        Ok(MorseConverter {
            samples: Queue::new(),
            tles: Queue::new(),
            morses: Queue::new(),
            hold_word: Queue::new(),
            cuts,
            to_tles_init: (start_time, LightState::Dark),
            morse_key: construct_key()?,
            dark_push_time,
            unit_time: unit_time,
        })
    }
    pub fn add_sample(&mut self, sample: SampledLightIntensity) -> Result<(), MorseErr> {
        match self.samples.enqueue(sample) {
            Ok(_) => Ok(()),
            Err(_) => Err(MorseErr::InputTooLarge),
        }
    }
    fn consume_samples(&mut self) -> Result<(), MorseErr> {
        let (_, mut consumer) = self.samples.split();
        let r = intensities_to_tles(
            &mut consumer,
            self.to_tles_init,
            self.cuts,
            self.dark_push_time,
        )
        .map_err(|e| MorseErr::FailedTLEConversion(e))?;
        let ConsumeSamplesInfo { tles, state } = r;
        for t in tles {
            self.tles.enqueue(t).map_err(|_| MorseErr::InputTooLarge)?;
        }
        self.to_tles_init = state;
        Ok(())
    }
    fn consume_tles(&mut self, unit_ms: Time) -> Result<(), MorseErr> {
        while !self.tles.is_empty() {
            let tle = self.tles.dequeue().ok_or(MorseErr::QueueBug)?;
            let m = tle_to_best_morse(&tle, unit_ms)?;
            self.morses
                .enqueue(m)
                .map_err(|_| MorseErr::InputTooLarge)?;
        }
        Ok(())
    }
    fn consume_morses<D>(&mut self) -> Result<Vec<char, D>, MorseErr>
    where
        D: ArrayLength<char>,
    {
        let mut outvec = Vec::new();
        let (_, mut producer) = self.morses.split();
        loop {
            //TODO! - remove the clone here
            let (char, newqueue) = definitive_consume_morses_produce_letter(
                &mut producer,
                self.hold_word.clone(),
                &self.morse_key,
            )?;
            self.hold_word = newqueue;
            match char {
                Some(c) => outvec.push(c).map_err(|_| MorseErr::InputTooLarge)?,
                None => break,
            }
        }
        Ok(outvec)
    }

    pub fn produce_chars<D>(&mut self) -> Result<Vec<char, D>, MorseErr>
    where
        D: ArrayLength<char>,
    {
        self.consume_samples()?;

        match self.unit_time {
            MorseUnitTimeDecision::EstimateProvided(unit_ms) => {
                self.consume_tles(unit_ms)?;
                self.consume_morses()
            }
            MorseUnitTimeDecision::EstimateToBeDetermined(DeriveUnitTimeConfig {
                guess_after_this_many_tles: cutoff,
                max_guess_ms: max,
                min_guess_ms: min,
            }) => {
                if self.tles.len() as u32 >= cutoff {
                    let v: Vec<_, C> = self.tles.iter().map(|x| x.clone()).collect();

                    self.unit_time = MorseUnitTimeDecision::EstimateProvided(
                        estimate_unit_time(&v[..], min, max)?.item,
                    );
                    self.produce_chars()
                } else {
                    Ok(Vec::new())
                }
            }
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct MorseCandidate {
    pub light_state: LightState,
    pub units: Time,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct IntensityCutoffs {
    pub low: LightIntensity,
    pub high: LightIntensity,
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

pub type MorseSequenceSerialization = (u8, u8);
pub type MorseKey = FnvIndexMap<MorseSequenceSerialization, char, U64>;

pub fn construct_key() -> Result<MorseKey, ()> {
    let elements = [
        ((2u8, 0b00000010u8), 'a'),
        ((4u8, 0b00000001u8), 'b'),
        ((4u8, 0b00000101u8), 'c'),
        ((3u8, 0b00000001u8), 'd'),
        ((1u8, 0b00000000u8), 'e'),
        ((4u8, 0b00000100u8), 'f'),
        ((3u8, 0b00000011u8), 'g'),
        ((4u8, 0b00000000u8), 'h'),
        ((2u8, 0b00000000u8), 'i'),
        ((4u8, 0b00001110u8), 'j'),
        ((3u8, 0b00000101u8), 'k'),
        ((4u8, 0b00000010u8), 'l'),
        ((2u8, 0b00000011u8), 'm'),
        ((2u8, 0b00000001u8), 'n'),
        ((3u8, 0b00000111u8), 'o'),
        ((4u8, 0b00000110u8), 'p'),
        ((4u8, 0b00001011u8), 'q'),
        ((3u8, 0b00000010u8), 'r'),
        ((3u8, 0b00000000u8), 's'),
        ((1u8, 0b00000001u8), 't'),
        ((3u8, 0b00000100u8), 'u'),
        ((4u8, 0b00001000u8), 'v'),
        ((3u8, 0b00000110u8), 'w'),
        ((4u8, 0b00001001u8), 'x'),
        ((4u8, 0b00001101u8), 'y'),
        ((4u8, 0b00000011u8), 'z'),
    ];

    let mut map: heapless::FnvIndexMap<_, _, U64> = heapless::FnvIndexMap::new();
    for ((count, rep), val) in elements.iter() {
        map.insert((*count, *rep), *val).map_err(|_| ())?;
    }
    Ok(map)
}

pub fn validate_morse_letter_tiny_spaces<C>(morse: Vec<Morse, C>) -> Result<Vec<Morse, C>, MorseErr>
where
    C: ArrayLength<Morse>,
{
    let mut expect_tiny_spaces = 0;
    let mut count_tiny_spaces = 0;

    let mut ret_vec = Vec::new();

    for m in morse.iter() {
        use Morse::*;
        let m = *m;
        match m {
            TinySpace => count_tiny_spaces += 1,
            Dot | Dash => {
                expect_tiny_spaces += 1;
                ret_vec.push(m).map_err(|_| MorseErr::InputTooLarge)?;
            }
            i => return Err(MorseErr::MorseInputCrossesLetterBound(i)),
        }

        if expect_tiny_spaces > count_tiny_spaces + 1 {
            return Err(MorseErr::InvalidLetterTinySpacing);
        }
    }

    Ok(ret_vec)
}

pub fn serialize_morse(morse: &[Morse]) -> Result<MorseSequenceSerialization, MorseErr> {
    if morse.len() == 0 {
        Err(MorseErr::EmptyInput)
    } else if morse.len() <= 8 {
        let mut rep = 0u8;
        let mut mask = 1u8;
        for m in morse {
            use Morse::*;
            let bit_set = match m {
                Dot => Ok(false),
                Dash => Ok(true),
                other => Err(MorseErr::MorseInputCrossesLetterBound(*other)),
            }?;
            if bit_set {
                rep |= mask;
            }
            mask <<= 1;
        }
        Ok((morse.len() as u8, rep))
    } else {
        Err(MorseErr::InputTooLarge)
    }
}

pub fn tle_to_best_morse(tle: &TimedLightEvent, unit_millis: Time) -> Result<Morse, MorseErr> {
    let c = best_error(tle, unit_millis)?;
    Ok(mc_to_morse(c.item)?)
}

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
    best.ok_or(MorseErr::BestErrorBug)
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
    let splits = 20;
    // Iterate over possible unit times from 1 to 5000 ms
    (0..splits)
        // For each time, score it by summing the scores of the best candidate for each event
        .map(|ratio| {
            let ratio = ratio as f32;
            let ratio = ratio / (splits as f32);
            let plus = (max_millis - min_millis) as f32 * ratio;
            let plus = plus as Time;
            score_possible_unit_millis(min_millis + plus, timings)
            // score_possible_unit_millis(ratio, timings)
        })
        // Converge on the minimum scoring unit time
        .fold(None, poisoned_min)
        // Ignore possible errors and pull out the best scoring unit time
        .unwrap_or(Err(MorseErr::EmptyInput))
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum CalcDigitalCutoffsErrs {
    TooBig(core::num::TryFromIntError),
    NoIntensities,
    NoLows,
    NoHighs,
}

pub fn calc_digital_cutoffs(
    intensities: &[SampledLightIntensity],
) -> Result<IntensityCutoffs, CalcDigitalCutoffsErrs> {
    use CalcDigitalCutoffsErrs::*;
    let mut intensity_sum: u32 = 0;

    for SampledLightIntensity {
        sample_time: _,
        intensity: li,
    } in intensities
    {
        intensity_sum += *li as u32;
    }

    if intensities.len() == 0 {
        Err(NoIntensities)?
    }

    let intensity_avg: u32 = intensity_sum / (intensities.len() as u32);

    let mut lows = (0u32, 0u32);
    let mut highs = (0u32, 0u32);
    for SampledLightIntensity {
        sample_time: _,
        intensity: li,
    } in intensities
    {
        let li = *li as u32;
        if li > intensity_avg {
            highs = (highs.0 + 1, highs.1 + li);
        } else {
            lows = (lows.0 + 1, lows.1 + li);
        }
    }

    if lows.0 == 0 {
        Err(NoLows)
    } else if highs.0 == 0 {
        Err(NoHighs)
    } else {
        let lows_avg = lows.1 / lows.0;
        let highs_avg = highs.1 / highs.0;

        let diff = highs_avg - lows_avg;
        let low_cut = lows_avg + (diff / 4);
        let high_cut = lows_avg + ((3 * diff) / 4);

        Ok(IntensityCutoffs {
            low: u16::try_from(low_cut).map_err(|e| TooBig(e))?,
            high: u16::try_from(high_cut).map_err(|e| TooBig(e))?,
        })
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum ConvertErrs {
    BadQueueCode,
    TooSmallOutgoingCapacity,
}

pub fn intensities_to_tles<C>(
    intensities: &mut Consumer<SampledLightIntensity, C, usize>,
    init: (Time, LightState),
    cuts: IntensityCutoffs,
    dark_push_time: Option<Time>,
) -> Result<ConsumeSamplesInfo<C>, ConvertErrs>
where
    C: heapless::ArrayLength<SampledLightIntensity> + ArrayLength<TimedLightEvent>,
{
    use ConvertErrs::*;
    use LightState::*;
    let (mut start_time, mut curr_light_state) = init;

    let mut out_vec: Vec<_, C> = Vec::new();

    while intensities.ready() {
        let it: Option<SampledLightIntensity> = intensities.dequeue();
        let SampledLightIntensity {
            sample_time: time,
            intensity: light,
        } = it.ok_or(BadQueueCode)?;

        let mut next_light_state = match (curr_light_state, light) {
            (Dark, light) if light > cuts.high => Some(Light),
            (Light, light) if light < cuts.low => Some(Dark),
            _ => None,
        };

        match (next_light_state, dark_push_time) {
            (None, Some(dark_push_time)) => {
                if time - start_time > dark_push_time && light < cuts.low {
                    next_light_state = Some(Dark);
                }
            }
            _ => (),
        };

        match next_light_state {
            Some(next_light_state) => {
                let tle = TimedLightEvent {
                    light_state: curr_light_state,
                    duration: time - start_time,
                };

                out_vec.push(tle).map_err(|_| TooSmallOutgoingCapacity)?;
                curr_light_state = next_light_state;
                start_time = time;
            }
            _ => (),
        };
    }
    Ok(ConsumeSamplesInfo {
        tles: out_vec,
        state: (start_time, curr_light_state),
    })
}

pub fn definitive_consume_morses_produce_letter<C>(
    incoming: &mut Consumer<Morse, C, usize>,
    mut hold_word: Queue<Morse, C, usize>,
    mkey: &MorseKey,
) -> Result<(Option<char>, Queue<Morse, C, usize>), MorseErr>
where
    C: ArrayLength<Morse>,
{
    loop {
        match private_consume_morses_produce_letter(incoming, hold_word, mkey) {
            Err(e) => break Err(e),
            Ok((None, q)) if incoming.ready() => hold_word = q,
            Ok((None, q)) => break Ok((None, q)),
            Ok((Some(c), q)) => break Ok((Some(c), q)),
        }
    }
}

fn private_consume_morses_produce_letter<C>(
    incoming: &mut Consumer<Morse, C, usize>,
    mut hold_word: Queue<Morse, C, usize>,
    mkey: &MorseKey,
) -> Result<(Option<char>, Queue<Morse, C, usize>), MorseErr>
where
    C: ArrayLength<Morse>,
{
    use Morse::*;
    if hold_word.peek() == Some(&WordSpace) {
        hold_word.dequeue();
        Ok((Some(' '), hold_word))
    } else {
        let mut next_queue = None;

        // We set next_queue based on the break condition
        while incoming.ready() && next_queue.is_none() {
            let morse = incoming.dequeue().ok_or(MorseErr::QueueBug)?;

            if morse == LetterSpace {
                next_queue = Some(Queue::new());
            } else if morse == WordSpace {
                let mut q = Queue::new();
                q.enqueue(WordSpace).map_err(|_| MorseErr::InputTooLarge)?;
                next_queue = Some(q);
            } else {
                hold_word
                    .enqueue(morse)
                    .map_err(|_| MorseErr::InputTooLarge)?;
            }
        }
        match next_queue {
            Some(next_queue) => {
                if hold_word.len() == 0 {
                    Ok((None, next_queue))
                } else if hold_word.len() <= 8 {
                    let v: Vec<Morse, U8> = hold_word.iter().map(|m| *m).collect();
                    let v = validate_morse_letter_tiny_spaces(v)?;

                    let ser = serialize_morse(&v[..])?;
                    let c = mkey.get(&ser).ok_or(MorseErr::UnknownChar(ser))?;
                    Ok((Some(*c), next_queue))
                } else {
                    Err(MorseErr::InputTooLarge)
                }
            }
            None => Ok((None, hold_word)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::consts::*;
    use heapless::Vec;
    extern crate std;
    use std::{cmp::min, println};

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
            estimate_unit_time(&timed_light_events, 0, 1000).unwrap()
        );
    }

    #[test]
    fn test_queue_convert() {
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
        ];

        use heapless::spsc::Queue;

        let mut sample_queue: Queue<_, U64, _> = Queue::new();
        let results = my_intensities
            .iter()
            .map(|(i, t)| SampledLightIntensity {
                sample_time: *t,
                intensity: *i,
            })
            .try_for_each(|i| sample_queue.enqueue(i));

        assert_eq!(true, results.is_ok());

        let (mut consumer, mut producer) = sample_queue.split();

        let popresult = intensities_to_tles(
            &mut producer,
            (0, LightState::Dark),
            IntensityCutoffs {
                low: 200,
                high: 800,
            },
            None,
        )
        .unwrap();

        let rmorses: Result<Vec<_, U64>, _> = popresult
            .tles
            .iter()
            .map(|t| tle_to_best_morse(t, 20))
            .collect();
        let mut morses = rmorses.unwrap();

        consumer
            .enqueue(SampledLightIntensity {
                sample_time: 520,
                intensity: 100,
            })
            .unwrap();
        for i in 0..3 {
            consumer
                .enqueue(SampledLightIntensity {
                    sample_time: 540 + (40 * i),
                    intensity: 900,
                })
                .unwrap();
            consumer
                .enqueue(SampledLightIntensity {
                    sample_time: 560 + (40 * i),
                    intensity: 100,
                })
                .unwrap();
        }

        let popresult = intensities_to_tles(
            &mut producer,
            popresult.state,
            IntensityCutoffs {
                low: 200,
                high: 800,
            },
            None,
        );

        let rmorses: Result<Vec<_, U64>, _> = popresult
            .unwrap()
            .tles
            .iter()
            .map(|t| tle_to_best_morse(t, 20))
            .collect();
        let latemorses = rmorses.unwrap();
        morses.extend_from_slice(&latemorses).unwrap();

        use Morse::*;
        assert_eq!(
            &[
                LetterSpace,
                Dash,
                TinySpace,
                Dot,
                TinySpace,
                Dot,
                TinySpace,
                Dot,
                WordSpace,
                Dot,
                TinySpace,
                Dot,
                TinySpace,
                Dot
            ],
            &morses[..]
        );
    }

    #[test]
    fn test_lookup() {
        use Morse::*;
        let arr = [Dash, Dot, Dot, Dot];
        let key = construct_key().unwrap();
        let ser = serialize_morse(&arr).unwrap();
        assert_eq!(Some(&'b'), key.get(&ser));
    }

    #[test]
    fn test_consume() {
        use Morse::*;

        let key = construct_key().unwrap();

        let mut morse_queue: Queue<_, U64, _> = Queue::new();
        let (mut consumer, mut producer) = morse_queue.split();

        consumer.enqueue(Dot).unwrap();
        consumer.enqueue(TinySpace).unwrap();
        consumer.enqueue(Dot).unwrap();
        consumer.enqueue(TinySpace).unwrap();

        let q = Queue::new();

        let (char, q) = private_consume_morses_produce_letter(&mut producer, q, &key).unwrap();
        assert_eq!(None, char);

        consumer.enqueue(Dot).unwrap();

        let (char, q) = private_consume_morses_produce_letter(&mut producer, q, &key).unwrap();
        assert_eq!(None, char);

        consumer.enqueue(LetterSpace).unwrap();

        let (char, q) = private_consume_morses_produce_letter(&mut producer, q, &key).unwrap();
        assert_eq!(Some('s'), char);
        assert!(q.is_empty());
    }

    #[test]
    fn test_consume2() {
        use Morse::*;

        let key = construct_key().unwrap();

        let mut morse_queue: Queue<_, U64, _> = Queue::new();
        let (mut consumer, mut producer) = morse_queue.split();

        consumer.enqueue(Dot).unwrap();
        consumer.enqueue(TinySpace).unwrap();
        consumer.enqueue(Dot).unwrap();
        consumer.enqueue(TinySpace).unwrap();
        consumer.enqueue(Dot).unwrap();
        consumer.enqueue(LetterSpace).unwrap();
        consumer.enqueue(LetterSpace).unwrap();
        consumer.enqueue(Dot).unwrap();
        consumer.enqueue(TinySpace).unwrap();
        consumer.enqueue(Dot).unwrap();
        consumer.enqueue(TinySpace).unwrap();
        consumer.enqueue(Dot).unwrap();
        consumer.enqueue(LetterSpace).unwrap();
        consumer.enqueue(WordSpace).unwrap();
        consumer.enqueue(LetterSpace).unwrap();
        consumer.enqueue(WordSpace).unwrap();
        consumer.enqueue(LetterSpace).unwrap();
        consumer.enqueue(Dash).unwrap();
        consumer.enqueue(TinySpace).unwrap();
        consumer.enqueue(Dot).unwrap();
        consumer.enqueue(TinySpace).unwrap();
        consumer.enqueue(Dot).unwrap();
        consumer.enqueue(TinySpace).unwrap();
        consumer.enqueue(Dot).unwrap();
        consumer.enqueue(LetterSpace).unwrap();
        consumer.enqueue(Dot).unwrap();
        consumer.enqueue(LetterSpace).unwrap();
        consumer.enqueue(LetterSpace).unwrap();

        let mut cvec: Vec<char, U64> = Vec::new();
        let mut q = Queue::new();

        loop {
            let (char, newqueue) =
                definitive_consume_morses_produce_letter(&mut producer, q, &key).unwrap();
            q = newqueue;
            println!("{:?}", q);
            match char {
                Some(c) => cvec.push(c).unwrap(),
                None => break,
            }
        }

        assert_eq!(['s', 's', ' ', ' ', 'b', 'e'], cvec[..])
    }

    #[test]
    fn test_object() {
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
            (100, 800),
        ];

        let mut converter: MorseConverter<U64> = MorseConverter::new(
            0,
            MorseUnitTimeDecision::EstimateProvided(20),
            IntensityCutoffs {
                low: 200,
                high: 800,
            },
            Some(200),
        )
        .unwrap();

        for (light, time) in my_intensities.iter() {
            converter
                .add_sample(SampledLightIntensity {
                    intensity: *light,
                    sample_time: *time,
                })
                .unwrap();
        }

        let vec: Vec<_, U32> = converter.produce_chars().unwrap();

        assert_eq!(&['b', ' ', 'e', ' '], &vec[..]);
    }

    #[test]
    fn test_manager() {
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
            converter
                .add_sample(SampledLightIntensity {
                    intensity: *light,
                    sample_time: *time,
                })
                .unwrap();
        }

        let vec: Vec<_, U32> = converter.produce_chars().unwrap();

        assert_eq!(&['b', ' ', 'e', 'd', 'o', 'g', ' '], &vec[..]);
    }
}

pub fn mc_to_morse(mc: &MorseCandidate) -> Result<Morse, MorseErr> {
    use Morse::*;
    match mc {
        MorseCandidate {
            light_state: LightState::Light,
            units: 1,
        } => Ok(Dot),
        MorseCandidate {
            light_state: LightState::Light,
            units: 3,
        } => Ok(Dash),
        MorseCandidate {
            light_state: LightState::Dark,
            units: 1,
        } => Ok(TinySpace),
        MorseCandidate {
            light_state: LightState::Dark,
            units: 3,
        } => Ok(LetterSpace),
        MorseCandidate {
            light_state: LightState::Dark,
            units: 7,
        } => Ok(WordSpace),
        _ => Err(MorseErr::InvalidMorseCandidate(mc.clone())),
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
