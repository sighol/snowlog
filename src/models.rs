use anyhow::{self, bail, Context};
use chrono::{NaiveDate, TimeZone};
use chrono_tz::Europe::Oslo;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Serialize, FromRow)]
pub struct ActivityRow {
    pub id: i64,
    pub date_epoch_seconds: i64,
    pub duration_hours: Option<f64>,
    pub activity_type: String,
    pub score: Option<f64>,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Activity {
    pub id: Option<i64>,
    pub date: NaiveDate,
    pub duration_hours: Option<f64>,
    pub activity_type: String,
    pub score: Option<f64>,
    pub description: String,
}

pub fn to_epoch_seconds(date: &NaiveDate) -> anyhow::Result<i64> {
    Ok(date
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(Oslo)
        .earliest()
        .context("Time is not valid")?
        .timestamp())
}

impl From<ActivityRow> for Activity {
    fn from(value: ActivityRow) -> Self {
        let date = Oslo
            .timestamp_opt(value.date_epoch_seconds, 0)
            .unwrap()
            .naive_local()
            .date();

        Activity {
            id: Some(value.id),
            date,
            duration_hours: value.duration_hours,
            activity_type: value.activity_type,
            score: value.score,
            description: value.description,
        }
    }
}

#[derive(Debug, Serialize, FromRow)]
pub struct ActivityType {
    pub id: i64,
    pub type_: String,
}

pub async fn get_activities_from(
    con: &SqlitePool,
    from: NaiveDate,
) -> anyhow::Result<Vec<Activity>> {
    let timestamp = from
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(Oslo)
        .earliest()
        .unwrap()
        .timestamp();

    let response = sqlx::query_as!(
        ActivityRow,
        "select 
            sa.id,
            sa.date_epoch_seconds,
            sa.duration_hours,
            sat.type as activity_type,
            sa.description,
            sa.score
            from snowboard_activities as sa
            join snowboard_activity_types as sat on sat.id = sa.type
            where date_epoch_seconds >= ?
            order by date_epoch_seconds asc",
        timestamp,
    )
    .fetch_all(con)
    .await?;

    Ok(response.into_iter().map(|x| x.into()).collect())
}

pub async fn get_all_types(con: &SqlitePool) -> anyhow::Result<Vec<ActivityType>> {
    let results = sqlx::query_as!(
        ActivityType,
        "select id, type as type_ from snowboard_activity_types",
    )
    .fetch_all(con)
    .await?;
    Ok(results)
}

pub async fn insert_activity(con: &SqlitePool, activity: Activity) -> anyhow::Result<()> {
    let types = get_all_types(con).await?;
    let Some(type_) = types.iter().find(|x| x.type_ == activity.activity_type) else {
        bail!("Unknown activity type");
    };
    let type_id = type_.id;

    let date = to_epoch_seconds(&activity.date)?;

    sqlx::query!(
        r"
            insert into snowboard_activities (
                date_epoch_seconds,
                duration_hours,
                type,
                description,
                score
            ) VALUES (?, ?, ?, ?, ?)
        ",
        date,
        activity.duration_hours,
        type_id,
        activity.description,
        activity.score,
    )
    .execute(con)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    async fn setup() -> SqlitePool {
        let db_path = "sqlite::memory:";
        let pool = SqlitePoolOptions::new().connect(&db_path).await.unwrap();

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        pool
    }

    #[tokio::test]
    async fn insert_and_retrieve() {
        let pool = setup().await;
        let activities = get_activities_from(&pool, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
            .await
            .unwrap();
        assert_eq!(0, activities.len());

        insert_activity(
            &pool,
            Activity {
                id: None,
                date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                duration_hours: Some(3.14),
                activity_type: "Skis".into(),
                score: Some(0.8),
                description: "This was fun".into(),
            },
        )
        .await
        .unwrap();

        let activities = get_activities_from(&pool, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
            .await
            .unwrap();
        assert_eq!(1, activities.len());

        let activity = activities.into_iter().next().unwrap();
        assert_eq!(Some(1), activity.id);
        assert_eq!(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(), activity.date);
        assert_eq!(Some(3.14), activity.duration_hours);
        assert_eq!("Skis".to_owned(), activity.activity_type);
        assert_eq!(Some(0.8), activity.score);
        assert_eq!("This was fun".to_owned(), activity.description);
    }
}
