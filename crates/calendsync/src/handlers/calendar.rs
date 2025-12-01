use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use calendsync_core::calendar::{build_day_data, get_week_dates, Calendar, CalendarEntry, DayData};
use chrono::{Datelike, Local, NaiveDate};
use uuid::Uuid;

use crate::{assets::get_asset_path, state::AppState};

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
struct CalendarTemplate<'a> {
    today: NaiveDate,
    days: Vec<DayData>,
    current_month: String,
    current_year: i32,
    calendars: Vec<Calendar>,
    default_calendar_id: Option<Uuid>,
    calendar_js: &'a str,
}

/// Handler for the calendar page (GET /calendar).
pub async fn calendar_demo(State(state): State<AppState>) -> impl IntoResponse {
    let today = Local::now().date_naive();
    let week_dates = get_week_dates(today);

    // Get all entries from state
    let entries: Vec<CalendarEntry> = state
        .entries
        .read()
        .expect("Failed to acquire read lock")
        .values()
        .cloned()
        .collect();

    // Build day data for the week
    let days = build_day_data(&week_dates, entries);

    // Get calendars for the dropdown
    let calendars: Vec<Calendar> = state
        .calendars
        .read()
        .expect("Failed to acquire read lock")
        .values()
        .cloned()
        .collect();

    let default_calendar_id = calendars.first().map(|c| c.id);

    // Get the hashed JS filename
    let calendar_js = get_asset_path("calendar.js").unwrap_or("calendar.js");

    HtmlTemplate(CalendarTemplate {
        today,
        days,
        current_month: today.format("%B").to_string(),
        current_year: today.year(),
        calendars,
        default_calendar_id,
        calendar_js,
    })
}
