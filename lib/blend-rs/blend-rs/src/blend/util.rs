use std::str::Utf8Error;


pub trait StringLike {

    fn to_str(&self) -> Result<&str, Utf8Error>;

    fn to_str_unchecked(&self) -> &str {
        self.to_str().expect("Failed to extract &str!")
    }

    fn to_string(&self) -> Result<String, Utf8Error> {
        self.to_str().map(|value| String::from(value))
    }

    fn to_string_unchecked(&self) -> String {
        self.to_string().expect("Failed to extract String!")
    }
}

impl <A> StringLike for A
where A: AsRef<[i8]> {

    fn to_str(&self) -> Result<&str, Utf8Error> {
        let self_ref = self.as_ref();
        if !self_ref.is_empty() {
            let slice: &[u8] = unsafe {
                core::slice::from_raw_parts(self_ref.as_ptr() as *const u8, self_ref.len())
            };
            let null = slice.iter()
                .position(|element| *element == 0x00)
                .unwrap_or(slice.len());
            std::str::from_utf8(&slice[0..null])
        }
        else {
            Ok("")
        }
    }
}

const NAME_PREFIXES: [&str; 17] = [
    "OB", "ME", "WM", "IM", "SN",
    "WS", "BR", "SC", "PL", "OB",
    "GR", "CA", "LA", "ME", "WO",
    "LS", "MA",
];

pub trait NameLike {

    fn to_name_str(&self) -> Result<&str, Utf8Error>;

    fn to_name_string(&self) -> Result<String, Utf8Error> {
        self.to_name_str().map(|value| String::from(value))
    }

    fn to_name_str_unchecked(&self) -> &str {
        self.to_name_str().expect("Failed to convert to name!")
    }

    fn to_name_string_unchecked(&self) -> String {
        self.to_name_string().expect("Failed to convert to name!")
    }
}

impl <A> NameLike for A
    where A: StringLike {

    fn to_name_str(&self) -> Result<&str, Utf8Error> {
        self.to_str().map(|value| {
            if NAME_PREFIXES.contains(&&value[0..2]) {
                &value[2..]
            }
            else {
                &value
            }
        })
    }
}
