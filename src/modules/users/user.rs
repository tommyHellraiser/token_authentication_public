
use std::fmt::{Display, Formatter};
use chrono::NaiveDateTime;
use error_mapper::{map_to_new_error, SystemErrorCodes, TheError, TheResult};
use mysql_async::prelude::{FromRow, Queryable};
use mysql_async::{FromRowError, Row};
use serde::{Deserialize, Serialize};
use crate::{auth, database, row_to_enum, row_to_naive_datetime};
use crate::database::db_conn::get_conn;
use crate::general::types::UsersIdType;
use crate::{row_to_data};
use crate::modules::users;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct User {
    id: UsersIdType,
    #[serde(skip_serializing)]
    username: String,
    #[serde(skip_serializing)]
    hashed_pass: String,
    #[serde(skip_serializing)]
    email: String,
    #[serde(skip_serializing, skip_deserializing)]
    level: Level,
    #[serde(skip_serializing, skip_deserializing)]
    created_at: NaiveDateTime,
    #[serde(skip_serializing, skip_deserializing)]
    updated_at: NaiveDateTime
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Copy, PartialEq, PartialOrd)]
pub enum Level {
    #[default]
    View = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Super = 4
}

impl User {

    pub async fn create_user(
        username: &str,
        pass: &str,
        email: &str,
        level: &Level
    ) -> TheResult<(UsersIdType, String)> {

        let mut user = User::default();

        //  Get ID from db and add one
        user.id = User::select_last_id().await? + 1;

        //  Set the user fields
        user.username = username.to_owned();
        user.email = email.to_owned();
        user.level = *level;
        user.created_at = chrono::Utc::now().naive_utc();
        user.updated_at = user.created_at;

        //  Set the hashed pass that'll be inserted into db
        let string_to_hash = user.build_string_to_hash(pass);
        user.hashed_pass = auth::crypt::generate_hash(string_to_hash.as_str());

        //  Check username availabilty

        //  Insert the user into db
        user.insert().await?;

        //  Start a session, a logged in user gets created with an open session
        let token = auth::crypt::generate_session_token()?;
        users::users_sessions::activate_user_session(&user, &token).await?;

        //  Return the user id
        Ok((user.id, token))
    }

    pub fn create_super_user() -> Self {
        Self {
            id: 1,
            username: "super".to_string(),
            hashed_pass: "".to_string(),
            email: "super_user@yomama.com".to_string(),
            level: Level::Super,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc()
        }
    }

    pub async fn select_all() -> TheResult<Vec<User>> {

        let conn = &mut get_conn().await?;

        let user = conn.query::<User, _>(
                "SELECT * FROM users WHERE deleted_at IS NULL"
        ).await.map_err(|e| map_to_new_error!(e))?;

        if !user.is_empty() {
            Ok(user)
        } else {
            Err(
                TheError::new(
                    SystemErrorCodes::NotFound,
                    "No users found".to_string()
                )
            )
        }
    }

    pub async fn select_by_username(username: &str) -> TheResult<Option<User>> {

        let conn = &mut get_conn().await?;

        let user = conn.query_first::<User, _>(
            format!(
                "SELECT * FROM users WHERE username = '{}' AND deleted_at IS NULL",
                username
            )
        ).await.map_err(|e| map_to_new_error!(e))?;

        if let Some(user) = user {
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }
    
    pub async fn select_by_id(user_id: &UsersIdType) -> TheResult<Option<User>> {

        let conn = &mut get_conn().await?;

        let user = conn.query_first::<User, _>(
            format!(
                "SELECT * FROM users WHERE ID = {} AND deleted_at IS NULL",
                user_id
            )
        ).await.map_err(|e| map_to_new_error!(e))?;

        if let Some(user) = user {
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    async fn select_last_id() -> TheResult<UsersIdType> {

        let conn = &mut get_conn().await?;

        let id = conn.query_first::<Option<UsersIdType>, _>(
            "SELECT MAX(ID) FROM users LIMIT 1"
        ).await.map_err(|e| map_to_new_error!(e))?;

        if let Some(Some(user_id)) = id {
            return Ok(user_id)
        }

        Ok(UsersIdType::default())
    }

    pub(super) async fn insert(&self) -> TheResult<()> {

        let conn = &mut get_conn().await?;

        conn.query_drop(
            format! (
                "INSERT INTO users (ID, username, hashed_pass, email, level, created_at, updated_at) \
                    VALUES ({}, '{}', '{}', '{}', '{}', '{}', '{}')",
                self.id,
                self.username.as_str(),
                self.hashed_pass.as_str(),
                self.email.as_str(),
                self.level,
                self.created_at,
                self.updated_at
            )
        ).await.map_err(|e| map_to_new_error!(e))?;

        Ok(())
    }

    pub(super) async fn delete_account(&self) -> TheResult<()>{

        let conn = &mut get_conn().await?;

        conn.query_drop(
            format!(
                "UPDATE users SET deleted_at = '{}' WHERE ID = {}",
                chrono::Utc::now().naive_utc().format(database::DATETIME_FORMAT),
                self.id
            )
        ).await.map_err(|e| map_to_new_error!(e))?;

        Ok(())
    }

    pub(super) async fn restore_user(user_id: Option<UsersIdType>, username: Option<String>) -> TheResult<Option<bool>> {

        let conn = &mut get_conn().await?;

        let stmt;
        if let Some(user_id) = user_id {
            stmt = format!("UPDATE users SET deleted_at = NULL WHERE ID = {}", user_id);
        } else if let Some(username) = username {
            stmt = format!("UPDATE users SET deleted_at = NULL WHERE username = '{}'", username);
        } else {
            return Ok(None)
        }

        conn.query_drop(
            stmt
        ).await.map_err(|e| map_to_new_error!(e))?;

        if conn.affected_rows() > 0 {
            return Ok(Some(true))
        }

        Ok(Some(false))
    }

    pub fn build_string_to_hash(&self, pass: &str) -> String {
        //  Here's where the magic happens. Make up your own algorithm to hash. In this
        // public crate, I'll only use the password so I don't expose my own hashing method.
        // But you could convert the pass to an int,
        // multiply it by some fixed number, do any calculation you want.
        // Choose what you'd like best to protect the privacy and security of your users
        pass.to_string()
    }

    pub fn validate_hashed_password(&self, pass: &str) -> bool {
        let hashed_pass = self.build_string_to_hash(pass);
        self.hashed_pass == auth::crypt::generate_hash(hashed_pass.as_str())
    }

    pub(super) async fn change_password(&self, new_password: &str) -> TheResult<()> {

        let conn = &mut get_conn().await?;

        //  Set the hashed pass that'll be inserted into db
        let string_to_hash = self.build_string_to_hash(new_password);

        let hashed_new_pass = auth::crypt::generate_hash(string_to_hash.as_str());

        conn.query_drop(
            format!(
                "UPDATE users SET hashed_pass = '{}' WHERE ID = {}",
                hashed_new_pass,
                self.id
            )
        ).await.map_err(|e| map_to_new_error!(e))?;

        Ok(())
    }

    pub(super) fn validate_password(pass: &String) -> Vec<String> {

        let mut errors = vec![];

        //  Minimum 10 chars
        if pass.len() < 10 {
            errors.push("Password must be at least 10 chars long".to_string());
        }
        //  Maximum 25 chars
        if pass.len() > 25 {
            errors.push("Password must be at most 25 characters long".to_string());
        }
        //  At least a lowercase letter
        if !pass.chars().any(|c| c.is_lowercase()) {
            errors.push("Password must contain at least a lowercase letter".to_string());
        }
        //  At least a upper case letter
        if !pass.chars().any(|c| c.is_uppercase()) {
            errors.push("Password must contain at least a uppercase letter".to_string());
        }
        //  At least a number
        if !pass.chars().any(|c| c.is_numeric()) {
            errors.push("Password must contain at least a number".to_string());
        }
        //  At least a symbol
        if !pass.chars().any(|c| {
            for symbol in "/[!@#$%^&*()_+-=[]{};':\"\\|,.<>/?]+".chars() {
                if c.eq(&symbol) {
                    return true
                }
            }
            false
        }) {
            errors.push("Password must contain at least a uppercase letter".to_string());
        }
        
        errors
    }

    pub(super) async fn change_user_level(user_id: &UsersIdType, target_level: &Level) -> TheResult<()> {

        let conn = &mut get_conn().await?;

        conn.query_drop(
            format!(
                "UPDATE users SET level = '{}' WHERE ID = {}",
                target_level,
                user_id
            )
        ).await.map_err(|e| map_to_new_error!(e))?;

        if conn.affected_rows() > 0 {
            return Ok(())
        }

        Err(
            TheError::new(
                SystemErrorCodes::NotFound,
                format!("User with id {} not found", user_id)
            )
        )
    }

    pub fn get_id(&self) -> &UsersIdType {
        &self.id
    }

    pub fn get_username(&self) -> &str {
        self.username.as_str()
    }

    pub fn get_email(&self) -> &str {
        self.email.as_str()
    }

    pub fn get_level(&self) -> &Level {
        &self.level
    }

    pub fn set_hashed_pass(&mut self, pass: String) {
        self.hashed_pass = pass
    }
}

pub(super) async fn username_available(username: &str) -> TheResult<bool> {

    let conn = &mut get_conn().await?;

    let user = conn.query_first::<Option<UsersIdType>, _>(
        format!(
            "SELECT ID FROM users WHERE username = '{}'",
            username
        )
    ).await.map_err(|e| map_to_new_error!(e))?;

    if let Some(Some(_)) = user {
        return Ok(false)
    }

    Ok(true)
}

impl FromRow for User {
    fn from_row(row: Row) -> Self where Self: Sized {
        Self {
            id: row_to_data!(row, "ID", "users", UsersIdType),
            username: row_to_data!(row, "username", "users", String),
            hashed_pass: row_to_data!(row, "hashed_pass", "users", String),
            email: row_to_data!(row, "email", "users", String),
            level: row_to_enum!(row, "level", "users", Level),
            created_at: row_to_naive_datetime!(row, "created_at", "users"),
            updated_at: row_to_naive_datetime!(row, "updated_at", "users"),
        }
    }

    fn from_row_opt(_: Row) -> Result<Self, FromRowError> where Self: Sized {
        unimplemented!()
    }
}

impl Display for Level {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Level::View => write!(f, "View"),
            Level::Low => write!(f, "Low"),
            Level::Medium => write!(f, "Medium"),
            Level::High => write!(f, "High"),
            Level::Super => write!(f, "Super"),
        }
    }
}

impl From<String> for Level {
    fn from(value: String) -> Self {
        match value.as_str() {
            "View" => Level::View,
            "Low" => Level::Low,
            "Medium" => Level::Medium,
            "High" => Level::High,
            "Super" => Level::Super,
            _ => {
                //  TODO remove when logger is implemented
                println!("Unknown user level: {}", value);
                Level::View
            }
        }
    }
}

impl From<u8> for Level {
    fn from(value: u8) -> Self {
        match value {
            0 => Level::View,
            1 => Level::Low,
            2 => Level::Medium,
            3 => Level::High,
            4 => Level::Super,
            _ => {
                println!("Unknown user level: {}. Defaulting to Level::View", value);
                Level::View
            }
        }
    }
}

impl Level {
    pub fn one_level_below(&self) -> Self {
        match self {
            Level::Super => Level::High,
            Level::High => Level::Medium,
            Level::Medium => Level::Low,
            Level::Low => Level::View,
            Level::View => Level::View
        }
    }
}
