use std::collections::HashMap;

#[derive(Debug)]
pub struct ParseError {
    pub position: usize,
    pub message: &'static str,
}

pub struct Stream<'a> {
    data: &'a [u8],
    pointer: usize,
}

impl<'a> Stream<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pointer: 0 }
    }

    pub fn create_context(&mut self) -> StreamCtx<'_, 'a> {
        let p = self.pointer;

        StreamCtx {
            stream: self,
            init_pointer: p,
            finish: false,
        }
    }

    pub fn line(&self, position: usize) -> usize {
        let mut count = 0;

        for i in (0..position).rev() {
            if self.data[i] == b'\n' {
                count += 1;
            }
        }

        return count;
    }
}

pub struct StreamCtx<'a, 'b> {
    stream: &'a mut Stream<'b>,
    init_pointer: usize,
    finish: bool,
}

impl<'a, 'b> StreamCtx<'a, 'b> {
    pub fn create_context(&mut self) -> StreamCtx<'_, 'b> {
        let p = self.stream.pointer;
        StreamCtx {
            stream: &mut self.stream,
            init_pointer: p,
            finish: false,
        }
    }

    pub fn current_position(&self) -> usize {
        self.stream.pointer
    }

    pub fn peek(&mut self) -> Option<u8> {
        if self.stream.pointer >= self.stream.data.len() {
            return None;
        }

        let c = self.stream.data[self.stream.pointer];

        return Some(c);
    }

    pub fn next_char(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.stream.pointer += 1;

        return Some(b);
    }

    pub fn is_eof(&mut self) -> bool {
        self.stream.pointer >= self.stream.data.len()
    }

    pub fn finish(&mut self) {
        self.finish = true;
    }
}

impl<'a, 'b> Drop for StreamCtx<'a, 'b> {
    fn drop(&mut self) {
        if !self.finish {
            self.stream.pointer = self.init_pointer;
        }
    }
}

pub trait Parser<Target = Self>: Sized {
    fn parse(stream: &mut StreamCtx) -> Result<Target, ParseError>;
}

#[derive(Debug)]
pub struct Config {
    pub sections: Vec<Section>,
}

impl Parser for Config {
    fn parse(stream: &mut StreamCtx) -> Result<Self, ParseError> {
        let mut stream = stream.create_context();

        let mut sections = Vec::new();

        // remove top comments
        Comments::parse(&mut stream).unwrap();

        // parse sections
        while !stream.is_eof() {
            let s = Section::parse(&mut stream)?;
            sections.push(s);

            Whitspaces::parse(&mut stream)?;
        }

        stream.finish();

        return Ok(Self { sections });
    }
}

#[derive(Debug)]
pub struct Section {
    pub prefix_name: String,
    pub suffix_name: Option<String>,
    pub values: HashMap<String, Value>,
}

impl Parser for Section {
    fn parse(stream: &mut StreamCtx) -> Result<Self, ParseError> {
        let mut stream = stream.create_context();

        // a section should start with '['
        if stream.next_char() != Some(b'[') {
            return Err(ParseError {
                position: stream.current_position(),
                message: "expecting character '['".into(),
            });
        }

        // prefix name 'mcu' in '[mcu my_mcu]'
        let prefix_name = Ident::parse(&mut stream)?;
        // suffix name, 'my_mcu' in '[mcu my_mcu]'
        let mut suffix_name = None;
        // suffix name is optional
        match stream.next_char() {
            Some(b' ') => {
                suffix_name = Some(Ident::parse(&mut stream)?);
                // end header
                if stream.next_char() != Some(b']') {
                    return Err(ParseError {
                        position: stream.current_position(),
                        message: "expecting character ']'".into(),
                    });
                }
            }
            Some(b']') => {}
            _ => {
                return Err(ParseError {
                    position: stream.current_position(),
                    message: "expecting character ']'".into(),
                });
            }
        };

        // remove white spaces
        Whitspaces::parse(&mut stream)?;

        println!("{}", stream.peek().unwrap() as char);

        // should open a new line
        if stream.next_char() != Some(b'\n') {
            return Err(ParseError {
                position: stream.current_position(),
                message: "expecting new line after section header".into(),
            });
        };

        // parse the key values
        let values = KeyValues::parse(&mut stream)?;

        stream.finish();

        return Ok(Self {
            prefix_name,
            suffix_name,
            values,
        });
    }
}

pub struct Ident;

impl Parser<String> for Ident {
    fn parse(stream: &mut StreamCtx) -> Result<String, ParseError> {
        let mut stream = stream.create_context();

        let mut s = String::new();

        match stream.next_char() {
            Some(c) if unicode_id_start::is_id_start(c as char) => s.push(c as char),
            _ => {
                return Err(ParseError {
                    position: stream.current_position(),
                    message: "expecting unicode id start".into(),
                });
            }
        };

        while let Some(c) = stream.peek() {
            if !unicode_id_start::is_id_continue(c as char) {
                break;
            }

            stream.next_char();

            s.push(c as char);
        }

        stream.finish();

        return Ok(s);
    }
}

/// whitspaces exluding line feed
pub struct Whitspaces;

impl Parser<()> for Whitspaces {
    fn parse(stream: &mut StreamCtx) -> Result<(), ParseError> {
        let mut stream = stream.create_context();

        while let Some(c) = stream.peek() {
            match c {
                b' ' | b'\r' | b'\t' => stream.next_char(),
                _ => break,
            };
        }

        stream.finish();

        return Ok(());
    }
}

pub struct EmptyLine;

impl Parser<()> for EmptyLine {
    fn parse(stream: &mut StreamCtx) -> Result<(), ParseError> {
        let mut stream = stream.create_context();

        if stream.next_char() != Some(b'\n') {
            return Err(ParseError {
                position: stream.current_position(),
                message: "",
            });
        }

        stream.finish();

        return Ok(());
    }
}

pub struct Comments;

impl Parser<()> for Comments {
    fn parse(stream: &mut StreamCtx) -> Result<(), ParseError> {
        let mut stream = stream.create_context();

        while Comment::parse(&mut stream).is_ok() || EmptyLine::parse(&mut stream).is_ok() {
            // parse
        }

        stream.finish();

        return Ok(());
    }
}

pub struct Comment;

impl Parser<()> for Comment {
    fn parse(stream: &mut StreamCtx) -> Result<(), ParseError> {
        let mut stream = stream.create_context();

        match stream.next_char() {
            Some(b'#') => (),
            _ => {
                return Err(ParseError {
                    position: stream.current_position(),
                    message: "",
                });
            }
        };

        while let Some(c) = stream.next_char() {
            if c == b'\n' {
                break;
            }
        }

        stream.finish();

        return Ok(());
    }
}

pub struct LineEndComment;

impl Parser<()> for LineEndComment {
    fn parse(stream: &mut StreamCtx) -> Result<(), ParseError> {
        let mut stream = stream.create_context();

        Whitspaces::parse(&mut stream).ok();

        if stream.next_char() == Some(b';') {
            while stream.peek() != Some(b'\n') {
                stream.next_char();
            }
        }

        stream.finish();

        return Ok(());
    }
}

pub struct KeyValues;

impl Parser<HashMap<String, Value>> for KeyValues {
    fn parse(stream: &mut StreamCtx) -> Result<HashMap<String, Value>, ParseError> {
        let mut stream = stream.create_context();

        let mut key_values = HashMap::new();

        // parse the comments
        Comments::parse(&mut stream).unwrap();

        while !stream.is_eof() {
            // start of the next section, break
            if stream.peek() == Some(b'[') {
                break;
            }
            // parse the key
            let key = Ident::parse(&mut stream)?;
            // remove white spaces
            Whitspaces::parse(&mut stream)?;
            // parse ':'
            if stream.next_char() != Some(b':') {
                return Err(ParseError {
                    position: stream.current_position(),
                    message: "expecting character ':'".into(),
                });
            }
            // remove white spaces
            Whitspaces::parse(&mut stream)?;
            // parse value
            let value = Value::parse(&mut stream)?;
            // parse comments
            Comments::parse(&mut stream)?;

            key_values.insert(key, value);
        }

        stream.finish();

        return Ok(key_values);
    }
}

#[derive(Debug, PartialEq)]
pub enum Value {
    Number(f64),
    NumberArray(Vec<f64>),
    /// calculated ratio, for example 80:8 would become 10
    Ratio(f64),
    String(String),
    StringArray(Vec<String>),
    Gcode(String),
}

impl Parser for Value {
    fn parse(stream: &mut StreamCtx) -> Result<Self, ParseError> {
        let mut stream = stream.create_context();

        let value = 'label: {
            match stream.peek() {
                // the first character is line end
                Some(b'\n') => {
                    // increment pointer
                    stream.next_char();
                    // if next line is indented, it is a gcode
                    match stream.peek() {
                        Some(b' ') => Value::Gcode(Gcode::parse(&mut stream)?),
                        _ => Value::String(String::new()),
                    }
                }
                Some(c) => {
                    // the first digit is a number
                    if c >= b'0' && c <= b'9' {
                        match NumberOrRatioValue::parse(&mut stream) {
                            Ok(v) => break 'label v,
                            Err(_) => {}
                        }
                    }

                    let mut is_comment = false;

                    // create buffer
                    let mut s = String::new();
                    // read all characters in string
                    while let Some(c) = stream.next_char() {
                        // break at new line
                        if c == b'\n' {
                            break;
                        }
                        if c == b';' {
                            is_comment = true;
                        }
                        if !is_comment {
                            s.push(c as char);
                        }
                    }

                    // pop off whitespaces at the end
                    for i in (0..s.len()).rev() {
                        match s.as_bytes()[i] {
                            b' ' | b'\r' | b'\t' => s.pop(),
                            _ => break,
                        };
                    }

                    Value::String(s)
                }
                // just an empty string
                None => Value::String(String::new()),
            }
        };

        stream.finish();

        return Ok(value);
    }
}

pub struct NumberOrRatioValue;

impl Parser<Value> for NumberOrRatioValue {
    fn parse(stream: &mut StreamCtx) -> Result<Value, ParseError> {
        let mut stream = stream.create_context();

        let value = match Ratio::parse(&mut stream) {
            Ok(r) => Value::Ratio(r),
            Err(_) => Value::Number(Number::parse(&mut stream)?),
        };

        LineEndComment::parse(&mut stream).ok();

        if stream.next_char() != Some(b'\n') {
            return Err(ParseError {
                position: stream.current_position(),
                message: "expecting line end".into(),
            });
        }
        stream.finish();

        return Ok(value);
    }
}

pub struct Number;

impl Parser<f64> for Number {
    fn parse(stream: &mut StreamCtx) -> Result<f64, ParseError> {
        let mut stream = stream.create_context();

        let start = stream.current_position();

        let n = match fast_float::parse_partial::<f64, &[u8]>(&stream.stream.data[start..]) {
            Ok((i, l)) => {
                stream.stream.pointer += l;
                i
            }
            Err(_) => {
                return Err(ParseError {
                    position: start,
                    message: "expecting number",
                });
            }
        };

        stream.finish();

        return Ok(n);
    }
}

pub struct Ratio;

impl Parser<f64> for Ratio {
    fn parse(stream: &mut StreamCtx) -> Result<f64, ParseError> {
        let mut stream = stream.create_context();

        let a = Number::parse(&mut stream)?;

        if stream.next_char() != Some(b':') {
            return Err(ParseError {
                position: stream.current_position(),
                message: "expecting character ':'".into(),
            });
        }

        let b = Number::parse(&mut stream)?;

        let mut r = a / b;

        Whitspaces::parse(&mut stream)?;

        if stream.peek() == Some(b',') {
            stream.next_char();

            Whitspaces::parse(&mut stream)?;

            let n = Ratio::parse(&mut stream)?;

            r = r * n;
        }

        stream.finish();

        return Ok(r);
    }
}

pub struct Gcode;

impl Parser<String> for Gcode {
    fn parse(stream: &mut StreamCtx) -> Result<String, ParseError> {
        let mut stream = stream.create_context();

        let mut lines = String::new();

        loop {
            match stream.peek() {
                Some(b' ') => {
                    if EmptyLine::parse(&mut stream).is_ok() {
                        continue;
                    }

                    Whitspaces::parse(&mut stream).ok();

                    let line = GcodeLine::parse(&mut stream)?;
                    lines.push_str(&line);
                    lines.push('\n');
                }
                Some(b'#') => {
                    Comment::parse(&mut stream).ok();
                }
                Some(b'\n') => {
                    stream.next_char();
                    continue;
                }
                _ => break,
            }
        }

        stream.finish();

        return Ok(lines);
    }
}

pub struct GcodeLine;

impl Parser<String> for GcodeLine {
    fn parse(stream: &mut StreamCtx) -> Result<String, ParseError> {
        let mut stream = stream.create_context();

        let mut is_comment = false;

        // create buffer
        let mut s = String::new();
        // read all characters in string
        while let Some(c) = stream.next_char() {
            // break at new line
            if c == b'\n' {
                break;
            }

            if c == b';' {
                is_comment = true;
            }

            if !is_comment {
                s.push(c as char);
            }
        }

        // pop off whitespaces at the end
        for i in (0..s.len()).rev() {
            match s.as_bytes()[i] {
                b' ' | b'\r' | b'\t' => s.pop(),
                _ => break,
            };
        }

        stream.finish();

        return Ok(s);
    }
}

#[test]
fn test_number_parser() {
    const TEST_DATA: &[(&[u8], f64)] = &[
        (b"3.1415926535", 3.1415926535),
        (b"9999999999", 9999999999.0),
        (b"1.23e-02", 1.23e-02),
    ];

    for (data, expected) in TEST_DATA {
        let mut stream = Stream::new(data);
        let mut ctx = stream.create_context();

        let re = Number::parse(&mut ctx).unwrap();

        assert_eq!(re, *expected);
    }
}

#[test]
fn test_ratio_parser() {
    const TEST_DATA: &[(&[u8], f64)] = &[
        (b"5:1", 5.0),
        (b"57:2", 28.5),
        (b"80:10, 2:1", 16.0),
        (b"90:1 , 56:7, 45:2", 16200.0),
    ];

    for (data, expected) in TEST_DATA {
        let mut stream = Stream::new(data);
        let mut ctx = stream.create_context();

        let re = Ratio::parse(&mut ctx).unwrap();

        assert_eq!(re, *expected);
    }
}

#[test]
fn test_number_or_ratio_parser() {
    const TEST_DATA: &[(&[u8], Value)] = &[
        (b"5:1\n", Value::Ratio(5.0)),
        (b"57:2  \r\n", Value::Ratio(28.5)),
        (b"80:10, 2:1\n", Value::Ratio(16.0)),
        (b"90:1 , 56:7, 45:2\n", Value::Ratio(16200.0)),
        (b"3.1415926535\n", Value::Number(3.1415926535)),
        (b"9999999999   \r\n", Value::Number(9999999999.0)),
        (b"1.23e-02 \t  \n", Value::Number(1.23e-02)),
    ];

    for (data, expected) in TEST_DATA {
        let mut stream = Stream::new(data);
        let mut ctx = stream.create_context();

        let re = NumberOrRatioValue::parse(&mut ctx).unwrap();

        assert_eq!(re, *expected);
    }
}

#[test]
fn test_gcode_parser() {
    const TEST_DATA: &[(&[u8], &str)] = &[
        (
            b"  SET_PIN PIN=my_led VALUE=1 \n G4 P2000\n SET_PIN PIN=my_led VALUE=0\n",
            "SET_PIN PIN=my_led VALUE=1\nG4 P2000\nSET_PIN PIN=my_led VALUE=0\n",
        ),
        (
            b" #some comment\n G1 x=0 y=9\n #another comment\n",
            "G1 x=0 y=9\n",
        ),
    ];

    for (data, expected) in TEST_DATA {
        let mut stream = Stream::new(data);
        let mut ctx = stream.create_context();

        let re = Gcode::parse(&mut ctx).unwrap();

        assert_eq!(re, *expected);
    }
}

#[test]
fn test_value_parser() {
    let test_data: &[(&[u8], Value)] = &[
        (b"5:1\n", Value::Ratio(5.0)),
        (b"57:2  \r\n", Value::Ratio(28.5)),
        (b"80:10, 2:1\n", Value::Ratio(16.0)),
        (b"90:1 , 56:7, 45:2\n", Value::Ratio(16200.0)),
        (b"3.1415926535\n", Value::Number(3.1415926535)),
        (b"9999999999   \r\n", Value::Number(9999999999.0)),
        (b"1.23e-02 \t  \n", Value::Number(1.23e-02)),
        (
            b"some random text   \r\n",
            Value::String("some random text".into()),
        ),
        (
            b"00 should fall back to string    \t \r\n",
            Value::String("00 should fall back to string".into()),
        ),
        (
            b"\n  SET_PIN PIN=my_led VALUE=1 \n G4 P2000\n SET_PIN PIN=my_led VALUE=0\n",
            Value::Gcode(
                "SET_PIN PIN=my_led VALUE=1\nG4 P2000\nSET_PIN PIN=my_led VALUE=0\n".into(),
            ),
        ),
        (
            b"\n #some comment\n G1 x=0 y=9\n #another comment\n",
            Value::Gcode("G1 x=0 y=9\n".into()),
        ),
    ];

    for (data, expected) in test_data {
        let mut stream = Stream::new(data);
        let mut ctx = stream.create_context();

        let re = Value::parse(&mut ctx).unwrap();

        assert_eq!(re, *expected);
    }
}

#[test]
fn test_key_values() {
    let test_data: &[(&[u8], &str, Value)] = &[
        (b"gear_ratio : 5:1\n", "gear_ratio", Value::Ratio(5.0)),
        (b"gear_ratio: 57:2  \r\n", "gear_ratio", Value::Ratio(28.5)),
        (
            b"gear_ratio :80:10, 2:1\n",
            "gear_ratio",
            Value::Ratio(16.0),
        ),
        (
            b"gear_ratio:90:1 , 56:7, 45:2\n",
            "gear_ratio",
            Value::Ratio(16200.0),
        ),
        (b"pi: 3.1415926535\n", "pi", Value::Number(3.1415926535)),
        (
            b"duration:9999999999   \r\n",
            "duration",
            Value::Number(9999999999.0),
        ),
        (
            b"response: 1.23e-02 \t  \n",
            "response",
            Value::Number(1.23e-02),
        ),
        (
            b"desc :some random text   \r\n",
            "desc",
            Value::String("some random text".into()),
        ),
        (
            b"fallback : 00 should fall back to string    \t \r\n",
            "fallback",
            Value::String("00 should fall back to string".into()),
        ),
        (
            b"gcode: \n  SET_PIN PIN=my_led VALUE=1 \n G4 P2000\n SET_PIN PIN=my_led VALUE=0\n",
            "gcode",
            Value::Gcode(
                "SET_PIN PIN=my_led VALUE=1\nG4 P2000\nSET_PIN PIN=my_led VALUE=0\n".into(),
            ),
        ),
        (
            b"gcode:\n #some comment\n G1 x=0 y=9\n #another comment\n",
            "gcode",
            Value::Gcode("G1 x=0 y=9\n".into()),
        ),
    ];

    for (data, name, expected) in test_data {
        let mut stream = Stream::new(data);
        let mut ctx = stream.create_context();

        let re = KeyValues::parse(&mut ctx).unwrap();

        assert_eq!(re.get(*name), Some(expected));
    }
}

#[test]
fn test_cartesian_cfg() {
    const CARTESIAN_CFG: &[u8] = include_bytes!("../../../config/example-cartesian.cfg");

    let mut stream = Stream::new(CARTESIAN_CFG);
    let mut ctx = stream.create_context();

    let re = Config::parse(&mut ctx);

    println!("{:#?}", re);
}

#[test]
fn test_kit_voron_cfg() {
    const KIT_VORON_CFG: &[u8] = include_bytes!("../../../config/kit-voron2-250mm.cfg");

    let mut stream = Stream::new(KIT_VORON_CFG);
    let mut ctx = stream.create_context();

    let re = Config::parse(&mut ctx);

    drop(ctx);

    println!("{:#?}", re);

    if let Err(e) = re {
        println!("line {}", stream.line(e.position));
    }
}
