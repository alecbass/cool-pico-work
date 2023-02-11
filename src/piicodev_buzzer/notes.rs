use libm::powf;

pub enum Note {
    Rest = 0,
    C3 = 48,
    C3S = 49,
    D3 = 50,
    D3S = 51,
    E3 = 52,
    F3 = 53,
    F3S = 54,
    G3 = 55,
    G3S = 56,
    A3 = 57,
    A3S = 58,
    B3 = 59,
    C4 = 60,
    C4S = 61,
    D4 = 62,
    D4S = 63,
    E4 = 64,
    F4 = 65,
    F4S = 66,
    G4 = 67,
    G4S = 68,
    A4 = 69,
    B4 = 71,
    C5 = 72,
    C5S = 73,
    D5 = 74,
    D5S = 75,
    E5 = 76,
    F5 = 77,
    F5S = 78,
    G5 = 79,
    G5S = 80,
    A5 = 81,
    A5S = 82,
    B5 = 83,
    C6 = 84,
    C6S = 85,
    D6 = 86,
    D6S = 87,
    E6 = 88,
    F6 = 89,
    F6S = 90,
    G6 = 91,
    G6S = 92,
    A6 = 93,
    B6 = 94,
    B6S = 95,
}

pub fn note_to_frequency(key: Note) -> f32 {
    let note: f32 = key as u32 as f32;

    // 2 ** ((key - 69) / 12) * 440;
    let x2: f32 = 2.0;
    powf(x2, (note - 69.0) / 12.0) * 440.0
}

pub const EIGHT_MELODIES: [(Note, u16); 41] = [
    (Note::C5, 1000),
    (Note::D5, 1000),
    (Note::E5, 1000),
    (Note::G5, 1000),
    (Note::D5, 4000),
    (Note::C6, 1000),
    (Note::B5, 1000),
    (Note::A5, 1000),
    (Note::E5, 1000),
    (Note::G5, 4000),
    (Note::A5, 1000),
    (Note::B5, 1000),
    (Note::C6, 1000),
    (Note::G5, 2000),
    (Note::C5, 3500),
    (Note::F5, 1000),
    (Note::E5, 1000),
    (Note::C5, 1000),
    (Note::G4, 5000),
    (Note::A4, 2000),
    (Note::B4, 2000),
    (Note::C5, 2000),
    (Note::F5, 2000),
    (Note::E5, 1000),
    (Note::F5, 1000),
    (Note::G5, 1000),
    (Note::E5, 1000),
    (Note::D5, 1000),
    (Note::E5, 1000),
    (Note::F5, 1000),
    (Note::D5, 1000),
    (Note::A4, 1000),
    (Note::F5, 1000),
    (Note::E5, 1000),
    (Note::G4, 1000),
    (Note::D5, 2000),
    (Note::B4, 2000),
    (Note::C5, 2000),
    (Note::G4, 1000),
    (Note::D5, 1000),
    (Note::C5, 4000),
];
