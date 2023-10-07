#![allow(dead_code)]

/// ## Description
/// Macro that extracts any primitive, plus Strings values from a Row element
///
/// ### Parameters
/// - row: row element in FromRow implementation
/// - field: column name in database table
/// - table: table name in database
/// - datatype: the type of the data to be converted from the Row element
#[macro_export]
macro_rules! row_to_data {
    ($row:ident, $field:expr, $table:expr, $datatype:ty) => {
        match $row.get::<$datatype, _>($field) {
            Some(value) => value,
            None => {
                panic!("Unknown column {} in table {}", $field.to_string(), $table);
            }
        }
    }
}


/// ## Description
/// Macro that extracts the DateTime value in NaiveDateTime format from a Row element
///
/// ### Parameters
/// - row: row element in FromRow implementation
/// - field: column name in database table
/// - table: table name in database
#[macro_export]
macro_rules! row_to_naive_datetime {
    ($row:ident, $field:expr, $table:expr) => {
        if let Some(string) = $row.get::<String, _>($field) {
            if let Ok(date) = chrono::NaiveDateTime::parse_from_str(string.as_str(), $crate::database::DATETIME_FORMAT) {
                date
            } else {
                panic!("Datetime incorrectly formatted in database for table {} and column {}", $table, $field)
            }
        } else {
            panic!("Unknown column {} in table {}", $field.to_string(), $table);
        }
    };
}

/// ## Description
/// Macro that extracts an Enum value from a Row element
///
/// ## Warning
/// The Enum must implement the From<String> trait so that the macro can extract the value from
/// the Row element and match it with the String in the From implementation
///
/// ### Parameters
/// - row: row element in FromRow implementation
/// - field: column name in database table
/// - table: table name in database
/// - datatype: the type of the data to be converted from the Row element
#[macro_export]
macro_rules! row_to_enum {
    ($row:ident, $field:expr, $table:expr, $datatype:ty) => {
        match $row.get::<String, _>($field) {
            Some(value) => {
                Level::from(value)
            },
            None => {
                panic!("Unknown column {} in table {}", $field.to_string(), $table);
            }
        }
    }
}
