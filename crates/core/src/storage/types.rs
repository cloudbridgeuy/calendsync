use chrono::{Datelike, NaiveDate};

use super::DateRangeError;

/// A date range with inclusive start and end dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl DateRange {
    /// Creates a new date range, validating that start <= end.
    pub fn new(start: NaiveDate, end: NaiveDate) -> Result<Self, DateRangeError> {
        if start > end {
            return Err(DateRangeError::InvalidRange);
        }
        Ok(Self { start, end })
    }

    /// Creates a date range for an entire month.
    ///
    /// # Panics
    /// Panics if the year/month combination is invalid.
    pub fn month(year: i32, month: u32) -> Self {
        let start = NaiveDate::from_ymd_opt(year, month, 1)
            .expect("Invalid year/month for DateRange::month");

        // Get the last day of the month by going to the first of next month and subtracting a day
        let end = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .expect("Invalid year/month for DateRange::month end calculation")
        .pred_opt()
        .expect("Failed to get last day of month");

        Self { start, end }
    }

    /// Creates a date range for the ISO week containing the given date.
    ///
    /// ISO weeks start on Monday and end on Sunday.
    pub fn week(date: NaiveDate) -> Self {
        // Get the Monday of the week
        let days_from_monday = date.weekday().num_days_from_monday();
        let start = date - chrono::Duration::days(days_from_monday as i64);

        // Sunday is 6 days after Monday
        let end = start + chrono::Duration::days(6);

        Self { start, end }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Weekday;

    #[test]
    fn test_valid_range_construction() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();

        let range = DateRange::new(start, end).unwrap();

        assert_eq!(range.start, start);
        assert_eq!(range.end, end);
    }

    #[test]
    fn test_same_day_range_is_valid() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();

        let range = DateRange::new(date, date).unwrap();

        assert_eq!(range.start, date);
        assert_eq!(range.end, date);
    }

    #[test]
    fn test_invalid_range_returns_error() {
        let start = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let result = DateRange::new(start, end);

        assert_eq!(result, Err(DateRangeError::InvalidRange));
    }

    #[test]
    fn test_month_factory_january() {
        let range = DateRange::month(2024, 1);

        assert_eq!(range.start, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(range.end, NaiveDate::from_ymd_opt(2024, 1, 31).unwrap());
    }

    #[test]
    fn test_month_factory_february_leap_year() {
        let range = DateRange::month(2024, 2);

        assert_eq!(range.start, NaiveDate::from_ymd_opt(2024, 2, 1).unwrap());
        assert_eq!(range.end, NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
    }

    #[test]
    fn test_month_factory_february_non_leap_year() {
        let range = DateRange::month(2023, 2);

        assert_eq!(range.start, NaiveDate::from_ymd_opt(2023, 2, 1).unwrap());
        assert_eq!(range.end, NaiveDate::from_ymd_opt(2023, 2, 28).unwrap());
    }

    #[test]
    fn test_month_factory_december() {
        let range = DateRange::month(2024, 12);

        assert_eq!(range.start, NaiveDate::from_ymd_opt(2024, 12, 1).unwrap());
        assert_eq!(range.end, NaiveDate::from_ymd_opt(2024, 12, 31).unwrap());
    }

    #[test]
    fn test_week_factory_from_monday() {
        // 2024-01-01 is a Monday
        let monday = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let range = DateRange::week(monday);

        assert_eq!(range.start, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(range.end, NaiveDate::from_ymd_opt(2024, 1, 7).unwrap());
        assert_eq!(range.start.weekday(), Weekday::Mon);
        assert_eq!(range.end.weekday(), Weekday::Sun);
    }

    #[test]
    fn test_week_factory_from_wednesday() {
        // 2024-01-03 is a Wednesday
        let wednesday = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        let range = DateRange::week(wednesday);

        assert_eq!(range.start, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(range.end, NaiveDate::from_ymd_opt(2024, 1, 7).unwrap());
        assert_eq!(range.start.weekday(), Weekday::Mon);
        assert_eq!(range.end.weekday(), Weekday::Sun);
    }

    #[test]
    fn test_week_factory_from_sunday() {
        // 2024-01-07 is a Sunday
        let sunday = NaiveDate::from_ymd_opt(2024, 1, 7).unwrap();
        let range = DateRange::week(sunday);

        assert_eq!(range.start, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(range.end, NaiveDate::from_ymd_opt(2024, 1, 7).unwrap());
        assert_eq!(range.start.weekday(), Weekday::Mon);
        assert_eq!(range.end.weekday(), Weekday::Sun);
    }

    #[test]
    fn test_week_factory_crossing_month_boundary() {
        // 2024-01-31 is a Wednesday, week spans Jan 29 - Feb 4
        let date = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let range = DateRange::week(date);

        assert_eq!(range.start, NaiveDate::from_ymd_opt(2024, 1, 29).unwrap());
        assert_eq!(range.end, NaiveDate::from_ymd_opt(2024, 2, 4).unwrap());
        assert_eq!(range.start.weekday(), Weekday::Mon);
        assert_eq!(range.end.weekday(), Weekday::Sun);
    }
}
