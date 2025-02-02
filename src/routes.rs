use axum::extract::State;
use axum::response::{Html, Redirect};
use axum::Form;
use chrono::{Datelike, NaiveDate, Utc};
use chrono_tz::Europe::Oslo;
use minijinja::context;

use crate::models::{get_activities_from, get_all_types, insert_activity, Activity};
use crate::AppState;

pub async fn get_index(State(state): State<AppState>) -> Html<String> {
    let year = Utc::now().year();
    let started = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let activities = get_activities_from(&state.pool, started).await.unwrap();

    state.render("index.html", context!(activities => activities))
}

pub async fn get_add(State(state): State<AppState>) -> Html<String> {
    let date = Utc::now().with_timezone(&Oslo).naive_local().date();

    let activity_types = get_all_types(&state.pool).await.unwrap();
    let activity = Activity {
        id: None,
        date,
        duration_hours: None,
        activity_type: activity_types[0].type_.clone(),
        score: None,
        description: "".to_owned(),
    };

    state.render(
        "edit.html",
        context!(activity => activity, activity_types => activity_types),
    )
}

pub async fn post_edit(State(state): State<AppState>, Form(payload): Form<Activity>) -> Redirect {
    dbg!(&payload);
    match payload.id {
        None => insert_activity(&state.pool, payload).await.unwrap(),
        Some(_) => todo!(),
    }

    Redirect::to("/")
}
