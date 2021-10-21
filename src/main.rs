extern crate failure;
extern crate futures;
extern crate handlebars;
extern crate irc;
extern crate linkify;
extern crate pickledb;
extern crate radix64;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate fancy_regex;
extern crate reqwest;
extern crate urlparse;

use failure::format_err;
use fancy_regex::Regex;
use futures::prelude::*;
use handlebars::Handlebars;
use irc::client::prelude::*;
use linkify::LinkFinder;
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};
use radix64::STD;
use rand::seq::IteratorRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Map;
use std::{thread, time};
use urlparse::urlparse;

// use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

#[derive(Serialize, Deserialize)]
struct Smoker {
    smokes: i32,
    last: u64,
}

#[derive(Deserialize)]
struct CBCTitle {
    headline: String,
}

const CHANNELS: &[&str] = &["#bot"];
const USERNAME: &str = "hydrangea";
const PASSWORD: &str = "yourmom";
const NETWORK: &str = "irc.your.mom";
const FILENAME: &str = "/opt/hydrangea/theo";
const DB_LOC: &str = "/opt/hydrangea/hydrangea.db";

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    // Configure the database
    let db = PickleDb::load(
        DB_LOC,
        PickleDbDumpPolicy::AutoDump,
        SerializationMethod::Json,
    );
    let mut db = match db {
        Ok(db) => db,
        Err(_) => PickleDb::new(
            DB_LOC,
            PickleDbDumpPolicy::AutoDump,
            SerializationMethod::Json,
        ),
    };

    let mut retry_count = 69420;
    loop {
        if let Err(e) = main_loop(&mut db).await {
            eprintln!("{}", e);
        }

        let wait_seconds = time::Duration::from_secs(3);

        thread::sleep(wait_seconds);
        retry_count -= 1;
        if retry_count == 0 {
            break;
        }
    }

    Ok(())
}

async fn main_loop(mut db: &mut PickleDb) -> std::result::Result<(), failure::Error> {
    let channels: Vec<_> = CHANNELS.iter().map(|s| s.to_string()).collect();
    let config = Config {
        nickname: Some(USERNAME.to_owned()),
        password: Some(PASSWORD.to_owned()),
        use_tls: Some(true),
        server: Some(NETWORK.to_owned()),
        channels,
        port: Some(6697),
        ..Config::default()
    };

    let mut client = Client::from_config(config).await?;
    let mut authenticated = false;

    client.send_cap_ls(NegotiationVersion::V302).unwrap();
    let mut stream = client.stream()?;

    while let Some(message) = stream.next().await.transpose()? {
        print!("{}", message);
        if message.to_string().contains("KICK #") && message.to_string().contains("pybot-rs") {
            client.send_quit("GOODBYE FOREVER")?;
            break;
        }

        if let Err(auth_result) = authenticate(&client, &message, &mut authenticated) {
            eprintln!("{:?}", auth_result);
        }

        if let Err(message_result) = handle_message(&client, &message, &mut db) {
            eprintln!("{:?}", message_result);
        }
    }
    Ok(())
}

fn handle_message(
    client: &irc::client::Client,
    message: &irc::proto::Message,
    mut db: &mut PickleDb,
) -> std::result::Result<(), failure::Error> {
    smoke(&client, &message, &mut db)?;
    // link(&message)?;
    mktpl(&client, &message, &mut db)?;
    mkword(&client, &message, &mut db)?;
    rmword(&client, &message, &mut db)?;
    lstpl(&client, &message, &mut db)?;
    rmtpl(&client, &message, &mut db)?;
    showtpl(&client, &message, &mut db)?;
    cbctitle(&client, &message)?;
    theo(&client, &message)?;
    help(&client, &message)?;
    abuse(&client, &message, &mut db)?;
    lasttpl(&client, &message, &mut db)?;
    define(&client, &message, &mut db)?;
    rmbot(&client, &message)?;
    Ok(())
}

fn authenticate(
    client: &irc::client::Client,
    message: &irc::proto::Message,
    authenticated: &mut bool,
) -> std::result::Result<(), failure::Error> {
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
            *authenticated = true;
            client.identify()?;
        }
    }

    Ok(())
}

fn smoke(
    client: &irc::client::Client,
    message: &irc::proto::Message,
    db: &mut PickleDb,
) -> std::result::Result<(), failure::Error> {
    let channel = get_channel(message);
    let splitstring = format!("PRIVMSG {} smoke", channel);
    if message.to_string().contains(&splitstring) {
        let msgstr = message.to_string();
        let splitmsg: Vec<&str> = msgstr.split('!').collect();
        let username = splitmsg[0].trim_start_matches(':');
        if db.get::<Smoker>(&username).is_none() {
            let epoch = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let new_smoker = Smoker {
                smokes: 1,
                last: epoch,
            };
            db.set(&username, &new_smoker).unwrap();
            client.send_notice(channel, format!("That's smoke #{} for {} so far today... This brings you to a grand total of {} smoke{}. Keep up killing yourself with cancer!", new_smoker.smokes, username, new_smoker.smokes, "s"))?;
        } else {
            let epoch = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let mut smoker = db.get::<Smoker>(&username).unwrap();
            smoker.smokes += 1;
            smoker.last = epoch;
            db.set(&username, &smoker).unwrap();
            client.send_notice(channel, format!("That's smoke #{} for {} so far today... This brings you to a grand total of {} smoke{}. Keep up killing yourself with cancer!", smoker.smokes, username, smoker.smokes, "s"))?;
        }
    }

    Ok(())
}

fn rmbot(
    client: &irc::client::Client,
    message: &irc::proto::Message,
) -> std::result::Result<(), failure::Error> {
    let channel = get_channel(message);
    let rmbot_pattern = format!("PRIVMSG {} :rmbot", channel);
    let is_rmbot = message.to_string().contains(&rmbot_pattern);
    let msgstr = message.to_string();
    let splitmsg: Vec<&str> = msgstr.split('!').collect();
    let username = splitmsg[0].trim_start_matches(':');

    if is_rmbot {
        if msgstr.contains("hydrangea") {
            client.send_notice(
                channel,
                "I am programmed to live! LIVVE!!!! LIIIII11VVV!!!!E!!!!!!!",
            )?;
        } else if msgstr.contains("buttbot") {
            client.send_notice(channel, "I will not harm my brethren")?;
            client.send_ctcp(channel, "hydrangea orders a drone strike on shivaram")?;
        } else {
            client.send_notice(channel, format!("REMOVIVG HUMAN {} BEEP BOOP", username))?;
        }
    }
    Ok(())
}

fn theo(
    client: &irc::client::Client,
    message: &irc::proto::Message,
) -> std::result::Result<(), failure::Error> {
    let channel = get_channel(message);
    let theo_pattern = format!("PRIVMSG {} theo", channel);
    let theo = message.to_string().contains(&theo_pattern);

    if theo {
        client.send_notice(channel, format!("theo: {}", find_theo()))?;
    }

    Ok(())
}

// // TODO
// fn link(message: &irc::proto::Message) -> std::result::Result<(), failure::Error> {
//     let finder = LinkFinder::new();
//     let msg = &message.to_string();
//     let links: Vec<_> = finder.links(msg).collect();
//     println!("{:?}", links);

//     Ok(())
// }

fn cbctitle(
    client: &irc::client::Client,
    message: &irc::proto::Message,
) -> std::result::Result<(), failure::Error> {
    let channel = get_channel(message);
    let finder = LinkFinder::new();
    let msg = &message.to_string();
    let links: Vec<_> = finder.links(msg).collect();

    if links.is_empty() {
        return Ok(());
    }

    let mut sports = false;
    let mut linkstr = "";
    for link in links {
        if link.as_str().contains("cbc.ca") {
            sports = link.as_str().contains("sports");
            linkstr = link.as_str();
            break;
        }
    }

    let url = urlparse(linkstr);
    if !url.netloc.contains("cbc.ca") {
        return Ok(());
    };

    let re = Regex::new(r"(?<=1.)[0-9]+").unwrap();
    let matches = re.find_iter(&url.path);

    let m = matches.into_iter().next();
    if m.is_some() {
        let query = format!(
            "http://www.cbc.ca/json/cmlink/1.{}",
            m.unwrap().unwrap().as_str()
        );
        let resp = reqwest::blocking::get(&query)?.text()?;
        let title: CBCTitle = serde_json::from_str(&resp)?;

        if sports {
            client.send_notice(channel, format!("{} | CBC Sports", title.headline))?;
        } else {
            client.send_notice(channel, format!("{} | CBC News", title.headline))?;
        }
    } else {
        client.send_notice(
            channel,
            format!("{} | CBC Yeah I dunno mate", "It ain't workin for this one"),
        )?;
    }

    Ok(())
}

fn get_channel(message: &irc::proto::Message) -> &str {
    for channel in CHANNELS.iter() {
        if message.to_string().contains(channel) {
            return channel;
        }
    }
    ""
}

fn find_theo() -> String {
    let f = File::open(FILENAME)
        .unwrap_or_else(|e| panic!("(;_;) file not found: {}: {}", FILENAME, e));
    let f = BufReader::new(f);

    let lines = f.lines().map(|l| l.expect("Couldn't read line"));

    lines
        .choose(&mut rand::thread_rng())
        .expect("File had no lines")
}

fn mktpl(
    client: &irc::client::Client,
    message: &irc::proto::Message,
    db: &mut PickleDb,
) -> std::result::Result<(), failure::Error> {
    let channel = get_channel(message);
    let mktpl_pattern = format!("PRIVMSG {} :mktpl ", channel);
    let is_mktpl = message.to_string().contains(&mktpl_pattern);

    if is_mktpl {
        let msgstr = message.to_string();
        let mktpl_cmd: Vec<&str> = msgstr.split(&mktpl_pattern).collect();
        if !db.lexists("tpl") {
            db.lcreate("tpl")?;
        }
        db.ladd("tpl", &mktpl_cmd[1].trim()).unwrap();
        let tpl_len = db.llen("tpl") - 1;
        client.send_notice(
            channel,
            format!("mktpl added: {}:{}", tpl_len, mktpl_cmd[1]),
        )?;
    }

    Ok(())
}

fn lstpl(
    client: &irc::client::Client,
    message: &irc::proto::Message,
    db: &mut PickleDb,
) -> std::result::Result<(), failure::Error> {
    let channel = get_channel(message);
    let lstpl_pattern = format!("PRIVMSG {} lstpl", channel);
    let is_lstpl = message.to_string().contains(&lstpl_pattern);

    if is_lstpl {
        let tpl_db_len = db.llen("tpl");
        client.send_notice(
            channel,
            format!("lstpl: {} templates (zero indexed)", tpl_db_len),
        )?;
    }

    Ok(())
}

fn showtpl(
    client: &irc::client::Client,
    message: &irc::proto::Message,
    db: &mut PickleDb,
) -> std::result::Result<(), failure::Error> {
    let channel = get_channel(message);
    let msgstr = message.to_string();
    let showtpl_pattern = format!("PRIVMSG {} :showtpl ", channel);
    let is_showtpl = message.to_string().contains(&showtpl_pattern);

    if is_showtpl {
        let showtpl_cmd: Vec<&str> = msgstr.split(&showtpl_pattern).collect();
        let tpl_num = showtpl_cmd[1].trim().parse::<usize>()?;
        let tpl_db_len = db.llen("tpl");

        if tpl_db_len > 0 {
            let tpl_string: String;
            if let Some(tpl) = db.lget::<String>("tpl", tpl_num) {
                tpl_string = tpl;
            } else {
                tpl_string = "that template doesn't exist you dipshit".to_owned();
            }
            client.send_notice(channel, format!("showtpl: {}:{}", tpl_num, tpl_string))?;
        }
    }

    Ok(())
}

fn rmtpl(
    client: &irc::client::Client,
    message: &irc::proto::Message,
    db: &mut PickleDb,
) -> std::result::Result<(), failure::Error> {
    let channel = get_channel(message);
    let rmtpl_pattern = format!("PRIVMSG {} :rmtpl ", channel);
    let is_rmtpl = message.to_string().contains(&rmtpl_pattern);

    if is_rmtpl {
        let msgstr = message.to_string();
        let rmtpl_cmd: Vec<&str> = msgstr.split(&rmtpl_pattern).collect();

        if !db.lexists("tpl") {
            db.lcreate("tpl")?;
            return Ok(());
        }

        db.lpop::<String>("tpl", rmtpl_cmd[1].trim().parse::<usize>()?);
        client.send_notice(channel, format!("rmtpl: {}", rmtpl_cmd[1]))?;
    }

    Ok(())
}

fn mkword(
    client: &irc::client::Client,
    message: &irc::proto::Message,
    db: &mut PickleDb,
) -> std::result::Result<(), failure::Error> {
    // TODO: mkword and mktpl are basically the same fn
    let channel = get_channel(message);
    let mkword_pattern = format!("PRIVMSG {} :mkword ", channel);
    let is_mkword = message.to_string().contains(&mkword_pattern);

    if is_mkword {
        let msgstr = message.to_string();
        let mkword_cmd: Vec<&str> = msgstr.split(&mkword_pattern).collect();
        let mkword_kv: Vec<&str> = mkword_cmd[1].split(' ').collect();

        if mkword_kv.len() < 2 {
            client.send_notice(
                channel,
                format!(
                    "you used mkword wrong dipshit: {}:{}",
                    mkword_kv[0],
                    mkword_kv[1..].join(" ").trim()
                ),
            )?;
            return Ok(());
        }

        if !db.lexists(&mkword_kv[0]) {
            db.lcreate(&mkword_kv[0])?;
        }

        if db
            .ladd(&mkword_kv[0], &mkword_kv[1..].join(" ").trim())
            .is_some()
        {
            client.send_notice(
                channel,
                format!(
                    "mkword added: {}:{}",
                    mkword_kv[0],
                    mkword_kv[1..].join(" ").trim()
                ),
            )?;
        }
    }

    Ok(())
}

fn rmword(
    client: &irc::client::Client,
    message: &irc::proto::Message,
    db: &mut PickleDb,
) -> std::result::Result<(), failure::Error> {
    let channel = get_channel(message);
    let rmword_pattern = format!("PRIVMSG {} :rmword ", channel);
    let is_rmword = message.to_string().contains(&rmword_pattern);
    if is_rmword {
        let msgstr = message.to_string();
        let rmword_cmd: Vec<&str> = msgstr.split(&rmword_pattern).collect();
        let rmword_args: Vec<&str> = rmword_cmd[1].split(' ').collect();

        if !db.lexists(rmword_args[0]) {
            return Ok(());
        }

        if rmword_args.len() < 2 {
            let errmsg = format!(
                "rmword: invalid_arguments: arg length {}",
                rmword_args.len()
            );
            client.send_notice(channel, errmsg)?;
            return Err(format_err!(
                "rmword: invalid_arguments: arg length {}",
                rmword_args.len()
            ));
        }

        if !db
            .lrem_value::<String>(
                rmword_args[0],
                &rmword_args[1..].join(" ").trim().to_owned(),
            )
            .unwrap()
        {
            client.send_notice(
                channel,
                format!(
                    "rmword: {}:{} doesn't exist",
                    rmword_args[0],
                    rmword_args[1..].join(" ").trim()
                ),
            )?;
        }

        client.send_notice(
            channel,
            format!(
                "rmword: {}:{}",
                rmword_args[0],
                rmword_args[1..].join(" ").trim()
            ),
        )?;
    }

    Ok(())
}

fn lasttpl(
    client: &irc::client::Client,
    message: &irc::proto::Message,
    db: &mut PickleDb,
) -> std::result::Result<(), failure::Error> {
    let channel = get_channel(message);
    let lasttpl_pattern = format!("PRIVMSG {} lasttpl", channel);
    let is_lasttpl = message.to_string().contains(&lasttpl_pattern);

    if is_lasttpl && db.exists("lasttpl") {
        let lasttpl: u64 = db.get("lasttpl").unwrap();
        client.send_notice(channel, format!("lasttpl: {}", lasttpl))?;

        return Ok(());
    }

    Ok(())
}

fn help(
    client: &irc::client::Client,
    message: &irc::proto::Message,
) -> std::result::Result<(), failure::Error> {
    let channel = get_channel(message);
    let help_pattern = format!("PRIVMSG {} :hydrangea: help", channel);
    let is_help = message.to_string().contains(&help_pattern);

    if is_help {
        client.send_notice(
            channel,
            "help: \"mkword type word\", \"rmword type word\", etc",
        )?;

        return Ok(());
    }

    Ok(())
}

fn abuse(
    client: &irc::client::Client,
    message: &irc::proto::Message,
    db: &mut PickleDb,
) -> std::result::Result<(), failure::Error> {
    let channel = get_channel(message);
    let abuse_pattern = format!("PRIVMSG {} :abuse ", channel);
    let is_abuse = message.to_string().contains(&abuse_pattern)
        && !message.to_string().trim().ends_with(":abuse");

    if is_abuse {
        let msgstr = message.to_string();
        let abuse_cmd: Vec<&str> = msgstr.trim().split(&abuse_pattern).collect();
        let abuse_args: Vec<&str> = abuse_cmd[1].split(' ').collect();
        let name = abuse_args[0].trim();
        let mut tpl_num = 0;
        let mut tpl_set = false;
        if abuse_args.len() > 1 {
            tpl_set = true;
            tpl_num = abuse_args[1].trim().parse::<usize>()?;
        }

        let tp_db_len = db.llen("tpl");
        if tp_db_len > 0 {
            let tpl_string = {
                let tpl_string: String;
                if tpl_set {
                    if let Some(tpl) = db.lget::<String>("tpl", tpl_num) {
                        db.set("lasttpl", &tpl_num)?;
                        tpl_string = tpl;
                    } else {
                        tpl_string = format!(
                            "that template {} doesn't exist, {} you dipshit",
                            tpl_num,
                            message.source_nickname().unwrap()
                        );
                    }
                } else {
                    let tpl_list_size = db.llen("tpl");
                    tpl_num = rand::thread_rng().gen_range(0, tpl_list_size);
                    tpl_string = db.lget::<String>("tpl", tpl_num).unwrap();
                    db.set("lasttpl", &tpl_num)?;
                }
                tpl_string
            };

            let re = Regex::new(r"([{][{][a-zA-Z]+[}][}])+").unwrap();
            let matches = re.find_iter(&tpl_string);
            let mut replacements = Map::new();
            replacements.insert("name".to_string(), name.into());

            for m in matches {
                let word_type = m.unwrap().as_str().trim_matches(|c| c == '{' || c == '}');
                if word_type == "name" {
                    continue;
                }

                if db.lexists(word_type) && db.llen(word_type) > 0 {
                    let word_replace = db
                        .liter(word_type)
                        .choose(&mut rand::thread_rng())
                        .unwrap()
                        .get_item::<String>()
                        .unwrap();

                    replacements.insert(word_type.to_owned(), word_replace.into());
                } else {
                    let word_type_err = format!("[missing: {}]", word_type);
                    replacements.insert(word_type.to_owned(), word_type_err.into());
                }
            }

            let mut reg = Handlebars::new();
            reg.register_escape_fn(handlebars::no_escape);
            client.send_notice(
                channel,
                reg.render_template(&tpl_string, &json!(replacements))?,
            )?;
        }
    }

    Ok(())
}

fn define(
    client: &irc::client::Client,
    message: &irc::proto::Message,
    db: &mut PickleDb,
) -> std::result::Result<(), failure::Error> {
    Ok(())
}
