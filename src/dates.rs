use std::fmt;

use chrono::{Datelike, Days, NaiveDate, Weekday};

mod vocabulary;

use vocabulary::{Every, Keyword, PeriodWord, every, keyword, means_no_date, period};

use crate::recurrence::{DayOfMonth, Recurrence, Unit};

pub const DEFAULT_WEEK_END: Weekday = Weekday::Fri;
pub const DEFAULT_MONTH_END: DayOfMonth = DayOfMonth::Last;

#[derive(Debug, Clone, Copy)]
pub struct Calendar {
    pub today: NaiveDate,
    pub week_ends_on: Weekday,
    pub month_ends_on: DayOfMonth,
}

#[cfg(test)]
impl Calendar {
    pub fn on(today: NaiveDate) -> Self {
        Self {
            today,
            week_ends_on: DEFAULT_WEEK_END,
            month_ends_on: DEFAULT_MONTH_END,
        }
    }
}

pub fn weekday_named(word: &str) -> Option<Weekday> {
    match keyword(&normalize(word))? {
        Keyword::On(day) => Some(day),
        _ => None,
    }
}

#[derive(Debug, PartialEq)]
pub struct UnknownDate(pub String);

impl fmt::Display for UnknownDate {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "unknown date '{}': use e.g. tomorrow, friday, nextweek, thismonth, 25/11, 2026-11-25, every-friday or every-month (Dutch words work too)",
            self.0
        )
    }
}

impl std::error::Error for UnknownDate {}

fn parse_date(input: &str, calendar: &Calendar) -> Result<NaiveDate, UnknownDate> {
    let today = calendar.today;
    let word = normalize(input);

    keyword(&word)
        .and_then(|keyword| resolve(keyword, calendar))
        .or_else(|| day_and_month(input, today))
        .or_else(|| NaiveDate::parse_from_str(input, "%Y-%m-%d").ok())
        .ok_or_else(|| UnknownDate(input.to_string()))
}

#[derive(Debug, PartialEq)]
pub enum Schedule {
    NoDate,
    Due(NaiveDate),
    Repeating {
        due: NaiveDate,
        recurrence: Recurrence,
    },
}

#[derive(Debug, PartialEq)]
pub enum ScheduleError {
    Unknown(UnknownDate),
    TwoDates(String, String),
    TwoRepetitions(String, String),
    WeekdayAndDate(String, String),
    NoDateWithOthers(String),
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Unknown(unknown) => unknown.fmt(formatter),
            Self::TwoDates(first, second) => {
                write!(formatter, "@{first} and @{second} are both dates; give one")
            }
            Self::TwoRepetitions(first, second) => {
                write!(formatter, "@{first} and @{second} both repeat; give one")
            }
            Self::WeekdayAndDate(repetition, date) => write!(
                formatter,
                "@{repetition} already sets the day; leave out @{date}, or use @every-week from that date"
            ),
            Self::NoDateWithOthers(word) => write!(
                formatter,
                "@{word} clears the date and its repetition; give it on its own"
            ),
        }
    }
}

impl std::error::Error for ScheduleError {}

enum AtWord {
    NoDate,
    Date(NaiveDate),
    Every(Every),
}

fn at_word(word: &str, calendar: &Calendar) -> Result<AtWord, ScheduleError> {
    let normalized = normalize(word);
    if means_no_date(&normalized) {
        return Ok(AtWord::NoDate);
    }
    if let Some(repetition) = every(&normalized) {
        return Ok(AtWord::Every(repetition));
    }
    parse_date(word, calendar)
        .map(AtWord::Date)
        .map_err(ScheduleError::Unknown)
}

pub fn parse_schedule(words: &[&str], calendar: &Calendar) -> Result<Schedule, ScheduleError> {
    let mut date: Option<(&str, NaiveDate)> = None;
    let mut repetition: Option<(&str, Every)> = None;

    for &word in words {
        match at_word(word, calendar)? {
            AtWord::NoDate if words.len() == 1 => return Ok(Schedule::NoDate),
            AtWord::NoDate => return Err(ScheduleError::NoDateWithOthers(word.to_string())),
            AtWord::Date(due) => {
                if let Some((first, _)) = date.replace((word, due)) {
                    return Err(ScheduleError::TwoDates(first.to_string(), word.to_string()));
                }
            }
            AtWord::Every(every) => {
                if let Some((first, _)) = repetition.replace((word, every)) {
                    return Err(ScheduleError::TwoRepetitions(
                        first.to_string(),
                        word.to_string(),
                    ));
                }
            }
        }
    }

    match (date, repetition) {
        (None, None) => Ok(Schedule::NoDate),
        (Some((_, due)), None) => Ok(Schedule::Due(due)),
        (date, Some((word, every))) => repeating(word, every, date, calendar),
    }
}

fn repeating(
    word: &str,
    every: Every,
    date: Option<(&str, NaiveDate)>,
    calendar: &Calendar,
) -> Result<Schedule, ScheduleError> {
    let not_understood = || ScheduleError::Unknown(UnknownDate(word.to_string()));

    let (due, recurrence) = match (every, date) {
        (Every::Weekday(_), Some((date_word, _))) => {
            return Err(ScheduleError::WeekdayAndDate(
                word.to_string(),
                date_word.to_string(),
            ));
        }
        (Every::Weekday(day), None) => (
            next_after(calendar.today, day),
            Recurrence::every(1, Unit::Week, None),
        ),
        (Every::Unit(interval, unit), date) => {
            let due = match date {
                Some((_, due)) => due,
                None => first_due(unit, calendar).ok_or_else(not_understood)?,
            };
            let day_of_month = (unit == Unit::Month).then(|| match date {
                Some(_) => DayOfMonth::Fixed(due.day()),
                None => calendar.month_ends_on,
            });
            (due, Recurrence::every(interval, unit, day_of_month))
        }
    };

    Ok(Schedule::Repeating {
        due,
        recurrence: recurrence.ok_or_else(not_understood)?,
    })
}

fn first_due(unit: Unit, calendar: &Calendar) -> Option<NaiveDate> {
    let today = calendar.today;
    match unit {
        Unit::Day => Some(today),
        Unit::Week => Some(next_or_same(today, calendar.week_ends_on)),
        Unit::Month => end_of_month(calendar, 0),
        Unit::Year => NaiveDate::from_ymd_opt(today.year(), 12, 31),
    }
}

#[derive(Debug, PartialEq)]
pub struct Period {
    pub from: NaiveDate,
    pub until: NaiveDate,
}

#[derive(Debug, PartialEq)]
pub struct UnknownPeriod(pub String);

impl fmt::Display for UnknownPeriod {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "unknown period '{}': use today, yesterday, thisweek, lastweek, thismonth, lastmonth, thisyear or lastyear (Dutch words work too)",
            self.0
        )
    }
}

impl std::error::Error for UnknownPeriod {}

pub fn parse_period(input: &str, calendar: &Calendar) -> Result<Period, UnknownPeriod> {
    let today = calendar.today;
    let unknown = || UnknownPeriod(input.to_string());
    let word = period(&normalize(input.trim_start_matches('@'))).ok_or_else(unknown)?;
    let monday = today - Days::new(u64::from(today.weekday().num_days_from_monday()));

    let (from, until) = match word {
        PeriodWord::Today => (today, today),
        PeriodWord::Yesterday => {
            let yesterday = today.pred_opt().ok_or_else(unknown)?;
            (yesterday, yesterday)
        }
        PeriodWord::ThisWeek => (monday, today),
        PeriodWord::LastWeek => (monday - Days::new(7), monday - Days::new(1)),
        PeriodWord::ThisMonth => (today.with_day(1).ok_or_else(unknown)?, today),
        PeriodWord::LastMonth => {
            let end = today.with_day(1).and_then(|first| first.pred_opt());
            let start = end.and_then(|end| end.with_day(1));
            (start.ok_or_else(unknown)?, end.ok_or_else(unknown)?)
        }
        PeriodWord::ThisYear => (
            NaiveDate::from_ymd_opt(today.year(), 1, 1).ok_or_else(unknown)?,
            today,
        ),
        PeriodWord::LastYear => {
            let year = today.year() - 1;
            (
                NaiveDate::from_ymd_opt(year, 1, 1).ok_or_else(unknown)?,
                NaiveDate::from_ymd_opt(year, 12, 31).ok_or_else(unknown)?,
            )
        }
    };
    Ok(Period { from, until })
}

impl Period {
    pub fn contains(&self, date: NaiveDate) -> bool {
        (self.from..=self.until).contains(&date)
    }
}

fn normalize(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .filter(|character| !matches!(character, '-' | '_' | ' '))
        .collect()
}

fn resolve(keyword: Keyword, calendar: &Calendar) -> Option<NaiveDate> {
    let today = calendar.today;
    match keyword {
        Keyword::Today => Some(today),
        Keyword::Tomorrow => today.checked_add_days(Days::new(1)),
        Keyword::DayAfterTomorrow => today.checked_add_days(Days::new(2)),
        Keyword::ThisWeek => Some(next_or_same(today, calendar.week_ends_on)),
        Keyword::NextWeek => {
            next_or_same(today, calendar.week_ends_on).checked_add_days(Days::new(7))
        }
        Keyword::ThisMonth => end_of_month(calendar, 0),
        Keyword::NextMonth => end_of_month(calendar, 1),
        Keyword::On(day) => Some(next_after(today, day)),
    }
}

fn next_or_same(today: NaiveDate, day: Weekday) -> NaiveDate {
    let days_ahead = (day.num_days_from_monday() + 7 - today.weekday().num_days_from_monday()) % 7;
    today + Days::new(u64::from(days_ahead))
}

fn next_after(today: NaiveDate, day: Weekday) -> NaiveDate {
    next_or_same(today + Days::new(1), day)
}

fn end_of_month(calendar: &Calendar, months_ahead: u32) -> Option<NaiveDate> {
    let month_end = calendar.month_ends_on;
    let this_month = month_end.months_after(calendar.today, 0)?;
    let already_passed = u32::from(this_month < calendar.today);
    month_end.months_after(calendar.today, months_ahead + already_passed)
}

fn day_and_month(input: &str, today: NaiveDate) -> Option<NaiveDate> {
    let (day, month) = input.split_once('/')?;
    let (day, month): (u32, u32) = (day.parse().ok()?, month.parse().ok()?);

    let this_year = NaiveDate::from_ymd_opt(today.year(), month, day);
    match this_year {
        Some(date) if date >= today => Some(date),
        _ => NaiveDate::from_ymd_opt(today.year() + 1, month, day),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    const FRIDAY: (i32, u32, u32) = (2026, 11, 20);

    fn on_friday(input: &str) -> Result<NaiveDate, UnknownDate> {
        parse_date(input, &Calendar::on(date(FRIDAY.0, FRIDAY.1, FRIDAY.2)))
    }

    #[test]
    fn relative_days_work_in_both_languages() {
        assert_eq!(on_friday("today"), Ok(date(2026, 11, 20)));
        assert_eq!(on_friday("vandaag"), Ok(date(2026, 11, 20)));
        assert_eq!(on_friday("tomorrow"), Ok(date(2026, 11, 21)));
        assert_eq!(on_friday("morgen"), Ok(date(2026, 11, 21)));
        assert_eq!(on_friday("overmorgen"), Ok(date(2026, 11, 22)));
    }

    #[test]
    fn a_weekday_means_the_next_one_after_today() {
        assert_eq!(on_friday("monday"), Ok(date(2026, 11, 23)));
        assert_eq!(on_friday("maandag"), Ok(date(2026, 11, 23)));
        assert_eq!(on_friday("vrijdag"), Ok(date(2026, 11, 27)));
    }

    #[test]
    fn a_week_ends_on_friday() {
        assert_eq!(on_friday("thisweek"), Ok(date(2026, 11, 20)));
        assert_eq!(on_friday("dezeweek"), Ok(date(2026, 11, 20)));
        assert_eq!(on_friday("nextweek"), Ok(date(2026, 11, 27)));
        assert_eq!(on_friday("volgendeweek"), Ok(date(2026, 11, 27)));
    }

    #[test]
    fn this_week_on_a_saturday_means_the_coming_friday() {
        assert_eq!(
            parse_date("thisweek", &Calendar::on(date(2026, 11, 21))),
            Ok(date(2026, 11, 27))
        );
    }

    #[test]
    fn a_month_ends_on_its_last_day() {
        assert_eq!(on_friday("thismonth"), Ok(date(2026, 11, 30)));
        assert_eq!(on_friday("dezemaand"), Ok(date(2026, 11, 30)));
        assert_eq!(on_friday("nextmonth"), Ok(date(2026, 12, 31)));
        assert_eq!(on_friday("volgendemaand"), Ok(date(2026, 12, 31)));
    }

    #[test]
    fn a_month_can_end_on_a_fixed_day() {
        let calendar = Calendar {
            month_ends_on: DayOfMonth::Fixed(25),
            ..Calendar::on(date(2026, 11, 20))
        };

        assert_eq!(parse_date("thismonth", &calendar), Ok(date(2026, 11, 25)));
        assert_eq!(parse_date("nextmonth", &calendar), Ok(date(2026, 12, 25)));
    }

    #[test]
    fn a_month_end_that_has_passed_moves_to_the_next_month() {
        let calendar = Calendar {
            month_ends_on: DayOfMonth::Fixed(15),
            ..Calendar::on(date(2026, 11, 20))
        };

        assert_eq!(parse_date("thismonth", &calendar), Ok(date(2026, 12, 15)));
        assert_eq!(parse_date("nextmonth", &calendar), Ok(date(2027, 1, 15)));
    }

    #[test]
    fn a_month_end_that_a_month_lacks_is_its_last_day() {
        let calendar = Calendar {
            month_ends_on: DayOfMonth::Fixed(31),
            ..Calendar::on(date(2027, 2, 10))
        };

        assert_eq!(parse_date("thismonth", &calendar), Ok(date(2027, 2, 28)));
        assert_eq!(parse_date("nextmonth", &calendar), Ok(date(2027, 3, 31)));
    }

    #[test]
    fn next_month_after_the_end_of_january_is_the_end_of_february() {
        assert_eq!(
            parse_date("nextmonth", &Calendar::on(date(2027, 1, 31))),
            Ok(date(2027, 2, 28))
        );
    }

    #[test]
    fn dashes_and_capitals_are_ignored() {
        assert_eq!(on_friday("Next-Week"), Ok(date(2026, 11, 27)));
        assert_eq!(on_friday("volgende-week"), Ok(date(2026, 11, 27)));
    }

    #[test]
    fn day_and_month_means_the_next_time_that_date_comes() {
        assert_eq!(on_friday("25/11"), Ok(date(2026, 11, 25)));
        assert_eq!(on_friday("5/1"), Ok(date(2027, 1, 5)));
    }

    #[test]
    fn a_full_date_is_used_as_is() {
        assert_eq!(on_friday("2026-12-01"), Ok(date(2026, 12, 1)));
    }

    #[test]
    fn impossible_or_unknown_dates_are_rejected() {
        assert!(on_friday("31/02").is_err());
        assert!(on_friday("2026-02-30").is_err());
        assert!(on_friday("someday").is_err());
    }

    fn schedule_on_friday(words: &[&str]) -> Result<Schedule, String> {
        parse_schedule(words, &Calendar::on(date(FRIDAY.0, FRIDAY.1, FRIDAY.2)))
            .map_err(|error| error.to_string())
    }

    fn repeating(due: NaiveDate, rule: &str) -> Result<Schedule, String> {
        Ok(Schedule::Repeating {
            due,
            recurrence: Recurrence::parse(rule).unwrap(),
        })
    }

    #[test]
    fn none_or_geen_means_no_due_date() {
        assert_eq!(schedule_on_friday(&["none"]), Ok(Schedule::NoDate));
        assert_eq!(schedule_on_friday(&["geen"]), Ok(Schedule::NoDate));
        assert_eq!(schedule_on_friday(&[]), Ok(Schedule::NoDate));
    }

    #[test]
    fn a_single_date_is_a_due_date() {
        assert_eq!(
            schedule_on_friday(&["tomorrow"]),
            Ok(Schedule::Due(date(2026, 11, 21)))
        );
    }

    #[test]
    fn every_weekday_starts_on_the_next_one_and_repeats_weekly() {
        let expected = repeating(date(2026, 11, 27), "every week");

        assert_eq!(schedule_on_friday(&["every-friday"]), expected);
        assert_eq!(schedule_on_friday(&["elke-vrijdag"]), expected);
        assert_eq!(schedule_on_friday(&["Every Friday"]), expected);
    }

    #[test]
    fn every_day_starts_today() {
        assert_eq!(
            schedule_on_friday(&["every-day"]),
            repeating(date(2026, 11, 20), "every day")
        );
        assert_eq!(
            schedule_on_friday(&["elke-3-dagen"]),
            repeating(date(2026, 11, 20), "every 3 days")
        );
    }

    #[test]
    fn every_week_starts_at_the_end_of_this_week() {
        assert_eq!(
            parse_schedule(&["every-week"], &Calendar::on(date(2026, 11, 18))),
            Ok(Schedule::Repeating {
                due: date(2026, 11, 20),
                recurrence: Recurrence::parse("every week").unwrap(),
            })
        );
        assert_eq!(
            schedule_on_friday(&["elke-2-weken"]),
            repeating(date(2026, 11, 20), "every 2 weeks")
        );
    }

    #[test]
    fn every_month_starts_at_the_end_of_this_month_and_keeps_that_day() {
        assert_eq!(
            schedule_on_friday(&["every-month"]),
            repeating(date(2026, 11, 30), "every month on the last")
        );

        let calendar = Calendar {
            month_ends_on: DayOfMonth::Fixed(25),
            ..Calendar::on(date(2026, 11, 20))
        };
        assert_eq!(
            parse_schedule(&["elke-maand"], &calendar),
            Ok(Schedule::Repeating {
                due: date(2026, 11, 25),
                recurrence: Recurrence::parse("every month on the 25th").unwrap(),
            })
        );
    }

    #[test]
    fn every_year_starts_at_the_end_of_this_year() {
        assert_eq!(
            schedule_on_friday(&["elk-jaar"]),
            repeating(date(2026, 12, 31), "every year")
        );
    }

    #[test]
    fn a_date_next_to_a_repetition_is_the_first_occurrence() {
        assert_eq!(
            schedule_on_friday(&["1/12", "every-month"]),
            repeating(date(2026, 12, 1), "every month on the 1st")
        );
        assert_eq!(
            schedule_on_friday(&["every-2-weeks", "monday"]),
            repeating(date(2026, 11, 23), "every 2 weeks")
        );
    }

    #[test]
    fn two_dates_are_an_error() {
        assert_eq!(
            schedule_on_friday(&["friday", "monday"]),
            Err("@friday and @monday are both dates; give one".to_string())
        );
    }

    #[test]
    fn two_repetitions_are_an_error() {
        assert_eq!(
            schedule_on_friday(&["every-week", "elke-maand"]),
            Err("@every-week and @elke-maand both repeat; give one".to_string())
        );
    }

    #[test]
    fn a_repeating_weekday_next_to_a_date_is_an_error() {
        assert_eq!(
            schedule_on_friday(&["every-friday", "1/12"]),
            Err(
                "@every-friday already sets the day; leave out @1/12, or use @every-week from that date"
                    .to_string()
            )
        );
    }

    #[test]
    fn none_next_to_another_word_is_an_error() {
        assert_eq!(
            schedule_on_friday(&["geen", "every-week"]),
            Err("@geen clears the date and its repetition; give it on its own".to_string())
        );
    }

    #[test]
    fn a_repetition_it_does_not_know_is_an_unknown_date() {
        for word in [
            "every-0-days",
            "every-2-fridays",
            "every-fortnight",
            "elke-friday",
        ] {
            assert!(
                schedule_on_friday(&[word])
                    .unwrap_err()
                    .starts_with(&format!("unknown date '{word}'")),
                "{word}"
            );
        }
    }

    fn period_on_friday(input: &str) -> Result<(NaiveDate, NaiveDate), UnknownPeriod> {
        parse_period(input, &Calendar::on(date(FRIDAY.0, FRIDAY.1, FRIDAY.2)))
            .map(|period| (period.from, period.until))
    }

    #[test]
    fn periods_work_in_both_languages() {
        let monday = date(2026, 11, 16);
        let friday = date(2026, 11, 20);

        assert_eq!(period_on_friday("thisweek"), Ok((monday, friday)));
        assert_eq!(period_on_friday("dezeweek"), Ok((monday, friday)));
        assert_eq!(period_on_friday("today"), Ok((friday, friday)));
        assert_eq!(
            period_on_friday("gisteren"),
            Ok((date(2026, 11, 19), date(2026, 11, 19)))
        );
    }

    #[test]
    fn last_week_runs_from_monday_to_sunday() {
        assert_eq!(
            period_on_friday("vorigeweek"),
            Ok((date(2026, 11, 9), date(2026, 11, 15)))
        );
    }

    #[test]
    fn this_month_starts_on_the_first() {
        assert_eq!(
            period_on_friday("thismonth"),
            Ok((date(2026, 11, 1), date(2026, 11, 20)))
        );
    }

    #[test]
    fn last_month_is_the_whole_previous_month() {
        assert_eq!(
            period_on_friday("lastmonth"),
            Ok((date(2026, 10, 1), date(2026, 10, 31)))
        );
        assert_eq!(
            period_on_friday("vorige maand"),
            Ok((date(2026, 10, 1), date(2026, 10, 31)))
        );
    }

    #[test]
    fn last_month_in_january_is_last_years_december() {
        assert_eq!(
            parse_period("vorigemaand", &Calendar::on(date(2027, 1, 15)))
                .map(|period| (period.from, period.until)),
            Ok((date(2026, 12, 1), date(2026, 12, 31)))
        );
    }

    #[test]
    fn this_year_runs_from_new_year_until_today() {
        assert_eq!(
            period_on_friday("thisyear"),
            Ok((date(2026, 1, 1), date(2026, 11, 20)))
        );
        assert_eq!(
            period_on_friday("dit jaar"),
            Ok((date(2026, 1, 1), date(2026, 11, 20)))
        );
    }

    #[test]
    fn last_year_is_the_whole_previous_year() {
        assert_eq!(
            period_on_friday("lastyear"),
            Ok((date(2025, 1, 1), date(2025, 12, 31)))
        );
        assert_eq!(
            period_on_friday("vorig-jaar"),
            Ok((date(2025, 1, 1), date(2025, 12, 31)))
        );
    }

    #[test]
    fn a_period_may_start_with_an_at_sign() {
        assert_eq!(
            period_on_friday("@vorigeweek"),
            period_on_friday("vorigeweek")
        );
    }

    #[test]
    fn an_unknown_period_is_an_error() {
        let error = period_on_friday("nextweek").unwrap_err();

        assert!(
            error
                .to_string()
                .starts_with("unknown period 'nextweek': use today, yesterday")
        );
    }

    #[test]
    fn a_week_can_end_on_sunday() {
        let calendar = Calendar {
            week_ends_on: Weekday::Sun,
            ..Calendar::on(date(2026, 11, 20))
        };

        assert_eq!(parse_date("thisweek", &calendar), Ok(date(2026, 11, 22)));
        assert_eq!(
            parse_date("volgendeweek", &calendar),
            Ok(date(2026, 11, 29))
        );
    }

    #[test]
    fn weekdays_are_named_in_both_languages() {
        assert_eq!(weekday_named("sunday"), Some(Weekday::Sun));
        assert_eq!(weekday_named("Zondag"), Some(Weekday::Sun));
        assert_eq!(weekday_named("tomorrow"), None);
    }

    #[test]
    fn the_error_names_the_input() {
        assert!(
            on_friday("someday")
                .unwrap_err()
                .to_string()
                .contains("'someday'")
        );
    }
}
