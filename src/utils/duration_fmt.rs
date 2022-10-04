use std::time::Duration;

use serde::Serializer;
use serde::Deserializer;
use serde::de::Error as DeError;
use serde::de::Unexpected;
use serde::de::Visitor;


pub fn serialize<S>(value: &Duration, serializer: S) -> Result<S::Ok, S::Error> 
where 
    S: Serializer
{
    if value.subsec_millis() == 0 {
        let s = value.as_secs();
        let m = s / 60;
        let r = s % 60;
        if r == 0 {
            let h = m / 60;
            let r = m % 60;
            if r == 0 {
                let s = format!("{}h", h);
                return serializer.serialize_str(s.as_str());
            }
            let s = format!("{}m", m);
            return serializer.serialize_str(s.as_str());
        }
        let s = format!("{}s", s);
        return serializer.serialize_str(s.as_str());
    }
    let v = value.as_millis();
    return serializer.serialize_u128(v);
}


struct DurationVisitor;

impl<'de> Visitor<'de> for DurationVisitor {
    type Value = Duration;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("integer (ms) or string \"<integer>[h|m|s|ms]\"")
    }

    fn visit_u64<E>(self,v:u64) -> Result<Self::Value, E>
    where 
        E: DeError, 
    {
        Ok(Duration::from_millis(v))
    }

    fn visit_i64<E>(self,v: i64) -> Result<Self::Value, E>
    where 
        E: DeError, 
    {
        let v: u64 = unsafe { std::mem::transmute(v) };
        Ok(Duration::from_millis(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: DeError, 
    {
        let mut value = v;
        let mut unit = "ms";
        if let Some(n) = v.find(|c: char| !c.is_ascii_digit()) {
            (value, unit) = v.split_at(n)
        }
        let out = match unit {
            "ms" => {
                let n: u64 = value.parse().map_err(DeError::custom)?;
                Duration::from_millis(n)
            }
            "s" => {
                let n: u64 = value.parse().map_err(DeError::custom)?;
                Duration::from_secs(n)
            }
            "m" => {
                let n: u64 = value.parse().map_err(DeError::custom)?;
                Duration::from_secs(n * 60)
            },
            "h" => {
                let n: u64 = value.parse().map_err(DeError::custom)?;
                Duration::from_secs(n * 3600)
            }
            _ => {
                return Err(DeError::invalid_value(Unexpected::Str(v), &self));
            }
        };
        Ok(out)
    }
}

pub fn deserialize<'de, D>(deserialzier: D) -> Result<Duration, D::Error>
where 
    D: Deserializer<'de>
{
    deserialzier.deserialize_any(DurationVisitor)
}