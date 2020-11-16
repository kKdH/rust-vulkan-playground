use crate::util::Version;

impl From<&Version> for u32 {
    fn from(value: &Version) -> Self {
        ((value.major) << 22) | ((value.minor) << 12) | (value.patch)
    }
}

impl From<u32> for Version {
    fn from(value: u32) -> Self {
        Version {
            major: value >> 22,
            minor: (value >> 12) & 0x3ff,
            patch: value & 0xfff
        }
    }
}
