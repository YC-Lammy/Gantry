use std::pin::Pin;

use ahash::AHashMap;

pub type GcodeHandler = Box<dyn Fn(&GcodeVM, &[&str]) -> Pin<Box<dyn Future<Output = anyhow::Result<String>>>>>;

pub struct BGcodeFile{

}

pub struct GcodeVM{
    functions: AHashMap<String, GcodeHandler>
}

impl GcodeVM{
    pub fn new() -> Self{
        Self { 
            functions: AHashMap::new() 
        }
    }

    pub async fn run_gcodes(&self, file: &str) -> anyhow::Result<()>{
        for line in file.split_terminator('\n'){
            self.run_gcode_line(line.trim()).await?;
        }
        return Ok(())
    }

    pub async fn run_gcode_line(&self, mut line: &str) -> anyhow::Result<String>{
        // either it is empty or a comment
        if line == "" || line.starts_with(';'){
            return Ok(String::new())
        }
        // remove comment at line end
        if let Some((l, _)) = line.split_once(';'){
            line = l;
        }

        let mut iter = line.split(' ');

        let command = iter.next().unwrap();

        let mut params = Vec::new();

        for p in iter{
            if p == ""{
                continue;
            }

            params.push(p);
        }

        let handler = self.functions.get(command).ok_or(anyhow::Error::msg(format!("Unknown command: {}", command)))?;

        return (handler)(self, &params).await
    }
}

