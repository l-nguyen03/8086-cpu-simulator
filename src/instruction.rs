use std::fmt;

pub struct Instruction {
    mnemonic: String,
    dst: String,
    src: Option<String>,
}

impl Instruction {
    pub fn binary(
        mnemonic: impl Into<String>,
        dst: impl Into<String>,
        src: impl Into<String>,
    ) -> Self {
        Self {
            mnemonic: mnemonic.into(),
            dst: dst.into(),
            src: Some(src.into()),
        }
    }

    pub fn unary(mnemonic: impl Into<String>, dst: impl Into<String>) -> Self {
        Self {
            mnemonic: mnemonic.into(),
            dst: dst.into(),
            src: None,
        }
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.src {
            Some(src) => write!(f, "{} {}, {}", self.mnemonic, self.dst, src),
            None => write!(f, "{} {}", self.mnemonic, self.dst),
        }
    }
}
