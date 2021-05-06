use super::schema::*;
use crate::schema_dsl::*;
use diesel::*;

#[test]
fn selecting_basic_data() {
    use crate::schema::users::dsl::*;

    let mut connection = connection();
    connection
        .execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let expected_data = vec![
        ("Sean".to_string(), None::<String>),
        ("Tess".to_string(), None::<String>),
    ];
    let actual_data: Vec<_> = users
        .select((name, hair_color))
        .load(&mut connection)
        .unwrap();
    assert_eq!(expected_data, actual_data);
}

#[test]
fn selecting_a_struct() {
    use crate::schema::users::dsl::*;
    let mut connection = connection();
    connection
        .execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let expected_users = vec![NewUser::new("Sean", None), NewUser::new("Tess", None)];
    let actual_users: Vec<_> = users
        .select((name, hair_color))
        .load(&mut connection)
        .unwrap();
    assert_eq!(expected_users, actual_users);
}

#[test]
fn with_safe_select() {
    use crate::schema::users::dsl::*;

    let mut connection = connection();
    connection
        .execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let select_name = users.select(name);
    let names: Vec<String> = select_name.load(&mut connection).unwrap();

    assert_eq!(vec!["Sean".to_string(), "Tess".to_string()], names);
}

#[test]
fn with_select_sql() {
    use diesel::dsl::sql;

    let mut connection = connection();
    connection
        .execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let select_count = users::table.select(sql::<sql_types::BigInt>("COUNT(*)"));

    assert_eq!(Ok(2), select_count.clone().first(&mut connection));

    connection
        .execute("INSERT INTO users (name) VALUES ('Jim')")
        .unwrap();

    assert_eq!(Ok(3), select_count.clone().first(&mut connection));
}

#[test]
fn selecting_nullable_followed_by_non_null() {
    use crate::schema::users::dsl::*;

    let mut connection = connection();
    connection
        .execute("INSERT INTO users (name) VALUES ('Sean')")
        .unwrap();

    let source = users.select((hair_color, name));
    let expected_data = vec![(None::<String>, "Sean".to_string())];
    let data: Vec<_> = source.load(&mut connection).unwrap();

    assert_eq!(expected_data, data);
}

#[test]
fn selecting_expression_with_bind_param() {
    use crate::schema::users::dsl::*;

    let mut connection = connection();
    connection
        .execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let source = users.select(name.eq("Sean".to_string()));
    let expected_data = vec![true, false];
    let actual_data = source.load::<bool>(&mut connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

table! {
    select {
        id -> Integer,
        join -> Integer,
    }
}

#[test]
#[cfg(not(feature = "mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn selecting_columns_and_tables_with_reserved_names() {
    use self::select::dsl::*;

    let mut connection = connection();
    create_table(
        "select",
        (
            integer("id").primary_key().auto_increment(),
            integer("join").not_null(),
        ),
    )
    .execute(&mut connection)
    .unwrap();
    connection
        .execute("INSERT INTO \"select\" (\"join\") VALUES (1), (2), (3)")
        .unwrap();

    let expected_data = vec![(1, 1), (2, 2), (3, 3)];
    let actual_data: Vec<(i32, i32)> = select.load(&mut connection).unwrap();
    assert_eq!(expected_data, actual_data);

    let expected_data = vec![1, 2, 3];
    let actual_data: Vec<i32> = select.select(join).load(&mut connection).unwrap();
    assert_eq!(expected_data, actual_data);
}

#[test]
#[cfg(not(feature = "mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn selecting_columns_with_different_definition_order() {
    let mut connection = connection();
    drop_table_cascade(&mut connection, "users");
    create_table(
        "users",
        (
            integer("id").primary_key().auto_increment(),
            string("hair_color"),
            string("name").not_null(),
        ),
    )
    .execute(&mut connection)
    .unwrap();
    let expected_user = User::with_hair_color(1, "Sean", "black");
    insert_into(users::table)
        .values(&NewUser::new("Sean", Some("black")))
        .execute(&mut connection)
        .unwrap();
    let user_from_select = users::table.first(&mut connection);

    assert_eq!(Ok(&expected_user), user_from_select.as_ref());
}

#[test]
fn selection_using_subselect() {
    use crate::schema::posts::dsl::*;

    let mut connection = connection_with_sean_and_tess_in_users_table();
    let ids: Vec<i32> = users::table
        .select(users::id)
        .load(&mut connection)
        .unwrap();
    let query = format!(
        "INSERT INTO posts (user_id, title) VALUES ({}, 'Hello'), ({}, 'World')",
        ids[0], ids[1]
    );
    connection.execute(&query).unwrap();

    let users = users::table
        .filter(users::name.eq("Sean"))
        .select(users::id);
    let data: Vec<String> = posts
        .select(title)
        .filter(user_id.eq_any(users))
        .load(&mut connection)
        .unwrap();

    assert_eq!(vec!["Hello".to_string()], data);
}

table! {
    users_select_for_update_modifieres {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}

table! {
    users_select_for_update {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}

table! {
    users_select_for_no_key_update {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}

table! {
    users_fk_for_no_key_update {
        id -> Integer,
        users_fk -> Integer,
    }
}

#[cfg(not(feature = "sqlite"))]
#[test]
fn select_for_update_locks_selected_rows() {
    use self::users_select_for_update::dsl::*;
    use diesel::connection::SimpleConnection;
    use std::mem::drop;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    let mut conn_1 = connection_without_transaction();
    conn_1
        .execute("DROP TABLE IF EXISTS users_select_for_update")
        .unwrap();
    create_table(
        "users_select_for_update",
        (
            integer("id").primary_key().auto_increment(),
            string("name").not_null(),
            string("hair_color"),
        ),
    )
    .execute(&mut conn_1)
    .unwrap();
    conn_1
        .batch_execute(
            "
            -- MySQL locks the whole table without this index
            CREATE UNIQUE INDEX users_select_for_update_name ON users_select_for_update (name);
            INSERT INTO users_select_for_update (name) VALUES ('Sean'), ('Tess');
        ",
        )
        .unwrap();
    conn_1.begin_test_transaction().unwrap();

    let _sean = users_select_for_update
        .for_update()
        .filter(name.eq("Sean"))
        .first::<User>(&mut conn_1)
        .unwrap();

    let (send, recv) = mpsc::channel();
    let send2 = send.clone();

    let _blocked_thread = thread::spawn(move || {
        let mut conn_2 = connection();
        update(users_select_for_update.filter(name.eq("Sean")))
            .set(name.eq("Jim"))
            .execute(&mut conn_2)
            .unwrap();
        send.send("Sean").unwrap();
    });

    let _unblocked_thread = thread::spawn(move || {
        let mut conn_3 = connection();
        update(users_select_for_update.filter(name.eq("Tess")))
            .set(name.eq("Bob"))
            .execute(&mut conn_3)
            .unwrap();
        send2.send("Tess").unwrap();
    });

    let timeout = Duration::from_secs(1);
    let next_selected_name = recv.recv_timeout(timeout).unwrap();
    // conn_3 will always complete before conn_2, as conn_2 is blocked
    assert_eq!("Tess", next_selected_name);
    let next_selected_name = recv.recv_timeout(timeout);
    // conn_2 should still be blocked
    assert!(next_selected_name.is_err());
    drop(conn_1);
    let next_selected_name = recv.recv_timeout(timeout).unwrap();
    // Dropping conn_1 unblocks conn_2
    assert_eq!("Sean", next_selected_name);
}

#[cfg(feature = "postgres")]
#[test]
fn select_for_update_modifiers() {
    use self::users_select_for_update_modifieres::dsl::*;

    // We need to actually commit some data for the
    // test
    let mut conn_1 = connection_without_transaction();
    let mut conn_2 = connection();
    let mut conn_3 = connection();

    // Recreate the table
    conn_1
        .execute("DROP TABLE IF EXISTS users_select_for_update_modifieres")
        .unwrap();
    create_table(
        "users_select_for_update_modifieres",
        (
            integer("id").primary_key().auto_increment(),
            string("name").not_null(),
            string("hair_color"),
        ),
    )
    .execute(&mut conn_1)
    .unwrap();

    // Add some test data
    conn_1
        .execute(
            "
            INSERT INTO users_select_for_update_modifieres (name)
            VALUES ('Sean'), ('Tess')
            ",
        )
        .unwrap();

    // Now both connections have begun a transaction
    conn_1.begin_test_transaction().unwrap();

    // Lock the "Sean" row
    let _sean = users_select_for_update_modifieres
        .order(name)
        .for_update()
        .first::<User>(&mut conn_1)
        .unwrap();

    // Try to access the "Sean" row with `NOWAIT`
    conn_2.execute("SET STATEMENT_TIMEOUT TO 1000").unwrap();
    let result = users_select_for_update_modifieres
        .order(name)
        .for_update()
        .no_wait()
        .first::<User>(&mut conn_2);

    // Make sure we errored in the correct way (without timing out)
    assert!(result.is_err());
    if !format!("{:?}", result).contains("could not obtain lock on row") {
        panic!("{:?}", result);
    }

    // Try to access the "Sean" row with `SKIP LOCKED`
    let tess = users_select_for_update_modifieres
        .order(name)
        .for_update()
        .skip_locked()
        .first::<User>(&mut conn_3)
        .unwrap();

    // Make sure got back "Tess"
    assert_eq!(tess.name, "Tess");
}

#[cfg(feature = "postgres")]
#[test]
fn select_for_no_key_update_modifiers() {
    use self::users_fk_for_no_key_update::dsl::*;
    use self::users_select_for_no_key_update::dsl::*;

    // We need to actually commit some data for the
    // test
    let mut conn_1 = connection_without_transaction();
    let mut conn_2 = connection();
    let mut conn_3 = connection();
    let mut conn_4 = connection();

    // Recreate the table
    conn_1
        .execute("DROP TABLE IF EXISTS users_fk_for_no_key_update")
        .unwrap();
    conn_1
        .execute("DROP TABLE IF EXISTS users_select_for_no_key_update")
        .unwrap();

    create_table(
        "users_select_for_no_key_update",
        (
            integer("id").primary_key().auto_increment(),
            string("name").not_null(),
            string("hair_color"),
        ),
    )
    .execute(&mut conn_1)
    .unwrap();

    create_table(
        "users_fk_for_no_key_update",
        (
            integer("id").primary_key().auto_increment(),
            integer("users_fk").not_null(),
        ),
    )
    .execute(&mut conn_1)
    .unwrap();

    // Add a foreign key
    conn_1
        .execute(
            "ALTER TABLE users_fk_for_no_key_update ADD CONSTRAINT users_fk \
             FOREIGN KEY (users_fk) REFERENCES users_select_for_no_key_update(id)",
        )
        .unwrap();

    // Add some test data
    conn_1
        .execute(
            "INSERT INTO users_select_for_no_key_update (name) VALUES ('Sean'), ('Tess'), ('Will')",
        )
        .unwrap();

    conn_1.begin_test_transaction().unwrap();

    // Lock the "Sean" row, except the key
    let _sean = users_select_for_no_key_update
        .order(name)
        .for_no_key_update()
        .first::<User>(&mut conn_1)
        .unwrap();

    // Try to add an object referencing the "Sean" row
    conn_2
        .execute(
            "INSERT INTO users_fk_for_no_key_update (users_fk) \
             SELECT id FROM users_select_for_no_key_update where name='Sean'",
        )
        .unwrap();

    // Check that it was successfully added
    let expected_data = vec![(1, 1)];
    let actual_data: Vec<(i32, i32)> = users_fk_for_no_key_update.load(&mut conn_2).unwrap();
    assert_eq!(expected_data, actual_data);

    // Try to access the "Sean" row with `for no key update` and `SKIP LOCKED`
    let tess = users_select_for_no_key_update
        .order(name)
        .for_no_key_update()
        .skip_locked()
        .first::<User>(&mut conn_3)
        .unwrap();

    // Make sure got back "Tess"
    assert_eq!(tess.name, "Tess");

    // Lock the "Will" row completely
    let will = users_select_for_no_key_update
        .order(name)
        .for_update()
        .skip_locked()
        .first::<User>(&mut conn_4)
        .unwrap();

    assert_eq!(will.name, "Will");

    conn_2.execute("SET STATEMENT_TIMEOUT TO 1000").unwrap();
    let result = conn_2.execute(
        "INSERT INTO users_fk_for_no_key_update (users_fk) \
         SELECT id FROM users_select_for_no_key_update where name='Will'",
    );

    // Times out instead of inserting row
    assert!(result.is_err());
    if !format!("{:?}", result).contains("canceling statement due to statement timeout") {
        panic!("{:?}", result);
    }
}

#[test]
fn select_can_be_called_on_query_that_is_valid_subselect_but_invalid_query() {
    let mut connection = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &mut connection);
    let tess = find_user_by_name("Tess", &mut connection);
    insert_into(posts::table)
        .values(&vec![
            tess.new_post("Tess", None),
            sean.new_post("Hi", None),
        ])
        .execute(&mut connection)
        .unwrap();

    let invalid_query_but_valid_subselect = posts::table
        .filter(posts::title.eq(users::name))
        .select(posts::user_id);
    let users_with_post_using_name_as_title = users::table
        .filter(users::id.eq_any(invalid_query_but_valid_subselect))
        .load(&mut connection);

    assert_eq!(Ok(vec![tess]), users_with_post_using_name_as_title);
}

#[test]
fn selecting_multiple_aggregate_expressions_without_group_by() {
    use self::users::dsl::*;
    use diesel::dsl::{count_star, max};

    let mut connection = connection_with_sean_and_tess_in_users_table();
    let (count, max_name) = users
        .select((count_star(), max(name)))
        .get_result::<(i64, _)>(&mut connection)
        .unwrap();

    assert_eq!(2, count);
    assert_eq!(Some(String::from("Tess")), max_name);
}
