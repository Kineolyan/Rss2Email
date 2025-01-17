use crate::email::email_provider::get_email_provider;
use crate::email::email_provider::EmailProvider;
use dotenv::dotenv;
use env_logger::Env;
use log::{error, info};
use rss2email::{download_blogs, map_to_html, time_func};

mod email;

fn core_main() -> Result<(), String> {
  env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

  let _env = dotenv().ok().ok_or("Failed to load .env file")?;
  let days_default = 7;

  let days = match std::env::var("DAYS") {
    Ok(txt) => {
      if let Ok(n) = txt.parse::<i64>() {
        n
      } else {
        error!("Days variable is set to \"{}\" which is not a number.", txt);
        return Err(format!(
          "Days variable is set to \"{}\" which is not a number.",
          txt
        ));
      }
    }
    Err(_) => days_default,
  };

  info!("Days set to {:?}", days);

  let blogs = time_func(|| download_blogs(days), "download_blogs");

  let posts_amt = blogs.iter().flat_map(|x| &x.posts).count();
  info!(
    "Downloaded {} blogs with {} posts total.",
    blogs.len(),
    posts_amt
  );

  let html = map_to_html(&blogs);

  if cfg!(debug_assertions) {
    info!("{}", html);
  } else {
    // Only load email related variables if ran on release
    let api_key = std::env::var("API_KEY").expect("API_KEY must be set.");
    let address = std::env::var("EMAIL_ADDRESS").expect("EMAIL_ADDRESS must be set.");

    get_email_provider().send_email(&address, &api_key, &html);
  }

  Ok(())
}

#[cfg(not(feature = "aws-lambda"))]
fn main() -> Result<(), String> {
  core_main()
}

#[cfg(feature = "aws-lambda")]
fn main() -> Result<(), aws_lambda::LambdaErr> {
  aws_lambda::lambda_wrapper()
}

#[cfg(feature = "aws-lambda")]
mod aws_lambda {
  use crate::core_main;
  use lambda_runtime::{run, service_fn, Error, LambdaEvent};
  use serde::Deserialize;
  pub type LambdaErr = Error;

  #[derive(Deserialize)]
  struct Request {}

  #[allow(clippy::unused_async)]
  async fn function_handler(_event: LambdaEvent<Request>) -> Result<(), Error> {
    // Extract some useful information from the request
    let _res = core_main();
    Ok(())
  }

  #[tokio::main]
  pub async fn lambda_wrapper() -> Result<(), Error> {
    tracing_subscriber::fmt()
      .with_max_level(tracing::Level::INFO)
      // disable printing the name of the module in every log line.
      .with_target(false)
      // disabling time is handy because CloudWatch will add the ingestion time.
      .without_time()
      .init();

    run(service_fn(function_handler)).await
  }
}
