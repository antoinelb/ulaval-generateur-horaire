use std::io::{self, IsTerminal, Write};
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

pub const BLUE: &str = "\x1b[34m";
pub const BOLD_GREEN: &str = "\x1b[1;32m";
pub const BOLD_YELLOW: &str = "\x1b[1;33m";
pub const BOLD_RED: &str = "\x1b[1;31m";
pub const NORMAL: &str = "\x1b[0m";
pub const ERASE_LINE: &str = "\r\x1b[2K";

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
    started: Instant,
    finished: bool,
}

static STATE: LazyLock<Mutex<PrintState>> = LazyLock::new(|| {
    Mutex::new(PrintState {
        indent: 0,
        pending: None,
        progress: None,
        next_id: 0,
        is_tty: io::stdout().is_terminal(),
    })
});

impl Task {
    pub fn done(mut self) {
        self.close("+", BOLD_GREEN, true);
        self.finished = true;
    }

    // for a closing message the task could not know when it opened — a
    // tally of the work it just did
    pub fn done_with(mut self, done_msg: &str) {
        self.done_msg = done_msg.to_string();
        self.done()
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
        let time = format!("[{:.1}s]", self.started.elapsed().as_secs_f64());
        let symbol = paint_symbol(symbol, colour);

        if state.progress.as_ref().is_some_and(|p| p.id == self.id) {
            state.progress = None;
        }

        let line = if state.pending.as_ref().is_some_and(|p| p.id == self.id) {
            state.pending = None;
            &format_line(state.indent, msg, &symbol)
        } else {
            &format_line(state.indent + 1, msg, &symbol)
        };

        let line = format_close_line(state.is_tty, line, &time);
        write(&state, Some(&line))
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        if !self.finished {
            self.close("x", BOLD_RED, false);
        }
    }
}

#[allow(dead_code)]
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
        started: Instant::now(),
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
        started: Instant::now(),
        finished: false,
    }
}

pub fn paint(text: &str, colour: &str) -> String {
    format!("{colour}{text}{NORMAL}")
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
    // point-free on purpose: a closure here would be a new never-executed
    // region (poisoning is untestable), while a fn path adds no code
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
    print!("{}", render_output(state, permanent));
    // display is non-critical: a broken pipe must never kill the scrape
    io::stdout().flush().ok();
}

fn render_output(state: &PrintState, permanent: Option<&str>) -> String {
    let mut output = String::new();

    if state.is_tty {
        output.push_str(ERASE_LINE);
    }

    if let Some(line) = permanent {
        output.push_str(line);
        output.push('\n');
    }

    if state.is_tty {
        if let Some(line) = render_bottom_line(state) {
            output.push_str(&line);
        }
    }

    output
}

fn format_line(indent: usize, msg: &str, symbol: &str) -> String {
    let spaces = "  ".repeat(indent);
    format!("{spaces}{symbol} {msg}")
}

fn format_progress_symbol(done: usize, total: usize) -> String {
    let width = total.to_string().len();
    format!("{done:>width$}/{total}")
}

// flush-right time: cursor-forward 999 clamps at the last column, then back
// up by the visible length — measured before painting, since the colour
// escapes occupy zero columns; off-tty (logs) plain text only
fn format_close_line(is_tty: bool, line: &str, time: &str) -> String {
    if is_tty {
        format!(
            "{line}\x1b[999C\x1b[{}D{}",
            time.len() - 1,
            paint(time, BLUE)
        )
    } else {
        format!("{line} {time}")
    }
}

fn paint_symbol(text: &str, colour: &str) -> String {
    paint(&format!("[{}]", text), colour)
}

// tests (here or in other modules) that drive the public API mutate the
// global STATE and cargo runs tests concurrently: they must hold this lock
// for their whole duration; all other tests build local states
#[cfg(test)]
pub(crate) static TEST_STATE_LOCK: std::sync::Mutex<()> =
    std::sync::Mutex::new(());

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
    fn close_line_places_the_time_flush_right_on_tty() {
        // "[1.2s]" is 6 columns visible: forward to the edge, back 5,
        // print — the painted string moves the cursor only 6 columns
        assert_eq!(
            format_close_line(true, "[+] msg", "[1.2s]"),
            format!("[+] msg\x1b[999C\x1b[5D{}", paint("[1.2s]", BLUE))
        );
    }

    #[test]
    fn close_line_appends_the_time_plainly_off_tty() {
        assert_eq!(
            format_close_line(false, "[+] msg", "[1.2s]"),
            "[+] msg [1.2s]"
        );
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
    fn render_output_erases_and_redraws_bottom_line_on_tty() {
        let mut state = test_state(Some(test_pending(0, 1)), None);
        state.is_tty = true;
        let bottom = format_line(1, "pending 0", &paint_symbol("✱", BLUE));
        assert_eq!(
            render_output(&state, Some("permanent")),
            format!("{ERASE_LINE}permanent\n{bottom}")
        );
    }

    #[test]
    fn render_output_erases_without_redraw_when_nothing_transient() {
        let mut state = test_state(None, None);
        state.is_tty = true;
        assert_eq!(
            render_output(&state, Some("permanent")),
            format!("{ERASE_LINE}permanent\n")
        );
    }

    #[test]
    fn render_output_emits_only_permanent_lines_off_tty() {
        let state = test_state(Some(test_pending(0, 1)), None);
        assert_eq!(render_output(&state, Some("permanent")), "permanent\n");
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

    #[test]
    fn public_api_lifecycle_returns_state_to_neutral() {
        let _guard = TEST_STATE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
