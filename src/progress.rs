//! A one-line progress bar for the stages that take long enough to wonder
//! whether anything is happening.
//!
//! Rasterising a two-hundred-page document at 300 dpi takes minutes, and until
//! now it printed nothing at all — indistinguishable from a hang.
//!
//! Two things decide whether the bar is drawn, and both are about not making a
//! mess of someone else's output:
//!
//! - `--quiet` silences it. A progress bar is exactly the kind of stderr
//!   chatter that flag exists to remove.
//! - It is drawn only when stderr is a terminal. The bar works by returning to
//!   the start of the line and overwriting it; a log file or a CI transcript
//!   has no cursor to move, so the same run would leave thousands of fragments
//!   in the log. Redirected, it stays silent and the ordinary notes still come
//!   through.
//!
//! No dependency: `IsTerminal` is in the standard library, and the bar itself
//! is a line of text.

use std::io::{IsTerminal, Write};

const WIDTH: usize = 24;

pub struct Progress {
    label: String,
    total: usize,
    done: usize,
    /// False when quiet or redirected, which makes every method a no-op.
    active: bool,
}

impl Progress {
    pub fn new(label: impl Into<String>, total: usize) -> Self {
        let active = !crate::quiet() && std::io::stderr().is_terminal();
        let mut p = Progress { label: label.into(), total, done: 0, active };
        p.draw();
        p
    }

    /// One more unit done.
    pub fn tick(&mut self) {
        self.done += 1;
        self.draw();
    }

    /// Clears the line. Anything printed afterwards starts on clean ground —
    /// without this the bar's tail would sit behind the next message.
    pub fn finish(&mut self) {
        if !self.active {
            return;
        }
        self.active = false;
        let width = bar(&self.label, self.done, self.total).chars().count();
        let mut err = std::io::stderr();
        let _ = write!(err, "\r{}\r", " ".repeat(width));
        let _ = err.flush();
    }

    fn draw(&mut self) {
        if !self.active {
            return;
        }
        let mut err = std::io::stderr();
        let _ = write!(err, "\r{}", bar(&self.label, self.done, self.total));
        let _ = err.flush();
    }
}

/// Dropping without finishing still clears the line: an early return on an
/// error should not leave half a bar on the screen with the error printed over
/// it.
impl Drop for Progress {
    fn drop(&mut self) {
        self.finish();
    }
}

/// The bar as text. Kept separate from the writing so it can be tested without
/// a terminal.
fn bar(label: &str, done: usize, total: usize) -> String {
    // A total of zero means there was nothing to do, which is complete rather
    // than undefined — so a full bar, not a division by zero.
    let filled = (WIDTH * done.min(total)).checked_div(total).unwrap_or(WIDTH);
    // The counter is padded to the width of the total, so the line does not
    // jitter when it goes from 9 to 10 — and, more to the point, so every
    // redraw is exactly as wide as the one it overwrites.
    let digits = total.to_string().len();
    format!(
        "{label} [{}{}] {done:>digits$}/{total}",
        "#".repeat(filled),
        "-".repeat(WIDTH - filled)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_fills_in_proportion() {
        assert!(bar("x", 0, 4).contains(&format!("[{}]", "-".repeat(WIDTH))));
        assert!(bar("x", 4, 4).contains(&format!("[{}]", "#".repeat(WIDTH))));
        let half = bar("x", 2, 4);
        assert!(half.contains(&format!("[{}{}]", "#".repeat(WIDTH / 2), "-".repeat(WIDTH / 2))), "{half}");
    }

    #[test]
    fn it_names_the_stage_and_counts() {
        assert_eq!(
            bar("rasterising a.pdf", 3, 10),
            format!("rasterising a.pdf [{}{}]  3/10", "#".repeat(7), "-".repeat(17))
        );
    }

    /// A document with no pages to compare must not divide by zero, and an
    /// overshoot must not make the bar longer than the bar.
    #[test]
    fn the_edges_do_not_break_the_line() {
        let empty = bar("x", 0, 0);
        assert!(empty.contains(&"#".repeat(WIDTH)), "{empty}");
        let over = bar("x", 99, 4);
        assert_eq!(over.matches('#').count(), WIDTH);
        assert_eq!(over.matches('-').count(), 0);
    }

    /// Every line the bar draws is the same length, so overwriting the previous
    /// one covers it completely — a shorter line would leave the tail of the
    /// longer one behind on the terminal. The counter crossing from one digit
    /// to two is the case that would break it.
    #[test]
    fn every_step_is_the_same_width() {
        for total in [8, 10, 100, 207] {
            let widths: Vec<usize> =
                (0..=total).map(|i| bar("stage", i, total).chars().count()).collect();
            assert!(
                widths.windows(2).all(|w| w[0] == w[1]),
                "total {total} produced widths {:?}",
                &widths[..widths.len().min(12)]
            );
        }
    }
}
