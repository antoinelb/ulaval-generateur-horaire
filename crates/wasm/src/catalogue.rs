use ulaval_scheduler_core::Course;

// Where the courses of one call come from: the call itself, or the snapshot
// the worker loaded once through `init_snapshot`. Passing the 8 800-course
// catalogue in every message costs a structured clone plus a full
// deserialization each time, so a long-lived worker initializes it instead
// and stops sending it (ADR `2026-08-snapshot-en-cache-dans-le-module-wasm`).
//
// Neither one is an error, never an empty catalogue: every question would
// then answer « nothing is placeable », a false verdict dressed as an answer.
// An explicitly empty `courses` is a different thing — a caller's own list,
// honoured as given.
pub fn resolve<'a>(
    inline: Option<&'a [Course]>,
    cached: &'a [Course],
) -> Result<&'a [Course], String> {
    match inline {
        Some(courses) => Ok(courses),
        None if !cached.is_empty() => Ok(cached),
        None => Err("no catalogue : pass `courses` in the call, or load one \
                     once with `init_snapshot`"
            .to_string()),
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn course(code: &str) -> Course {
        serde_json::from_str(&format!(
            r#"{{"code":"{code}","title":"T","credits":3,"cycle":1,
                 "prerequisites":null,"equivalents":[],"seasons":{{}}}}"#
        ))
        .unwrap_or_else(|e| panic!("course literal: {e}"))
    }

    #[test]
    fn the_call_wins_over_the_cache_even_when_it_carries_nothing() {
        let cached = [course("GEX-1000")];
        let inline = [course("GEX-1001")];

        let resolved =
            resolve(Some(&inline), &cached).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(resolved[0].code, "GEX-1001");

        let empty =
            resolve(Some(&[]), &cached).unwrap_or_else(|e| panic!("{e}"));
        assert!(empty.is_empty(), "an explicit empty list is the caller's");
    }

    #[test]
    fn the_cache_answers_a_call_that_carries_no_catalogue() {
        let cached = [course("GEX-1000")];
        let resolved =
            resolve(None, &cached).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(resolved[0].code, "GEX-1000");
    }

    #[test]
    fn neither_one_is_an_error_never_an_empty_catalogue() {
        let error = resolve(None, &[])
            .expect_err("answering on nothing would be a false verdict");
        assert!(error.contains("init_snapshot"), "{error}");
    }
}
