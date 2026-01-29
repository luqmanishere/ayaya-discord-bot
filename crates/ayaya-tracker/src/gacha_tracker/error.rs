use std::num::ParseIntError;

use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum TrackerError {
    #[snafu(display("not enough arguments to build"))]
    WuwaRequestIncomplete,
    #[snafu(display("You are not the owner of this player id."))]
    UserGameIdMismatch,
    #[snafu(display("The player id format is invalid"))]
    WuwaPlayerIdInvalid { source: ParseIntError },

    #[snafu(display("The provided url is invalid."))]
    InvalidUrl,
    #[snafu(display("Failed to send request to Wuwa API."))]
    WuwaRequestFailed { source: reqwest::Error },
    #[snafu(display("Failed to read Wuwa API response."))]
    WuwaResponseRead { source: reqwest::Error },
    #[snafu(display("Failed to decode Wuwa API response."))]
    WuwaResponseDecode { source: serde_json::Error },
    #[snafu(display("Failed to encode Wuwa API request."))]
    WuwaRequestEncode { source: serde_json::Error },
}
