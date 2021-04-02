



            use embedded_hal::digital::v2::OutputPin;

pub struct LcdPin<'a>
{
    pin: &'a mut dyn OutputPin<Error = ()>,
}

impl<'a> LcdPin<'a>
{
    pub fn new(pin: &'a mut dyn OutputPin<Error = ()>) -> Self
    {
LcdPin
{
   pin 
}
    }

    pub fn set_low(&mut self) -> Result<(), ()>
    {
    self.pin.set_low()
    }
    pub fn set_high(&mut self) -> Result<(), ()>
    {
    self.pin.set_high()
    }
} 

