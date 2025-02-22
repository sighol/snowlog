create table activities_temp(
    id integer primary key autoincrement,
    date text not null,
    duration_hours real,
    location text,
    type text not null,
    description text not null,
    score real
);

insert into activities_temp
    select a.id,
           a.date,
           a.duration_hours,
           a.location,
           at.type,
           a.description,
           a.score
        from activities as a
        join activity_types as at on at.id = a.type;

drop table activities;
drop table activity_types;
alter table activities_temp rename to activities;
