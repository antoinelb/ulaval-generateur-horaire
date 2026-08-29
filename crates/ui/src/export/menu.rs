// The « Exporter » menu: one control in the status strip, three
// destinations grouped by document (ADR `2026-08-menu-d-export-unique`).
// The table is pure so the view only prints it (AP-5), and so the wording
// — the only place the app promises a format — is tested rather than typed
// into rsx.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportChoice {
    OrganigrammePdf,
    OrganigrammeJson,
    HorairePdf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportEntry {
    pub choice: ExportChoice,
    // stable identity for the keyed loop (AP-8): two rows carry the label
    // « PDF », so the label cannot key the list
    pub key: &'static str,
    pub label: &'static str,
    // the group heading printed above this row, `None` when the row
    // continues the group above it
    pub group: Option<&'static str>,
}

pub fn entries() -> Vec<ExportEntry> {
    vec![
        ExportEntry {
            choice: ExportChoice::OrganigrammePdf,
            key: "organigramme-pdf",
            label: "PDF",
            group: Some("Organigramme"),
        },
        ExportEntry {
            choice: ExportChoice::OrganigrammeJson,
            key: "organigramme-json",
            label: "JSON",
            group: None,
        },
        ExportEntry {
            choice: ExportChoice::HorairePdf,
            key: "horaire-pdf",
            label: "PDF",
            group: Some("Horaire"),
        },
    ]
}

// What the student is told once the file has left the app — or has not.
// A download the browser refused must never be reported as a success
// (TRU-1), and « enregistré » would be a lie either way: the file is
// handed to the browser, which decides where it lands.
pub fn download_note(file_name: &str, taken: bool) -> String {
    if taken {
        format!("{file_name} téléchargé.")
    } else {
        format!(
            "{file_name} n'a pas pu être téléchargé — le navigateur a \
             refusé l'enregistrement. Réessayez, ou vérifiez ses réglages \
             de téléchargement."
        )
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn the_menu_groups_two_organigramme_formats_and_one_horaire() {
        let entries = entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].group, Some("Organigramme"));
        assert_eq!(entries[1].group, None, "the JSON row continues the group");
        assert_eq!(entries[2].group, Some("Horaire"));
        assert_eq!(entries[1].choice, ExportChoice::OrganigrammeJson);
        for entry in &entries {
            assert!(!entry.label.is_empty());
        }
        // the two PDF rows share their label: only the key tells the loop
        // they are two different rows
        let keys: std::collections::BTreeSet<&str> =
            entries.iter().map(|entry| entry.key).collect();
        assert_eq!(keys.len(), entries.len(), "every row needs its own key");
    }

    #[test]
    fn a_refused_download_is_never_reported_as_a_success() {
        let taken = download_note("B-GEX-A26.json", true);
        assert!(taken.contains("B-GEX-A26.json"));
        assert!(taken.contains("téléchargé"));
        let refused = download_note("B-GEX-A26.json", false);
        assert!(refused.contains("n'a pas pu"));
        assert!(refused.contains("Réessayez"));
    }
}
