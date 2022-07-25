use crate::client_bound::login::LoginProperty;

pub mod login;
pub mod play;

#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
pub struct Property {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl From<&LoginProperty> for Property {
    fn from(property: &LoginProperty) -> Self {
        Self {
            name: property.name.to_string(),
            value: property.value.to_string(),
            signature: property.signature.1.as_ref().map(ToString::to_string),
        }
    }
}

#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
pub struct GameProfile {
    pub id: uuid::Uuid,
    pub name: String,
    pub properties: Vec<Property>,
}