use anyhow::{self};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use chrono_tz::Europe::Oslo;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Serialize, FromRow)]
pub struct ActivityRow {
    pub id: i64,
    pub date: String,
    pub location: String,
    pub duration_hours: Option<f64>,
    pub r#type: String,
    pub score: Option<f64>,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Activity {
    pub id: Option<i64>,
    pub date: NaiveDateTime,
    pub location: String,
    pub duration_hours: Option<f64>,
    pub r#type: String,
    pub type_color: String,
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
            type_color: string_to_rgb(&value.r#type),
            r#type: value.r#type,
            score: value.score,
            description: value.description,
        }
    }
}

pub fn string_to_rgb(input: &str) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    let hash = hasher.finish();

    let mut r = (hash & 0xFF) as u8;
    let mut g = ((hash >> 8) & 0xFF) as u8;
    let mut b = ((hash >> 16) & 0xFF) as u8;

    // Ensure the color is not too dark by boosting brightness
    let min_brightness = 100; // Minimum value for each component
    let max_component = r.max(g).max(b);

    if max_component < min_brightness {
        let factor = min_brightness as f32 / max_component as f32;
        r = ((r as f32 * factor).min(255.0)) as u8;
        g = ((g as f32 * factor).min(255.0)) as u8;
        b = ((b as f32 * factor).min(255.0)) as u8;
    }
    format!("#{:02X}{:02X}{:02X}", r, g, b)
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
        r#"select
            sa.id,
            sa.date,
            coalesce(sa.location, '') as location,
            sa.duration_hours,
            sa.type as type,
            sa.description,
            sa.score
            from activities as sa
            where date >= ?
            order by date desc"#,
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
            coalesce(sa.location, '') as location,
            sa.duration_hours,
            sa.type,
            sa.description,
            sa.score
            from activities as sa
            where sa.id == ?",
        id,
    )
    .fetch_optional(con)
    .await?;

    Ok(response.map(|x| x.into()))
}

#[derive(Debug, Serialize, FromRow)]
pub struct ActivityType {
    pub r#type: String,
}

pub async fn get_all_types(
    con: &SqlitePool,
    start: Option<NaiveDateTime>,
    end: Option<NaiveDateTime>,
) -> anyhow::Result<Vec<String>> {
    let start = start.unwrap_or(NaiveDateTime::new(
        NaiveDate::from_yo_opt(1970, 1).unwrap(),
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    ));
    let end = end.unwrap_or(NaiveDateTime::new(
        NaiveDate::from_yo_opt(2050, 1).unwrap(),
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    ));
    let results = sqlx::query_as!(
        ActivityType,
        "select distinct type
        from activities
        where date >= ? and date <= ?
        order by 1",
        start,
        end,
    )
    .fetch_all(con)
    .await?;
    Ok(results.into_iter().map(|x| x.r#type).collect())
}

pub async fn get_all_locations(
    con: &SqlitePool,
    start: Option<NaiveDateTime>,
    end: Option<NaiveDateTime>,
) -> anyhow::Result<Vec<String>> {
    let start = start.unwrap_or(NaiveDateTime::new(
        NaiveDate::from_yo_opt(1970, 1).unwrap(),
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    ));
    let end = end.unwrap_or(NaiveDateTime::new(
        NaiveDate::from_yo_opt(2050, 1).unwrap(),
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    ));
    let results = sqlx::query_as!(
        ActivityType,
        "select distinct coalesce(location, '') as type
        from activities
        where date >= ? and date <= ? and location != null
        order by 1",
        start,
        end,
    )
    .fetch_all(con)
    .await?;
    Ok(results.into_iter().map(|x| x.r#type).collect())
}

pub async fn insert_activity(con: &SqlitePool, activity: Activity) -> anyhow::Result<()> {
    sqlx::query!(
        r"
            insert into activities (
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
        activity.r#type,
        activity.description,
        activity.score,
    )
    .execute(con)
    .await?;

    Ok(())
}

pub async fn update_activity(con: &SqlitePool, activity: Activity) -> anyhow::Result<()> {
    let id = activity.id.unwrap();
    sqlx::query!(
        r"
            update activities
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
        activity.r#type,
        activity.description,
        activity.score,
        id,
    )
    .execute(con)
    .await?;

    Ok(())
}

pub async fn delete_activity(con: &SqlitePool, id: i64) -> anyhow::Result<()> {
    sqlx::query!("delete from activities where id = ?", id)
        .execute(con)
        .await?;

    Ok(())
}

#[derive(Debug, Serialize, FromRow)]
pub struct Summary {
    pub r#type: String,
    pub days: i64,
    pub hours: f64,
}

pub async fn get_summary(
    con: &SqlitePool,
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> anyhow::Result<Vec<Summary>> {
    let response = sqlx::query_as!(
        Summary,
        r"
            select
                type,
                count(*) as days,
                coalesce(sum(duration_hours), 0.0) as hours
            from activities
            where date >= ? and date < ?
            group by type
            order by 2
        ",
        from,
        to,
    )
    .fetch_all(con)
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
                location: "Norefjell".to_owned(),
                duration_hours: Some(3.14),
                r#type: "Skis".into(),
                type_color: "#123456".to_owned(),
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
        assert_eq!("Skis".to_owned(), activity.r#type);
        assert_eq!("Norefjell".to_owned(), activity.location);
        assert_eq!(Some(0.8), activity.score);
        assert_eq!("This was fun".to_owned(), activity.description);

        update_activity(
            &pool,
            Activity {
                id: Some(activity.id.unwrap()),
                date: NaiveDateTime::from_str("2025-02-03T04:05:06").unwrap(),
                location: "Tryvann".to_owned(),
                duration_hours: Some(56.55),
                r#type: "Snowboarding".to_owned(),
                type_color: "#123456".to_owned(),
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
        assert_eq!("Snowboarding".to_owned(), activity.r#type);
        assert_eq!("Tryvann", activity.location.as_str());
        assert_eq!(Some(1.0), activity.score);
        assert_eq!("This was OK".to_owned(), activity.description);

        let start = NaiveDateTime::from_str("2025-01-01T00:00:00").unwrap();
        let stopped = NaiveDateTime::from_str("2026-01-01T00:00:00").unwrap();
        let summary = get_summary(&pool, start, stopped).await.unwrap();
        assert_eq!(1, summary[0].days);
    }
}
