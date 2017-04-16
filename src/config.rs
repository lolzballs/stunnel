use errors::*;

use std::fs::File;
use std::io::Read;
use std::path::Path;
use toml;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub listen: String,
    pub remote: String,
    pub sni_addr: Option<String>,
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(name: P) -> ::Result<Self> {
        let file = try!(File::open(name).map(|mut f| {
                                                 let mut s = String::new();
                                                 f.read_to_string(&mut s);
                                                 s
                                             }));
        Ok(try!(toml::from_str(&file)))
    }
}
