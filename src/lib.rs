#![feature(box_syntax)]

pub type Array = Vec<JsonObject>;
pub type Object = Vec<(String, JsonObject)>;

#[derive(Debug, PartialEq)]
pub enum JsonObject {
    Object(Object),
    Array(Array),
    String(String),
    Boolean(bool),
    Number(f64),
    Null,
}

impl JsonObject {
    pub fn object(self) -> Option<Object> {
        match self {
            JsonObject::Object(object) => Some(object),
            _ => None,
        }
    }

    pub fn array(self) -> Option<Array> {
        match self {
            JsonObject::Array(array) => Some(array),
            _ => None,
        }
    }

    pub fn boolean(self) -> Option<bool> {
        match self {
            JsonObject::Boolean(boolean) => Some(boolean),
            _ => None,
        }
    }

    pub fn is_null(self) -> bool {
        matches!(self, JsonObject::Null)
    }
}

#[derive(Debug, PartialEq)]
pub enum JsonError {
    UnexpectedChar(char),
    UnexpectedKeyword,
    UnknownEscapeCharacter(char),
    ExtraChars(Vec<char>),
    EarlyEndOfStream,
    LeadingZero,
}

#[inline]
pub fn parse_json_string(json_str: &str) -> Result<JsonObject, JsonError> {
    return parse_json_from_iter(&mut json_str.chars());
}

#[inline]
pub fn parse_json_from_iter(
    json_iter: &mut dyn Iterator<Item = char>,
) -> Result<JsonObject, JsonError> {
    use core::iter::once;

    let (value, excess) = parse_json_impl(json_iter)?;

    let mut should_be_empty = excess
        .into_iter()
        .chain(json_iter)
        .skip_while(|ch| ch.is_whitespace());

    if let Some(ch) = should_be_empty.next() {
        Err(JsonError::ExtraChars(
            once(ch).chain(should_be_empty).collect(),
        ))
    } else {
        Ok(value)
    }
}

fn parse_json_impl(
    json_iter: &mut dyn Iterator<Item = char>,
) -> Result<(JsonObject, Option<char>), JsonError> {
    let mut chars = json_iter.skip_while(|ch| ch.is_whitespace());

    let result = match chars.next().ok_or(JsonError::EarlyEndOfStream)? {
        //_n_ull
        'n' => parse_null_impl(&mut chars),
        //_t_rue
        't' => parse_true_impl(&mut chars),
        //_f_alse
        'f' => parse_false_impl(&mut chars),
        //array
        '[' => parse_array_impl(&mut chars).map(JsonObject::Array),
        //string
        '"' => parse_string_impl(&mut chars).map(JsonObject::String),
        //object
        '{' => parse_object_impl(&mut chars).map(JsonObject::Object),
        //has to be a number
        ch @ _ => {
            return parse_number_impl(json_iter, ch)
                .map(|(n, excess)| (JsonObject::Number(n), excess));
        }
    };

    result.map(|obj| (obj, None))
}

fn parse_number_impl(
    iter: &mut dyn Iterator<Item = char>,
    starting_character: char,
) -> Result<(f64, Option<char>), JsonError> {
    let sign;

    let first_char = match starting_character {
        '-' => {
            sign = -1.;
            iter.next().ok_or(JsonError::EarlyEndOfStream)?
        }
        other @ _ => {
            sign = 1.;
            other
        }
    };

    let mut number = match first_char {
        digit @ '1'..='9' => digit.to_digit(10).unwrap() as f64,
        //no leading 0 allowed other than for fraction
        '0' => match iter.next().ok_or(JsonError::EarlyEndOfStream)? {
            '.' => return parse_fraction_part_impl(iter).map(|(number, ch)| (number * sign, ch)),
            _ => return Err(JsonError::LeadingZero),
        },
        _ => return Err(JsonError::UnexpectedChar(first_char)),
    };

    loop {
        match iter.next() {
            Some(digit @ '0'..='9') => {
                number *= 10.;
                number += digit.to_digit(10).unwrap() as f64;
            }
            Some('.') => {
                return parse_fraction_part_impl(iter)
                    .map(|(fraction, ch)| ((number + fraction) * sign, ch));
            }
            //jesus…
            option @ _ => return Ok((number * sign, option)),
        }
    }
}

//to be called when '.' is encountered while parsing number, should return a fraction (0.something)
fn parse_fraction_part_impl(
    iter: &mut dyn Iterator<Item = char>,
) -> Result<(f64, Option<char>), JsonError> {
    let mut number = 0.;

    for n in 1.. {
        match iter.next() {
            Some(digit @ '0'..='9') => {
                let digit = digit.to_digit(10).unwrap() as f64;
                number += digit / 10_f64.powi(n);
            }
            //jesus…
            option @ _ => return Ok((number, option)),
        }
    }

    unreachable!();
}

//expects starting '"' to already be eaten
fn parse_string_impl(json_iter: &mut dyn Iterator<Item = char>) -> Result<String, JsonError> {
    let mut result = String::new();

    loop {
        match json_iter.next().ok_or(JsonError::EarlyEndOfStream)? {
            '"' => {
                return Ok(result);
            }
            '\\' => result.push(parse_escape_character_impl(json_iter)?),
            ch @ _ => {
                result.push(ch);
            }
        }
    }
}

//expects '\' to already be eaten
fn parse_escape_character_impl(
    json_iter: &mut dyn Iterator<Item = char>,
) -> Result<char, JsonError> {
    let ch = json_iter.next().ok_or(JsonError::EarlyEndOfStream)?;

    match ch {
        '"' | '\\' | '/' => Ok(ch),
        'n' => Ok('\n'),
        'r' => Ok('\r'),
        't' => Ok('\t'),
        'f' => todo!("implement \\f escape char"),
        'b' => todo!("implement \\b escape char"),
        'u' => todo!("unicode"),
        _ => Err(JsonError::UnknownEscapeCharacter(ch)),
    }
}

fn parse_object_impl(mut json_iter: &mut dyn Iterator<Item = char>) -> Result<Object, JsonError> {
    let mut could_be_empty = true;

    let mut object = vec![];

    loop {
        let mut skipped = json_iter.skip_while(|ch| ch.is_whitespace());

        match skipped.next().ok_or(JsonError::EarlyEndOfStream)? {
            '"' => {}
            ch @ _ => {
                if could_be_empty && ch == '}' {
                    return Ok(object);
                } else {
                    return Err(JsonError::UnexpectedChar(ch));
                }
            }
        }

        could_be_empty = false;

        let key = parse_string_impl(json_iter)?;

        let mut skipped = json_iter.skip_while(|ch| ch.is_whitespace());

        match skipped.next().ok_or(JsonError::EarlyEndOfStream)? {
            ':' => {}
            ch @ _ => return Err(JsonError::UnexpectedChar(ch)),
        }

        let (value, maybe_excess) = parse_json_impl(json_iter)?;

        object.push((key, value));

        let mut skipped = maybe_excess
            .into_iter()
            .chain(&mut json_iter)
            .skip_while(|ch| ch.is_whitespace());

        match skipped.next().ok_or(JsonError::EarlyEndOfStream)? {
            ',' => continue,
            '}' => return Ok(object),
            ch @ _ => return Err(JsonError::UnexpectedChar(ch)),
        }
    }
}

fn parse_null_impl(json_iter: &mut dyn Iterator<Item = char>) -> Result<JsonObject, JsonError> {
    //                    "_n_ull"
    if json_iter.take(3).eq("ull".chars()) {
        return Ok(JsonObject::Null);
    } else {
        return Err(JsonError::UnexpectedKeyword);
    }
}

fn parse_true_impl(json_iter: &mut dyn Iterator<Item = char>) -> Result<JsonObject, JsonError> {
    //                    "_t_rue"
    if json_iter.take(3).eq("rue".chars()) {
        return Ok(JsonObject::Boolean(true));
    } else {
        return Err(JsonError::UnexpectedKeyword);
    }
}

fn parse_false_impl(json_iter: &mut dyn Iterator<Item = char>) -> Result<JsonObject, JsonError> {
    //                    "_f_alse"
    if json_iter.take(4).eq("alse".chars()) {
        return Ok(JsonObject::Boolean(false));
    } else {
        return Err(JsonError::UnexpectedKeyword);
    }
}

fn parse_array_impl(mut json_iter: &mut dyn Iterator<Item = char>) -> Result<Array, JsonError> {
    let mut vec: Vec<JsonObject> = Vec::new();

    let mut could_be_empty = true;

    loop {
        let result = parse_json_impl(json_iter);

        let excess;

        if could_be_empty {
            match result {
                Ok((value, maybe_excess)) => {
                    excess = maybe_excess;

                    vec.push(value)
                }
                Err(JsonError::UnexpectedChar(']')) => {
                    //empty array
                    return Ok(vec);
                }
                Err(err) => return Err(err),
            }

            could_be_empty = false;
        } else {
            let (value, maybe_excess) = result?;
            excess = maybe_excess;
            vec.push(value);
        }

        let chars = &mut excess
            .into_iter()
            .chain(&mut json_iter)
            .skip_while(|ch| ch.is_whitespace());

        //this is such a hack

        match chars.next().ok_or(JsonError::EarlyEndOfStream)? {
            ',' => continue,
            ']' => return Ok(vec),
            ch @ _ => return Err(JsonError::UnexpectedChar(ch)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_type() {
        assert_eq!(parse_json_string("null").unwrap(), JsonObject::Null);
    }

    #[test]
    fn basic_boolean() {
        assert!(matches!(
            parse_json_string("true").unwrap(),
            JsonObject::Boolean(true)
        ));

        assert!(matches!(
            parse_json_string("false").unwrap(),
            JsonObject::Boolean(false)
        ));
    }
    #[test]
    fn array_one_element() {
        let result = parse_json_string("[ true ]").unwrap();

        match result {
            JsonObject::Array(array) => {
                assert!(matches!(array.as_slice(), [JsonObject::Boolean(true),]));
            }
            _ => panic!(),
        }

        let result = parse_json_string("[ 123 ]").unwrap();

        match result {
            JsonObject::Array(array) => match array[0] {
                JsonObject::Number(n @ _) => assert_eq!(n, 123.),
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn array_multiple_elements() {
        let result = parse_json_string("[null, true, false]").unwrap();

        match result {
            JsonObject::Array(array) => {
                assert!(matches!(
                    array.as_slice(),
                    [
                        JsonObject::Null,
                        JsonObject::Boolean(true),
                        JsonObject::Boolean(false)
                    ]
                ));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn empty_array() {
        //empty array
        let result = parse_json_string("    [ ]    ").unwrap();

        match result {
            JsonObject::Array(array) => {
                assert!(matches!(array.as_slice(), []));
            }
            _ => panic!(),
        }

        let result = parse_json_string("[]").unwrap();

        match result {
            JsonObject::Array(array) => {
                assert!(matches!(array.as_slice(), []));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn nested_array_type() {
        parse_json_string("[true, [ null, 123.321 ] ]").unwrap();
        parse_json_string("[true, [ null, 123] ]").unwrap();
    }

    #[test]
    fn empty_object() {
        parse_json_string("{}").unwrap();
    }

    #[test]
    fn just_a_number() {
        assert!(
            matches!(parse_json_string("123").unwrap(), JsonObject::Number(ch @ _) if {ch == 123.})
        );

        parse_json_string("    3216546549879876214351.25416546546545646546546321   ").unwrap();

        //parse_json_string(r#"{ "my_number" : 1233.32465 }"#).unwrap();

        assert!(
            matches!(parse_json_string("123 ").unwrap(), JsonObject::Number(ch @ _) if {ch == 123.})
        );
    }

    #[test]
    fn complex_object() {
        parse_json_string(
            r#"{
                "my_array" : [   true,     false, true      ],
                "my_null" : null   ,
                "my_object"   :   {
                    "inner key" : 123.3214
                },
                "empty object" : { }
        }"#,
        )
        .unwrap();
    }
}
