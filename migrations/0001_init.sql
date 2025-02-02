create table snowboard_activity_types(
    id integer primary key autoincrement,
    type text not null
);

insert into snowboard_activity_types (type) VALUES
    ("Snowboarding"),
    ("Snowboarding hiking"),
    ("Skis"),
    ("Skis hiking");

create table snowboard_activities(
    id integer primary key autoincrement,
    date_epoch_seconds integer not null,
    duration_hours real,
    type integer not null references snowboard_activity_types(id),
    description text not null,
    score real
);
