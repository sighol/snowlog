use anyhow::{self, bail};
use chrono::NaiveDateTime;
use chrono_tz::Europe::Oslo;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Serialize, FromRow)]
pub struct ActivityRow {
    pub id: i64,
    pub date: String,
    pub location: Option<String>,
    pub duration_hours: Option<f64>,
    pub activity_type: String,
    pub score: Option<f64>,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Activity {
    pub id: Option<i64>,
    pub date: NaiveDateTime,
    pub location: Option<String>,
    pub duration_hours: Option<f64>,
    pub activity_type: String,
    pub score: Option<f64>,
    pub description: String,
}

impl From<ActivityRow> for Activity {
    fn from(value: ActivityRow) -> Self {
        let date = NaiveDateTime::parse_from_str(&value.date, "%Y-%m-%d %H:%M:%S")
            .expect("Date time is not valid");

        Activity {
            id: Some(value.id),
            date,
            location: value.location,
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
    from: NaiveDateTime,
) -> anyhow::Result<Vec<Activity>> {
    let timestamp = from
        .and_local_timezone(Oslo)
        .earliest()
        .unwrap()
        .timestamp();

    let response = sqlx::query_as!(
        ActivityRow,
        "select 
            sa.id,
            sa.date,
            sa.location,
            sa.duration_hours,
            sat.type as activity_type,
            sa.description,
            sa.score
            from snowboard_activities as sa
            join snowboard_activity_types as sat on sat.id = sa.type
            where date >= ?
            order by date asc",
        timestamp,
    )
    .fetch_all(con)
    .await?;

    Ok(response.into_iter().map(|x| x.into()).collect())
}

pub async fn get_activity(con: &SqlitePool, id: i64) -> anyhow::Result<Option<Activity>> {
    let response = sqlx::query_as!(
        ActivityRow,
        "select 
            sa.id,
            sa.date,
            sa.location,
            sa.duration_hours,
            sat.type as activity_type,
            sa.description,
            sa.score
            from snowboard_activities as sa
            join snowboard_activity_types as sat on sat.id = sa.type
            where sa.id == ?",
        id,
    )
    .fetch_optional(con)
    .await?;

    Ok(response.map(|x| x.into()))
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
    assert!(activity.id.is_none());

    sqlx::query!(
        r"
            insert into snowboard_activities (
                date,
                location,
                duration_hours,
                type,
                description,
                score
            ) VALUES (?, ?, ?, ?, ?, ?)
        ",
        activity.date,
        activity.location,
        activity.duration_hours,
        type_id,
        activity.description,
        activity.score,
    )
    .execute(con)
    .await?;

    Ok(())
}

pub async fn update_activity(con: &SqlitePool, activity: Activity) -> anyhow::Result<()> {
    let types = get_all_types(con).await?;
    let Some(type_) = types.iter().find(|x| x.type_ == activity.activity_type) else {
        bail!("Unknown activity type");
    };
    let type_id = type_.id;

    let id = activity.id.unwrap();

    sqlx::query!(
        r"
            update snowboard_activities 
                set date = ?,
                    location = ?,
                    duration_hours = ?,
                    type = ?,
                    description = ?,
                    score = ?
                where id = ?
        ",
        activity.date,
        activity.location,
        activity.duration_hours,
        type_id,
        activity.description,
        activity.score,
        id,
    )
    .execute(con)
    .await?;

    Ok(())
}

#[derive(Debug, Serialize, FromRow)]
pub struct Summary {
    pub days: i64,
    pub hours: f64,
}

pub async fn get_summary(
    con: &SqlitePool,
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> anyhow::Result<Summary> {
    let response = sqlx::query_as!(
        Summary,
        r"
            select 
                count(*) as days,
                coalesce(sum(duration_hours), 0.0) as hours
            from snowboard_activities
            where date >= ? and date < ?
        ",
        from,
        to,
    )
    .fetch_one(con)
    .await?;

    Ok(response)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

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
        let activities = get_activities_from(
            &pool,
            NaiveDateTime::from_str("2025-01-01T00:00:00").unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(0, activities.len());

        insert_activity(
            &pool,
            Activity {
                id: None,
                date: NaiveDateTime::from_str("2025-01-01T00:00:00").unwrap(),
                location: Some("Norefjell".to_owned()),
                duration_hours: Some(3.14),
                activity_type: "Skis".into(),
                score: Some(0.8),
                description: "This was fun".into(),
            },
        )
        .await
        .unwrap();

        let activities = get_activities_from(
            &pool,
            NaiveDateTime::from_str("2025-01-01T00:00:00").unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(1, activities.len());

        let activity = activities.into_iter().next().unwrap();
        assert_eq!(Some(1), activity.id);
        assert_eq!(
            NaiveDateTime::from_str("2025-01-01T00:00:00").unwrap(),
            activity.date
        );
        assert_eq!(Some(3.14), activity.duration_hours);
        assert_eq!("Skis".to_owned(), activity.activity_type);
        assert_eq!(Some("Norefjell".to_owned()), activity.location);
        assert_eq!(Some(0.8), activity.score);
        assert_eq!("This was fun".to_owned(), activity.description);

        update_activity(
            &pool,
            Activity {
                id: Some(activity.id.unwrap()),
                date: NaiveDateTime::from_str("2025-02-03T04:05:06").unwrap(),
                location: Some("Tryvann".to_owned()),
                duration_hours: Some(56.55),
                activity_type: "Snowboarding".to_owned(),
                score: Some(1.0),
                description: "This was OK".into(),
            },
        )
        .await
        .unwrap();

        let activities = get_activities_from(
            &pool,
            NaiveDateTime::from_str("2025-01-01T00:00:00").unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(1, activities.len());

        let activity = get_activity(&pool, activity.id.unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(Some(1), activity.id);
        assert_eq!(
            NaiveDateTime::from_str("2025-02-03T04:05:06").unwrap(),
            activity.date
        );
        assert_eq!(Some(56.55), activity.duration_hours);
        assert_eq!("Snowboarding".to_owned(), activity.activity_type);
        assert_eq!("Tryvann", activity.location.unwrap().as_str());
        assert_eq!(Some(1.0), activity.score);
        assert_eq!("This was OK".to_owned(), activity.description);

        let start = NaiveDateTime::from_str("2025-01-01T00:00:00").unwrap();
        let stopped = NaiveDateTime::from_str("2026-01-01T00:00:00").unwrap();
        let summary = get_summary(&pool, start, stopped).await.unwrap();
        assert_eq!(1, summary.days);
    }
}
