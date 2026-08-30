// The « Exporter » menu: one control in the status strip, one row per
// document (ADR `2026-08-menu-d-export-unique`, restreint par
// `2026-08-retrait-de-l-aller-retour-json-du-cheminement`). Le menu ne
// nomme plus de format : chaque entrée est un document, et l'impression
// décide seule de ce qu'elle produit. La table reste pure pour que la vue
// se contente de l'imprimer (AP-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportChoice {
    Organigramme,
    Horaire,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportEntry {
    pub choice: ExportChoice,
    // stable identity for the keyed loop (AP-8)
    pub key: &'static str,
    pub label: &'static str,
}

pub fn entries() -> Vec<ExportEntry> {
    vec![
        ExportEntry {
            choice: ExportChoice::Organigramme,
            key: "organigramme",
            label: "Organigramme",
        },
        ExportEntry {
            choice: ExportChoice::Horaire,
            key: "horaire",
            label: "Horaire",
        },
    ]
}

// Un export lancé pendant un recalcul fige un état transitoire : le
// document part avec ce que l'écran montre, pas avec le placement final
// (rapport persona 2026-08-29, ADR
// `2026-08-le-debut-n-herite-pas-d-un-placement-hors-saison`). Le menu le
// dit à l'endroit et au moment du geste — jamais un blocage ni une boîte
// « êtes-vous sûr ? » (AIR §E) : attendre une seconde suffit, et un
// export volontairement provisoire reste légitime. `None` hors recalcul,
// pour que le menu ne réserve rien qu'il n'ait à dire.
pub fn pending_note(searching: bool) -> Option<&'static str> {
    if searching {
        Some(
            "⟳ recalcul en cours — un document exporté maintenant fige un \
             placement provisoire.",
        )
    } else {
        None
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn the_menu_offers_one_row_per_document_and_names_no_format() {
        let entries = entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].choice, ExportChoice::Organigramme);
        assert_eq!(entries[0].label, "Organigramme");
        assert_eq!(entries[1].choice, ExportChoice::Horaire);
        assert_eq!(entries[1].label, "Horaire");
        // le menu ne promet aucun format : l'entrée nomme le document
        for entry in &entries {
            assert!(!entry.label.contains("PDF"), "{}", entry.label);
            assert!(!entry.label.contains("JSON"), "{}", entry.label);
        }
        let keys: std::collections::BTreeSet<&str> =
            entries.iter().map(|entry| entry.key).collect();
        assert_eq!(keys.len(), entries.len(), "every row needs its own key");
    }

    #[test]
    fn a_running_search_is_announced_where_the_export_is_chosen() {
        let note = pending_note(true).unwrap_or_default();
        assert!(note.contains("recalcul en cours"), "{note}");
        assert!(note.contains("provisoire"), "{note}");
        assert_eq!(
            pending_note(false),
            None,
            "hors recalcul le menu n'a rien à avertir"
        );
    }
}
