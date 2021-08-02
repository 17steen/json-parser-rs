#![feature(box_syntax)]
#![feature(try_blocks)]

pub type Array = Vec<JsonObject>;
pub type ObjectImpl = Vec<(String, JsonObject)>;

#[derive(Debug, PartialEq)]
pub struct Object {
    entries: ObjectImpl,
}

impl Object {
    pub fn get(&self, index: &str) -> Option<&JsonObject> {
        Some(&self.entries.iter().find(|(key, _)| key == index)?.1)
    }

    pub fn get_mut(&mut self, index: &str) -> Option<&mut JsonObject> {
        Some(&mut self.entries.iter_mut().find(|(key, _)| key == index)?.1)
    }

    #[inline]
    pub fn entries(&self) -> &ObjectImpl {
        &self.entries
    }

    #[inline]
    pub fn entries_mut(&mut self) -> &mut ObjectImpl {
        &mut self.entries
    }

    pub fn keys(&self) -> impl DoubleEndedIterator + '_ {
        self.entries().iter().map(|(key, _)| key)
    }

    pub fn keys_mut(&mut self) -> impl DoubleEndedIterator + '_ {
        self.entries_mut().iter_mut().map(|(key, _)| key)
    }

    pub fn values(&self) -> impl DoubleEndedIterator + '_ {
        self.entries().iter().map(|(_, value)| value)
    }

    pub fn values_mut(&mut self) -> impl DoubleEndedIterator + '_ {
        self.entries_mut().iter_mut().map(|(_, value)| value)
    }

    fn from_impl(entries: ObjectImpl) -> Self {
        Object { entries }
    }
}

#[derive(Debug, PartialEq)]
pub enum JsonObject {
    Object(Object),
    Array(Array),
    String(String),
    Boolean(bool),
    Number(f64),
    Null,
}

macro_rules! getter {
    ($pat:path, $ident:ident, $name:ident) => {
        #[inline]
        pub fn $name(&self) -> Option<&$ident> {
            match self {
                $pat($name) => Some($name),
                _ => None,
            }
        }
    };
}

macro_rules! getter_mut {
    ($pat:path, $ident:ident, $name:ident) => {
        #[inline]
        pub fn $name(&mut self) -> Option<&mut $ident> {
            match self {
                $pat($name) => Some($name),
                _ => None,
            }
        }
    };
}

macro_rules! getter_into {
    ($pat:path, $ident:ident, $name:ident) => {
        #[inline]
        pub fn $name(self) -> Option<$ident> {
            match self {
                $pat($name) => Some($name),
                _ => None,
            }
        }
    };
}

impl JsonObject {
    getter!(JsonObject::Object, Object, object);
    getter!(JsonObject::Array, Array, array);
    getter!(JsonObject::Boolean, bool, boolean);
    getter!(JsonObject::Number, f64, number);
    getter!(JsonObject::String, String, string);
    getter_mut!(JsonObject::Object, Object, object_mut);
    getter_mut!(JsonObject::Array, Array, array_mut);
    getter_mut!(JsonObject::Boolean, bool, boolean_mut);
    getter_mut!(JsonObject::Number, f64, number_mut);
    getter_mut!(JsonObject::String, String, string_mut);
    getter_into!(JsonObject::Object, Object, into_object);
    getter_into!(JsonObject::Array, Array, into_array);
    getter_into!(JsonObject::Boolean, bool, into_boolean);
    getter_into!(JsonObject::Number, f64, into_number);
    getter_into!(JsonObject::String, String, into_string);

    #[inline]
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

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for JsonError {}

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
                    return Ok(Object::from_impl(object));
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
            '}' => return Ok(Object::from_impl(object)),
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
    fn getters() -> Result<(), Box<dyn std::error::Error>> {
        let result = parse_json_string(" 123456789 ")?
            .into_number()
            .ok_or("not a number")?;

        assert_eq!(123456789., result);

        Ok(())
    }

    #[test]
    fn complex_object() -> Result<(), Box<dyn std::error::Error>> {
        let mut json = parse_json_string(
            r#"{
                "my_array" : [   727 ,     42 , 73      ],
                "my_null" : null   ,
                "my_object"   :   {
                    "inner key" : 123.3214
                },
                "empty object" : { }
        }"#,
        )?;

        let maybe: Option<()> = try {
            json.object_mut()?
                .get_mut("my_array")?
                .array_mut()?
                .sort_by(|a, b| a.number().partial_cmp(&b.number()).unwrap());

            assert!(json
                .object()?
                .get("my_array")?
                .array()?
                .iter()
                .map(JsonObject::number)
                .map(Option::unwrap)
                .eq(&[42., 73., 727.]));
        };

        maybe.ok_or("nope".into())
    }
}
