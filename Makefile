watch:
	watchexec -e rs -rc -- cargo run

migrate:
	touch db.sqlite
	sqlx migrate run --database-url "sqlite://db.sqlite"
