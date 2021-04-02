



            use embedded_hal::digital::v2::OutputPin;

pub trait Delay {
    fn delay_us(self, us : u32) -> ();
    fn delay_ms(self, ms : u32) -> ();
}

pub struct LcdPin<'a>
{
    pin: &'a mut dyn OutputPin<Error = ()>,
}

pub enum DataPinCollection<'a>
{
    Four([LcdPin<'a>; 4]),
    Eight([LcdPin<'a>; 8]),
}

pub struct LcdObject<'a, 'b, 'c, 'd, 'e>
{
    pub data_pins : DataPinCollection<'a>,
    pub control_enable_pin : LcdPin<'b>,
    pub control_rs_pin : LcdPin<'c>,
    pub control_rw_pin : LcdPin<'d>,
    pub delay : &'e dyn Delay,
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

impl<'a,'b,'c,'d,'e,> LcdObject<'a,'b,'c,'d,'e,>
{
   pub fn initialize(&mut self) -> Result<(), ()> 
   {
       unimplemented!()
   }

   fn send_generic_4(&mut self,  rs: bool) -> Result<(), ()>
   {
       self.control_enable_pin.set_low()?
       self.control_rw_pin.set_low()?

       if rs{
           self.control_rs_pin.set_high()?
       }
       else
       {

           self.control_rs_pin.set_low()?
       }



   }
}
