use std::fmt;

use chrono::{Datelike, Days, Months, NaiveDate, Weekday};

mod vocabulary;

use vocabulary::{Keyword, PeriodWord, keyword, means_no_date, period};

const WORKWEEK_END: Weekday = Weekday::Fri;

#[derive(Debug, PartialEq)]
pub struct UnknownDate(pub String);

impl fmt::Display for UnknownDate {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "unknown date '{}': use e.g. tomorrow, friday, nextweek, thismonth, 25/11 or 2026-11-25 (Dutch words work too)",
            self.0
        )
    }
}

impl std::error::Error for UnknownDate {}

pub fn parse_date(input: &str, today: NaiveDate) -> Result<NaiveDate, UnknownDate> {
    let word = normalize(input);

    keyword(&word)
        .and_then(|keyword| resolve(keyword, today))
        .or_else(|| day_and_month(input, today))
        .or_else(|| NaiveDate::parse_from_str(input, "%Y-%m-%d").ok())
        .ok_or_else(|| UnknownDate(input.to_string()))
}

pub fn parse_due(input: &str, today: NaiveDate) -> Result<Option<NaiveDate>, UnknownDate> {
    if means_no_date(&normalize(input)) {
        return Ok(None);
    }
    parse_date(input, today).map(Some)
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

pub fn parse_period(input: &str, today: NaiveDate) -> Result<Period, UnknownPeriod> {
    let unknown = || UnknownPeriod(input.to_string());
    let word = period(&normalize(input)).ok_or_else(unknown)?;
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

fn resolve(keyword: Keyword, today: NaiveDate) -> Option<NaiveDate> {
    match keyword {
        Keyword::Today => Some(today),
        Keyword::Tomorrow => today.checked_add_days(Days::new(1)),
        Keyword::DayAfterTomorrow => today.checked_add_days(Days::new(2)),
        Keyword::ThisWeek => Some(next_or_same(today, WORKWEEK_END)),
        Keyword::NextWeek => next_or_same(today, WORKWEEK_END).checked_add_days(Days::new(7)),
        Keyword::ThisMonth => last_day_of_month(today, 0),
        Keyword::NextMonth => last_day_of_month(today, 1),
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

fn last_day_of_month(today: NaiveDate, months_ahead: u32) -> Option<NaiveDate> {
    today
        .with_day(1)?
        .checked_add_months(Months::new(months_ahead + 1))?
        .pred_opt()
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
        parse_date(input, date(FRIDAY.0, FRIDAY.1, FRIDAY.2))
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
            parse_date("thisweek", date(2026, 11, 21)),
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
    fn next_month_after_the_end_of_january_is_the_end_of_february() {
        assert_eq!(
            parse_date("nextmonth", date(2027, 1, 31)),
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

    #[test]
    fn none_or_geen_means_no_due_date() {
        let friday = date(FRIDAY.0, FRIDAY.1, FRIDAY.2);

        assert_eq!(parse_due("none", friday), Ok(None));
        assert_eq!(parse_due("geen", friday), Ok(None));
        assert_eq!(parse_due("tomorrow", friday), Ok(Some(date(2026, 11, 21))));
    }

    fn period_on_friday(input: &str) -> Result<(NaiveDate, NaiveDate), UnknownPeriod> {
        parse_period(input, date(FRIDAY.0, FRIDAY.1, FRIDAY.2))
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
            parse_period("vorigemaand", date(2027, 1, 15))
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
    fn an_unknown_period_is_an_error() {
        assert!(period_on_friday("nextweek").is_err());
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
