use nom::{self, Err, ErrorKind, IResult, Needed};

#[derive(Debug)]
pub enum Error {
    Nom(u32),
    Marker(u8),
}

pub fn parse_array_length(input: &[u8]) -> IResult<&[u8], u32, Error> {
    match input.get(0) {
        Some(&b) if b & 0b1111_0000 == 0b1001_0000 => IResult::Done(&input[1..], b as u32 & 0b1111),
        Some(&0xdc) => nom::be_u16(&input[1..]).map(|n| n as u32).map_err(|n| unreachable!()),
        Some(&0xdd) => nom::be_u32(&input[1..]).map_err(|n| unreachable!()),
        Some(&b) => IResult::Error(Err::Position(ErrorKind::Custom(Error::Marker(b)), &input[..1])),
        None => IResult::Incomplete(Needed::Size(1)),
    }
}
