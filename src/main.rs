use actix_web::{
    web, HttpServer, App, middleware::Logger
};
use serde::Deserialize;
use env_logger;
use log::info;
use futures::Future;
use failure::Error;
use sustechcourse::{Course, UserAgent};

#[derive(Deserialize)]
struct CourseQueryInfo {
    username: String,
    password: String,
}

fn query_course(info: web::Json<CourseQueryInfo>)
    -> impl Future<Item = web::Json<Vec<Course>>, Error = Error> 
{
    UserAgent::new()
        .login(info.username.clone(), info.password.clone())
        .and_then(|mut agent| agent.all_courses())
        .map(|courses| web::Json(courses))
}

fn main() {
    //std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let bind = std::env::var("HTTP_BIND")
        .unwrap_or("127.0.0.1:8000".to_string());
    info!("Start server on {}", bind);

    HttpServer::new( ||
        App::new()
            .wrap(Logger::default())
            .service(web::resource("/").route(web::post().to_async(query_course)))
    ).bind(bind)
        .expect("Can not bind to port 8000")
        .run()
        .expect("Error on running HTTP server")
}
