use regex::Regex;
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt::Display;

use crate::perm::roles::ROLE_TABLES;

use super::cache::STORE_HOUSE_CACHE;

pub fn deser_f32<'de, D>(de: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(de)?;
    match value {
        Value::Number(value) => {
            if let Some(f) = value.as_f64() {
                Ok(f as f32)
            } else if let Some(f) = value.as_i64() {
                Ok(f as f32)
            } else {
                Ok(value.as_u64().unwrap_or(0) as f32)
            }
        }
        Value::String(value) => {
            if let Ok(f) = value.parse::<f32>() {
                Ok(f)
            } else {
                Err(serde::de::Error::custom(
                    "浮点数格式格式错误，请检查所有字符串浮点数是否格式正确",
                ))
            }
        }
        _ => Err(serde::de::Error::custom(
            "浮点数格式格式错误，请检查所有字符串浮点数是否格式正确",
        )),
    }
}

pub fn split_files<S>(value: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let split = if let Some(value) = value {
        value.split('&').collect::<Vec<&str>>()
    } else {
        Vec::new()
    };
    Serialize::serialize(&split, serializer)
}

pub fn serialize_f32_to_string<S>(value: &f32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(value.to_string().as_str())
}

struct BoolVisitor;

impl<'de> Visitor<'de> for BoolVisitor {
    type Value = bool;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_fmt(format_args!(
            "无法转换成bool格式，请确保值为0/1，true/false"
        ))
    }
    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v)
    }
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v == 1)
    }
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v == 1)
    }
}

pub fn deserialize_any_to_bool<'de, D>(de: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    de.deserialize_any(BoolVisitor)
}

pub fn deserialize_bool_to_i32<'de, D>(de: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    let value: bool = Deserialize::deserialize(de)?;
    Ok(op::ternary!(value => 0; 1))
}
pub fn serialize_empty_to_none<S>(value: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(k) if !k.is_empty() => serializer.serialize_str(k),
        _ => serializer.serialize_none(),
    }
}
pub fn deser_empty_to_none<'de, D>(de: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<String> = Deserialize::deserialize(de)?;
    Ok(value.and_then(|s| op::ternary!(s.is_empty() => None; Some(s))))
}
pub fn serialize_i32_to_bool<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize + Display,
    S: Serializer,
{
    let flag = value.to_string().parse().unwrap_or(1);
    if flag == 0 {
        serializer.serialize_bool(true)
    } else {
        serializer.serialize_bool(false)
    }
}
pub fn serialize_bool_to_i32<S>(value: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if *value {
        serializer.serialize_i32(1)
    } else {
        serializer.serialize_i32(0)
    }
}

pub fn serialize_null_to_default<S>(
    value: &Option<String>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(value) => serializer.serialize_str(value),
        _ => serializer.serialize_str(""),
    }
}

// pub fn serialize_custom_fields(
//     value: &HashMap<String, Vec<crate::pages::>>,
//     serializer: S,
// ) -> Result<S::Ok, S::Error>
// where
//     S: Serializer,
// {
//     match value {
//         Some(value) => serializer.serialize_str(value),
//         _ => serializer.serialize_str(""),
//     }
// }

pub fn serialize_role<S>(id: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let name = unsafe {
        ROLE_TABLES
            .get_name(id)
            .map_or(id.into(), |v| v.to_string())
    };

    serializer.serialize_str(&name)
}

pub fn op_deserialize_storehouse<'de, D>(de: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let Ok::<String, _>(name) = Deserialize::deserialize(de) else {
        return Ok(None);
    };
    if name.is_empty() {
        return Ok(None);
    }
    let flag = STORE_HOUSE_CACHE.iter().any(|s| s.name.eq(&name));
    if flag {
        Ok(Some(name))
    } else {
        Err(serde::de::Error::custom("库房不匹配"))
    }
}
pub fn deserialize_storehouse<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let name: String = Deserialize::deserialize(de)?;
    let flag = STORE_HOUSE_CACHE.iter().any(|s| s.name.eq(&name));
    if flag {
        Ok(name)
    } else {
        Err(serde::de::Error::custom("库房不匹配"))
    }
}

pub fn deserialize_inventory<'de, D>(de: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    let num: Value = Deserialize::deserialize(de)?;
    match num {
        Value::Number(n) => Ok(n.as_i64().map_or(0, |i| i as i32)),
        Value::String(s) => Ok(s.parse().unwrap_or_default()),
        _ => Err(serde::de::Error::custom("库存数量格式错误")),
    }
}

pub fn deserialize_role<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let name: String = Deserialize::deserialize(de)?;
    let id = unsafe { ROLE_TABLES.get_id(&name).map_or(name, |v| v.to_string()) };
    Ok(id)
}

pub fn deserialize_roles<'de, D>(de: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let roles: Vec<String> = Deserialize::deserialize(de)?;
    Ok(unsafe {
        roles
            .into_iter()
            .map(|r| ROLE_TABLES.get_id(&r).map_or(r, |v| v.to_string()))
    }
    .collect())
}

pub fn deserialize_mm_dd<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    regex_time(r"(\d{2})-(\d{2})", de, "MM-DD")
}
fn regex_time<'de, D: Deserializer<'de>>(re: &str, de: D, err: &str) -> Result<String, D::Error> {
    let time: String = Deserialize::deserialize(de)?;
    println!("{}", time);
    if time.is_empty() {
        return Ok(String::new());
    }
    let regex = Regex::new(re).unwrap();
    if let Some(value) = op::catch!(regex.captures(&time)?.get(0)) {
        Ok(value.as_str().to_owned())
    } else {
        Err(serde::de::Error::custom(format!(
            "Invalid Time Format. 时间格式应当为'{err}'"
        )))
    }
}

static YYYY_MM_DD: &str = r"(\d{4})-(\d{2})-(\d{2})";

pub fn deserialize_time_scope<'de, D>(de: D) -> Result<(String, String), D::Error>
where
    D: Deserializer<'de>,
{
    let time: String = Deserialize::deserialize(de)?;
    let split: Vec<_> = time.splitn(2, '~').collect();
    let err = serde::de::Error::custom("时间范围格式错误，必须为`YYYY-MM-DD~YYYY-MM-DD`");
    if split.len() != 2 {
        return Err(err);
    }
    let regex = Regex::new(YYYY_MM_DD).unwrap();
    if split.iter().all(|s| regex.is_match(s)) {
        Ok((split[0].to_owned(), split[1].to_owned()))
    } else {
        Err(err)
    }
}

pub fn deser_yyyy_mm_dd<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    regex_time(r"(\d{4})-(\d{2})-(\d{2})", de, "YYYY-MM-DD")
}
pub fn deser_yyyy_mm_dd_hh_mm_ss<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    regex_time(
        r"(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2})",
        de,
        "YYYY-MM-DD HH:MM:SS",
    )
}
pub fn deser_yyyy_mm_dd_hh_mm<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    regex_time(
        r"(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2})",
        de,
        "YYYY-MM-DD HH:MM",
    )
}

pub fn op_deser_yyyy_mm_dd_hh_mm_ss<'de, D>(de: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    op_regex_time(
        r"(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2})",
        de,
        "YYYY-MM-DD HH:MM",
    )
}
fn op_regex_time<'de, D: Deserializer<'de>>(
    re: &str,
    de: D,
    err: &str,
) -> Result<Option<String>, D::Error> {
    let Some::<String>(time) = Deserialize::deserialize(de).ok() else {
        return Ok(None);
    };
    if time.is_empty() {
        return Ok(None);
    }
    let regex = Regex::new(re).unwrap();
    if let Some(value) = op::catch!(regex.captures(&time)?.get(0)) {
        Ok(Some(value.as_str().to_owned()))
    } else {
        Err(serde::de::Error::custom(format!(
            "Invalid Time Format. 时间格式应当为'{err}'"
        )))
    }
}

pub fn op_deser_yyyy_mm_dd_hh_mm<'de, D>(de: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    regex_time(
        r"(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2})",
        de,
        "YYYY-MM-DD HH:MM",
    )
    .map(|s| op::ternary!(s.is_empty() => None; Some(s)))
}
