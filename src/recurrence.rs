use std::fmt;

use chrono::{Datelike, Days, Months, NaiveDate};

const MONTHS_PER_YEAR: u32 = 12;
const DAYS_PER_WEEK: u32 = 7;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unit {
    Day,
    Week,
    Month,
    Year,
}

impl Unit {
    fn name(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
            Self::Year => "year",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DayOfMonth {
    Fixed(u32),
    Last,
}

impl DayOfMonth {
    pub fn fixed(day: u32) -> Option<Self> {
        (1..=31).contains(&day).then_some(Self::Fixed(day))
    }

    pub fn months_after(self, date: NaiveDate, months: u32) -> Option<NaiveDate> {
        let first_of_month = date.with_day(1)?.checked_add_months(Months::new(months))?;
        let last_of_month = first_of_month
            .checked_add_months(Months::new(1))?
            .pred_opt()?;
        let day = match self {
            Self::Fixed(day) => day.min(last_of_month.day()),
            Self::Last => last_of_month.day(),
        };
        first_of_month.with_day(day)
    }
}

impl fmt::Display for DayOfMonth {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Last => write!(formatter, "last"),
            Self::Fixed(day) => {
                let suffix = match (day % 10, day % 100) {
                    (_, 11..=13) => "th",
                    (1, _) => "st",
                    (2, _) => "nd",
                    (3, _) => "rd",
                    _ => "th",
                };
                write!(formatter, "{day}{suffix}")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Recurrence {
    interval: u32,
    unit: Unit,
    day_of_month: Option<DayOfMonth>,
    when_done: bool,
}

impl Recurrence {
    pub fn every(interval: u32, unit: Unit, day_of_month: Option<DayOfMonth>) -> Option<Self> {
        let only_months_have_a_day = day_of_month.is_none() || unit == Unit::Month;
        (interval > 0 && only_months_have_a_day).then_some(Self {
            interval,
            unit,
            day_of_month,
            when_done: false,
        })
    }

    /// @see https://publish.obsidian.md/tasks/Getting+Started/Recurring+Tasks
    pub fn parse(rule: &str) -> Option<Self> {
        let rule = rule.to_lowercase();
        let words: Vec<&str> = rule.split_whitespace().collect();

        let ["every", rest @ ..] = words.as_slice() else {
            return None;
        };
        let (rest, when_done) = match rest {
            [rest @ .., "when", "done"] => (rest, true),
            _ => (rest, false),
        };
        let (rest, day_of_month) = match rest {
            [rest @ .., "on", "the", day] => (rest, Some(day_of_month(day)?)),
            _ => (rest, None),
        };
        let (interval, unit) = match rest {
            [unit] => (1, unit_named(unit)?),
            [count, unit] => (count.parse().ok()?, unit_named(unit)?),
            _ => return None,
        };

        Some(Self {
            when_done,
            ..Self::every(interval, unit, day_of_month)?
        })
    }

    pub fn next_due(&self, due: NaiveDate, completed: NaiveDate) -> Option<NaiveDate> {
        let start = if self.when_done { completed } else { due };
        (1..)
            .map(|steps| self.advance(start, steps))
            .find(|date| date.is_none_or(|date| date > completed))
            .flatten()
    }

    fn advance(&self, start: NaiveDate, steps: u32) -> Option<NaiveDate> {
        let periods = self.interval.checked_mul(steps)?;
        match self.unit {
            Unit::Day => start.checked_add_days(Days::new(u64::from(periods))),
            Unit::Week => start.checked_add_days(Days::new(u64::from(periods * DAYS_PER_WEEK))),
            Unit::Month => self.months_later(start, periods),
            Unit::Year => self.months_later(start, periods.checked_mul(MONTHS_PER_YEAR)?),
        }
    }

    fn months_later(&self, start: NaiveDate, months: u32) -> Option<NaiveDate> {
        self.day_of_month
            .unwrap_or(DayOfMonth::Fixed(start.day()))
            .months_after(start, months)
    }
}

impl fmt::Display for Recurrence {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let unit = self.unit.name();
        match self.interval {
            1 => write!(formatter, "every {unit}")?,
            interval => write!(formatter, "every {interval} {unit}s")?,
        }
        if let Some(day) = self.day_of_month {
            write!(formatter, " on the {day}")?;
        }
        if self.when_done {
            write!(formatter, " when done")?;
        }
        Ok(())
    }
}

fn unit_named(word: &str) -> Option<Unit> {
    match word {
        "day" | "days" => Some(Unit::Day),
        "week" | "weeks" => Some(Unit::Week),
        "month" | "months" => Some(Unit::Month),
        "year" | "years" => Some(Unit::Year),
        _ => None,
    }
}

fn day_of_month(word: &str) -> Option<DayOfMonth> {
    if word == "last" {
        return Some(DayOfMonth::Last);
    }
    let number = ["st", "nd", "rd", "th"]
        .iter()
        .find_map(|suffix| word.strip_suffix(suffix))?;
    DayOfMonth::fixed(number.parse().ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn next(rule: &str, due: NaiveDate, completed: NaiveDate) -> NaiveDate {
        Recurrence::parse(rule)
            .unwrap()
            .next_due(due, completed)
            .unwrap()
    }

    #[test]
    fn understands_the_basic_units() {
        for rule in ["every day", "every week", "every month", "every year"] {
            assert!(Recurrence::parse(rule).is_some(), "{rule}");
        }
    }

    #[test]
    fn understands_an_interval_in_singular_or_plural() {
        assert_eq!(
            Recurrence::parse("every 2 weeks"),
            Recurrence::parse("every 2 week")
        );
    }

    #[test]
    fn ignores_capitals_and_extra_spaces() {
        assert_eq!(
            Recurrence::parse("Every  Month on the Last"),
            Recurrence::parse("every month on the last")
        );
    }

    #[test]
    fn rejects_what_it_does_not_understand() {
        for rule in [
            "",
            "every",
            "weekly",
            "every fortnight",
            "every 0 days",
            "every week on Friday",
            "every monday and thursday",
            "every week on the 3rd",
            "every month on the 32nd",
            "every month on the 0th",
        ] {
            assert_eq!(Recurrence::parse(rule), None, "{rule}");
        }
    }

    #[test]
    fn writes_the_rule_back_as_obsidian_tasks_reads_it() {
        for rule in [
            "every day",
            "every 3 days",
            "every week",
            "every 2 weeks when done",
            "every month on the 1st",
            "every month on the 2nd",
            "every month on the 3rd",
            "every month on the 11th",
            "every month on the 12th",
            "every month on the 13th",
            "every month on the 21st",
            "every month on the 22nd",
            "every month on the 23rd",
            "every month on the 25th",
            "every 3 months on the last",
            "every year",
        ] {
            assert_eq!(Recurrence::parse(rule).unwrap().to_string(), rule);
        }
    }

    #[test]
    fn a_day_of_the_month_lies_between_1_and_31() {
        assert_eq!(DayOfMonth::fixed(0), None);
        assert_eq!(DayOfMonth::fixed(1), Some(DayOfMonth::Fixed(1)));
        assert_eq!(DayOfMonth::fixed(31), Some(DayOfMonth::Fixed(31)));
        assert_eq!(DayOfMonth::fixed(32), None);
    }

    #[test]
    fn only_a_monthly_recurrence_has_a_day_of_the_month() {
        assert!(Recurrence::every(1, Unit::Month, Some(DayOfMonth::Last)).is_some());
        assert_eq!(
            Recurrence::every(1, Unit::Week, Some(DayOfMonth::Last)),
            None
        );
        assert_eq!(Recurrence::every(0, Unit::Day, None), None);
    }

    #[test]
    fn the_next_week_keeps_the_weekday_of_the_due_date() {
        assert_eq!(
            next("every week", date(2026, 11, 27), date(2026, 11, 26)),
            date(2026, 12, 4)
        );
    }

    #[test]
    fn counts_from_the_due_date_when_checked_off_late() {
        assert_eq!(
            next("every week", date(2026, 11, 27), date(2026, 11, 30)),
            date(2026, 12, 4)
        );
    }

    #[test]
    fn when_done_counts_from_the_day_it_was_checked_off() {
        assert_eq!(
            next(
                "every week when done",
                date(2026, 11, 27),
                date(2026, 11, 30)
            ),
            date(2026, 12, 7)
        );
    }

    #[test]
    fn skips_the_occurrences_that_were_missed() {
        assert_eq!(
            next("every week", date(2026, 11, 6), date(2026, 11, 26)),
            date(2026, 11, 27)
        );
    }

    #[test]
    fn checking_off_on_the_due_date_moves_to_the_next_occurrence() {
        assert_eq!(
            next("every day", date(2026, 11, 26), date(2026, 11, 26)),
            date(2026, 11, 27)
        );
    }

    #[test]
    fn an_interval_skips_that_many_periods() {
        assert_eq!(
            next("every 3 days", date(2026, 11, 26), date(2026, 11, 26)),
            date(2026, 11, 29)
        );
    }

    #[test]
    fn a_month_keeps_the_day_of_the_due_date() {
        assert_eq!(
            next("every month", date(2026, 12, 1), date(2026, 12, 1)),
            date(2027, 1, 1)
        );
    }

    #[test]
    fn a_day_that_a_month_lacks_becomes_its_last_day() {
        assert_eq!(
            next("every month", date(2027, 1, 31), date(2027, 1, 31)),
            date(2027, 2, 28)
        );
    }

    #[test]
    fn missed_months_are_counted_from_the_due_date_so_the_day_does_not_drift() {
        assert_eq!(
            next("every month", date(2027, 1, 31), date(2027, 3, 1)),
            date(2027, 3, 31)
        );
    }

    #[test]
    fn a_fixed_day_of_the_month() {
        assert_eq!(
            next(
                "every month on the 25th",
                date(2026, 11, 25),
                date(2026, 11, 26)
            ),
            date(2026, 12, 25)
        );
    }

    #[test]
    fn a_fixed_day_that_a_month_lacks_becomes_its_last_day() {
        assert_eq!(
            next(
                "every month on the 31st",
                date(2027, 1, 31),
                date(2027, 1, 31)
            ),
            date(2027, 2, 28)
        );
        assert_eq!(
            next(
                "every month on the 31st",
                date(2027, 2, 28),
                date(2027, 2, 28)
            ),
            date(2027, 3, 31)
        );
    }

    #[test]
    fn the_last_day_of_the_month() {
        assert_eq!(
            next(
                "every month on the last",
                date(2027, 2, 28),
                date(2027, 2, 28)
            ),
            date(2027, 3, 31)
        );
    }

    #[test]
    fn a_leap_day_falls_back_to_the_last_day_of_february() {
        assert_eq!(
            next("every year", date(2028, 2, 29), date(2028, 2, 29)),
            date(2029, 2, 28)
        );
    }
}
