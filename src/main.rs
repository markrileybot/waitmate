use std::env;

use getopts::Options;

use crate::waitmate::app::App;

mod waitmate;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("s", "server", "Run a server", "tcp://*:12134");
    opts.optopt("c", "client", "Run a client", "tcp://localhost:12134");
    opts.optflag("d", "dump", "dump");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }
    if matches.opt_present("d") {
        App::new().dump();
        return;
    }

    let server = matches.opt_str("s");
    let client = matches.opt_str("c");
    if server.is_some() {
        App::new().run_server(server.unwrap().as_str());
    } else if client.is_some() {
        App::new().run_client(client.unwrap().as_str());
    } else {
        App::new().run();
    }
}