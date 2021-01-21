use std::cmp::Ordering;
use std::time::Instant;

use crate::window::TimerToken;

/// A timer is a deadline (`std::Time::Instant`) and a `TimerToken`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Timer {
    deadline: Instant,
    token: TimerToken,
}

impl Timer {
    pub(crate) fn new(deadline: Instant) -> Self {
        let token = TimerToken::next();
        Self { deadline, token }
    }

    pub(crate) fn deadline(&self) -> Instant {
        self.deadline
    }

    pub(crate) fn token(&self) -> TimerToken {
        self.token
    }
}

impl Ord for Timer {
    /// Ordering is so that earliest deadline sorts first
    // "Earliest deadline first" that a std::collections::BinaryHeap will have the earliest timer
    // at its head, which is just what is needed for timer management.
    fn cmp(&self, other: &Self) -> Ordering {
        self.deadline.cmp(&other.deadline).reverse()
    }
}

impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
