use axum::extract::{Query, State};
use axum::response::{Html, Redirect};
use axum::Form;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, SubsecRound, Utc};
use chrono_tz::Europe::Oslo;
use minijinja::context;

use crate::models::{
    get_activities_from, get_activity, get_all_types, insert_activity, update_activity, Activity,
};
use crate::AppState;

pub async fn get_index(State(state): State<AppState>) -> Html<String> {
    let year = Utc::now().year();
    let started = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(year, 1, 1).unwrap(),
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    );
    let activities = get_activities_from(&state.pool, started).await.unwrap();

    state.render("index.html", context!(activities => activities))
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct AddFormStruct {
    id: Option<i64>,
}

pub async fn get_add(
    Query(q): Query<AddFormStruct>,
    State(state): State<AppState>,
) -> Html<String> {
    let activity_types = get_all_types(&state.pool).await.unwrap();

    let activity = if let Some(id) = q.id {
        get_activity(&state.pool, id).await.unwrap().unwrap()
    } else {
        let date = Utc::now()
            .with_timezone(&Oslo)
            .naive_local()
            .trunc_subsecs(0);

        Activity {
            id: None,
            date,
            duration_hours: None,
            activity_type: activity_types[0].type_.clone(),
            score: None,
            description: "".to_owned(),
        }
    };

    state.render(
        "edit.html",
        context!(activity => activity, activity_types => activity_types),
    )
}

pub async fn post_edit(State(state): State<AppState>, Form(activity): Form<Activity>) -> Redirect {
    dbg!(&activity);
    match activity.id {
        None => insert_activity(&state.pool, activity).await.unwrap(),
        Some(_) => update_activity(&state.pool, activity).await.unwrap(),
    }

    Redirect::to("/")
}
