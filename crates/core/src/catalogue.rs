#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Catalogue {
    pub courses: Vec<CatalogueEntry>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CatalogueEntry {
    pub code: String,
    pub title: String,
    pub url: String,
}
