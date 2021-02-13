use core::{cmp::min, convert::TryFrom, num::TryFromIntError};

pub fn be_busted() -> Result<u8, core::num::TryFromIntError> {
    let z = 2567;
    let u: u8 = u8::try_from(z)?;
    Ok(u)
}
