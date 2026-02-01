use sea_orm::DbErr;
use snafu::Snafu;

pub trait ErrorName {
    fn name(&self) -> String;
}

#[derive(Snafu, Debug)]
#[snafu(visibility(pub))]
pub enum DataError {
    #[snafu(display("Error connecting to database: {source}"))]
    DatabaseConnectionError { source: DbErr },

    #[snafu(display("Error performing migration: {source}"))]
    MigrationError { source: DbErr },

    #[snafu(display("Error incrementing command counter: {source}"))]
    IncrementCommandCounterError { source: DbErr },

    #[snafu(display("Error logging command call: {source}"))]
    LogCommandCallError { source: DbErr },

    #[snafu(display("Error finding allowed user: {error}"))]
    FindAllowedUserError { error: DbErr },

    #[snafu(display("Error finding allowed command roles: {error}"))]
    FindCommandRolesAllowedError { error: DbErr },

    #[snafu(display("Error finding allowed category roles: {source}"))]
    FindCategoryRolesAllowedDatabaseError { source: DbErr },

    #[snafu(display("Database error while creating new command role restriction: {source}"))]
    NewCommandRoleRestrictionDatabaseError { source: DbErr },

    #[snafu(display("A duplicate entry is found while creating new command role restriction"))]
    NewCommandRoleRestrictionDuplicate,

    #[snafu(display("Database error while creating new category role restriction: {source}"))]
    NewCategoryRoleRestrictionDatabaseError { source: DbErr },

    #[snafu(display("A duplicate entry is found while creating new category role restriction"))]
    NewCategoryRoleRestrictionDuplicate,

    #[snafu(display("A duplicate entry is found while creating new allowed user"))]
    NewCommandAllowedUserDuplicate,

    #[snafu(display("Database error while creating new command allowed user: {source}"))]
    NewCommandAllowedUserDatabaseError { source: DbErr },

    #[snafu(display("Database error while finding all allowed users: {source}"))]
    FindAllAllowedUserDatabaseError { source: DbErr },

    #[snafu(display("Database error while finding command call log: {source}"))]
    Find5CommandCallLogDatabaseError { source: DbErr },

    #[snafu(display("Database error while finding user all time command stats: {source}"))]
    FindUserAllTimeCommandStatsDatabaseError { source: DbErr },

    #[snafu(display("Database error while getting cookies: {source}"))]
    GetLatestCookiesDatabaseError { source: DbErr },

    #[snafu(display("Database error while adding cookies: {source}"))]
    AddNewCookieDatabaseError { source: DbErr },

    #[snafu(display("Database error while getting user command stats: {source}"))]
    FindSingleUsersSingleCommandCallError { source: DbErr },

    #[snafu(display("Database error in operation {operation}: {source}"))]
    DatabaseError { operation: String, source: DbErr },

    #[snafu(display("Duplicate item {object} found."))]
    DuplicateEntry { object: String },

    #[snafu(display(
        "This sound is already present in the database for the user {user_id}. OP: {sound_name}"
    ))]
    DuplicateSoundError {
        sound_name: String,
        user_id: poise::serenity_prelude::UserId,
    },

    #[snafu(display("Not found in database. Input error? : {err}"))]
    NotFound { err: String },

    #[snafu(transparent)]
    BincodeDecodeError { source: bincode::error::DecodeError },

    #[snafu(transparent)]
    BincodeEncodeError { source: bincode::error::EncodeError },

    #[snafu(display("User {user_id} is not in the dashboard allowlist"))]
    NotAllowlisted { user_id: i64 },

    #[snafu(display("Failed to hash token: {message}"))]
    TokenHashError { message: String },
}

impl ErrorName for DataError {
    fn name(&self) -> String {
        let name = match self {
            DataError::DatabaseConnectionError { .. } => "database_connection_error",
            DataError::MigrationError { .. } => "migration_error",
            DataError::IncrementCommandCounterError { .. } => "increment_command_counter_error",
            DataError::LogCommandCallError { .. } => "log_command_call_error",
            DataError::FindAllowedUserError { .. } => "find_user_allowed_error",
            DataError::FindCommandRolesAllowedError { .. } => "find_command_roles_allowed_error",
            DataError::FindCategoryRolesAllowedDatabaseError { .. } => {
                "find_category_roles_allowed_database_error"
            }
            DataError::NewCommandRoleRestrictionDatabaseError { .. } => {
                "new_command_role_restriction_database_error"
            }
            DataError::NewCommandRoleRestrictionDuplicate => {
                "new_command_role_restriction_duplicate"
            }
            DataError::NewCategoryRoleRestrictionDatabaseError { .. } => {
                "new_category_role_restriction_database_error"
            }
            DataError::NewCategoryRoleRestrictionDuplicate => {
                "new_category_role_restriction_duplicate"
            }
            DataError::NewCommandAllowedUserDuplicate => "new_command_allowed_user_duplicate",
            DataError::NewCommandAllowedUserDatabaseError { .. } => {
                "new_command_allowed_user_database_error"
            }
            DataError::FindAllAllowedUserDatabaseError { .. } => {
                "find_all_allowed_user_database_error"
            }
            DataError::Find5CommandCallLogDatabaseError { .. } => {
                "find_5_command_call_log_database_error"
            }
            DataError::FindUserAllTimeCommandStatsDatabaseError { .. } => {
                "find_user_all_time_command_stats_database_error"
            }
            DataError::GetLatestCookiesDatabaseError { .. } => "get_latest_cookies_database_error",
            DataError::AddNewCookieDatabaseError { .. } => "add_new_cookie_database_error",
            DataError::FindSingleUsersSingleCommandCallError { .. } => {
                "find_single_user_single_all_time_command_stats"
            }
            DataError::DatabaseError { operation, .. } => operation,
            DataError::DuplicateSoundError { .. } => "duplicate_sound_error",
            DataError::DuplicateEntry { .. } => "duplicate_entry",
            DataError::NotFound { .. } => "not_found",
            DataError::BincodeDecodeError { .. } => "bincode_decode_error",
            DataError::BincodeEncodeError { .. } => "bincode_encode_error",
            DataError::NotAllowlisted { .. } => "not_allowlisted",
            DataError::TokenHashError { .. } => "token_hash_error",
        };
        format!("data::{name}")
    }
}
