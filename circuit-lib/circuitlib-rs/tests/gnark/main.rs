mod constants;
mod helpers;
mod prove;

use std::{
    process::{Child, Command},
    thread,
    time::Duration,
};

use circuitlib_rs::helpers::init_logger;
