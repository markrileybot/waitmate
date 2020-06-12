#[macro_use] extern crate actix_web;

use std::env;

use clap::Clap;

use crate::waitmate::app::App;

mod waitmate;

#[derive(Clap)]
#[clap(version = "1.0", author = "mark@markriley.net")]
struct Opts {
    #[clap(short, long)]
    config: Option<String>,

    #[clap(subcommand)]
    sub_command: SubCommand
}

#[derive(Clap)]
enum SubCommand {
    #[clap(version = "1.0", author = "mark@markriley.net")]
    Server(ServerOpts),

    #[clap(version = "1.0", author = "mark@markriley.net")]
    Client(ClientOpts),

    #[clap(version = "1.0", author = "mark@markriley.net")]
    Dump,
}

#[derive(Clap)]
struct ServerOpts {
    #[clap(short, long, default_value = "tcp://*:12345")]
    listen: String
}

#[derive(Clap)]
struct ClientOpts {
    #[clap(short, long, default_value = "tcp://127.0.0.1:12345")]
    connect: String
}

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let opts: Opts = Opts::parse();

    match opts.sub_command {
        SubCommand::Client(a) => App::new(true).run_client(a.connect.as_str()),
        SubCommand::Server(a) => App::new(false).run_server(a.listen.as_str()),
        SubCommand::Dump => App::new(false).dump(),
    }
}