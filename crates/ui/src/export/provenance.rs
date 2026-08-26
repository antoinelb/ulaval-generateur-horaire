// EXP-1: the provenance both export documents carry, modelled on the
// on-screen footer (`components/shell.rs`'s `Footer`/`Commit`) so the
// screen and the printed documents never drift apart — same wording, same
// three "unknown stays unknown" refusals (TRU-1).

pub const REPO: &str =
    "https://github.com/antoinelb/ulaval-generateur-horaire";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportProvenance {
    pub generated: String,
    pub scraped: String,
    pub build: String,
    pub data: String,
    pub version: String,
    pub repo: String,
    pub build_url: Option<String>,
    pub data_url: Option<String>,
    pub line: String,
}

// `generated_at` is read at the view boundary (the impure
// `browser::now_iso()`, exactly like `now_iso()` feeds
// `import::build_local_program` in `components/panel.rs`) and handed in —
// this module never calls a clock itself, which is what lets it be tested
// natively with fixed instants.
pub fn export_provenance(
    generated_at: &str,
    scraped_at: Option<&str>,
) -> ExportProvenance {
    let build = option_env!("BUILD_HASH").unwrap_or("dev");
    let data = option_env!("DATA_HASH").unwrap_or("dev");
    let version = env!("CARGO_PKG_VERSION");

    let generated = format_generated(generated_at);
    let scraped = scraped_at
        .map(str::to_string)
        .unwrap_or_else(|| "date de récolte inconnue".to_string());
    let build_url = commit_url(build);
    let data_url = commit_url(data);

    let line = format!(
        "Document généré par le générateur d'horaire (v{version}, code \
         {build}, données {data} — {scraped}), {generated}. Code et \
         données : {REPO}"
    );

    ExportProvenance {
        generated,
        scraped,
        build: build.to_string(),
        data: data.to_string(),
        version: version.to_string(),
        repo: REPO.to_string(),
        build_url,
        data_url,
        line,
    }
}

// A local build has no commit to point at (TRU-1) — same rule as the
// on-screen `Commit` component.
fn commit_url(sha: &str) -> Option<String> {
    if sha == "dev" {
        None
    } else {
        Some(format!("{REPO}/commit/{sha}"))
    }
}

// `browser::now_local()` emits `YYYY-MM-DDTHH:MM:SS` (the reader's own
// zone, printed with no zone name — heure de l'Est at ULaval) and
// `now_iso()` the same with a trailing `Z`, spelled out as UTC. Any other
// shape (a malformed or future wire format) degrades to the raw string
// rather than panicking or being silently dropped.
fn format_generated(generated_at: &str) -> String {
    if let Some((date, rest)) = generated_at.split_once('T') {
        let (time, zone) = match rest.strip_suffix('Z') {
            Some(time) => (time, " UTC"),
            None => (rest, ""),
        };
        let mut parts = time.split(':');
        if let (Some(hour), Some(minute), Some(_second), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        {
            if !date.is_empty() && !hour.is_empty() && !minute.is_empty() {
                return format!("généré le {date} à {hour}:{minute}{zone}");
            }
        }
    }
    format!("généré le {generated_at}")
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn a_well_formed_instant_renders_date_time_and_utc() {
        let provenance =
            export_provenance("2026-08-25T14:03:07Z", Some("2026-08-01"));
        assert_eq!(provenance.generated, "généré le 2026-08-25 à 14:03 UTC");
    }

    #[test]
    fn a_malformed_instant_degrades_without_panicking() {
        let provenance = export_provenance("not-a-date", Some("2026-08-01"));
        assert_eq!(provenance.generated, "généré le not-a-date");
    }

    #[test]
    fn a_local_instant_without_z_prints_no_zone() {
        // `browser::now_local()`'s shape: the reader's wall time, printed
        // without claiming any zone
        let provenance =
            export_provenance("2026-08-25T14:03:07", Some("2026-08-01"));
        assert_eq!(provenance.generated, "généré le 2026-08-25 à 14:03");
    }

    #[test]
    fn an_instant_with_too_few_time_parts_degrades() {
        let provenance = export_provenance("2026-08-25T14:03Z", Some("d"));
        assert_eq!(provenance.generated, "généré le 2026-08-25T14:03Z");
    }

    #[test]
    fn an_instant_with_an_empty_date_degrades() {
        // The 3-part time still matches the pattern, but the date half of
        // the split is empty — the `!date.is_empty()` guard must catch it
        // too, not just the hour/minute halves.
        let provenance = export_provenance("T14:03:07Z", Some("d"));
        assert_eq!(provenance.generated, "généré le T14:03:07Z");
    }

    #[test]
    fn no_scraped_at_yields_the_french_fallback() {
        let provenance = export_provenance("2026-08-25T14:03:07Z", None);
        assert_eq!(provenance.scraped, "date de récolte inconnue");
    }

    #[test]
    fn dev_hashes_yield_no_commit_urls() {
        // In this native test build BUILD_HASH/DATA_HASH are never set,
        // so both fall back to "dev" and must not link anywhere (TRU-1).
        let provenance = export_provenance("2026-08-25T14:03:07Z", Some("d"));
        assert_eq!(provenance.build, "dev");
        assert_eq!(provenance.data, "dev");
        assert_eq!(provenance.build_url, None);
        assert_eq!(provenance.data_url, None);
    }

    #[test]
    fn a_real_commit_sha_yields_a_github_url() {
        assert_eq!(
            commit_url("abc123"),
            Some(format!("{REPO}/commit/abc123"))
        );
    }

    #[test]
    fn the_line_carries_every_provenance_fact() {
        let provenance =
            export_provenance("2026-08-25T14:03:07Z", Some("2026-08-01"));
        assert!(provenance.line.contains(&provenance.version));
        assert!(provenance.line.contains(&provenance.build));
        assert!(provenance.line.contains(&provenance.data));
        assert!(provenance.line.contains(REPO));
        assert!(provenance.line.contains(&provenance.generated));
        assert!(provenance.line.contains(&provenance.scraped));
    }
}
