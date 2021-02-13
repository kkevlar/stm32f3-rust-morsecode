
use anyhow::Result;
use core::{cmp::min, convert::TryFrom};

fn be_busted() -> Result<u8>
{
    let z = 2567;
    let u : u8 = u8::try_from(z)?;
    Ok(u);
}

