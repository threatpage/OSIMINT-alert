use chrono::prelude::*;
use std::fs::File;
use std::fs::OpenOptions;
use std::hash::{Hash, Hasher};
use std::io::prelude::*;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::hashing;
use crate::map_services;
extern crate serde_derive;

extern crate serde;
extern crate serde_json;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Jobs {
    pub name: String,
    pub source: String,
    pub layer: String,
    pub api_key: String,
    pub lat_min: f64,
    pub lat_max: f64,
    pub lon_min: f64,
    pub lon_max: f64,
    pub frequency_hours: u32,
    pub frequency_minutes: u32,
    pub frequency_seconds: u32,
    pub frequency_days: u32,
}

pub fn create_directories() {
    if !(Path::new("./logs/").exists()) {
        std::fs::create_dir("./logs").expect("failed to create log dir");
    }
    if !(Path::new("./imgs/").exists()) {
        std::fs::create_dir("./imgs").expect("failed to create imgs dir");
    }
    if !(Path::new("./jsons/").exists()) {
        std::fs::create_dir("./jsons").expect("failed to create jsons dir");
    }
}

pub fn write_log(log_path: String, hash_value: u64) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(&log_path)
        .unwrap();
    let time: DateTime<Utc> = Utc::now();
    if let Err(e) = writeln!(file, "{},{}", hash_value, time.to_string()) {
        eprintln!("Couldn't write to file: {}", e);
    };
}

pub fn read_conf_json(json_path: String) -> Vec<Jobs> {
    let mut f = std::fs::File::open(json_path).expect("failed to load config json");
    let mut conf_string = String::new();
    f.read_to_string(&mut conf_string).unwrap();
    let v: Vec<Jobs> = serde_json::from_str(&conf_string).expect("cannot serialize json");
    v
}

pub fn get_url_function(
    job: &Jobs,
) -> fn(layer: String, lon0: f64, lat0: f64, lon1: f64, lat1: f64, key: String) -> String {
    match job.source.as_ref() {
        "Yandex" => map_services::get_yandex_url,
        "Google" => map_services::get_google_url,
        "Bing" => map_services::get_bing_url,
        "Wikimapia" => map_services::get_wikimapia_url,
        "OSM" => map_services::get_osm_url,
        _ => map_services::get_yandex_url,
    }
}

pub fn get_img_extension(job: &Jobs) -> String {
    if job.source == "Yandex" {
        match job.layer.as_ref() {
            "map" => "png".to_string(),
            _ => "jpg".to_string(),
        }
    } else {
        "jpg".to_string()
    }
}

fn save_wikimapia_json(site_name: String, hash_value: u64, buffer: Vec<WikiData>) {
    serde_json::to_writer(
        &File::create(format!("./jsons/{}_{}.json", site_name, hash_value)).unwrap(),
        &buffer,
    )
    .expect("failed to write json");
}

fn save_osm_json(site_name: String, hash_value: u64, buffer: Vec<OsmNode>) {
    serde_json::to_writer(
        &File::create(format!("./jsons/{}_{}.json", site_name, hash_value)).unwrap(),
        &buffer,
    )
    .expect("failed to write json");
}

pub fn save_image(site_name: String, hash_value: u64, img_extension: String, buffer: Vec<u8>) {
    let mut out = File::create(format!(
        "./imgs/{}_{}.{}",
        site_name, hash_value, img_extension
    ))
    .expect("failed to create file");
    let mut pos = 0;
    while pos < buffer.len() {
        let bytes_written = out.write(&buffer[pos..]);
        pos += bytes_written.unwrap();
    }
}

pub fn process_image_request(url: &String, site_name: &String, img_extension: &String) -> bool {
    create_directories(); // check if directories exist
    let resp = reqwest::get(url);
    if !resp.is_err() {
        let mut resp_cont = resp.unwrap();
        let mut buffer: Vec<u8> = vec![];
        resp_cont
            .copy_to(&mut buffer)
            .expect("Failed to copy image data"); // Copy requested image data to buffer
        let hash_value = hashing::calculate_hash(&buffer); // Compute Hash of image
                                                           // read previous hash:
        let mut last_hash: u64 = 0;
        let log_path = format!("./logs/{}.txt", &site_name);
        if Path::new(&log_path).exists() {
            let file = File::open(&log_path).unwrap();
            let reader = BufReader::new(file);
            let lines: Vec<String> = reader.lines().collect::<Result<_, _>>().unwrap();
            let last_line = lines.last(); // read last line of log
            last_hash = last_line.unwrap().split(',').collect::<Vec<&str>>()[0]
                .parse::<u64>()
                .unwrap();
        } else {
            File::create(&log_path).expect("Failed to create log file");
        }

        if hash_value != last_hash {
            // Image is different from last hash !
            // Save image, log into file
            save_image(
                site_name.to_string(),
                hash_value,
                img_extension.to_string(),
                buffer,
            );
            write_log(log_path, hash_value);
            true
        } else {
            false
        }
    } else {
        println!("Connection error: couldn't reach url");
        false
    }
}

#[derive(Deserialize)]
struct WikiResponse {
    #[allow(dead_code)]
    version: Option<String>,
    debug: Option<WikiDebug>,
    #[allow(dead_code)]
    language: Option<String>,
    #[allow(dead_code)]
    page: Option<u32>,
    #[allow(dead_code)]
    count: Option<u32>,
    found: Option<String>,
    folder: Option<Vec<WikiData>>,
}

#[derive(Deserialize, Serialize)]
struct WikiDebug {
    #[allow(dead_code)]
    code: Option<u32>,
    message: String,
}

#[derive(Deserialize, Serialize)]
struct LocationData {
    #[allow(dead_code)]
    north: f64,
    east: f64,
    south: f64,
    west: f64,
}
#[derive(Deserialize, Serialize)]
struct CoordsData {
    #[allow(dead_code)]
    x: f64,
    y: f64,
}
#[derive(Deserialize, Serialize)]
struct WikiData {
    #[allow(dead_code)]
    id: Option<String>, // Option because of badly formated strings
    name: Option<String>,
    url: Option<String>,
    location: LocationData,
    polygon: Vec<CoordsData>,
}
impl Hash for WikiData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.location.hash(state);
        self.polygon.hash(state);
        match &self.name {
            Some(x) => x.hash(state),
            &None => (),
        }
    }
}

impl Hash for LocationData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ((1000.0 * self.north).floor() as u32).hash(state);
        ((1000.0 * self.east).floor() as u32).hash(state);
        ((1000.0 * self.south).floor() as u32).hash(state);
        ((1000.0 * self.west).floor() as u32).hash(state);
    }
}

impl Hash for CoordsData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ((1000.0 * self.x).floor() as u32).hash(state);
        ((1000.0 * self.y).floor() as u32).hash(state);
    }
}

pub fn process_wikimapia_json_request(url: &String, site_name: &String) -> bool {
    create_directories(); // check if directories exist
    let mut url_mod = format!("{}&page=1", url);
    let resp = reqwest::get(&url_mod);
    if !resp.is_err() {
        let mut resp = resp.unwrap();
        let mut page_id = 2;
        let mut all_data: Vec<WikiData> = vec![];
        if resp.status().is_success() {
            let content: WikiResponse = resp.json().unwrap();
            if content.version.is_none() {
                let debug = content.debug.unwrap();
                println!("error {}, {}", &debug.code.unwrap(), &debug.message);
            } else {
                let mut data = content.folder.unwrap();
                all_data.append(&mut data);
                let number_of_items = content.found;
                if let Some(number_of_items_string) = number_of_items {
                    let number_of_items_int = number_of_items_string.parse::<i32>().unwrap();
                    let number_of_pages = (number_of_items_int as f64 / 100.0).floor() as u32 + 1;
                    while page_id < number_of_pages + 1 {
                        url_mod = format!("{}&page={}", url, page_id);
                        let mut resp = reqwest::get(&url_mod).unwrap();
                        if resp.status().is_success() {
                            let content: WikiResponse = resp.json().unwrap();
                            if content.version.is_none() {
                                let debug = content.debug.unwrap();
                                println!("error {}, {}", &debug.code.unwrap(), &debug.message);
                            } else {
                                let mut data = content.folder.unwrap();
                                all_data.append(&mut data);
                            }
                        }
                        page_id += 1;
                    }
                }
            }

            let hash_value = hashing::calculate_hash(&all_data);
            let mut last_hash: u64 = 0;
            let log_path = format!("./logs/{}.txt", &site_name);
            if Path::new(&log_path).exists() {
                let file = File::open(&log_path).expect("Couldn't open log file");
                let reader = BufReader::new(file);
                let lines: Vec<String> = reader.lines().collect::<Result<_, _>>().unwrap();
                let last_line = lines.last(); // read last line of log
                last_hash = last_line.unwrap().split(',').collect::<Vec<&str>>()[0]
                    .parse::<u64>()
                    .unwrap();
            } else {
                File::create(&log_path).expect("Failed to create log file");
            }
            if hash_value != last_hash {
                // Image is different from last hash !
                // Save image, log into file
                save_wikimapia_json(site_name.to_string(), hash_value, all_data);
                write_log(log_path, hash_value);
                true
            } else {
                false
            }
        } else if resp.status().is_server_error() {
            println!("server error!");
            false
        } else {
            println!("Something else happened. Status: {:?}", resp.status());
            false
        }
    } else {
        println!("Couldn't send request. {}", resp.err().unwrap());
        false
    }
}

#[derive(Deserialize)]
struct OverpassResponse {
    #[allow(dead_code)]
    version: Option<f64>,
    #[allow(dead_code)]
    generator: Option<String>,
    #[allow(dead_code)]
    osm3s: Option<Osm3s>,
    elements: Option<Vec<OsmNode>>,
}

#[derive(Deserialize, Serialize)]
struct OsmNode {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    type_: String,
    id: i64,
    lat: Option<f64>,
    lon: Option<f64>,
    nodes: Option<Vec<i64>>,
    tags: Option<serde_json::Value>, // Various OSM tag > not strongly typed
}

#[derive(Deserialize)]
struct Osm3s {
    #[allow(dead_code)]
    timestamp_osm_base: String,
    #[allow(dead_code)]
    copyright: String,
}
impl Hash for OsmNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        if !self.nodes.is_none() {
            self.nodes.hash(state);
        }
    }
}
pub fn process_osm_json_request(url: &String, site_name: &String) -> bool {
    create_directories(); // check if directories exist

    let resp = reqwest::get(url);
    if !resp.is_err() {
        let mut resp = resp.unwrap();
        if resp.status().is_success() {
            let content: OverpassResponse = resp.json().unwrap();
            let data = content.elements.unwrap();
            let hash_value = hashing::calculate_hash(&data);
            let mut last_hash: u64 = 0;
            let log_path = format!("./logs/{}.txt", &site_name);
            if Path::new(&log_path).exists() {
                let file = File::open(&log_path).expect("Couldn't open log file");
                let reader = BufReader::new(file);
                let lines: Vec<String> = reader.lines().collect::<Result<_, _>>().unwrap();
                let last_line = lines.last(); // read last line of log
                last_hash = last_line.unwrap().split(',').collect::<Vec<&str>>()[0]
                    .parse::<u64>()
                    .unwrap();
            } else {
                File::create(&log_path).expect("Failed to create log file");
            }
            if hash_value != last_hash {
                // Image is different from last hash !
                // Save image, log into file
                save_osm_json(site_name.to_string(), hash_value, data);
                write_log(log_path, hash_value);
                true
            } else {
                false
            }
        } else if resp.status().is_server_error() {
            println!("server error!");
            false
        } else {
            println!("Something else happened. Status: {:?}", resp.status());
            false
        }
    } else {
        println!("Couldn't send request. {}", resp.err().unwrap());
        false
    }
}
