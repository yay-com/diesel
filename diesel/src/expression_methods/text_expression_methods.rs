use crate::dsl;
use crate::expression::grouped::Grouped;
use crate::expression::operators::{Concat, Like, NotLike};
use crate::expression::{AsExpression, Expression};
use crate::sql_types::{Nullable, SqlType, Text};

/// Methods present on text expressions
pub trait TextExpressionMethods: Expression + Sized {
    /// Concatenates two strings using the `||` operator.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #         hair_color -> Nullable<Text>,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     use diesel::insert_into;
    /// #
    /// #     let mut connection = connection_no_data();
    /// #     connection.execute("CREATE TABLE users (
    /// #         id INTEGER PRIMARY KEY,
    /// #         name VARCHAR(255) NOT NULL,
    /// #         hair_color VARCHAR(255)
    /// #     )").unwrap();
    /// #
    /// #     insert_into(users)
    /// #         .values(&vec![
    /// #             (id.eq(1), name.eq("Sean"), hair_color.eq(Some("Green"))),
    /// #             (id.eq(2), name.eq("Tess"), hair_color.eq(None)),
    /// #         ])
    /// #         .execute(&mut connection)
    /// #         .unwrap();
    /// #
    /// let names = users.select(name.concat(" the Greatest")).load(&mut connection);
    /// let expected_names = vec![
    ///     "Sean the Greatest".to_string(),
    ///     "Tess the Greatest".to_string(),
    /// ];
    /// assert_eq!(Ok(expected_names), names);
    ///
    /// // If the value is nullable, the output will be nullable
    /// let names = users.select(hair_color.concat("ish")).load(&mut connection);
    /// let expected_names = vec![
    ///     Some("Greenish".to_string()),
    ///     None,
    /// ];
    /// assert_eq!(Ok(expected_names), names);
    /// # }
    /// ```
    fn concat<T>(self, other: T) -> dsl::Concat<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(Concat::new(self, other.as_expression()))
    }

    /// Returns a SQL `LIKE` expression
    ///
    /// This method is case insensitive for SQLite and MySQL.
    /// On PostgreSQL, `LIKE` is case sensitive. You may use
    /// [`ilike()`](../expression_methods/trait.PgTextExpressionMethods.html#method.ilike)
    /// for case insensitive comparison on PostgreSQL.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let mut connection = establish_connection();
    /// #
    /// let starts_with_s = users
    ///     .select(name)
    ///     .filter(name.like("S%"))
    ///     .load::<String>(&mut connection)?;
    /// assert_eq!(vec!["Sean"], starts_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    fn like<T>(self, other: T) -> dsl::Like<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(Like::new(self, other.as_expression()))
    }

    /// Returns a SQL `NOT LIKE` expression
    ///
    /// This method is case insensitive for SQLite and MySQL.
    /// On PostgreSQL `NOT LIKE` is case sensitive. You may use
    /// [`not_ilike()`](../expression_methods/trait.PgTextExpressionMethods.html#method.not_ilike)
    /// for case insensitive comparison on PostgreSQL.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let mut connection = establish_connection();
    /// #
    /// let doesnt_start_with_s = users
    ///     .select(name)
    ///     .filter(name.not_like("S%"))
    ///     .load::<String>(&mut connection)?;
    /// assert_eq!(vec!["Tess"], doesnt_start_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    fn not_like<T>(self, other: T) -> dsl::NotLike<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(NotLike::new(self, other.as_expression()))
    }
}

#[doc(hidden)]
/// Marker trait used to implement `TextExpressionMethods` on the appropriate
/// types. Once coherence takes associated types into account, we can remove
/// this trait.
pub trait TextOrNullableText {}

impl TextOrNullableText for Text {}
impl TextOrNullableText for Nullable<Text> {}

impl<T> TextExpressionMethods for T
where
    T: Expression,
    T::SqlType: TextOrNullableText,
{
}
