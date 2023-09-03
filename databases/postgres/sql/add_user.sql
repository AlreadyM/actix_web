INSERT INTO register.users(username, pwd, email, phone)
VALUES ($1, $2, $3, $4)
RETURNING $table_fields;
