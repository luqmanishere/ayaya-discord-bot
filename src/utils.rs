use std::sync::Arc;

use serenity::Result as SerenityResult;
use serenity::client::Context;
use serenity::model::channel::Message;
use songbird::Songbird;

/// Checks that a message successfully sent; if not, then logs why to stdout.
pub fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

pub async fn get_manager(ctx: &Context) -> Arc<Songbird> {
   songbird::get(
       ctx
   ).await.expect("Songbird Voice client placed in at initialisation.")
}

pub async fn search_youtube(term: &str) {
    let ytdl_args = ["--print-json", ] ;
    todo!()
}