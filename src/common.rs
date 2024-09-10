use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Deserialize, Serialize, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Person {
    pub name: String,
    pub id: String,
}

impl Person {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn id(&self) -> &str {
        &self.id
    }
}
impl From<String> for Person {
    fn from(value: String) -> Self {
        Self {
            name: String::new(),
            id: value,
        }
    }
}
pub fn empty_deserialize_to_none<'de, D, T: From<String>>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<String> = Deserialize::deserialize(de)?;
    Ok(value.and_then(|v| op::ternary!(v.is_empty() => None; Some(T::from(v)))))
}
