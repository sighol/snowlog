alter table snowboard_activities rename to snowboard_activities_old;

create table snowboard_activities(
    id integer primary key autoincrement,
    date text not null,
    duration_hours real,
    type integer not null references snowboard_activity_types(id),
    description text not null,
    score real
);

insert into snowboard_activities
	select
		id,
		strftime('%Y-%m-%dT%H:%M:%S', datetime(date_epoch_seconds, 'unixepoch', 'localtime')) as date,
		duration_hours,
		type,
		description,
		score
	from snowboard_activities_old;
	
drop table snowboard_activities_old;
