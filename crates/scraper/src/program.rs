use futures::stream::{self, StreamExt};

use crate::fetch::{FetchError, Fetcher};
use crate::parser::{self, ParseError};
use crate::print;
use ulaval_scheduler_core::Program;

const n_concurrent: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum ProgramError {
    // `FetchError` already names the URL it failed on
    #[error(transparent)]
    Fetch(#[from] FetchError),
    // `ParseError` only names a selector, so the page has to be added for
    // the log line to be actionable
    #[error("Parsing {url}: {source}")]
    Parse {
        url: String,
        #[source]
        source: ParseError,
    },
}

pub async fn scrape(
    fetcher: &Fetcher,
    urls: &[String],
) -> (Vec<Program>, Vec<ProgramError>) {
    let task = print::progress_task(
        "Scraping programs...",
        "Scraped programs.",
        urls.len(),
    );
    let progress = &task;

    // `collect`, not `try_collect`: the URLs are named by hand, and one of
    // them being unreachable must not throw away the programs that did
    // parse (ADR `2026-07-echec-de-page-programme-non-bloquant`)
    let scraped: Vec<(Option<Program>, Vec<ProgramError>)> =
        stream::iter(urls)
            .map(|url| async move {
                let scraped = scrape_program(fetcher, url).await;
                progress.increment();
                scraped
            })
            .buffer_unordered(n_concurrent)
            .collect()
            .await;
    task.done();

    let mut programs = Vec::with_capacity(scraped.len());
    let mut anomalies = Vec::new();
    for (program, mut errors) in scraped {
        programs.extend(program);
        anomalies.append(&mut errors);
    }
    (programs, anomalies)
}

async fn scrape_program(
    fetcher: &Fetcher,
    url: &str,
) -> (Option<Program>, Vec<ProgramError>) {
    let html = match fetcher.fetch(url).await {
        Ok(html) => html,
        Err(source) => return (None, vec![source.into()]),
    };
    // a page whose skeleton is missing yields no program at all: there is
    // nothing to write a file from
    let page = match parser::program::parse(&html) {
        Ok(page) => page,
        Err(source) => {
            let error = ProgramError::Parse {
                url: url.to_string(),
                source,
            };
            return (None, vec![error]);
        }
    };

    // the page is readable, so the program is kept — the holes it carries
    // travel alongside it rather than discarding it
    let anomalies = page
        .anomalies
        .into_iter()
        .map(|source| ProgramError::Parse {
            url: url.to_string(),
            source,
        })
        .collect();

    (Some(page.program), anomalies)
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
pub(crate) mod tests {
    // TEST_STATE_LOCK serializes whole tests around the global print state,
    // so holding it across await points is the intent, not an oversight
    #![allow(clippy::await_holding_lock)]

    use std::time::Duration;

    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    async fn a_scraped_program_is_returned_without_anomalies() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount(&server, "genie-civil", program_html("genie-civil")).await;

        let (programs, anomalies) =
            scrape_urls(&[url(&server, "genie-civil")]).await;

        assert!(anomalies.is_empty(), "{anomalies:?}");
        assert_eq!(programs[0].code, "genie-civil");
        assert_eq!(programs[0].mandatory, ["GEX-1000"]);
    }

    #[tokio::test]
    async fn an_unreachable_page_is_an_anomaly_and_the_run_continues() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount(&server, "genie-civil", program_html("genie-civil")).await;
        // nothing mounted for the second URL, so it 404s

        let (programs, anomalies) = scrape_urls(&[
            url(&server, "genie-civil"),
            url(&server, "genie-absent"),
        ])
        .await;

        assert_eq!(programs.len(), 1, "the reachable program still lands");
        assert!(
            matches!(&anomalies[0], ProgramError::Fetch(error)
                if error.to_string().contains("genie-absent")),
            "got {anomalies:?}"
        );
    }

    #[tokio::test]
    async fn an_unrecognized_page_is_an_anomaly_and_yields_no_program() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount(&server, "genie-civil", "<html></html>".to_string()).await;

        let (programs, anomalies) =
            scrape_urls(&[url(&server, "genie-civil")]).await;

        assert!(programs.is_empty(), "no program can be built from the page");
        assert!(
            matches!(&anomalies[0], ProgramError::Parse { url, .. }
                if url.contains("genie-civil")),
            "got {anomalies:?}"
        );
    }

    #[tokio::test]
    async fn a_program_parsed_with_anomalies_is_kept_and_its_holes_surfaced() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        // a rule stated in prose no grammar covers: the page is readable,
        // the rule is not — the same shape génie mécanique really has
        let html = program_page(
            "genie-civil",
            "Règle 1 – 3 crédits parmi :",
            r#"<p class="fe-bloc-regle--ligne">Réussir le cours ANL-2020.</p>"#,
        );
        mount(&server, "genie-civil", html).await;

        let (programs, anomalies) =
            scrape_urls(&[url(&server, "genie-civil")]).await;

        assert_eq!(programs[0].code, "genie-civil", "the program is kept");
        assert!(
            matches!(&anomalies[0], ProgramError::Parse { url, .. }
                if url.contains("genie-civil")),
            "the anomaly names the page it came from, got {anomalies:?}"
        );
    }

    async fn scrape_urls(
        urls: &[String],
    ) -> (Vec<Program>, Vec<ProgramError>) {
        // zero intervals: throttle timing is unit-tested on a virtual clock
        // in fetch.rs; these tests assert orchestration and must stay fast
        let fetcher = Fetcher::new(Duration::ZERO, Duration::ZERO)
            .unwrap_or_else(|e| panic!("build fetcher: {e}"));
        scrape(&fetcher, urls).await
    }

    fn url(server: &MockServer, slug: &str) -> String {
        format!("{}/{slug}", server.uri())
    }

    async fn mount(server: &MockServer, slug: &str, html: String) {
        Mock::given(method("GET"))
            .and(path(format!("/{slug}")))
            .respond_with(ResponseTemplate::new(200).set_body_string(html))
            .mount(server)
            .await;
    }

    // the smallest page the program parser accepts: title, canonical link,
    // total credits, and one block holding one accordion
    pub(crate) fn program_html(slug: &str) -> String {
        program_page(
            slug,
            "Cours obligatoires",
            concat!(
                r#"<ul class="fe--liste-cours"><li>"#,
                r#"<span class="cours-carte--sigle">GEX-1000</span>"#,
                "</li></ul>",
            ),
        )
    }

    fn program_page(slug: &str, heading: &str, body: &str) -> String {
        format!(
            concat!(
                "<html><body>",
                "<h1>Baccalauréat en {slug}</h1>",
                r#"<link rel="canonical" "#,
                r#"href="https://www.ulaval.ca/etudes/programmes/{slug}">"#,
                r#"<div class="bloc-promo">"#,
                r#"<span class="promo-entete--titre">120</span>"#,
                r#"<span class="promo-entete--contenu">Crédits</span>"#,
                "</div>",
                r#"<section id="section-structure">"#,
                r#"<div class="fe-bloc-section">"#,
                r#"<div class="collapsible-sections">"#,
                r#"<div class="controls-title fe-bloc-titre">"#,
                r#"<h4 class="fe-bloc-titre--texte">Programme</h4>"#,
                r#"<span class="fe-bloc-titre--credits">"#,
                "120 crédits exigés</span></div>",
                r#"<div class="toggle-section">"#,
                r#"<p class="toggle-section--header">"#,
                r#"<span class="item">{heading}</span></p>"#,
                r#"<div class="toggle-section--content">"#,
                r#"<div class="fe-bloc-regle--paragraphe">{body}</div>"#,
                "</div></div></div></div></section>",
                "</body></html>",
            ),
            slug = slug,
            heading = heading,
            body = body
        )
    }

    fn lock_print() -> std::sync::MutexGuard<'static, ()> {
        print::TEST_STATE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
