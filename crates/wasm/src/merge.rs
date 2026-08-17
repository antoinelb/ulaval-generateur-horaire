use ulaval_scheduler_core::Course;

// The catalogue the app actually works with: the scraped snapshot plus the
// student's hand-entered courses. On a code collision the scraped course
// wins and the collision is surfaced — displayed, never just logged (ADR
// `2026-07-contribution-de-cours-manuels`).
#[derive(Debug, Clone, PartialEq)]
pub struct MergedCatalogue {
    // sorted by code, the snapshot's own invariant
    pub courses: Vec<Course>,
    // manual codes shadowed by a scraped course of the same code
    pub collisions: Vec<String>,
}

pub fn merge_manual(
    scraped: Vec<Course>,
    manual: Vec<Course>,
) -> MergedCatalogue {
    let mut courses = scraped;
    let mut collisions = Vec::new();
    for course in manual {
        if courses.iter().any(|known| known.code == course.code) {
            collisions.push(course.code);
        } else {
            courses.push(course);
        }
    }
    courses.sort_by(|a, b| a.code.cmp(&b.code));
    MergedCatalogue {
        courses,
        collisions,
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
    fn a_manual_course_joins_the_catalogue_sorted_by_code() {
        let merged =
            merge_manual(vec![course("GEX-1000")], vec![course("ANL-2020")]);
        let codes: Vec<&str> = merged
            .courses
            .iter()
            .map(|course| course.code.as_str())
            .collect();
        assert_eq!(codes, ["ANL-2020", "GEX-1000"]);
        assert!(merged.collisions.is_empty());
    }

    #[test]
    fn on_a_collision_the_scraped_course_wins_and_the_code_is_surfaced() {
        let mut shadowed = course("GEX-1000");
        shadowed.title = "Version manuelle".to_string();
        let merged = merge_manual(vec![course("GEX-1000")], vec![shadowed]);
        assert_eq!(merged.courses.len(), 1);
        assert_eq!(merged.courses[0].title, "T", "the scraped one survives");
        assert_eq!(merged.collisions, ["GEX-1000"]);
    }
}
