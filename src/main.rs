use lazy_static::lazy_static;
use teloxide::prelude::*;
use teloxide::types::ParseMode::MarkdownV2;
use teloxide::types::{ReplyMarkup, ReplyKeyboardMarkup, ReplyKeyboardRemove, KeyboardButton};
use std::collections::HashMap;
use std::thread;
use std::time::{Duration, SystemTime};
use std::sync::Mutex;

/// An enum to store where the user is
#[repr(u8)]
enum PasswordPhase {
    /// User must choose length
    Length,
    /// User must choose whether to include lowercase characters or not
    Lowercase,
    /// User must choose whether to include uppercase characters or not
    Uppercase,
    /// User must choose whether to include numbers or not
    Numbers,
    /// User must choose whether to include symbols or not
    Symbols,
}

/// A struct to save
struct User {
    /// Where the user is
    page: PasswordPhase,
    /// Length of the password
    length: u8,
    lowercase: bool,
    uppercase: bool,
    numbers: bool,
    /// From when the cleaner thread must delete this entry
    expiry_date: u64,
}

lazy_static! {
    static ref USERS_MAP: Mutex<HashMap<i32, User>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };
}

const CLEANUP_INTERVAL: u64 = 10 * 60;
const MAX_LIFE_TIME: u64 = 5 * 60;
const VERSION: &'static str = env!("CARGO_PKG_VERSION"); // https://stackoverflow.com/a/27841363/4213397

#[tokio::main]
async fn main() {
    // This thread cleans up the users
    thread::spawn(|| {
        loop {
            thread::sleep(Duration::from_secs(CLEANUP_INTERVAL));
            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
            USERS_MAP.lock().unwrap().retain(|_, value| { // https://stackoverflow.com/a/45724688/4213397
                value.expiry_date < now
            });
        }
    });
    run().await;
}

async fn run() {
    // Is sent to user at /help
    const HELP_TEXT: &str = "This bot helps you generate random passwords. I DO NOT STORE ANYTHING ON MY SERVER, you can read the source code. Also this bot uses crypto/rand to generate secure randoms for password.
To quickly generate password use /generate , it generates a 16 letter password with combination of letters and numbers
If you want to create a customizable password, use /password";
    // is sent to user at /start
    const START_TEXT: &str = "Hello and welcome to password generator bot!
To quickly generate a password send run /generate
To customize your password use /password";
    // is sent to user at /password
    const PASSWORD_HELP1: &str = "Select the length of your password(1-255)";
    // is sent to user at /help
    const PASSWORD_HELP2: &str = "Do you want your password contain lowercase characters? (a,b,c...)";
    const PASSWORD_HELP3: &str = "Do you want your password contain uppercase characters? (A,B,C...)";
    const PASSWORD_HELP4: &str = "Do you want your password contain numbers characters? (1,2,3...)";
    const PASSWORD_HELP5: &str = "Do you want your password contain special characters? (!,#,%...)";

    // create bot from environmental variables
    teloxide::enable_logging!();
    let bot = Bot::from_env();

    teloxide::repl(bot, |message| async move {
        match message.update.text() { // get messages with text
            Some(txt) => {
                match txt {
                    "/start" => {
                        message.answer(START_TEXT).send().await?;
                    }
                    "/about" => {
                        message.answer(format!("Password Generator Bot v{}\nBy Hirbod Behnam\nSource: https://github.com/HirbodBehnam/Password-Generator-Bot-Rust", VERSION)).send().await?;
                    }
                    "/generate" => {
                        let pass = generate_password(16, true, true, true, false);
                        let mut msg = message.answer(pass);
                        msg.parse_mode = Option::from(MarkdownV2);
                        msg.send().await?;
                    }
                    "/password" => {
                        // put the entry in database
                        {
                            let id = message.update.from().unwrap().id;
                            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
                            let status = User {
                                page: PasswordPhase::Length,
                                length: 0,
                                lowercase: false,
                                uppercase: false,
                                numbers: false,
                                expiry_date: now + MAX_LIFE_TIME,
                            };
                            USERS_MAP.lock().unwrap().insert(id, status);
                        }
                        message.answer(PASSWORD_HELP1).send().await?;
                    }
                    "/help" => {
                        message.answer(HELP_TEXT).send().await?;
                    }
                    _ => {
                        // should the message be formatted as markdown?
                        let mut markdown = false;
                        // should we send the Yes/No keyboard to user
                        let mut keyboard = false;
                        // get user info
                        let result = {
                            let mut m = USERS_MAP.lock().unwrap();
                            match m.get_mut(&message.update.from().unwrap().id) {
                                Some(user) => {
                                    match user.page {
                                        PasswordPhase::Length => {
                                            // try parse the text
                                            match txt.parse::<u8>() {
                                                Ok(n) => {
                                                    if n == 0 {
                                                        "Please do not enter 0!".to_string()
                                                    } else {
                                                        user.length = n;
                                                        user.page = PasswordPhase::Lowercase;
                                                        keyboard = true;
                                                        PASSWORD_HELP2.to_string() // return help
                                                    }
                                                }
                                                Err(e) => {
                                                    e.to_string()
                                                }
                                            }
                                        }
                                        PasswordPhase::Lowercase => {
                                            keyboard = true;
                                            user.lowercase = txt == "Yes";
                                            user.page = PasswordPhase::Uppercase;
                                            PASSWORD_HELP3.to_string()
                                        }
                                        PasswordPhase::Uppercase => {
                                            keyboard = true;
                                            user.uppercase = txt == "Yes";
                                            user.page = PasswordPhase::Numbers;
                                            PASSWORD_HELP4.to_string()
                                        }
                                        PasswordPhase::Numbers => {
                                            keyboard = true;
                                            user.numbers = txt == "Yes";
                                            user.page = PasswordPhase::Symbols;
                                            PASSWORD_HELP5.to_string()
                                        }
                                        PasswordPhase::Symbols => {
                                            let result =
                                                generate_password(user.length,
                                                                  user.lowercase, user.uppercase,
                                                                  user.numbers, txt == "Yes");
                                            m.remove(&message.update.from().unwrap().id); // remove this user from database
                                            markdown = true;
                                            result
                                        }
                                    }
                                }
                                _ => {
                                    HELP_TEXT.to_string()
                                }
                            }
                        };
                        let mut msg = message.answer(result); // create the message
                        if markdown { // include markdown
                            msg.parse_mode = Option::from(MarkdownV2);
                        }
                        if keyboard { // add keyboard
                            let mut keys =
                                ReplyKeyboardMarkup::new(vec![vec![KeyboardButton::new("Yes"), KeyboardButton::new("No")]]);
                            keys.resize_keyboard = Option::from(true);
                            msg.reply_markup =
                                Option::from(ReplyMarkup::ReplyKeyboardMarkup(keys));
                        } else {
                            msg.reply_markup = Option::from(ReplyMarkup::ReplyKeyboardRemove(ReplyKeyboardRemove::new()));
                        }
                        msg.send().await?; // send
                    }
                }
            }
            None => { message.answer(HELP_TEXT).send().await?; }
        };
        ResponseResult::<()>::Ok(())
    })
        .await;
}

/// Generates a password with given length and arguments
fn generate_password(length: u8, lowercase: bool, uppercase: bool, number: bool, symbol: bool) -> String {
    // check valid arguments
    if !lowercase && !uppercase && !number && !symbol {
        return "Please at least choose one of the character types".to_string();
    }
    // create the master string to choose from it
    let master = {
        let mut m = String::new();
        if lowercase {
            m.push_str("qwertyuiopasdfghjklzxcvbnm");
        }
        if uppercase {
            m.push_str("QWERTYUIOPASDFGHJKLZXCVBNM");
        }
        if number {
            m.push_str("1234567890");
        }
        if symbol {
            m.push_str("!@#$%^&*()_+=-[]{};:'\"\\|,./~");
        }
        m
    }.into_bytes();
    // get the max
    let max = master.len() as u8;
    let and_value = {
        let num: u8;
        if max < 16 {
            num = 15;
        } else if max < 32 {
            num = 31;
        } else if max < 64 {
            num = 63;
        } else {
            num = 127;
        };
        num
    };
    // create a random string
    let mut random_number = [0u8; 1];
    let mut password = String::from("`");
    password.reserve(length as usize + 2); // reserve to speed things up
    for _ in 0..length {
        // create random numbers until the number becomes less than max
        loop {
            getrandom::getrandom(&mut random_number).expect("Cannot initialize rng.");
            random_number[0] &= and_value; // increase the chance of random in range
            if max > random_number[0] {
                break;
            }
        }
        // append number to password
        password.push(master[(random_number[0] as usize)] as char);
    }
    password.push('`');
    password // return password
}