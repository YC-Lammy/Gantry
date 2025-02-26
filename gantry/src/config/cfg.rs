use std::collections::HashMap;

#[derive(Debug)]
pub struct ParseError{
    pub position: usize,
    pub message: String
}

pub struct Stream{
    data: Vec<u8>,
    pointer: usize
}

impl Stream{
    pub fn new(data: Vec<u8>) -> Self{
        Self { data, pointer: 0 }
    }

    pub fn create_context(&mut self) -> StreamCtx{
        let p = self.pointer;

        StreamCtx{
            stream: self,
            init_pointer: p,
            finish: false
        }
    }
}

pub struct StreamCtx<'a>{
    stream: &'a mut Stream,
    init_pointer: usize,
    finish: bool,
}

impl<'a> StreamCtx<'a>{
    pub fn create_context(&mut self) -> StreamCtx{
        let p = self.stream.pointer;
        StreamCtx { 
            stream: &mut self.stream, 
            init_pointer: p, 
            finish: false
        }
    }

    pub fn current_position(&self) -> usize{
        self.stream.pointer
    }

    pub fn peek(&mut self) -> Option<u8>{
        if self.stream.pointer >= self.stream.data.len(){
            return None
        }

        let c = self.stream.data[self.stream.pointer];

        return Some(c)
    }

    pub fn next_char(&mut self) -> Option<u8>{
        let b = self.peek()?;
        self.stream.pointer += 1;

        return Some(b)
    }
    
    pub fn is_eof(&mut self) -> bool{
        self.stream.pointer >= self.stream.data.len()
    }

    pub fn finish(&mut self){
        self.finish = true;
    }
}

impl<'a> Drop for StreamCtx<'a>{
    fn drop(&mut self) {
        if !self.finish{
            self.stream.pointer = self.init_pointer;
        }
    }
}

pub trait Parser<Target = Self>: Sized{
    fn parse(stream: &mut StreamCtx) -> Result<Target, ParseError>;
}

pub struct Config{
    pub sections: Vec<Section>
}

impl Parser for Config{
    fn parse(stream: &mut StreamCtx) -> Result<Self, ParseError> {
        let mut stream = stream.create_context();

        let mut sections = Vec::new();

        // remove top comments
        Comments::parse(&mut stream).unwrap();

        // parse sections
        while !stream.is_eof(){
            let s = Section::parse(&mut stream)?;
            sections.push(s);

            Whitspaces::parse(&mut stream)?;
        }

        stream.finish();

        return Ok(Self { sections })
    }
}

pub struct Section{
    pub prefix_name: String,
    pub suffix_name: Option<String>,
    pub values: HashMap<String, Value>
}

impl Parser for Section{
    fn parse(stream: &mut StreamCtx) -> Result<Self, ParseError> {
        let mut stream = stream.create_context();

        // a section should start with '['
        if stream.next_char() != Some(b'['){
            return Err(ParseError{
                position: stream.current_position(),
                message: "expecting character '['".into()
            })
        }

        // prefix name 'mcu' in '[mcu my_mcu]'
        let prefix_name = Ident::parse(&mut stream)?;
        // suffix name, 'my_mcu' in '[mcu my_mcu]'
        let mut suffix_name = None;
        // suffix name is optional
        match stream.next_char(){
            Some(b' ') => suffix_name = Some(Ident::parse(&mut stream)?),
            Some(b']') => {},
            _ => return Err(ParseError{
                position: stream.current_position(),
                message: "expecting character ']'".into()
            }),
        };
        
        // remove white spaces
        Whitspaces::parse(&mut stream)?;

        // should open a new line
        if stream.next_char() != Some(b'\n'){
            return Err(ParseError{
                position: stream.current_position(),
                message: "expecting new line".into()
            })
        };

        // parse the key values
        let values = KeyValues::parse(&mut stream)?;

        stream.finish();

        return Ok(Self{
            prefix_name,
            suffix_name,
            values
        })
    }
}

pub struct Ident;

impl Parser<String> for Ident{
    fn parse(stream: &mut StreamCtx) -> Result<String, ParseError> {
        let mut stream = stream.create_context();

        let mut s = String::new();

        match stream.next_char(){
            Some(c) if unicode_id_start::is_id_start(c as char) => s.push(c as char),
            _ => return Err(ParseError{
                position: stream.current_position(),
                message: "expecting unicode id start".into()
            })
        };

        while let Some(c) = stream.peek(){
            if !unicode_id_start::is_id_continue(c as char){
                break;
            }

            stream.next_char();

            s.push(c as char);
        }

        stream.finish();

        return Ok(s)
    }
}

/// whitspaces exluding line feed
pub struct Whitspaces;

impl Parser<()> for Whitspaces{
    fn parse(stream: &mut StreamCtx) -> Result<(), ParseError> {
        let mut stream = stream.create_context();

        while let Some(c) = stream.peek(){
            match c{
                b' ' | b'\r' | b'\t' => stream.next_char(),
                _ => break
            };
        };

        stream.finish();

        return Ok(())
    }
}

pub struct EmptyLine;

impl Parser<()> for EmptyLine{
    fn parse(stream: &mut StreamCtx) -> Result<(), ParseError> {
        let mut stream = stream.create_context();

        Whitspaces::parse(&mut stream)?;

        if stream.peek() != Some(b'\n'){
            return Err(ParseError { position: stream.current_position(), message: String::new() })
        }

        stream.finish();

        return Ok(())
    }
}

pub struct Comments;

impl Parser<()> for Comments{
    fn parse(stream: &mut StreamCtx) -> Result<(), ParseError> {
        let mut stream = stream.create_context();

        loop{
            // try to parse empty line
            if EmptyLine::parse(&mut stream).is_err(){
                // try to parse a comment
                if Comment::parse(&mut stream).is_err(){
                    break;
                }
            }
        }

        stream.finish();

        return Ok(())
    }
}

pub struct Comment;

impl Parser<()> for Comment{
    fn parse(stream: &mut StreamCtx) -> Result<(), ParseError> {
        let mut stream = stream.create_context();

        Whitspaces::parse(&mut stream).unwrap();

        match stream.next_char(){
            Some(b'#') => (),
            _ => return Err(ParseError{ position: stream.current_position(), message: String::new()})
        };

        while let Some(c) = stream.next_char(){
            if c == b'\n'{
                break;
            }
        }

        stream.finish();

        return Ok(())
    }
}

pub struct KeyValues;

impl Parser<HashMap<String, Value>> for KeyValues{
    fn parse(stream: &mut StreamCtx) -> Result<HashMap<String, Value>, ParseError> {
        let mut stream = stream.create_context();

        let mut key_values = HashMap::new();
        
        // parse the comments
        Comments::parse(&mut stream).unwrap();

        while !stream.is_eof(){
            // start of the next section, break
            if stream.peek() == Some(b'['){
                break;
            }
            // parse the key
            let key = Ident::parse(&mut stream)?;
            // remove white spaces
            Whitspaces::parse(&mut stream)?;
            // parse ':'
            if stream.next_char() != Some(b':'){
                return Err(ParseError{
                    position: stream.current_position(),
                    message: "expecting character ':'".into()
                })
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

        return Ok(key_values)
    }
}

pub enum Value{
    Number(f64),
    /// calculated ratio, for example 80:8 would become 10
    Ratio(f64),
    String(String),
    Gcode(String),
}

impl Parser for Value{
    fn parse(stream: &mut StreamCtx) -> Result<Self, ParseError> {
        let mut stream = stream.create_context();

        let value = 'label: {match stream.peek(){
            // the first character is line end
            Some(b'\n') => {
                // increment pointer
                stream.next_char();
                // if next line is indented, it is a gcode
                match stream.peek(){
                    Some(b' ') => Value::Gcode(Gcode::parse(&mut stream)?),
                    _ => Value::String(String::new())
                }
            },
            Some(c) => {
                // the first digit is a number
                if c >= b'0' && c <= b'9'{
                    match NumberOrRatioValue::parse(&mut stream){
                        Ok(v) => break 'label v,
                        Err(_) => {}
                    }
                }

                // create buffer
                let mut s = String::new();
                // read all characters in string
                while let Some(c) = stream.next_char(){
                    // break at new line
                    if c == b'\n'{
                        break;
                    }
                    s.push(c as char);
                }

                // pop off whitespaces at the end
                for i in (0..s.len()).rev(){
                    match s.as_bytes()[i]{
                        b' ' | b'\r' | b'\t' => s.pop(),
                        _ => break
                    };
                }

                Value::String(s)
            },
            // just an empty string
            None => Value::String(String::new())
        }};

        stream.finish();

        return Ok(value)
    }
}

pub struct NumberOrRatioValue;

impl Parser<Value> for NumberOrRatioValue{
    fn parse(stream: &mut StreamCtx) -> Result<Value, ParseError> {
        let mut stream = stream.create_context();

        let value = match Ratio::parse(&mut stream){
            Ok(r) => Value::Ratio(r),
            Err(_) => {
                Value::Number(Number::parse(&mut stream)?)
            }
        };

        Whitspaces::parse(&mut stream)?;

        if stream.next_char() != Some(b'\n'){
            return Err(ParseError { position: stream.current_position(), message: "expecting line end".into() })
        }
        stream.finish();

        return Ok(value)
    }
}

pub struct Number;

impl Parser<f64> for Number{
    fn parse(stream: &mut StreamCtx) -> Result<f64, ParseError> {
        let mut stream = stream.create_context();

        let mut is_decimal = false;
        let mut has_digits = false;
        let mut i_digits = 0;
        let mut re = 0;
        
        while let Some(c) = stream.peek(){
            match c{
                b'0'..=b'9' => {
                    stream.next_char();

                    re = re * 10 + (c - b'0') as u64;

                    has_digits = true;

                    if is_decimal{
                        i_digits += 1;
                    }
                },
                b'.' if !is_decimal => {
                    stream.next_char();
                    is_decimal = true;
                },
                _ => break,
            }
        }

        if !has_digits{
            return Err(ParseError { 
                position: stream.current_position(), 
                message: "expecting numeric value".into()
            })
        }

        let n = (re as f64) / 10.0f64.powi(i_digits);
        stream.finish();

        return Ok(n)
    }
}

pub struct Ratio;

impl Parser<f64> for Ratio{
    fn parse(stream: &mut StreamCtx) -> Result<f64, ParseError> {
        let mut stream = stream.create_context();

        let a = Number::parse(&mut stream)?;
        
        if stream.next_char() != Some(b':'){
            return Err(ParseError{
                position: stream.current_position(),
                message: "expecting character ':'".into()
            })
        }

        let b = Number::parse(&mut stream)?;

        let mut r = a / b;

        Whitspaces::parse(&mut stream)?;

        if stream.peek() == Some(b','){
            stream.next_char();

            Whitspaces::parse(&mut stream)?;

            let n = Ratio::parse(&mut stream)?;

            r = r * n;
        }

        stream.finish();

        return Ok(r)
    }
}

pub struct Gcode;

impl Parser<String> for Gcode{
    fn parse(stream: &mut StreamCtx) -> Result<String, ParseError> {
        todo!()
    }
}

#[test]
fn test_parser(){
    let f = "8".parse::<f64>();
    println!("{:?}", f);
}