use chrono::Weekday;

const WEEK: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

struct Vocabulary {
    today: &'static str,
    yesterday: &'static str,
    tomorrow: &'static str,
    day_after_tomorrow: Option<&'static str>,
    this_week: &'static str,
    last_week: &'static str,
    next_week: &'static str,
    this_month: &'static str,
    last_month: &'static str,
    last_year: &'static str,
    next_month: &'static str,
    no_date: &'static str,
    weekdays: [&'static str; 7],
}

const ENGLISH: Vocabulary = Vocabulary {
    today: "today",
    yesterday: "yesterday",
    tomorrow: "tomorrow",
    day_after_tomorrow: None,
    this_week: "thisweek",
    last_week: "lastweek",
    next_week: "nextweek",
    this_month: "thismonth",
    last_month: "lastmonth",
    last_year: "lastyear",
    next_month: "nextmonth",
    no_date: "none",
    weekdays: [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ],
};

const DUTCH: Vocabulary = Vocabulary {
    today: "vandaag",
    yesterday: "gisteren",
    tomorrow: "morgen",
    day_after_tomorrow: Some("overmorgen"),
    this_week: "dezeweek",
    last_week: "vorigeweek",
    next_week: "volgendeweek",
    this_month: "dezemaand",
    last_month: "vorigemaand",
    last_year: "vorigjaar",
    next_month: "volgendemaand",
    no_date: "geen",
    weekdays: [
        "maandag",
        "dinsdag",
        "woensdag",
        "donderdag",
        "vrijdag",
        "zaterdag",
        "zondag",
    ],
};

const LANGUAGES: [Vocabulary; 2] = [ENGLISH, DUTCH];

#[derive(Clone, Copy)]
pub(super) enum Keyword {
    Today,
    Tomorrow,
    DayAfterTomorrow,
    ThisWeek,
    NextWeek,
    ThisMonth,
    NextMonth,
    On(Weekday),
}

#[derive(Clone, Copy)]
pub(super) enum PeriodWord {
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    ThisMonth,
    LastMonth,
    LastYear,
}

impl Vocabulary {
    fn period(&self, word: &str) -> Option<PeriodWord> {
        [
            (self.today, PeriodWord::Today),
            (self.yesterday, PeriodWord::Yesterday),
            (self.this_week, PeriodWord::ThisWeek),
            (self.last_week, PeriodWord::LastWeek),
            (self.this_month, PeriodWord::ThisMonth),
            (self.last_month, PeriodWord::LastMonth),
            (self.last_year, PeriodWord::LastYear),
        ]
        .into_iter()
        .find(|(spelling, _)| *spelling == word)
        .map(|(_, period)| period)
    }

    fn keyword(&self, word: &str) -> Option<Keyword> {
        let fixed_words = [
            (Some(self.today), Keyword::Today),
            (Some(self.tomorrow), Keyword::Tomorrow),
            (self.day_after_tomorrow, Keyword::DayAfterTomorrow),
            (Some(self.this_week), Keyword::ThisWeek),
            (Some(self.next_week), Keyword::NextWeek),
            (Some(self.this_month), Keyword::ThisMonth),
            (Some(self.next_month), Keyword::NextMonth),
        ];

        fixed_words
            .into_iter()
            .find(|(spelling, _)| *spelling == Some(word))
            .map(|(_, keyword)| keyword)
            .or_else(|| {
                let position = self.weekdays.iter().position(|day| *day == word)?;
                Some(Keyword::On(WEEK[position]))
            })
    }
}

pub(super) fn keyword(word: &str) -> Option<Keyword> {
    LANGUAGES.iter().find_map(|language| language.keyword(word))
}

pub(super) fn means_no_date(word: &str) -> bool {
    LANGUAGES.iter().any(|language| language.no_date == word)
}

pub(super) fn period(word: &str) -> Option<PeriodWord> {
    LANGUAGES.iter().find_map(|language| language.period(word))
}
