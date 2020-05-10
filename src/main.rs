extern crate irc;
extern crate futures;
extern crate radix64;
extern crate failure;
extern crate serde;
extern crate pickledb;

use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};
use std::fmt::{self, Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use irc::client::prelude::*;
use futures::prelude::*;
use radix64::STD;

#[derive(Serialize, Deserialize)]
struct Smoker {
    smokes: i32,
    last: u64
}

const USERNAME: &'static str = "pybot-rs";
const CHANNELS: &'static [&'static str] = &["#darwin", "#rosegold"];
const NETWORK: &'static str = "irc.darwin.network";
const PASSWORD: &'static str = "";

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    let channels: Vec<_> = CHANNELS.iter().map(|s| s.to_string()).collect();
        //let channels = CHANNELS.iter().map(|s: &&str| s.to_owned()).collect::<>();
    let config = Config {
        nickname: Some(USERNAME.to_owned()),
        password: Some(PASSWORD.to_owned()),
        use_tls: Some(true),
        server: Some(NETWORK.to_owned()),
        channels: channels,
        port: Some(6697),
        ..Config::default()
    };

    // Configure the database
    let db = PickleDb::load("pybot.db", PickleDbDumpPolicy::AutoDump, SerializationMethod::Json);
    let mut db = match db {
		Ok(db) => db,
		Err(_) => PickleDb::new("pybot.db", PickleDbDumpPolicy::AutoDump, SerializationMethod::Json),
    };

    let mut client = Client::from_config(config).await?;
    let mut authenticated = false;


    client.send_cap_ls(NegotiationVersion::V302).unwrap();
    let mut stream = client.stream()?;

    while let Some(message) = stream.next().await.transpose()? {
        print!("{}", message);
        authenticate(&client, &message, &mut authenticated)?;
        abuse(&client, &message)?;
        smoke(&client, &message, &mut db)?;
    }

    Ok(())
}

fn authenticate(client: &irc::client::Client, message: &irc::proto::Message, mut authenticated: &bool) -> std::result::Result<(), failure::Error> {
    if authenticated != &true {
        // Handle CAP LS
        if message.to_string().contains("sasl=PLAIN") {
            client.send_sasl_plain().unwrap();
            print!("sasl plain available");
        }
        if message.to_string().contains("AUTHENTICATE +") {
            let toencode = format!("{}\0{}\0{}", USERNAME, USERNAME, PASSWORD);
            let encoded = STD.encode(&toencode);
            client.send_sasl(encoded).unwrap();
            print!("prompt to authenticate");
        }
        if message.to_string().contains("Authentication successful") {
            authenticated = &true;
            client.identify()?;
        }
    }

    Ok(())
}

fn abuse(client: &irc::client::Client, message: &irc::proto::Message) -> std::result::Result<(), failure::Error> {
    let channel = get_channel(message);
    let splitstring = format!("PRIVMSG {} :abuse", channel);
    let pybotstring = format!("PRIVMSG {} :pybot-rs", channel);
    let evan = message.to_string().contains(":abuse daddy") || message.to_string().contains(":abuse evan");
    let vivi = message.to_string().contains(":abuse vivi");
    let pybot = message.to_string().contains(&pybotstring);
    let msgstr = message.to_string();
    if evan {
        let splitmsg: Vec<&str> = msgstr.split(&splitstring).collect();
        let username = splitmsg[1];
        let trimmed = username.trim();
        client.send_privmsg(channel, format!("{} loves c++", trimmed)).unwrap();
    }
    if vivi {
        let splitmsg: Vec<&str> = msgstr.split(&splitstring).collect();
        let username = splitmsg[1];
        let trimmed = username.trim();
        client.send_privmsg(channel, format!("{} is planning on becoming a front end developer because he loves JavaScript so much", trimmed)).unwrap();
    }
    if pybot {
        client.send_privmsg(channel, "sux to suck, luser").unwrap();
    }
    if message.to_string().contains(":abuse") && !evan && !vivi && !pybot {
        let splitmsg: Vec<&str> = msgstr.split(&splitstring).collect();
        let username = splitmsg[1];
        let trimmed = username.trim();
        client.send_privmsg(channel, format!("{} loves JavaScript", trimmed))?;
    }

    Ok(())
}

fn smoke(client: &irc::client::Client, message: &irc::proto::Message, db: &mut PickleDb) -> std::result::Result<(), failure::Error> {
    let channel = get_channel(message);
    let splitstring = format!("PRIVMSG {} smoke", channel);
    if message.to_string().contains(&splitstring) {
        let msgstr = message.to_string();
        let splitmsg: Vec<&str> = msgstr.split("!").collect();
        let username = splitmsg[0].trim_start_matches(":");
        if db.get::<Smoker>(&username).is_none() {
            let epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let new_smoker = Smoker {smokes: 1, last: epoch };
            db.set(&username, &new_smoker).unwrap();
            client.send_privmsg(channel, format!("That's smoke #{} for {} so far today... This brings you to a grand total of {} smoke{}. Keep up killing yourself with cancer!", new_smoker.smokes, username, new_smoker.smokes, "s"))?;
        } else {
            let epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let mut smoker = db.get::<Smoker>(&username).unwrap();
            smoker.smokes = smoker.smokes+1;
            smoker.last = epoch;
            db.set(&username, &smoker).unwrap();
            client.send_privmsg(channel, format!("That's smoke #{} for {} so far today... This brings you to a grand total of {} smoke{}. Keep up killing yourself with cancer!", smoker.smokes, username, smoker.smokes, "s"))?;
        }
    }

    Ok(())
}

fn get_channel(message: &irc::proto::Message) -> &str {
    for channel in CHANNELS.iter() {
        if message.to_string().contains(channel) {
            return channel
        }
    }
    ""
}
