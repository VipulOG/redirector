use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Bang {
    /// The category of the bang command (e.g., "Tech", "Entertainment").
    #[serde(alias = "category", rename = "c")]
    pub category: Option<Category>,
    /// The domain associated with the bang command (e.g., "www.example.com").
    #[serde(alias = "domain", rename = "d")]
    pub domain: Option<String>,
    /// The relevance score of the bang command.
    #[serde(alias = "relevance", rename = "r")]
    pub relevance: Option<u64>,
    /// The short name or abbreviation of the bang command.
    #[serde(alias = "short_name", rename = "s")]
    pub short_name: Option<String>,
    /// The subcategory of the bang command, if applicable.
    #[serde(alias = "subcategory", rename = "sc")]
    pub subcategory: Option<String>,
    /// The trigger text for the bang command (e.g., "g" for Google).
    #[serde(alias = "trigger", rename = "t")]
    pub trigger: String,
    /// The URL template where the search term is inserted.
    #[serde(alias = "url_template", rename = "u")]
    pub url_template: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub enum Category {
    Entertainment,
    Multimedia,
    News,
    #[serde(rename = "Online Services")]
    OnlineServices,
    Research,
    Shopping,
    Tech,
    Translation,
}
