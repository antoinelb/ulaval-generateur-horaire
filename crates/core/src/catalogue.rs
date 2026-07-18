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

impl Catalogue {
    pub fn from_entries(mut entries: Vec<CatalogueEntry>) -> Self {
        entries.sort_by(|a, b| a.code.cmp(&b.code));
        entries.dedup_by(|a, b| a.code == b.code);
        Self { courses: entries }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn entries_are_sorted_by_code() {
        let catalogue = Catalogue::from_entries(vec![
            entry("GEX-2000"),
            entry("GCI-1000"),
            entry("GEX-1000"),
        ]);

        let codes: Vec<&str> = catalogue
            .courses
            .iter()
            .map(|course| course.code.as_str())
            .collect();
        assert_eq!(codes, ["GCI-1000", "GEX-1000", "GEX-2000"]);
    }

    #[test]
    fn duplicate_codes_keep_the_first_occurrence() {
        // the same course can surface under several matière facets; the
        // stable sort preserves input order within a code, so the copy
        // listed first wins
        let mut duplicate = entry("GEX-1000");
        duplicate.title = "Autre titre".to_string();

        let catalogue = Catalogue::from_entries(vec![
            entry("GEX-2000"),
            entry("GEX-1000"),
            duplicate,
        ]);

        let titles: Vec<&str> = catalogue
            .courses
            .iter()
            .map(|course| course.title.as_str())
            .collect();
        assert_eq!(titles, ["Cours GEX-1000", "Cours GEX-2000"]);
    }

    #[test]
    fn no_entries_is_an_empty_catalogue() {
        let catalogue = Catalogue::from_entries(Vec::new());

        assert!(catalogue.courses.is_empty());
    }

    fn entry(code: &str) -> CatalogueEntry {
        CatalogueEntry {
            code: code.to_string(),
            title: format!("Cours {code}"),
            url: format!("https://ulaval.ca/etudes/cours/{code}"),
        }
    }
}
