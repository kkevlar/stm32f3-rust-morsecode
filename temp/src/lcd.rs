use core::u32;

use embedded_hal::digital::v2::OutputPin;

pub trait Delay {
    fn delay_us(&self, us: u32) -> ();
    fn delay_ms(&self, ms: u32) -> ();
}

pub struct LcdPin<'a> {
    pin: &'a mut dyn OutputPin<Error = ()>,
}

pub enum DataPinCollection<'a> {
    Four([LcdPin<'a>; 4]),
    Eight([LcdPin<'a>; 8]),
}
pub enum LcdCommand {
    ReturnHome,
    ReturnHomeAlt,
    ClearDisplay,
}

enum PrivateLcdCommand {
    SetDefault8Bit,
    SetDefault4Bit,
    DisplayOnCursorBlink,
    EntryModeDefault,
    FunctionSet4Bit1,
    FunctionSet4Bit2,
}

enum InternalLcdCommand {
    PrivateCommand(PrivateLcdCommand),
    PublicCommand(LcdCommand),
    RawCommand(u8),
}

#[derive(PartialEq, Eq, Debug)]
enum LcdRsValue {
    Command,
    Char,
}

enum FourPinNibbleChoice {
    LowNibble,
    HighNibble,
}

impl InternalLcdCommand {
    pub fn get_numeric(&self) -> u8 {
        use InternalLcdCommand::*;
        use LcdCommand::*;
        use PrivateLcdCommand::*;
        match self {
            PublicCommand(ReturnHome) => 0b00000010,
            PublicCommand(ClearDisplay) => 0b0000001,
            PublicCommand(ReturnHomeAlt) => 0b00000011,
            PrivateCommand(SetDefault8Bit) => 0b00111000,
            PrivateCommand(DisplayOnCursorBlink) => 0b00001111,
            PrivateCommand(EntryModeDefault) => 0b00000110,
            PrivateCommand(FunctionSet4Bit1) => 0b00110000,
            PrivateCommand(FunctionSet4Bit2) => 0b00100000,
            PrivateCommand(SetDefault4Bit) => 0b00101000,
            RawCommand(me) => *me,
        }
    }
}

struct LcdInternal<'b, 'c, 'd, 'e> {
    pub control_enable_pin: LcdPin<'b>,
    pub control_rs_pin: LcdPin<'c>,
    pub control_rw_pin: LcdPin<'d>,
    pub delay: &'e dyn Delay,
}

pub struct LcdObject<'a, 'b, 'c, 'd, 'e> {
    pub data_pins: DataPinCollection<'a>,
    lcd_internal: LcdInternal<'b, 'c, 'd, 'e>,
}

impl<'a> LcdPin<'a> {
    pub fn new(pin: &'a mut dyn OutputPin<Error = ()>) -> Self {
        LcdPin { pin }
    }

    pub fn set_low(&mut self) -> Result<(), ()> {
        self.pin.set_low()
    }
    pub fn set_high(&mut self) -> Result<(), ()> {
        self.pin.set_high()
    }
}

impl<'a, 'b, 'c, 'd, 'e> LcdObject<'a, 'b, 'c, 'd, 'e> {
    pub fn new(
        data_pins: DataPinCollection<'a>,
        control_enable_pin: LcdPin<'b>,
        control_rs_pin: LcdPin<'c>,
        control_rw_pin: LcdPin<'d>,
        delay: &'e dyn Delay,
    ) -> LcdObject<'a, 'b, 'c, 'd, 'e> {
        LcdObject {
            lcd_internal: LcdInternal {
                control_enable_pin,
                control_rs_pin,
                control_rw_pin,
                delay,
            },
            data_pins,
        }
    }

    pub fn initialize(&mut self) -> Result<(), ()> {
        use DataPinCollection::*;
        match &mut self.data_pins {
            Four(four) => {
                use FourPinNibbleChoice::*;
                use InternalLcdCommand::*;
                use PrivateLcdCommand::*;

                for _ in 0..3 {
                    self.lcd_internal.private_load_n_pulse4(
                        four,
                        PrivateCommand(FunctionSet4Bit1).get_numeric(),
                        HighNibble,
                    )?;
                    self.lcd_internal.delay.delay_us(40);
                }
                self.lcd_internal.private_load_n_pulse4(
                    four,
                    PrivateCommand(FunctionSet4Bit2).get_numeric(),
                    HighNibble,
                )?;
                self.lcd_internal.delay.delay_us(40);
                self.send_command_internal(PrivateCommand(SetDefault4Bit))?;
                self.send_command_internal(PrivateCommand(SetDefault4Bit))?;

                Ok(())
            }

            Eight(_) => {
                unimplemented!()
            }
        }
    }

    fn send_generic(&mut self, data: u8, rs: LcdRsValue) -> Result<(), ()> {
        use DataPinCollection::*;
        match &mut self.data_pins {
            Four(four) => self.lcd_internal.send_generic_4(four, data, rs),

            Eight(_) => unimplemented!(),
        }
    }

    pub fn send_command(&mut self, command: LcdCommand) -> Result<(), ()> {
        self.send_command_internal(InternalLcdCommand::PublicCommand(command))
    }

    fn send_command_internal(&mut self, command: InternalLcdCommand) -> Result<(), ()> {
        self.send_generic(command.get_numeric(), LcdRsValue::Command)?;

        use LcdCommand::*;
        match command {
            InternalLcdCommand::PublicCommand(ClearDisplay)
            | InternalLcdCommand::PublicCommand(ReturnHome)
            | InternalLcdCommand::PublicCommand(ReturnHomeAlt) => {
                self.lcd_internal.delay.delay_us(1520)
            }

            _ => self.lcd_internal.delay.delay_us(80),
        };

        Ok(())
    }

    pub fn send_char(&mut self, c: char) -> Result<(), ()> {
        self.send_generic(c as u8, LcdRsValue::Char)
    }

    pub fn set_cursor(&mut self, row: u8, col: u8) -> Result<(), ()> {
        let temp = row * 0x40 + col;
        let temp = temp | 0x80;
        self.send_command_internal(InternalLcdCommand::RawCommand(temp))
    }
}

impl<'b, 'c, 'd, 'e> LcdInternal<'b, 'c, 'd, 'e> {
    fn private_load_n_pulse4(
        &mut self,
        data_pins: &mut [LcdPin; 4],
        data: u8,
        nibble: FourPinNibbleChoice,
    ) -> Result<(), ()> {
        let mask_start = match nibble {
            FourPinNibbleChoice::HighNibble => 0x10,
            FourPinNibbleChoice::LowNibble => 0x01,
        };

        for i in 0..4 {
            // What should the mask be?
            let mask = mask_start << i;

            // Use the mask to set the data pin high or low
            if data & mask > 0 {
                data_pins[i].set_high()?;
            } else {
                data_pins[i].set_low()?;
            }
        }
        self.control_enable_pin.set_high()?;
        self.delay.delay_us(1);
        self.control_enable_pin.set_low()?;

        Ok(())
    }

    fn send_generic_4(
        &mut self,
        data_pins: &mut [LcdPin; 4],
        data: u8,
        rs: LcdRsValue,
    ) -> Result<(), ()> {
        if rs == LcdRsValue::Char {
            self.control_rs_pin.set_high()?;
        } else {
            self.control_rs_pin.set_low()?;
        }

        self.control_enable_pin.set_low()?;
        self.control_rw_pin.set_low()?;

        use FourPinNibbleChoice::*;

        self.private_load_n_pulse4(data_pins, data, HighNibble)?;
        self.delay.delay_us(40);
        self.private_load_n_pulse4(data_pins, data, LowNibble)?;

        Ok(())
    }
}
