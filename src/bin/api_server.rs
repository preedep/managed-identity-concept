use log::info;

#[actix_web::main]
async fn main() -> Result<(),Box<dyn std::error::Error>> {
    pretty_env_logger::init();
    info!("Starting server");

    Ok(())
}