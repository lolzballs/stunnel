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
        let string = File::open(name)
            .and_then(|mut file| {
                          let mut s = String::new();
                          file.read_to_string(&mut s).map(|_| s)
                      })?;
        Ok(toml::from_str(&string)?)
    }
}
