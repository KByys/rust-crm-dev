use regex::Regex;
lazy_static::lazy_static! {
    pub static ref YYYYMMDD_HHMMSS_REGEX: Regex = Regex::new(r"(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2})").unwrap();
    pub static ref YYYYMMDD_REGEX: Regex = Regex::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap();
}
