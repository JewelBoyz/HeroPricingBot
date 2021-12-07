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

use std::env;

// 1. We have to make our main function asynchronous
#[tokio::main]
async fn main() {
    // Grab our bot token from env
    let token = env::var("DISCORD_TOKEN").expect("token");

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

    // TODO: Need to better handle errors to send in discord
    let user_input = msg.content[4..].trim().parse::<i64>();

    // if the Result struct turns out to be an error, we'll tell the user to enter in a valid argument: a number
    if user_input.is_err() {
        msg.reply(ctx, format!("Please enter a valid hero id")).await?;
        return Ok(());
    };

    // At this point we know the user's input is i64, so we can grab the Option out of the Result, 
    //and then unwrap it, but it something wild happens we provide a default of 0
    let dfk_hero_id = user_input.ok().unwrap_or(0);

    // One more check to see if the now validated number entered is >= 0, otherwise show another error
    if dfk_hero_id < 0 {
        msg.reply(ctx, format!("Please enter a valid hero id")).await?;
        return Ok(());
    };

    // The shape of the variables expected by the query.
    let variables = initial_dfk_hero_query::Variables {
        id: dfk_hero_id
    };

    // Produce a GraphQL query struct that can be JSON serialized and sent to a GraphQL API
    let request_body = InitialDFKHeroQuery::build_query(variables);

    // Create a new client to send the request body to the graphql server
    let client = reqwest::Client::new();

    // Send the request with the request body being the variables we send in parsed from the user input, 
    //we (hopefully) get a response back and store it
    let res = client.post("https://graph4.defikingdoms.com/subgraphs/name/defikingdoms/apiv5").json(&request_body).send().await?;

    // let's test that we didn't get a nasty server error from the graphql api
    if res.status().is_server_error() {
        msg.reply(ctx, format!("Received a server error from the api")).await?;
        return Ok(());
    };

    // Parse the response into json and pop it into this custom ResponseData struct
    let response_body: Response<initial_dfk_hero_query::ResponseData> = res.json().await?;

    // 
    if response_body.data.is_none() {
        msg.reply(ctx, format!("That hero does not exist")).await?;
        return Ok(());
    };
    let response_data = response_body.data.unwrap();

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
            let result = find_comparable_heroes(hero);
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
pub struct InitialDFKHeroQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.graphql",
    query_path = "src/comparisonQuery",
    response_derives = "Debug, Serialize, Deserialize"
)]
pub struct ComparableDFKHerosQuery;


#[derive(Debug)]
pub struct Hero {
    id: String,
    summons: i64,
    rarity: i64,
    profession: String,
    level: i64
}

// ***** Custom Functionality ***** //

// Take hero response data and pipe into a new query that will:
// 1. Search for heros hero according to similar: class, rarity, profession, number of summons, generation