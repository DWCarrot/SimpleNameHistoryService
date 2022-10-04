use std::default;
use std::fmt;

use base64::Config;
use base64::decode_config;
use serde::Deserialize;
use serde::de;
use serde::de::Deserializer;
use serde::de::Visitor;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct Profile {

     pub id: Uuid,

     pub name: String,

     pub properties: Vec<Properity>
}

#[derive(Debug, Deserialize)]
pub struct Properity {

    pub name: String,

    #[serde(deserialize_with = "deserialize_base64")]
    pub value: Box<[u8]>,

    #[serde(default)]
    #[serde(deserialize_with = "deserialize_base64_optional")]
    pub signature: Option<Box<[u8]>>,
}


struct B64Data(pub Box<[u8]>);

impl<'de> Deserialize<'de> for B64Data {

    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct B64StringVisitor(Config);

        impl<'de> Visitor<'de> for B64StringVisitor {
            type Value = B64Data;
        
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_fmt(format_args!("a base-64 string in {:?} fromat", self.0))
            }
        
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let buf = decode_config(v, self.0).map_err(|e| de::Error::custom(e))?;
                let data = B64Data(buf.into_boxed_slice());
                Ok(data)
            }
        }


        deserializer.deserialize_str(B64StringVisitor(base64::STANDARD))
    }
}


fn deserialize_base64<'de, D>(deserializer: D) -> Result<Box<[u8]>, D::Error> 
where 
    D: Deserializer<'de>
{
    let d: B64Data = Deserialize::deserialize(deserializer)?;
    Ok(d.0)
}


fn deserialize_base64_optional<'de, D>(deserializer: D) -> Result<Option<Box<[u8]>>, D::Error> 
where 
    D: Deserializer<'de>
{
    let p: Option<B64Data> = Deserialize::deserialize(deserializer)?;
    Ok(p.map(|d| d.0))
}