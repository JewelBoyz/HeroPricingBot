#[allow(unused_imports)]
use std::error::Error;
#[allow(unused_imports)]
use serenity::{
    Client, // The Client is the way to be able to start sending authenticated requests over the REST API
    client::Context, // The context is a general utility struct provided on event dispatches, which helps with dealing with the current “context” of the event dispatch
    framework::{ //The framework is a customizable method of separating commands.
        StandardFramework, // A utility for easily managing dispatches to commands.
        standard::{
            CommandResult, 
            macros::{
                group, // A macro for creating a grouping of commands
                command // A macro for creating a single command
            }
        }
    }, 
    model::channel::Message
};
use graphql_client::*;
#[allow(unused_imports)]
use serde_derive::{Deserialize, Serialize};

// 1. We have to make our main function asynchronous
#[tokio::main]
async fn main() {
    // Grab our bot token
    let token = "";

    let framework = StandardFramework::new().configure(|c| {
        c.prefix("!")
    }).group(&HEROPRICING_GROUP);
    // HELLOWORLD_GROUP is the output of the #[group] macro

    let mut client = Client::builder(token).framework(framework).await.expect("Could not start Discord");
    client.start().await.expect("The bot stopped");
    
}


// ***** SERENITY DISCORD STUFF ***** //

// create a struct where we'll attach our commands
// once we've added our commands we'll enter them into the commands sub-macro
#[group]
#[commands(hpb)]
struct HeroPricing;

// Basic structure of a command
#[command]
async fn hpb(ctx: &Context, msg: &Message) -> CommandResult{

    // TODO: Need to better handle errors
    let user_input = msg.content[4..].trim().parse::<i64>().ok().expect("Enter a valid ID");

    // The shape of the variables expected by the query.
    let variables = dfk_query::Variables {
        id: user_input
    };

    // Produce a GraphQL query struct that can be JSON serialized and sent to a GraphQL API
    let request_body = DFKQuery::build_query(variables);

    // Create a new client to send the request body to the graphql server
    let client = reqwest::Client::new();
    // Send the request and (hopefully) get a response back
    let res = client.post("https://graph4.defikingdoms.com/subgraphs/name/defikingdoms/apiv5").json(&request_body).send().await?;
    // Parse the response into json
    let response_body: Response<dfk_query::ResponseData> = res.json().await?;

    let response_data = response_body.data.expect("missing response data");

    // First, lets pull stuff from the response_data
    let hero_obj = response_data.hero;

    match hero_obj {
        Some(hero_obj) => {
            let hero: Hero = Hero {
                id: hero_obj.id,
                summons: hero_obj.summons,
                rarity: hero_obj.rarity, // TODO: map rarity to actual string name in game
                profession: hero_obj.profession,
                level: hero_obj.level,
            };
            msg.reply(ctx, format!("{:?}", hero)).await?;
        },
        None => {
            msg.reply(ctx, format!("Could not find hero")).await?;
        }
    }

    Ok(())

}


// ***** GRAPHQL QUERY STUFF ***** //

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.graphql",
    query_path = "src/query.graphql",
    response_derives = "Debug, Serialize, Deserialize"
)]
pub struct DFKQuery;

#[derive(Debug)]
pub struct Hero {
    id: String,
    summons: i64,
    rarity: i64,
    profession: String,
    level: i64
}

