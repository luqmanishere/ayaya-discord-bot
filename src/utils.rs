use std::sync::Arc;

use serenity::client::Context;
use serenity::model::channel::Message;
use serenity::Result as SerenityResult;
use songbird::Songbird;

/// Checks that a message successfully sent; if not, then logs why to stdout.
pub fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

pub async fn get_manager(ctx: &Context) -> Arc<Songbird> {
    songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
}

#[allow(unused_variables)]
#[allow(dead_code)]
pub async fn search_youtube(term: &str) {
    let ytdl_args = ["--print-json"];
    todo!()
}
