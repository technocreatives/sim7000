use heapless::{String, Vec};

pub enum Command {
    Text(String<256>),
    Binary(Vec<u8, 256>),
}

impl From<&'_ str> for Command {
    fn from(s: &'_ str) -> Self {
        Command::Text(s.into())
    }
}

impl Command {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Command::Text(s) => s.as_bytes(),
            Command::Binary(b) => b,
        }
    }
}
