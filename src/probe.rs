use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

const W1_DEVICES_PATH: &str = "/sys/bus/w1/devices";

pub struct Probe {
    _id: String,
    pub name: String,
    pub path: String,
}

impl Probe {
    pub fn set_resolution(&self, bits: u8) -> io::Result<()> {
        let resolution_path = self.path.replace("/w1_slave", "/resolution");
        fs::write(resolution_path, bits.to_string())
    }

    pub fn read_temperature(&self) -> io::Result<f32> {
        let data = fs::read_to_string(&self.path)?;
        parse_temperature_data(&data)
    }
}

fn parse_temperature_data(data: &str) -> io::Result<f32> {
    // the file format looks like:
    // 6d 01 55 05 7f a5 a5 66 3e : crc=3e YES
    // 6d 01 55 05 7f a5 a5 66 3e t=22812

    // check the crc
    if !data.contains("YES") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "crc check failed",
        ));
    }

    // find and read the temperature data
    if let Some(temp_pos) = data.find("t=") {
        let temp_str = data[temp_pos + 2..].trim();
        if let Ok(temp_raw) = temp_str.parse::<i32>() {
            return Ok(temp_raw as f32 / 1000.0);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "failed to parse temperature",
    ))
}

pub fn discover_probes(labels: &HashMap<String, String>) -> io::Result<Vec<Probe>> {
    let mut probes = Vec::new();

    if !Path::new(W1_DEVICES_PATH).exists() {
        eprintln!(
            "warning: {} not found. make sure w1-gpio is enabled.",
            W1_DEVICES_PATH
        );
        return Ok(probes);
    }

    for entry in fs::read_dir(W1_DEVICES_PATH)? {
        let id = entry?.file_name().to_string_lossy().to_string();

        if id.starts_with("28-") {
            let name = labels.get(&id).cloned().unwrap_or_else(|| id.clone());
            probes.push(Probe {
                _id: id.clone(),
                name,
                path: format!("{}/{}/w1_slave", W1_DEVICES_PATH, id),
            });
        }
    }

    Ok(probes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_temperature() {
        let data = "6d 01 55 05 7f a5 a5 66 3e : crc=3e YES\n\
                    6d 01 55 05 7f a5 a5 66 3e t=22812\n";

        let temp = parse_temperature_data(data).unwrap();
        assert_eq!(temp, 22.812);
    }

    #[test]
    fn test_parse_negative_temperature() {
        let data = "50 05 4b 46 7f ff 0c 10 1c : crc=1c YES\n\
                    50 05 4b 46 7f ff 0c 10 1c t=-10562\n";

        let temp = parse_temperature_data(data).unwrap();
        assert_eq!(temp, -10.562);
    }

    #[test]
    fn test_parse_zero_temperature() {
        let data = "00 00 00 00 00 00 00 00 00 : crc=00 YES\n\
                    00 00 00 00 00 00 00 00 00 t=0\n";

        let temp = parse_temperature_data(data).unwrap();
        assert_eq!(temp, 0.0);
    }

    #[test]
    fn test_parse_crc_failure() {
        let data = "6d 01 55 05 7f a5 a5 66 3e : crc=3e NO\n\
                    6d 01 55 05 7f a5 a5 66 3e t=22812\n";

        let result = parse_temperature_data(data);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_parse_missing_temperature() {
        let data = "6d 01 55 05 7f a5 a5 66 3e : crc=3e YES\n\
                    6d 01 55 05 7f a5 a5 66 3e\n";

        let result = parse_temperature_data(data);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_parse_malformed_temperature() {
        let data = "6d 01 55 05 7f a5 a5 66 3e : crc=3e YES\n\
                    6d 01 55 05 7f a5 a5 66 3e t=invalid\n";

        let result = parse_temperature_data(data);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_parse_empty_string() {
        let data = "";

        let result = parse_temperature_data(data);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }
}
