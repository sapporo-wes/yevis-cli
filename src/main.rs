mod args;
use structopt::StructOpt;

fn main() {
    let args = args::Args::from_args();
    println!("{:?}", args);
}
