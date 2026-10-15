use serde::{Deserialize, Serialize};

pub const SUPPORTED_DEVICES: &[&str] = &["Mouse", "Keyboard", "Tablet", "Touchscreen", "Controller"];

pub const DEFAULT_SECTIONS: &[&str] = &[
    "me",
    "top-ranks",
    "historical",
    "beatmaps",
    "medals",
    "recent-activity",
];

pub fn default_section_order() -> Vec<String> {
    DEFAULT_SECTIONS.iter().map(|s| s.to_string()).collect()
}

pub fn default_playmode() -> String {
    "osu".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileDetails {
    #[serde(default)]
    pub about: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub devices: Vec<String>,
    #[serde(default = "default_section_order")]
    pub section_order: Vec<String>,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub interests: String,
    #[serde(default)]
    pub occupation: String,
    #[serde(default)]
    pub twitter: String,
    #[serde(default)]
    pub website: String,
    #[serde(default)]
    pub discord: String,
    #[serde(default = "default_playmode")]
    pub playmode: String,
    #[serde(default)]
    pub join_date: String,
}

impl Default for ProfileDetails {
    fn default() -> Self {
        Self {
            about: String::new(),
            location: String::new(),
            devices: Vec::new(),
            section_order: default_section_order(),
            country: String::new(),
            interests: String::new(),
            occupation: String::new(),
            twitter: String::new(),
            website: String::new(),
            discord: String::new(),
            playmode: default_playmode(),
            join_date: String::new(),
        }
    }
}

impl ProfileDetails {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.about.len() > 5000 {
            return Err("About text exceeds maximum length of 5000 characters");
        }
        if self.location.len() > 100 {
            return Err("Location exceeds maximum length of 100 characters");
        }
        for device in &self.devices {
            if !SUPPORTED_DEVICES.iter().any(|d| d.eq_ignore_ascii_case(device)) {
                return Err("Unsupported input device");
            }
        }
        if self.section_order.len() != DEFAULT_SECTIONS.len() {
            return Err("Section order must contain exactly 6 sections");
        }
        for section in DEFAULT_SECTIONS {
            if !self.section_order.iter().any(|s| s == section) {
                return Err("Section order is missing required sections");
            }
        }
        if !self.country.is_empty() && crate::utils::country_code_to_byte(&self.country) == 0 {
            return Err("Invalid country code");
        }
        for (value, max, msg) in [
            (&self.interests, 100usize, "Interests exceed maximum length of 100 characters"),
            (&self.occupation, 100, "Occupation exceeds maximum length of 100 characters"),
            (&self.twitter, 50, "Twitter handle exceeds maximum length of 50 characters"),
            (&self.website, 200, "Website exceeds maximum length of 200 characters"),
            (&self.discord, 50, "Discord exceeds maximum length of 50 characters"),
        ] {
            if value.chars().count() > max {
                return Err(msg);
            }
        }
        if !self.playmode.is_empty() && !["osu", "taiko", "fruits", "mania"].contains(&self.playmode.as_str()) {
            return Err("Invalid default game mode");
        }
        if !self.join_date.is_empty() {
            let ok = self.join_date.len() == 7
                && self.join_date.as_bytes()[4] == b'-'
                && self.join_date[..4].parse::<u32>().map_or(false, |y| (2007..=2100).contains(&y))
                && self.join_date[5..].parse::<u32>().map_or(false, |m| (1..=12).contains(&m));
            if !ok {
                return Err("Join date must look like 2016-08");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_details_validation() {
        let valid = ProfileDetails::default();
        assert!(valid.validate().is_ok());

        let mut invalid_about = ProfileDetails::default();
        invalid_about.about = "a".repeat(5001);
        assert!(invalid_about.validate().is_err());

        let mut invalid_location = ProfileDetails::default();
        invalid_location.location = "b".repeat(101);
        assert!(invalid_location.validate().is_err());

        let mut bad_join = ProfileDetails::default();
        bad_join.join_date = "2016-13".to_string();
        assert!(bad_join.validate().is_err());
        let mut good_join = ProfileDetails::default();
        good_join.join_date = "2016-08".to_string();
        assert!(good_join.validate().is_ok());

        let mut invalid_device = ProfileDetails::default();
        invalid_device.devices.push("SmartFridge".to_string());
        assert!(invalid_device.validate().is_err());

        let mut valid_device = ProfileDetails::default();
        valid_device.devices.push("Tablet".to_string());
        valid_device.devices.push("keyboard".to_string());
        assert!(valid_device.validate().is_ok());

        let mut invalid_sections = ProfileDetails::default();
        invalid_sections.section_order = vec!["me".to_string(), "me".to_string()];
        assert!(invalid_sections.validate().is_err());

        let mut invalid_country = ProfileDetails::default();
        invalid_country.country = "ZZZ".to_string();
        assert!(invalid_country.validate().is_err());

        let mut valid_country = ProfileDetails::default();
        valid_country.country = "DE".to_string();
        assert!(valid_country.validate().is_ok());
    }
}
