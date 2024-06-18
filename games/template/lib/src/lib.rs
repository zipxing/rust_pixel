#![allow(dead_code)]

use std::fmt::{self, Display, Formatter};

#[repr(C)]
#[derive(Ord, PartialOrd, Eq, Copy, Clone, PartialEq)]
pub struct TemplateData {
    pub number: u8,
}

impl TemplateData {
    pub fn new(v: u8) -> Result<Self, String> {
        match v {
            1..=52 => {
                number = (v - 1) % 13 + 1;
            }
            _ => return Err(String::from(format!("invaild number:{:?}", v))),
        }
        Ok(TemplateData { number })
    }

    pub fn add_one(&mut self) {
        self.number.wrapping_add(1);
    }
}

impl Display for TemplateData {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "template_data: number is {}", self.number)
    }
}

impl fmt::Debug for TemplateData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let result = TemplateData::new(2 + 2).unwrap();
        result.add_one();
        assert_eq!(result.number, 5);
    }
}
