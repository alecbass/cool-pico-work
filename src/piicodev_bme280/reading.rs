use defmt::info;

pub struct Reading {
    /** Temperature in celsius */
    pub temperature: f32,
    /** Air pressure in hPa */
    pub pressure: f32,
    /** Relative humidity percentage */
    pub humidity: f32,
    /** Altitude in meteres */
    pub altitude: f32,
}

impl Reading {
    pub fn report(&self) {
        info!(
            "READINGGG {} {} {} {}",
            self.temperature, self.pressure, self.humidity, self.altitude
        );
    }
}
