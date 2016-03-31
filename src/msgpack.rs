use nom::{self, Err, ErrorKind, IResult, Needed};

#[derive(Debug)]
pub enum Error {
    Nom(u32),
    Marker(u8),
}

pub enum Integer {
    Unsigned(u64),
    Signed(i64),
}

pub fn parse_array_length(input: &[u8]) -> IResult<&[u8], u32, Error> {
    match input.get(0) {
        Some(&b) if b & 0b1111_0000 == 0b1001_0000 => IResult::Done(&input[1..], b as u32 & 0b1111),
        Some(&0xdc) => nom::be_u16(&input[1..]).map(|n| n as u32).map_err(|err| unreachable!()),
        Some(&0xdd) => nom::be_u32(&input[1..]).map_err(|err| unreachable!()),
        Some(&b) => IResult::Error(Err::Position(ErrorKind::Custom(Error::Marker(b)), &input[..1])),
        None => IResult::Incomplete(Needed::Size(1)),
    }
}

pub fn parse_integer(input: &[u8]) -> IResult<&[u8], Integer, Error> {
    match input.get(0) {
        Some(&b) if b & 0b1_0000000 == 0 => IResult::Done(&input[1..], Integer::Unsigned(b as u64)),
        Some(&b) if b & 0b111_00000 == 0b111_00000 => IResult::Done(&input[1..], Integer::Signed(b as i8 as i64)),

        Some(&0xcc) => nom::be_u8(&input[1..]).map(|n| Integer::Unsigned(n as u64)).map_err(|n| unreachable!()),
        Some(&0xcd) => nom::be_u16(&input[1..]).map(|n| Integer::Unsigned(n as u64)).map_err(|n| unreachable!()),
        Some(&0xce) => nom::be_u32(&input[1..]).map(|n| Integer::Unsigned(n as u64)).map_err(|n| unreachable!()),
        Some(&0xcf) => nom::be_u64(&input[1..]).map(|n| Integer::Unsigned(n)).map_err(|n| unreachable!()),

        Some(&0xd0) => nom::be_u8(&input[1..]).map(|n| Integer::Signed(n as i8 as i64)).map_err(|n| unreachable!()),
        Some(&0xd1) => nom::be_u16(&input[1..]).map(|n| Integer::Signed(n as i16 as i64)).map_err(|n| unreachable!()),
        Some(&0xd2) => nom::be_u32(&input[1..]).map(|n| Integer::Signed(n as i32 as i64)).map_err(|n| unreachable!()),
        Some(&0xd3) => nom::be_u64(&input[1..]).map(|n| Integer::Signed(n as i64)).map_err(|n| unreachable!()),

        Some(&b) => IResult::Error(Err::Position(ErrorKind::Custom(Error::Marker(b)), &input[..1])),

        None => IResult::Incomplete(Needed::Size(1)),
    }
}
