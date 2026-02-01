//! Constants for testing.
#![allow(dead_code, reason = "These may be used at any times")]

use poise::serenity_prelude as serenity;

pub const GUILD_ID_1: u64 = 594465820151644180;
pub const ROLE_ID_1: serenity::RoleId = serenity::RoleId::new(888730479770091561);
pub const ROLE_ID_2: serenity::RoleId = serenity::RoleId::new(888730479770091560);
pub const USER_ID_1: serenity::UserId = serenity::UserId::new(594465820151644181);
pub const USER_ID_2: serenity::UserId = serenity::UserId::new(594465820151644182);
pub const USER_ID_3: serenity::UserId = serenity::UserId::new(594465820151644183);
pub const COMMAND_1: &str = "test_command";
pub const COMMAND_CATEGORY_1: &str = "test_category";
