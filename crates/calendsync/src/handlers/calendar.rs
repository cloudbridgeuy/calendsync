use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use calendsync_core::calendar::DayData;
use chrono::{Datelike, Local, NaiveDate};

use crate::mock_data::build_demo_calendar_data_for_date;

/// Template wrapper that converts Askama templates into HTML responses.
struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template: {err}"),
            )
                .into_response(),
        }
    }
}

/// Calendar demo page template.
#[derive(Template)]
#[template(path = "calendar.html")]
struct CalendarTemplate {
    today: NaiveDate,
    days: Vec<DayData>,
    current_month: String,
    current_year: i32,
}

/// Handler for the calendar demo (GET /calendar).
pub async fn calendar_demo() -> impl IntoResponse {
    let today = Local::now().date_naive();
    let (_, days) = build_demo_calendar_data_for_date(today);

    HtmlTemplate(CalendarTemplate {
        today,
        days,
        current_month: today.format("%B").to_string(),
        current_year: today.year(),
    })
}
