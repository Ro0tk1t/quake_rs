pub mod quake {
    // use log::{debug, error, info};
    use reqwest::header::{HeaderMap, HeaderValue};
    use serde_json::{Value, Map, Number};
    use crate::common::{Service, Output, Host, AggService};
    use crate::api::ApiKey;
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::{io, fs};
    use chrono::{Local, Duration, NaiveDate};
    use regex::{Regex};
    use ansi_term::Colour::Red;
    use reqwest::blocking::Response;

    //BaseUrl is the basis for all of our api requests.
    const BASE_URL: &'static str = "https://quake.360.cn";

    pub struct Quake{
        api_key: String,
    }

    impl Quake{


        pub fn new(api_key: String) -> Quake{
            Quake{
                api_key
            }
        }


        pub fn query_host(query_string:&str, start: i32, size:i32)->Value{
            Output::info(&format!("Search with {}", query_string));
            let res= ApiKey::get_api().expect("Failed to read apikey:\t");

            let h = Host{
                query: String::from(query_string),
                start,
                size,
                ignore_cache: false,
            };
            let response:Value = match Quake::new(res).search_host(&h) {
                Ok(response) => response,
                Err(e) =>{
                    Output::error(&format!("Query failed: {}", e.to_string()));
                    std::process::exit(1);
                }
            };
            response
        }


        pub fn search_host(&self, host:&Host)->Result<Value, serde_json::Error>{
            let mut url = String::new();
            url.push_str(BASE_URL);
            url.push_str("/api/v3/search/quake_host");
            let client = reqwest::blocking::Client::new();
            let resp = match client.post(&url).headers(self.header()).json(&host).send(){
                Ok(resp) => resp,
                Err(e) =>{
                    if e.is_timeout(){
                        Output::error("Connect Timeout!!");
                    }else {
                        Output::error(&format!("Connect error!!!\r\n{}", e.to_string()));
                    }
                    std::process::exit(1);
                }
            };
            let res = resp.text().unwrap();
            let response:Value = serde_json::from_str(&res)?;

            let code = response["code"].as_i64().unwrap() as i32;
            let message = response["message"].as_str().unwrap();
            if code != 0{
                Output::error(&format!("Query failed: {}", message));
                std::process::exit(1);
            }
            Ok(response)
        }


        pub fn query(query_string:&str, file_name:&str, start: i32, size:i32, time_start:&str, time_end:&str) -> Value{
            let res= ApiKey::get_api().expect("Failed to read apikey:\t");

            let mut s = Service{
                query: "".to_string(),
                start,
                size,
                ignore_cache: false,
                start_time: "".to_string(),
                end_time: "".to_string(),
                ip_list: vec![]
            };
            let (local, one_years_ago) = Self::getdate();
            if time_start == "" && time_end == ""{
                s.start_time = one_years_ago;
                s.end_time = local;
            }else if time_start != "" && time_end == ""{
                s.start_time = time_start.to_string();
                s.end_time = local;
            }else if time_start == "" && time_end != ""{
                s.start_time = Self::getdate_for_manual(time_end);
                s.end_time = time_end.to_string();
            }else if time_start != "" && time_end != ""{
                s.start_time = time_start.to_string();
                s.end_time = time_end.to_string();
            }
            if file_name != ""{
                let ips:String = match fs::read_to_string(file_name){
                    Ok(res) => res,
                    Err(err) =>{
                        Output::error(&format!("Failed to read {} : {}",file_name, err.to_string()));
                        std::process::exit(1);
                    }
                };
                s.ip_list = ips.lines().map(|s|Value::String(s.to_string())).collect();
            }
            if query_string != ""{
                s.query = format!("{}", query_string);
                Output::info(&format!("Search with {}", query_string));
            }else {
                Output::info(&format!("Search for {} IPs", s.ip_list.len()));
            }
            Output::info(&format!("Data time again {} to {}.", s.start_time, s.end_time));
            let response:Value = match Quake::new(res).search(s) {
                Ok(response) => response,
                Err(e) =>{
                    Output::error(&format!("Query failed: {}", e.to_string()));
                    std::process::exit(1);
                }
            };
            response
        }


        pub fn search(&self, service:Service) -> Result<Value, serde_json::Error>{
            let mut url = String::new();
            url.push_str(BASE_URL);
            url.push_str("/api/v3/search/quake_service");
            let client = reqwest::blocking::Client::new();
            let post_data:Map<String, Value> = Self::get_service_post_data(service);
            let resp:Response = match client.post(&url).headers(self.header()).json(&post_data).send(){
                Ok(resp) => resp,
                Err(e) =>{
                    if e.is_timeout(){
                        Output::error("Connect Timeout!!");
                    }else {
                        Output::error(&format!("Connect error!!!\r\n{}", e.to_string()));
                    }
                    std::process::exit(1);
                }
            };
            let res = match resp.text() {
                Ok(resp) => resp,
                Err(e) =>{
                    if e.is_timeout(){
                        Output::error("Connect Timeout!!");
                    }else {
                        Output::error(&format!("Connect error!!!\r\n{}", e.to_string()));
                    }
                    std::process::exit(1);
                }
            };
            let response:Value = serde_json::from_str(&res)?;

            let code = response["code"].as_i64().unwrap() as i32;
            let message = response["message"].as_str().unwrap();
            if code != 0{
                Output::error(&format!("Query failed: {}", message));
                std::process::exit(1);
            }
            Ok(response)
        }


        pub fn show(value:Value, showdata:bool, filter:&str, mut data_type: Vec<&str>) -> Vec<String>{
            let count = value["meta"]["pagination"]["count"].as_i64().unwrap() as usize;
            let total =  value["meta"]["pagination"]["total"].as_i64().unwrap() as i32;
            let mut res:Vec<String> = Vec::new();
            Output::success("Successful.");
            Output::success(&format!("count: {} \ttotal: {}", count, total));
            let re = Regex::new(filter).unwrap();
            for i in 0..count{
                let data_value = value["data"][i].as_object().unwrap();
                let title = data_value["service"]["http"]["title"].as_str().unwrap_or("").replace("\"", "").replace("\t", "").replace("\n","").replace("\r", "");
                let domain = data_value["service"]["http"]["host"].as_str().unwrap_or("").replace("\"", "").replace("\t", "").replace("\n","").replace("\r", "");
                let ip = data_value["ip"].as_str().unwrap().replace("\"", "");
                let port = &data_value["port"];
                let country = data_value["location"]["country_cn"].as_str().unwrap_or("");
                let province = data_value["location"]["province_cn"].as_str().unwrap_or("");
                let city = data_value["location"]["city_cn"].as_str().unwrap_or("");
                let owner = data_value["location"]["owner"].as_str().unwrap_or("");
                let time = data_value["time"].as_str().unwrap_or("");
                let ssl:&str = match data_value["service"]["tls"]["server_certificates"]
                    ["certificate"]["parsed"]["subject"]["common_name"].as_array(){
                    Some(ssl) =>{
                        ssl[0].as_str().unwrap_or("")
                    } ,
                    None => match data_value["service"]["tls"]["handshake_log"]["server_certificates"]
                            ["certificate"]["parsed"]["subject"]["common_name"].as_array(){
                            Some(ssl) =>{
                                ssl[0].as_str().unwrap_or("")
                            } ,
                            None => ""
                        }
                };
                let mut regex_data = String::new();
                if filter != ""{
                    let cert = data_value["service"]["cert"].as_str().unwrap_or("");
                    let response = data_value["service"]["response"].as_str().unwrap_or("");
                    let http_body = data_value["service"]["http"]["body"].as_str().unwrap_or("");
                    let http_header = data_value["service"]["http"]["response_headers"].as_str().unwrap_or("");
                    regex_data.push_str(cert);
                    regex_data.push_str(response);
                    regex_data.push_str(http_body);
                    regex_data.push_str(http_header);
                }
                let regex_res = match re.find(regex_data.as_str()){
                    Some(res) => res.as_str(),
                    None => ""
                };
                let mut f:String = String::new();
                for data in data_type.iter_mut(){
                    if data == &"title"{
                        f.push_str(&format!("{}\t", title));
                    }
                    if data == &"ip"{
                        f.push_str(&format!("{}\t", ip));
                    }
                    if data == &"port"{
                        f.push_str(&format!("{}\t", port));
                    }
                    if data == &"country"{
                        f.push_str(&format!("{}\t", country));
                    }
                    if data == &"province"{
                        f.push_str(&format!("{}\t", province));
                    }
                    if data == &"city"{
                        f.push_str(&format!("{}\t", city));
                    }
                    if data == &"owner"{
                        f.push_str(&format!("{}\t", owner));
                    }
                    if data == &"time"{
                        f.push_str(&format!("{}\t", time));
                    }
                    if data == &"domain"{
                        if !ipaddress::IPAddress::is_valid(domain.clone()){
                            f.push_str(&format!("{}\t", domain));
                        }else {
                            f.push_str(&format!("{}\t", ""));
                        }
                    }
                    if data == &"ssldomain"{
                        f.push_str(&format!("{}\t", ssl))
                    }
                }
                if showdata{
                    print!("{}", f);
                    println!("{}", Red.bold().paint(regex_res).to_string().as_str());
                }else {
                    f.push_str(regex_res);
                }
                res.push(f);
            }
            res
        }

        pub fn show_host(value: Value, show_data:bool) -> Vec<String>{
            let mut value = value;
            let mut res:Vec<String> = Vec::new();
            let count = value["meta"]["pagination"]["count"].as_i64().unwrap() as usize;
            let total =  value["meta"]["pagination"]["total"].as_i64().unwrap() as i32;
            Output::success("Successful.");
            Output::success(&format!("count: {} \ttotal: {}", count, total));
            for i in 0..count{
                // ip
                let data = value["data"][i].take();
                let ip = data["ip"].as_str().unwrap().replace("\"", "");
                let location = data["location"].as_object().unwrap();
                let country = location["country_en"].as_str().unwrap_or("");
                let province = location["province_en"].as_str().unwrap_or("");
                let city = location["city_en"].as_str().unwrap_or("");
                let service = data["services"].as_array().unwrap();
                let mut info = String::new();
                info.push_str(&format!("IP: {}\tCountry: {}\tProvince: {}\tCity: {}\n",
                                       ip, country, province, city));
                info.push_str(&format!("{port}\t{protocol:>width$}\t{time:>width$}\n",
                                       port="| Port", protocol="Protocol", time="time", width=20));
                for s in service{
                    let protocol = s["name"].as_str().unwrap().replace("\"", "");
                    let service_time = s["time"].as_str().unwrap().replace("\"", "").replace("unknown", "");
                    info.push_str(&format!("| {port}\t{protocol:>width$}\t{time:>width$}\n",
                                           port=s["port"], protocol=protocol, time=service_time, width=20));
                }
                info.push_str("\n");
                if show_data {
                    println!("{}", info);
                }
                res.push(info);
            }
            res
        }

        pub fn show_domain(value: Value, onlycount: bool, showdata:bool, mut data_type: Vec<&str>) -> Vec<String>{
            let mut value = value;
            let mut res:Vec<String> = Vec::new();
            let count = value["meta"]["pagination"]["count"].as_i64().unwrap() as usize;
            let total =  value["meta"]["pagination"]["total"].as_i64().unwrap() as i32;
            Output::success("Successful.");
            Output::success(&format!("count: {} \ttotal: {}", count, total));
            if !onlycount{
                for i in 0..count{
                    let data_value = value["data"][i].take();
                    let domain = data_value["service"]["http"]["host"].as_str().unwrap_or("").replace("\"", "");
                    let title = data_value["service"]["http"]["title"].as_str().unwrap_or("").replace("\"", "").replace("\t", "").replace("\n","").replace("\r", "");
                    let ip = data_value["ip"].as_str().unwrap().replace("\"", "");
                    let port = &data_value["port"];
                    let mut f = String::new();
                    for data in data_type.iter_mut(){
                        if data == &"domain"{
                            f.push_str(&format!("{}\t", domain));
                        }
                        if data == &"title"{
                            f.push_str(&format!("{}\t", title));
                        }
                        if data == &"ip"{
                            f.push_str(&format!("{}\t", ip));
                        }
                        if data == &"port"{
                            f.push_str(&format!("{}\t", port));
                        }
                    }
                    if showdata{
                        println!("{}", f);
                    }
                    res.push(f);
                }
            }else {
                if showdata{
                    println!("{}", total);
                }
            }
            res
        }

        pub fn save_domain_data(filename: &str, content: Value, data_type:Vec<&str>) ->io::Result<i32>{
            let mut f = OpenOptions::new().create(true).append(true).open(filename)?;
            let domains:Vec<String> = Self::show_domain(content, false, false, data_type);
            let mut count = 0;
            for domain in domains{
                f.write_all(format!("{}\n", domain).as_bytes())?;
                count += 1;
            }
            Ok(count)
        }

        pub fn save_host_data(filename: &str, content: Value)->io::Result<i32>{
            let mut f = OpenOptions::new().create(true).append(true).open(filename)?;
            let hosts = Self::show_host(content, false);
            let mut count = 0;
            for host in hosts{
                f.write_all(format!("{}\n", host).as_bytes())?;
                count += 1;
            }
            Ok(count)
        }


        pub fn save_search_data(filename: &str, content: Value, filter:&str, data_type: Vec<&str>)->io::Result<i32>{
            let mut f = OpenOptions::new().create(true).append(true).open(filename)?;
            let hosts = Self::show(content, false, filter, data_type);
            let mut count = 0;
            for host in hosts{
                f.write_all(format!("{}\n", host).as_bytes())?;
                count += 1;
            }
            Ok(count)
        }


        fn aggservice(&self, agg:&AggService) -> Result<Value, serde_json::Error>{
            let mut url = String::new();
            url.push_str(BASE_URL);
            url.push_str("/api/v3/aggregation/quake_service");
            let client = reqwest::blocking::Client::new();
            let resp = match client.post(&url).headers(self.header()).json(&agg).send(){
                Ok(resp) => resp,
                Err(e) =>{
                    if e.is_timeout(){
                        Output::error("Connect Timeout!!");
                    }else {
                        Output::error(&format!("Connect error!!!\r\n{}", e.to_string()));
                    }
                    std::process::exit(1);
                }
            };
            let res = resp.text().unwrap();
            let response:Value = serde_json::from_str(&res)?;

            let code = response["code"].as_i64().unwrap() as i32;
            let message = response["message"].as_str().unwrap();
            if code != 0{
                Output::error(&format!("Query failed: {}", message));
                std::process::exit(1);
            }
            Ok(response)
        }

        pub fn honeypot(ip:String){
            Output::info(&format!("Search with {}", ip));
            let mut query = String::from("app: \"*蜜罐*\" AND ip:");
            query += &ip;
            let res=ApiKey::get_api().expect("Failed to read apikey:\t");

            let s = AggService{
                query,
                start: 0,
                size: 5,
                ignore_cache: false,
                aggregation_list: vec![String::from("app")]
            };
            let response:Value = match Quake::new(res).aggservice(&s) {
                Ok(response) => response,
                Err(e) =>{
                    Output::error(&format!("Query failed: {}", e.to_string()));
                    std::process::exit(1);
                }
            };
            let app = response["data"]["app"].as_array().unwrap();
            if app.len() >0{
                let app_name = app[0].as_object().unwrap();
                let honeypot = app_name["key"].as_str().unwrap().replace("蜜罐", "")
                    .replace("\"", "");
                Output::error(&format!("Looks like a {} honeypot system! ", honeypot));
            }else {
                Output::success("Looks like a real system!");
            }
        }

        // Interface for obtaining user information
        // https://quake.360.cn/quake/#/help?id=5fdb2a58dd0705216cbaa480&title=%E7%94%A8%E6%88%B7%E4%BF%A1%E6%81%AF%E6%8E%A5%E5%8F%A3
        // URL: https://quake.360.cn/api/v3/user/info
        // Parameters: None
        // Method: GET
        // Return: Result<Value, serde_json::Error>
        fn info(&self) ->Result<Value, serde_json::Error>{
            let mut url:String = String::new();
            url.push_str(BASE_URL);
            url.push_str("/api/v3/user/info");
            let clinet = reqwest::blocking::Client::new();
            let resp = match clinet.get(&url).headers(self.header()).send(){
                Ok(resp) => resp,
                Err(e)  => {
                    if e.is_timeout(){
                        Output::error("Connect Timeout!!");
                    }else {
                        Output::error(&format!("Connect error!!!\r\n{}", e.to_string()));
                    }
                    std::process::exit(1);
                }
            };
            let res = resp.text().unwrap();
            let response:Value = serde_json::from_str(&res)?;

            let code = response["code"].as_i64().unwrap() as i32;
            let message = response["message"].as_str().unwrap();
            if code != 0{
                Output::error(&format!("Query failed: {}", message));
                std::process::exit(1);
            }
            Ok(response)
        }

        // TODO:Comment
        pub fn show_info(){
            let res = ApiKey::get_api().expect("Failed to read apikey:\t");
            let info = match Quake::new(res).info(){
                Ok(value) => value,
                Err(e) =>{
                    Output::error(&format!("Query failed: {}", e.to_string()));
                    std::process::exit(1);
                }
            };
            let code = info["code"].as_i64().unwrap_or(-1) as i32;
            let message = info["message"].as_str().unwrap();
            let data = info["data"].as_object().unwrap();
            if code == 0{
                let credit = data["credit"].as_i64().unwrap_or(0);
                let persistent_credit = data["persistent_credit"].as_i64().unwrap_or(0);
                let username = data["user"]["username"].as_str().unwrap_or("无");
                let email = data["user"]["email"].as_str().unwrap_or("无");
                let mobile_phone = data["mobile_phone"].as_str().unwrap_or("无");
                let role = data["role"].as_array().unwrap();
                let mut role_info = String::new();
                Output::success("Successful.");
                Output::info(&format!("用户名:  {}", username));
                Output::info(&format!("邮  箱:  {}", email));
                Output::info(&format!("手  机:  {}", mobile_phone));
                Output::info(&format!("月度积分: {}", credit));
                Output::info(&format!("长效积分: {}", persistent_credit));
                for r in role{
                    let r = r["fullname"].as_str().unwrap_or("");
                    role_info.push_str(r);
                    role_info.push_str(",");
                }
                role_info.remove(role_info.len() - 1);
                Output::info(&format!("角  色:  {}", role_info));

            }else {
                Output::error(message);
            }

        }

        fn get_service_post_data(s:Service)->Map<String, Value> {
            let mut data:Map<String, Value> = Map::new();
            data.insert("start".to_string(), Value::Number(Number::from(s.start)));
            data.insert("size".to_string(), Value::Number(Number::from(s.size)));
            data.insert("ignore_cache".to_string(), Value::Bool(false));
            data.insert("start_time".to_string(), Value::String(s.start_time));
            data.insert("end_time".to_string(), Value::String(s.end_time));
            let sc: Vec<Value> = vec![
                Value::String("610ce2adb1a2e3e1632e67b1".to_string()),
                Value::String("610ce2fbda6d29df72ac56eb".to_string()),
                Value::String("612f5a5ad6b3bdb87961727f".to_string()),
            ];
            data.insert("shortcuts".to_string(), Value::Array(sc));
            if !s.ip_list.is_empty(){
                data.insert("query".to_string(), Value::String("is_latest:true".to_string()));
                data.insert("ip_list".to_string(), Value::Array(s.ip_list));
            }else {
                data.insert("query".to_string(), Value::String(s.query));
            }
            data
        }


        fn header(&self) -> HeaderMap {
            let mut header = HeaderMap::new();
            header.insert("X-QuakeToken", HeaderValue::from_str(self.api_key.as_str()).unwrap());
            header
        }

        // 获取指定时间，一年前的日期
        pub(crate) fn getdate_for_manual(manual_date:&str)->String{
            let manual_date = NaiveDate::parse_from_str(manual_date, "%Y-%m-%d").unwrap();
            let one_years_ago= manual_date - Duration::days(365);
            one_years_ago.format("%Y-%m-%d").to_string()
        }

        // 获取当前时间和当前时间一年前的时间
        pub(crate) fn getdate() ->(String, String){
            let local = Local::now();
            let one_years_ago = local - Duration::days(365);
            (local.format("%Y-%m-%d %H:%M:%S"). to_string(), one_years_ago.format("%Y-%m-%d %H:%M:%S").to_string())
        }
    }
}

