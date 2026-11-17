//! The date and time operations used by the schedule search, implemented for
//! each supported date and time library.

use crate::ordinal::Ordinal;

/// The local calendar fields of a date and time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LocalFields {
    pub(crate) year: i32,
    pub(crate) month: Ordinal,
    pub(crate) day: Ordinal,
    pub(crate) hour: Ordinal,
    pub(crate) minute: Ordinal,
    pub(crate) second: Ordinal,
}

/// The instants that a local date and time maps to in a time zone.
pub(crate) enum LocalMapping<T> {
    /// The local time is skipped, for example by a daylight saving gap.
    None,
    Single(T),
    /// The local time is repeated, for example by a daylight saving fold.
    Ambiguous(T, T),
}

/// A date and time in a time zone.
pub(crate) trait CronDateTime: Clone + Ord {
    /// The local calendar fields, ignoring any fraction of a second.
    fn local_fields(&self) -> LocalFields;

    /// Sunday is 1 and Saturday is 7.
    fn weekday_from_sunday(&self) -> Ordinal;

    /// Whether the time has a nonzero fraction of a second.
    fn has_subsec_nanos(&self) -> bool;

    /// The instant one second earlier.
    fn one_second_earlier(&self) -> Self;

    /// The instants of `fields` in the time zone of `self`.
    fn with_local_fields(&self, fields: LocalFields) -> LocalMapping<Self>;

    /// The earlier and later instants of the local date and time of `self`,
    /// including any fraction of a second, when it is repeated in its time
    /// zone.
    fn fold(&self) -> Option<(Self, Self)>;

    /// The weekday of a calendar date, with Sunday as 1 and Saturday as 7, or
    /// `None` if the date does not exist.
    fn date_weekday_from_sunday(year: i32, month: Ordinal, day: Ordinal) -> Option<Ordinal>;
}

#[cfg(feature = "chrono")]
mod chrono_impl {
    use super::{CronDateTime, LocalFields, LocalMapping};
    use crate::ordinal::Ordinal;
    use chrono::offset::{LocalResult, TimeZone};
    use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike};
    use std::cmp::{max, min};

    impl<Z: TimeZone> CronDateTime for DateTime<Z> {
        fn local_fields(&self) -> LocalFields {
            LocalFields {
                year: self.year(),
                month: self.month(),
                day: self.day(),
                hour: self.hour(),
                minute: self.minute(),
                second: self.second(),
            }
        }

        fn weekday_from_sunday(&self) -> Ordinal {
            self.weekday().number_from_sunday()
        }

        fn has_subsec_nanos(&self) -> bool {
            self.timestamp_subsec_nanos() > 0
        }

        fn one_second_earlier(&self) -> Self {
            self.clone() - Duration::seconds(1)
        }

        fn with_local_fields(&self, fields: LocalFields) -> LocalMapping<Self> {
            let local = self.timezone().with_ymd_and_hms(
                fields.year,
                fields.month,
                fields.day,
                fields.hour,
                fields.minute,
                fields.second,
            );
            match local {
                LocalResult::None => LocalMapping::None,
                LocalResult::Single(datetime) => LocalMapping::Single(datetime),
                LocalResult::Ambiguous(earlier, later) => LocalMapping::Ambiguous(earlier, later),
            }
        }

        fn fold(&self) -> Option<(Self, Self)> {
            match self.timezone().from_local_datetime(&self.naive_local()) {
                LocalResult::Ambiguous(first, second) => {
                    Some((min(first.clone(), second.clone()), max(first, second)))
                }
                _ => None,
            }
        }

        fn date_weekday_from_sunday(year: i32, month: Ordinal, day: Ordinal) -> Option<Ordinal> {
            NaiveDate::from_ymd_opt(year, month, day)
                .map(|date| date.weekday().number_from_sunday())
        }
    }
}

#[cfg(feature = "jiff")]
mod jiff_impl {
    use super::{CronDateTime, LocalFields, LocalMapping};
    use crate::ordinal::Ordinal;
    use jiff::civil::{Date, DateTime};
    use jiff::tz::AmbiguousOffset;
    use jiff::{SignedDuration, ZonedDateTime};

    impl CronDateTime for ZonedDateTime {
        fn local_fields(&self) -> LocalFields {
            // Jiff represents years in -9999..=9999 and the other fields as
            // small positive numbers.
            LocalFields {
                year: i32::from(self.year()),
                month: self.month() as Ordinal,
                day: self.day() as Ordinal,
                hour: self.hour() as Ordinal,
                minute: self.minute() as Ordinal,
                second: self.second() as Ordinal,
            }
        }

        fn weekday_from_sunday(&self) -> Ordinal {
            self.weekday().to_sunday_one_offset() as Ordinal
        }

        fn has_subsec_nanos(&self) -> bool {
            self.subsec_nanosecond() != 0
        }

        fn one_second_earlier(&self) -> Self {
            self.checked_sub(SignedDuration::from_secs(1))
                .expect("one second before a supported date and time is supported")
        }

        fn with_local_fields(&self, fields: LocalFields) -> LocalMapping<Self> {
            let Some(datetime) = civil_datetime(fields) else {
                return LocalMapping::None;
            };
            let local = self.time_zone().to_ambiguous_zoned_datetime(datetime);
            match local.offset() {
                AmbiguousOffset::Unambiguous { .. } => match local.compatible() {
                    Ok(datetime) => LocalMapping::Single(datetime),
                    Err(_) => LocalMapping::None,
                },
                AmbiguousOffset::Gap { .. } => LocalMapping::None,
                AmbiguousOffset::Fold { .. } => match (local.clone().earlier(), local.later()) {
                    (Ok(earlier), Ok(later)) => LocalMapping::Ambiguous(earlier, later),
                    (Ok(datetime), Err(_)) | (Err(_), Ok(datetime)) => {
                        LocalMapping::Single(datetime)
                    }
                    (Err(_), Err(_)) => LocalMapping::None,
                },
            }
        }

        fn fold(&self) -> Option<(Self, Self)> {
            let local = self
                .time_zone()
                .to_ambiguous_zoned_datetime(self.datetime());
            if !matches!(local.offset(), AmbiguousOffset::Fold { .. }) {
                return None;
            }
            match (local.clone().earlier(), local.later()) {
                (Ok(earlier), Ok(later)) => Some((earlier, later)),
                _ => None,
            }
        }

        fn date_weekday_from_sunday(year: i32, month: Ordinal, day: Ordinal) -> Option<Ordinal> {
            let date = Date::new(
                i16::try_from(year).ok()?,
                i8::try_from(month).ok()?,
                i8::try_from(day).ok()?,
            )
            .ok()?;
            Some(date.weekday().to_sunday_one_offset() as Ordinal)
        }
    }

    fn civil_datetime(fields: LocalFields) -> Option<DateTime> {
        DateTime::new(
            i16::try_from(fields.year).ok()?,
            i8::try_from(fields.month).ok()?,
            i8::try_from(fields.day).ok()?,
            i8::try_from(fields.hour).ok()?,
            i8::try_from(fields.minute).ok()?,
            i8::try_from(fields.second).ok()?,
            0,
        )
        .ok()
    }
}
