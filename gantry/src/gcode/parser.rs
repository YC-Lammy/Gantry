use std::collections::HashMap;

use base64::Engine;
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

use tokio::io::AsyncRead;
use tokio::io::BufReader;
use tokio::io::AsyncBufReadExt;

#[derive(Parser)]
#[grammar = "gcode/gcode.pest"]
struct GcodeParser;

#[derive(Debug, Default)]
pub struct GcodeFile{
    pub slicer: SlicerInfo,
    pub thumbnails: Vec<Thumbnail>,
    pub meta: Meta,
    pub config: SlicerConfig,
    pub commands: Vec<GcodeCommand>
}

impl GcodeFile{
    pub fn blocking_parse(input: &str) -> anyhow::Result<GcodeFile>{
        let mut pairs = GcodeParser::parse(Rule::GcodeFile, input)?;

        let pair = pairs.next().unwrap();

        // create gcode file
        let mut gcode_file = GcodeFile::default();

        for p in pair.into_inner(){
            match p.as_rule(){
                Rule::SlicerInfo => gcode_file.slicer = SlicerInfo::parse_pairs(p),
                // thumbnail info, begin thumbnail
                Rule::Thumbnail => gcode_file.thumbnails.push(Thumbnail::parse_pairs(p)),
                // a gcode line
                Rule::GcodeLine => gcode_file.commands.push(GcodeCommand::parse_pairs(p)),
                // a metadata line
                Rule::Meta => gcode_file.meta.append_pair(p),
                // a config line
                Rule::Config => gcode_file.config.append_pair(p),
                Rule::EOI => {},
                _ => unreachable!()
            }
        };

        return Ok(gcode_file)
    }

    pub async fn async_parse<R: AsyncRead + Unpin>(file: R) -> anyhow::Result<GcodeFile>{
        // reader
        let mut reader = BufReader::new(file);
        // buffer for reader
        let mut buffer = Vec::new();
        // create gcode file
        let mut gcode_file = GcodeFile::default();

        // parse each line
        while reader.read_until(b'\n', &mut buffer).await? != 0{
            // decode utf8
            let line = core::str::from_utf8(&buffer)?;

            // parse a line
            let mut pairs = GcodeParser::parse(Rule::GcodeFileLine, &line)?;

            // only a single pair
            let pair = pairs.next().unwrap();
            
            for p in pair.into_inner(){
                match p.as_rule(){
                    Rule::SlicerInfo => gcode_file.slicer = SlicerInfo::parse_pairs(p),
                    // thumbnail info, begin thumbnail
                    Rule::ThumbnailInfo => {
                        let (width, height) = Thumbnail::parse_info(p);

                        // thumbnail data lines
                        let mut base64_data = Vec::new();
                        let mut ended = false;

                        let mut buffer = Vec::new();

                        // parse thumbnail data lines
                        while reader.read_until(b'\n', &mut buffer).await? != 0{
                            // decode utf8
                            let line = core::str::from_utf8(&buffer)?;

                            match GcodeParser::parse(Rule::ThumbnailLine, &line){
                                // parse the line and append to buffer
                                Ok(mut t) => Thumbnail::parse_line(t.next().unwrap(), &mut base64_data),
                                // not a data line, must be the end
                                Err(_) => {
                                    // try to parse the end line
                                    if GcodeParser::parse(Rule::ThumbnailEnd, &line).is_ok(){
                                        ended = true;
                                    }

                                    break;
                                }
                            }

                            buffer.clear();
                        };
                        // if thumbnail is not ended, treat it as comment and discard
                        if ended{
                            // decode base64 data
                            let data = base64::prelude::BASE64_STANDARD.decode(base64_data).unwrap();
                            // push thumbnail to file
                            gcode_file.thumbnails.push(Thumbnail::new(width, height, data));
                        }
                    },
                    // a gcode line
                    Rule::GcodeLine => gcode_file.commands.push(GcodeCommand::parse_pairs(p)),
                    // a metadata line
                    Rule::Meta => gcode_file.meta.append_pair(p),
                    // a config line
                    Rule::Config => gcode_file.config.append_pair(p),
                    Rule::EOI => {},
                    _ => unreachable!()
                }
            }

            // clear buffer
            buffer.clear();
        }
        
        return Ok(gcode_file)
    }
}

#[derive(Debug)]
pub struct GcodeCommand{
    pub cmd: String,
    pub params: Vec<String>
}

impl GcodeCommand{
    fn parse_pairs(pair: Pair<Rule>) -> Self{
        let mut line = pair.as_str();

        // remove comment at line end
        if let Some((l, _)) = line.split_once(';') {
            line = l;
        }

        // params are split by spaces
        let mut iter = line.trim().split(' ');

        // get the command
        let command = iter.next().unwrap();

        let mut params = Vec::new();

        for p in iter {
            // multiple whitespace will result in empty string
            if p == "" {
                continue;
            }
            // push param
            params.push(p.to_string());
        }

        return Self { 
            cmd: command.to_string(), 
            params
        }
    }
}

#[derive(Debug, Default)]
pub struct SlicerInfo{
    pub slicer: Option<String>,
    pub version: Option<String>,
    pub date: Option<String>,
    pub time: Option<String>,
}

impl SlicerInfo{
    fn parse_pairs(pair: Pair<Rule>) -> Self{
        let mut info = SlicerInfo::default();

        for p in pair.into_inner(){
            match p.as_rule(){
                Rule::Name => info.slicer = Some(p.as_str().to_string()),
                Rule::SlicerVersion => info.version = Some(p.as_str().to_string()),
                Rule::Date => info.date = Some(p.as_str().to_string()),
                Rule::Time => info.time = Some(p.as_str().to_string()),
                _ => unreachable!()
            }
        }

        return info
    }
}

#[derive(Debug)]
pub struct Thumbnail{
    pub width: u32,
    pub height: u32,
    /// decoded data
    pub data: Vec<u8>,
}

impl Thumbnail{
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Self{
        Self { width, height, data }
    }

    fn parse_pairs(pair: Pair<Rule>) -> Self{
        let mut width = 0;
        let mut height = 0;
        let mut base64_data = Vec::new();

        for p in pair.into_inner(){
            match p.as_rule(){
                Rule::ThumbnailInfo => {
                    // loop inner
                    for i in p.into_inner(){
                        match i.as_rule(){
                            // e.g. 300x300
                            Rule::ThumbnailPixels => {
                                let (w, h) = i.as_str().split_once('x').unwrap();
                                width = w.parse().unwrap();
                                height = h.parse().unwrap();
                            },
                            Rule::ThumbnailBytes => {},
                            _ => unreachable!()
                        }
                    }
                },
                Rule::ThumbnailLine => {
                    // push the base64 data
                    base64_data.extend_from_slice(p.as_str()[1..].trim().as_bytes());
                },
                _ => unreachable!()
            }
        };

        // decode base64 data
        let data = base64::prelude::BASE64_STANDARD.decode(base64_data).unwrap();

        return Thumbnail { width, height, data }
    }

    fn parse_info(pair: Pair<Rule>) -> (u32, u32){
        let mut width = 0;
        let mut height = 0;

        // loop inner
        for i in pair.into_inner(){
            match i.as_rule(){
                // e.g. 300x300
                Rule::ThumbnailPixels => {
                    let (w, h) = i.as_str().split_once('x').unwrap();
                    width = w.parse().unwrap();
                    height = h.parse().unwrap();
                },
                Rule::ThumbnailBytes => {},
                _ => unreachable!()
            }
        };

        return (width, height)
    }

    fn parse_line(line: Pair<Rule>, base64_buf: &mut Vec<u8>){
        // push the base64 data
        base64_buf.extend_from_slice(line.as_str()[1..].trim().as_bytes());
    }
}

#[derive(Debug, Default)]
pub struct Meta{
    pub filament_length_used: Option<f32>,
    pub filament_volume_used: Option<f32>,
    pub filament_weight_used: Option<f32>,
    pub filament_cost: Option<f32>,
    pub total_filament_length_used: Option<f32>,
    pub total_filament_volume_used: Option<f32>,
    pub total_filament_weight_used: Option<f32>,
    pub total_filament_cost: Option<f32>,
    /// layers count
    pub total_layers_count: Option<u32>,
    /// filament in grams used for wipe tower
    pub total_filament_used_wipe_tower: Option<f32>,
    /// estimated print time in seconds
    pub estimated_print_time: Option<u64>,
    /// estimated first layer print time in seconds
    pub estimated_first_layer_print_time: Option<u64>
}

impl Meta{
    fn append_pair(&mut self, pair: Pair<Rule>){
        for p in pair.into_inner(){
            match p.as_rule(){
                Rule::FilamentLengthUsed => {
                    self.filament_length_used = Some(fast_float::parse(p.into_inner().next().unwrap().as_str()).unwrap());
                }
                Rule::FilamentVolumeUsed => {
                    self.filament_volume_used = Some(fast_float::parse(p.into_inner().next().unwrap().as_str()).unwrap());
                }
                Rule::FilamentWeightUsed => {
                    self.filament_weight_used = Some(fast_float::parse(p.into_inner().next().unwrap().as_str()).unwrap());
                }
                Rule::FilamentCost => {
                    self.filament_cost = Some(fast_float::parse(p.into_inner().next().unwrap().as_str()).unwrap());
                }
                Rule::TotalFilamentLengthUsed => {
                    self.total_filament_length_used = Some(fast_float::parse(p.into_inner().next().unwrap().as_str()).unwrap());
                }
                Rule::TotalFilamentVolumeUsed => {
                    self.filament_volume_used = Some(fast_float::parse(p.into_inner().next().unwrap().as_str()).unwrap());
                }
                Rule::TotalFilamentWeightUsed => {
                    self.filament_weight_used = Some(fast_float::parse(p.into_inner().next().unwrap().as_str()).unwrap());
                }
                Rule::TotalLayersCount => {
                    self.total_layers_count = Some(p.into_inner().next().unwrap().as_str().parse().unwrap());
                }
                Rule::TotalFilamentWeightUsedWipeTower => {
                    self.total_filament_used_wipe_tower = Some(fast_float::parse(p.into_inner().next().unwrap().as_str()).unwrap());
                }
                Rule::TotalFilamentCost => {
                    self.total_filament_cost = Some(fast_float::parse(p.into_inner().next().unwrap().as_str()).unwrap());
                }
                Rule::EstimatedPrintTime => {
                    let time = p.into_inner().next().unwrap();

                    let mut t = 0;

                    for i in time.into_inner(){
                        match i.as_rule(){
                            Rule::PrintTimeHour => {
                                t += i.as_str().parse::<u64>().unwrap() * 60 * 60;
                            }
                            Rule::PrintTimeMinute => {
                                t += i.as_str().parse::<u64>().unwrap() * 60;
                            }
                            Rule::PrintTimeSeconds => {
                                t += i.as_str().parse::<u64>().unwrap();
                            }
                            _ => unreachable!()
                        }
                    }

                    self.estimated_print_time = Some(t)
                }
                Rule::EstimatedFirstLayerPrintTime => {
                    let time = p.into_inner().next().unwrap();

                    let mut t = 0;

                    for i in time.into_inner(){
                        match i.as_rule(){
                            Rule::PrintTimeHour => {
                                t += i.as_str().parse::<u64>().unwrap() * 60 * 60;
                            }
                            Rule::PrintTimeMinute => {
                                t += i.as_str().parse::<u64>().unwrap() * 60;
                            }
                            Rule::PrintTimeSeconds => {
                                t += i.as_str().parse::<u64>().unwrap();
                            }
                            _ => unreachable!()
                        }
                    }

                    self.estimated_first_layer_print_time = Some(t)
                }
                _ => unreachable!()
            }
        };
    }
}

#[derive(Debug, Default)]
pub struct SlicerConfig{
    pub properties: HashMap<String, String>,
}

impl SlicerConfig{
    fn append_pair(&mut self, pair: Pair<Rule>){
        let mut p = pair.into_inner();

        let name = p.next().unwrap().as_str().to_string();
        let value = p.next().unwrap().as_str().to_string();

        self.properties.insert(name, value);
    }
}

#[tokio::test]
async fn test_async(){
    const TESTS: &[(&[u8], &str, &str, u32, f32, u64)] = &[
        (
            include_bytes!("../../tests/3dbenchy_ABS_45m43s.gcode"),
            "OrcaSlicer",
            "2.2.0",
            240,
            3701.23,
            2743
        )
    ];

    for (data, slicer, version, layers, filament_length, est_time) in TESTS{
        let r = tokio::io::BufReader::new(*data);
        let gf = GcodeFile::async_parse(r).await.unwrap();

        assert!(gf.slicer.slicer.is_some_and(|s|s.eq(slicer)));
        assert!(gf.slicer.version.is_some_and(|v|v.eq(version)));
        assert!(gf.meta.total_layers_count == Some(*layers));
        assert!(gf.meta.filament_length_used == Some(*filament_length));
        assert!(gf.meta.estimated_print_time == Some(*est_time));
    }
}

#[test]
fn test_blocking(){
    const TESTS: &[(&str, &str, &str, u32, f32, u64)] = &[
        (
            include_str!("../../tests/3dbenchy_ABS_45m43s.gcode"),
            "OrcaSlicer",
            "2.2.0",
            240,
            3701.23,
            2743
        )
    ];

    for (data, slicer, version, layers, filament_length, est_time) in TESTS{
        let gf = GcodeFile::blocking_parse(*data).unwrap();

        assert!(gf.slicer.slicer.is_some_and(|s|s.eq(slicer)));
        assert!(gf.slicer.version.is_some_and(|v|v.eq(version)));
        assert!(gf.meta.total_layers_count == Some(*layers));
        assert!(gf.meta.filament_length_used == Some(*filament_length));
        assert!(gf.meta.estimated_print_time == Some(*est_time));
    }
}