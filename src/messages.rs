use dialoguer::Confirm;
use dialoguer::theme::ColorfulTheme;

use crate::ResType;
use crate::error::ErKind;

/* ----------------------------------------------------- */
/* [ COLOR STRING ] ------------------------------------ */

macro_rules! colorfmt {
    ($color:ident, $arg:expr) => {
        console::Style::new().apply_to(format_args!("{}", $arg)).$color().to_string()
    };
    ($color:ident, $($arg:tt)*) => {
        console::Style::new().apply_to(format_args!($($arg)*)).$color().to_string()
    };
}
pub(crate) use colorfmt;

macro_rules! green {
    ($($arg:tt)*) => { super::messages::colorfmt!(green, $($arg)*) };
}
pub(crate) use green;

macro_rules! red {
    ($($arg:tt)*) => { super::messages::colorfmt!(red, $($arg)*) };
}
pub(crate) use red;

macro_rules! blue {
    ($($arg:tt)*) => { super::messages::colorfmt!(blue, $($arg)*) };
}
pub(crate) use blue;

macro_rules! yellow {
    ($($arg:tt)*) => { super::messages::colorfmt!(yellow, $($arg)*) };
}
pub(crate) use yellow;

macro_rules! cyan {
    ($($arg:tt)*) => { super::messages::colorfmt!(cyan, $($arg)*) };
}
pub(crate) use cyan;

/* ----------------------------------------------------- */
/* [ TERMINAL PRINT ] ---------------------------------- */

/// Log status print to stdout
macro_rules! prtstd {
    ($status:expr, $arg:expr) => {
        println!("[{}] -- {}", $status, $arg)
    };
    ($status:expr, $($args:tt)*) => {{
        print!("[{}] -- ", $status); println!($($args)*);
    }};
}

/// Log status print to stderr
macro_rules! prterr {
    ($status:expr, $arg:expr) => {
        eprintln!("[{}] -- {}", $status, $arg)
    };
    ($status:expr, $($args:tt)*) => {{
        eprint!("[{}] -- ", $status); eprintln!($($args)*);
    }};
}

pub(crate) use prterr;
pub(crate) use prtstd;

/* ----------------------------------------------------- */
/* [ LOG PRINT ] --------------------------------------- */

macro_rules! __error__ {
    ($($args:tt)*) => {
        super::messages::prterr!(super::messages::red!(" eror "), $($args)*)
    };
}
pub(crate) use __error__ as error;

macro_rules! __warn__ {
    ($($args:tt)*) => {
        super::messages::prterr!(super::messages::yellow!(" warn "), $($args)*)
    };
}
pub(crate) use __warn__ as warn;

macro_rules! __skip__ {
    ($($args:tt)*) => {
        super::messages::prtstd!(super::messages::yellow!(" skip "), $($args)*)
    };
}
pub(crate) use __skip__ as skip;

macro_rules! __success__ {
    ($($args:tt)*) => {
        super::messages::prtstd!(super::messages::green!("  ok  "), $($args)*)
    };
}
pub(crate) use __success__ as success;

macro_rules! __info__ {
    ($($args:tt)*) => {
        super::messages::prtstd!(super::messages::cyan!(" info "), $($args)*)
    };
}
pub(crate) use __info__ as info;

/// Ask user to confirmation before continue.
/// With msg promt and default answ.
/// When not `false_continue`, this will output `Err()`
/// that can be use with `?` to exit function
pub fn ask_confirm<S>(msg: S, default_yes: bool, false_continue: bool) -> ResType<bool>
where
    S: Into<String>,
{
    let choice = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(msg)
        .default(default_yes)
        .interact()
        .unwrap_or(false);

    if choice || false_continue {
        Ok(choice)
    } else {
        Err(ErKind::UserAbort(None))
    }
}
