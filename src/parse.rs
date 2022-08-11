use nom::bytes::complete::{tag, take_until};
use nom::sequence::preceded;
use nom::IResult;
use nom::Parser;

const COLON: &str = ":";

fn first_colon() -> impl FnMut(&[u8]) -> IResult<&[u8], ()> {
    move |i| preceded(take_until(COLON), tag(COLON)).map(|_| ()).parse(i)
}

fn second_colon() -> impl FnMut(&[u8]) -> IResult<&[u8], &[u8]> {
    move |i| {
        take_until(COLON)
            .and(tag(COLON))
            .map(|(encoded_bin, _colon)| encoded_bin)
            .parse(i)
    }
}

pub fn parse(i: &[u8]) -> IResult<&[u8], &[u8]> {
    let encoded_bin_with_newlines = first_colon()
        .and(second_colon())
        .map(|(_first_colon, second_colon)| second_colon)
        .parse(i)
        .map(|(_, out)| out)?;

    Ok((&[], encoded_bin_with_newlines))
}
