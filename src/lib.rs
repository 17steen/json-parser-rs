#![feature(box_syntax)]

pub type Array = Vec<JsonObject>;
pub type Object = Vec<(String, JsonObject)>;

#[derive(Debug, PartialEq)]
pub enum JsonObject {
    Object(Object),
    Array(Array),
    String(String),
    Boolean(bool),
    Null,
}

#[derive(Debug, PartialEq)]
pub enum JsonError {
    UnexpectedChar(char),
    UnexpectedKeyword,
    UnknownEscapeCharacter(char),
    ExtraChars,
    EarlyEndOfStream,
}

#[inline]
pub fn parse_json_string(json_str: &str) -> Result<JsonObject, JsonError> {
    return parse_json_from_iter(&mut json_str.chars());
}

#[inline]
pub fn parse_json_from_iter(
    json_iter: &mut dyn Iterator<Item = char>,
) -> Result<JsonObject, JsonError> {
    let result = parse_json_impl(json_iter);

    let mut should_be_empty = json_iter.skip_while(|ch| ch.is_whitespace());

    if should_be_empty.next().is_some() {
        return Err(JsonError::ExtraChars);
    } else {
        return result;
    }
}

fn parse_json_impl(json_iter: &mut dyn Iterator<Item = char>) -> Result<JsonObject, JsonError> {
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
        ch @ _ => Err(JsonError::UnexpectedChar(ch)),
    };

    result
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
        c @ ('"' | '\\' | '/') => Ok(c),
        'n' => Ok('\n'),
        'r' => Ok('\r'),
        't' => Ok('\t'),
        'f' => todo!("implement \\f escape char"),
        'b' => todo!("implement \\b escape char"),
        'u' => todo!("unicode"),
        c @ _ => Err(JsonError::UnknownEscapeCharacter(c)),
    }
}

fn parse_object_impl(json_iter: &mut dyn Iterator<Item = char>) -> Result<Object, JsonError> {
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

        let value = parse_json_impl(json_iter)?;

        object.push((key, value));

        let mut skipped = json_iter.skip_while(|ch| ch.is_whitespace());

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
    let mut vec = Vec::new();

    let mut could_be_empty = true;

    loop {
        let result = parse_json_impl(&mut json_iter);

        if could_be_empty {
            match result {
                Ok(value) => vec.push(value),
                Err(JsonError::UnexpectedChar(']')) => {
                    //empty array
                    return Ok(vec);
                }
                Err(err) => return Err(err),
            }

            could_be_empty = false;
        } else {
            vec.push(result?);
        }

        let mut chars = json_iter.skip_while(|ch| ch.is_whitespace());

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
        assert!(parse_json_string("[true, [ null, null ] ]").is_ok());
    }

    #[test]
    fn empty_object() {
        parse_json_string("{}").unwrap();

        /*
        parse_json_string(
            r#"{
                "my_array" : [true, false, true],
                "my_null" : null,
                "my_object" : {
                    "inner key" : "inner value"
                },
                "empty object" : { }
        }"#,
        )
        .unwrap();
        */
    }

    #[test]
    fn complex_object() {
        parse_json_string(
            r#"{
                "my_array" : [true, false, true],
                "my_null" : null,
                "my_object" : {
                    "inner key" : "inner value"
                },
                "empty object" : { }
        }"#,
        )
        .unwrap();
    }
}
