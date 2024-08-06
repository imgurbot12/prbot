mod api;
mod cli;
mod message;

fn main() {
    env_logger::init();

    api::clean_old_reviews(
        "http://localhost:3000/api/v1/repos/Itatem/php-log/pulls/1".to_owned(),
        "itatem-service-account-1".to_owned(),
        "fd9c3d0d6138ff0da442426223a38257653e1d47".to_owned(),
    )
    .unwrap();

    // let cli = Cli::parse();
    // match cli.command {
    //     Command::Prepare(_args) => {}
    //     Command::Commit => {}
    // }
}
