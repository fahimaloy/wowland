mod compositor;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "wowland")]
#[command(version = "0.1.0")]
#[command(about = "A Wayland compositor", long_about = None)]
struct Args {
    #[arg(short, long, help = "Path to config file")]
    config: Option<String>,

    #[arg(long, help = "Print default config and exit")]
    print_default_config: bool,
}

fn main() {
    let args = Args::parse();

    if args.print_default_config {
        println!("{}", compositor::config::DEFAULT_CONFIG);
        return;
    }

    if let Err(e) = compositor::run(args.config.as_deref()) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
