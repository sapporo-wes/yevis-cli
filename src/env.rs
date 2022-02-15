use dotenv::dotenv;
use std::env;

pub fn yevis_dev() -> bool {
    dotenv().ok();
    match env::var("YEVIS_DEV") {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub fn default_pr_repo() -> &'static str {
    match yevis_dev() {
        true => "ddbj/yevis-workflows-dev",
        false => "ddbj/yevis-workflows",
    }
}
