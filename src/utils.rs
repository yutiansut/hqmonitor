use std::process::Command;
use std::io;
use serde_json;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Configuration {
    pub _env:String,
    pub _txt_file: String,
    pub _dbf_file: String,

    pub _txt_interval : i32,
    pub _dbf_interval : i32,

    pub _start_time : u32,
    pub _end_time : u32,

    pub _monitor_processes : Vec<String>,
    pub _sms_addr : String,
    pub _sms_tag : String,
}

impl Configuration {

    pub fn load() -> io::Result<Configuration> {
        use std::fs::OpenOptions;

        let file = OpenOptions::new().read(true).open("default.cfg")?;

        use std::io::BufReader;
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();

        use std::io::Read;
        buf_reader.read_to_string(&mut contents)?;

        let r: serde_json::Result<Configuration> = serde_json::from_str(&contents);

        if let Ok(c) = r {
            return Ok(c);
        } else {
            println!("{:?}", r);
        }

        Err(io::Error::from(io::ErrorKind::InvalidData))
    }

    pub fn is_trade_time(&self, time:u32)->bool {

        if time >= self._start_time && time <= self._end_time {
            return true;
        }

        false
    }
}

impl Clone for Configuration {

    fn clone(&self)->Configuration {

        Configuration {
            _env : self._env.clone(),
            _dbf_file : self._dbf_file.clone(),
            _txt_interval : self._txt_interval,
            _dbf_interval : self._dbf_interval,
            _txt_file : self._txt_file.clone(),
            _start_time : self._start_time,
            _end_time : self._end_time,
            _monitor_processes : self._monitor_processes.clone(),
            _sms_addr : self._sms_addr.clone(),
            _sms_tag : self._sms_tag.clone(),
        }
    }
}

pub fn get_seconds(time:u32)->i32 {

    let h = time / 10000;
    let m = (time / 100) % 100;
    let s = time % 100;
    //println!("{} {} {} {}", time, h, m, s);

    (h * 3600 + m * 60 + s) as i32
}

pub fn get_current_time()->u32 {
    use chrono;
    use chrono::Timelike;
    let now = chrono::Local::now();
    let hour = now.hour();
    let time = hour * 10000 + now.minute() * 100 + now.second();

    time
}

pub fn now_to_string()->String {
    use chrono;
    let now = chrono::Local::now();

    now.to_rfc2822()
}

pub fn report_alarm(mut alarm : ::Alarm) {
    alarm._time = now_to_string();

    ::ALARM_MANAGER.active_alarm(alarm);
}

#[derive(Debug)]
pub struct ProcessInfo {
    _name: String,
    _pid: String,
}

impl ProcessInfo {
    fn new(name: String, pid: String) -> ProcessInfo {
        ProcessInfo {
            _name: name,
            _pid: pid,
        }
    }
}

pub fn get_all_processes() -> Option<Vec<ProcessInfo>> {
    let o = Command::new("cmd")
        .args(&["/C", "tasklist"])
        .output();

    let mut v = vec![];

    if let Ok(output) = o {
        let s = String::from_utf8_lossy(&output.stdout);

        use std::str::Lines;
        let ls: Lines = s.lines();

        let cs = ls.skip(3);

        let ps = cs.map(|l| {
            (l[0..25].to_string(), l[26..34].to_string())
        });

        for (n, p) in ps {
            v.push(ProcessInfo::new(
                n.trim_right().to_string(),
                p.trim_left().to_string(),
            ));
        }

        if v.len() > 0 {
            return Some(v);
        }
    }

    None
}

pub fn check_process(env:&str, name:&str)->io::Result<()> {
    let mut report = false;
    let ids = get_process_id(name);
    if let Some(ids2) = ids {
        let count = ids2.len();
        if count <= 0 {
            report = true;

        }
    } else {
        report = true;
    }

    if report {

        let content = format!("Process {} stopped", name);
        let alarm = ::Alarm {
            _id : 1000,
            _source : "HQ MONITOR".to_string(),
            _target : name.to_string(),
            _description : content,
            _time : Default::default(),
            _env : env.to_string(),
        };

        report_alarm(alarm);
    }

    Ok(())
}

pub fn get_process_id(name :&str)->Option<Vec<String>> {

    let mut v = vec![];
    //use std::path::Path;
    //let pt = Path::new(p);
    let o = get_all_processes();
    if let Some(ps) = o {
        for p in ps {
            if p._name.starts_with(name) {
                v.push(p._pid);
            }
        }

        if v.len() > 0 {
            return Some(v);
        }
    }

    None
}

//xml related

use xml;

use std::fs::File;
use std::io::BufReader;

use xml::reader::{EventReader, XmlEvent};
use std::collections::HashMap;

pub fn parse_workdays(days: &mut HashMap<String, bool>)
                      -> io::Result<()> {

    parse_xml("workday.xml", days)
}

pub fn parse_xml(file_name: &str,
                         days: &mut HashMap<String, bool>)
                         -> io::Result<()> {
    let file = File::open(file_name)?;
    let file = BufReader::new(file);

    let parser = EventReader::new(file);
    let mut depth = 0;
    let mut day = String::default();
    let mut is_trade = String::default();;

    let mut parent_name = String::default();

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                depth += 1;
                if depth == 3 {
                    parent_name = name.local_name;
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                depth -= 1;
                parent_name == String::default();
                if depth == 1 {
                    if name.local_name.eq("RECORD") {

                        let value = days
                            .entry(day.clone())
                            .or_insert(false);

                        if is_trade == "1" {
                            *value = true;
                        }

                        day = String::default();
                        is_trade = String::default();
                    }
                }
            }
            Ok(XmlEvent::Characters(text)) => {
                if depth == 3 {
                    match parent_name.as_str() {
                        "TradingDate" => {
                            day = text.trim().to_owned();
                            day.split_off(8);
                        }
                        "IfTradingDay" => {
                            is_trade = text.trim().to_owned();
                        }
                        _ => {}
                    };
                }
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
            _ => {}
        }
    } //end fn?

    Ok(())
}

use chrono;
use chrono::Datelike;
use chrono::Timelike;
pub fn get_today_date_time() -> (u32, u32) {

    let now = chrono::Local::now();
    //println!("now: {:?}", now);
    let date = (now.year() as u32) * 10000 + now.month() * 100 + now.day();
    let time = now.hour() * 10000 + now.minute() * 100 + now.second();

    (date, time)
}

use hyper;
use hyper::Client;
use std::io::Read;

pub fn send_msg(config:&Configuration, message:&str) {

    let client = Client::new();

    #[derive(Clone, Serialize)]
    struct Content {
        content:String,
        topic:String,
        tag:String,
    }

    let c = Content{
        content:message.to_string(),
        topic:"hq".to_string(),
        tag:config._sms_tag.clone(),
    };

    use serde_json;
    let cr = serde_json::to_string(&c);
    let content = cr.unwrap_or_default();
    if content.len() <= 3 {
        return;
    }

    println!("sms: {}", content);
    let mut headers = hyper::header::Headers::new();

    use hyper::header::ContentType;
    headers.set(ContentType::json());
    let res = client.post(&config._sms_addr).headers(headers).body(&content).send();

    match res {
        Ok(mut r)=>{
            println!("headers:\n {}", r.headers);
            let mut body = String::new();

            if let Err(e) = r.read_to_string(&mut body) {
                println!("body read error: {:?}", e);
            } else {
                println!("body:\n {}", body);
            }
        }
        Err(e)=>{
            println!("{:?}", e);
        }
    };

}