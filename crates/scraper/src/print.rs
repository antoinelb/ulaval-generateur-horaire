use std::io::{IsTerminal, Write};
use std::sync::LazyLock;
use std::sync::Mutex;

const BLUE: &str = "\x1b[34m";
const BOLD_GREEN: &str = "\x1b[1;32m";
const BOLD_YELLOW: &str = "\x1b[1;33m";
const BOLD_RED: &str = "\x1b[1;31m";
const NORMAL: &str = "\x1b[0m";
const ERASE_LINE: &str = "\r\x1b[2K";

/// All mutable display state, behind one lock (`STATE`): the bottom line of
/// the screen is a pure function of this state (`render_bottom_line`) and is
/// redrawn after every mutation, so screen == render(state) whenever the
/// lock is released.
/// Blocks (`task`, `progress_task`, `done`) belong to the orchestrating
/// thread; worker threads may only `increment`.
struct PrintState {
    indent: usize,
    pending: Option<Pending>,
    progress: Option<Progress>,
    next_id: usize,
    // sampled once at first print: output redirected mid-run keeps the
    // original mode
    is_tty: bool,
}

struct Pending {
    id: usize,
    msg: String,
    indent: usize,
}

struct Progress {
    id: usize,
    msg: String,
    done: usize,
    total: usize,
    indent: usize,
}

pub struct Task {
    id: usize,
    msg: String,
    done_msg: String,
    finished: bool,
}

static STATE: LazyLock<Mutex<PrintState>> = LazyLock::new(|| {
    Mutex::new(PrintState {
        indent: 0,
        pending: None,
        progress: None,
        next_id: 0,
        is_tty: std::io::stdout().is_terminal(),
    })
});

impl Task {
    pub fn done(mut self) {
        self.close("+", BOLD_GREEN, true);
        self.finished = true;
    }

    pub fn increment(&self) {
        let mut state = lock_state();
        if let Some(progress) =
            state.progress.as_mut().filter(|p| p.id == self.id)
        {
            progress.done += 1;
        }
        write(&state, None)
    }

    fn close(&mut self, symbol: &str, colour: &str, success: bool) {
        let mut state = lock_state();
        state.indent = state.indent.saturating_sub(1);
        let msg = if success { &self.done_msg } else { &self.msg };
        let symbol = paint_symbol(symbol, colour);

        if state.progress.as_ref().is_some_and(|p| p.id == self.id) {
            state.progress = None;
        }

        if state.pending.as_ref().is_some_and(|p| p.id == self.id) {
            state.pending = None;
            write(&state, Some(&format_line(state.indent, msg, &symbol)))
        } else {
            write(&state, Some(&format_line(state.indent + 1, msg, &symbol)))
        }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        if !self.finished {
            self.close("x", BOLD_RED, false);
        }
    }
}

pub fn done_print(msg: &str) {
    print_permanent(msg, "+", BOLD_GREEN);
}

pub fn warn_print(msg: &str) {
    print_permanent(msg, "!", BOLD_YELLOW);
}

pub fn task(msg: &str, done_msg: &str) -> Task {
    let mut state = lock_state();
    materialize_pending(&mut state);
    let id = state.next_id;
    state.next_id += 1;
    state.pending = Some(Pending {
        id,
        msg: msg.to_string(),
        indent: state.indent,
    });
    state.indent += 1;
    write(&state, None);
    Task {
        id,
        msg: msg.to_string(),
        done_msg: done_msg.to_string(),
        finished: false,
    }
}

pub fn progress_task(msg: &str, done_msg: &str, total: usize) -> Task {
    let mut state = lock_state();
    materialize_pending(&mut state);

    // one counted task at a time; nested counters would need a progress
    // stack (Vec, render the last, increment by id)
    let collision = if state.progress.is_some() {
        write(
            &state,
            Some(&format_line(
                state.indent,
                "A progress task already exists.",
                &paint_symbol("!", BOLD_YELLOW),
            )),
        );
        true
    } else {
        false
    };

    let id = state.next_id;
    state.next_id += 1;
    state.pending = Some(Pending {
        id,
        msg: msg.to_string(),
        indent: state.indent,
    });

    if !collision {
        state.progress = Some(Progress {
            id,
            msg: msg.to_string(),
            indent: state.indent + 1,
            done: 0,
            total,
        })
    }

    state.indent += 1;

    write(&state, None);
    Task {
        id,
        msg: msg.to_string(),
        done_msg: done_msg.to_string(),
        finished: false,
    }
}

fn print_permanent(msg: &str, symbol: &str, colour: &str) {
    let mut state = lock_state();
    materialize_pending(&mut state);
    write(
        &state,
        Some(&format_line(
            state.indent,
            msg,
            &paint_symbol(symbol, colour),
        )),
    );
}

fn lock_state() -> std::sync::MutexGuard<'static, PrintState> {
    STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn materialize_pending(state: &mut PrintState) {
    if let Some(pending) = state.pending.take() {
        write(
            state,
            Some(&format_line(
                pending.indent,
                &pending.msg,
                &paint_symbol("→", BLUE),
            )),
        );
    }
}

// exactly one transient line, always the bottom one; rewriting anything
// above it (e.g. one live line per worker, or turning a materialized [→]
// back into [+]) would need pacman-style relative cursor tracking
fn render_bottom_line(state: &PrintState) -> Option<String> {
    match (&state.pending, &state.progress) {
        (Some(pending), Some(progress)) if pending.id == progress.id => {
            Some(format_line(
                pending.indent,
                &pending.msg,
                &paint_symbol(
                    &format_progress_symbol(progress.done, progress.total),
                    BLUE,
                ),
            ))
        }
        (Some(pending), _) => Some(format_line(
            pending.indent,
            &pending.msg,
            &paint_symbol("✱", BLUE),
        )),
        (None, Some(progress)) => Some(format_line(
            progress.indent,
            &progress.msg,
            &paint_symbol(
                &format_progress_symbol(progress.done, progress.total),
                BLUE,
            ),
        )),
        (None, None) => None,
    }
}

/// The only function that touches stdout.
fn write(state: &PrintState, permanent: Option<&str>) {
    if state.is_tty {
        print!("{}", ERASE_LINE);
    }

    if let Some(line) = permanent {
        println!("{}", line);
    }

    if state.is_tty {
        if let Some(line) = render_bottom_line(state) {
            print!("{}", line)
        }
    }

    // display is non-critical: a broken pipe must never kill the scrape
    std::io::stdout().flush().ok();
}

fn format_line(indent: usize, msg: &str, symbol: &str) -> String {
    let spaces = "  ".repeat(indent);
    format!("{spaces}{symbol} {msg}")
}

fn format_progress_symbol(done: usize, total: usize) -> String {
    let width = total.to_string().len();
    format!("{done:>width$}/{total}")
}

fn paint_symbol(text: &str, colour: &str) -> String {
    format!("{colour}[{text}]{NORMAL}")
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn format_line_indents_two_spaces_per_level() {
        assert_eq!(format_line(0, "msg", "[s]"), "[s] msg");
        assert_eq!(format_line(2, "msg", "[s]"), "    [s] msg");
    }

    #[test]
    fn format_progress_symbol_right_aligns_done_to_total_width() {
        assert_eq!(format_progress_symbol(7, 9743), "   7/9743");
        assert_eq!(format_progress_symbol(9743, 9743), "9743/9743");
    }

    #[test]
    fn paint_symbol_wraps_in_brackets_and_resets_colour() {
        assert_eq!(
            paint_symbol("+", BOLD_GREEN),
            format!("{BOLD_GREEN}[+]{NORMAL}")
        );
    }

    #[test]
    fn render_nothing_when_state_is_empty() {
        let state = test_state(None, None);
        assert_eq!(render_bottom_line(&state), None);
    }

    #[test]
    fn render_star_header_for_pending_alone() {
        let state = test_state(Some(test_pending(0, 1)), None);
        assert_eq!(
            render_bottom_line(&state),
            Some(format_line(1, "pending 0", &paint_symbol("✱", BLUE)))
        );
    }

    #[test]
    fn render_counter_header_when_progress_matches_pending() {
        let state =
            test_state(Some(test_pending(0, 1)), Some(test_progress(0, 3, 2)));
        assert_eq!(
            render_bottom_line(&state),
            Some(format_line(
                1,
                "pending 0",
                &paint_symbol(&format_progress_symbol(3, 5), BLUE)
            ))
        );
    }

    #[test]
    fn render_star_header_when_progress_belongs_to_another_task() {
        let state =
            test_state(Some(test_pending(0, 1)), Some(test_progress(1, 3, 2)));
        assert_eq!(
            render_bottom_line(&state),
            Some(format_line(1, "pending 0", &paint_symbol("✱", BLUE)))
        );
    }

    #[test]
    fn render_status_line_for_progress_alone() {
        let state = test_state(None, Some(test_progress(1, 3, 2)));
        assert_eq!(
            render_bottom_line(&state),
            Some(format_line(
                2,
                "progress 1",
                &paint_symbol(&format_progress_symbol(3, 5), BLUE)
            ))
        );
    }

    #[test]
    fn materialize_pending_empties_the_slot() {
        let mut state = test_state(Some(test_pending(0, 1)), None);
        materialize_pending(&mut state);
        assert!(state.pending.is_none());
    }

    #[test]
    fn materialize_pending_without_pending_is_a_noop() {
        let mut state = test_state(None, None);
        materialize_pending(&mut state);
        assert!(state.pending.is_none());
    }

    // The public API mutates the global STATE and cargo runs tests
    // concurrently, so every use of the global lives in this single test;
    // all other tests build local states.
    #[test]
    fn public_api_lifecycle_returns_state_to_neutral() {
        let outer = task("outer", "outer done");
        {
            let state = lock_state();
            assert_eq!(state.indent, 1);
            assert!(state.pending.as_ref().is_some_and(|p| p.msg == "outer"));
        }

        // a nested task materializes the outer header
        let inner = task("inner", "inner done");
        {
            let state = lock_state();
            assert_eq!(state.indent, 2);
            assert!(state.pending.as_ref().is_some_and(|p| p.msg == "inner"));
        }
        inner.done();
        {
            let state = lock_state();
            assert_eq!(state.indent, 1);
            assert!(state.pending.is_none());
        }

        // a counted task increments its own counter only
        let counted = progress_task("counted", "counted done", 3);
        counted.increment();
        counted.increment();
        {
            let state = lock_state();
            assert!(state.progress.as_ref().is_some_and(|p| p.done == 2));
        }

        // a permanent print materializes the counted header and demotes the
        // counter to a status line
        warn_print("anomaly kept raw");
        {
            let state = lock_state();
            assert!(state.pending.is_none());
            assert!(state.progress.as_ref().is_some_and(|p| p.done == 2));
        }

        // a colliding counted task keeps its structure, not its counter
        let collided = progress_task("collided", "collided done", 9);
        collided.increment();
        {
            let state = lock_state();
            assert!(state
                .progress
                .as_ref()
                .is_some_and(|p| p.msg == "counted" && p.done == 2));
        }
        collided.done();
        counted.done();
        {
            let state = lock_state();
            assert!(state.progress.is_none());
        }

        done_print("permanent success line");

        // dropping without done() (the failure path) still unwinds
        drop(outer);
        let state = lock_state();
        assert_eq!(state.indent, 0);
        assert!(state.pending.is_none());
        assert!(state.progress.is_none());
    }

    fn test_state(
        pending: Option<Pending>,
        progress: Option<Progress>,
    ) -> PrintState {
        PrintState {
            indent: 0,
            pending,
            progress,
            next_id: 0,
            is_tty: false,
        }
    }

    fn test_pending(id: usize, indent: usize) -> Pending {
        Pending {
            id,
            msg: format!("pending {id}"),
            indent,
        }
    }

    fn test_progress(id: usize, done: usize, indent: usize) -> Progress {
        Progress {
            id,
            msg: format!("progress {id}"),
            done,
            total: 5,
            indent,
        }
    }
}
