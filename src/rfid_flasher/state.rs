/// The mode the program is currently running in
#[derive(PartialEq, Eq)]
enum Mode {
    /// Reading an RFID chip
    Read,
    /// Writing to an RFID chip
    Write,
}

impl Into<u32> for Mode {
    fn into(self) -> u32 {
        match self {
            Mode::Read => 0,
            Mode::Write => 1,
        }
    }
}

/// The state of the RFID flasher application
pub struct State {
    /// Which mode is the application running in?
    mode: Mode,
}

impl State {
    pub fn new() -> Self {
        Self { mode: Mode::Read }
    }

    pub fn toggle_mode(&mut self) {
        if let Mode::Read = self.mode {
            self.mode = Mode::Write;
        } else {
            self.mode = Mode::Read;
        }
    }

    pub fn is_reading(&self) -> bool {
        self.mode == Mode::Read
    }
}
