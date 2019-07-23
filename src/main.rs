use actix_web::{
    web, HttpServer, App, Result, middleware::Logger
};
use serde::Deserialize;
use env_logger;
use log::info;
use sustechcourse::{Course, UserAgent};

#[derive(Deserialize)]
struct CourseQueryInfo {
    username: String,
    password: String,
    terms: Option<Vec<(u16, u8)>>,
}

fn query_course(info: web::Json<CourseQueryInfo>)
        -> Result<web::Json<Vec<Course>>> {
    let mut agent = UserAgent::new().login(&info.username, &info.password)?;

    if let Some(terms) = &info.terms {
        let mut courses = vec![];
        for (year, term) in terms {
            courses.append(&mut agent.query_course(*year, *term)?);
        }
        Ok(web::Json(courses))
    } else {
        Ok(web::Json(agent.all_courses()?))
    }
}

fn main() {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let bind = std::env::var("HTTP_BIND")
        .unwrap_or("127.0.0.1:8000".to_string());
    info!("Start server on {}", bind);

    HttpServer::new( ||
        App::new()
            .wrap(Logger::default())
            .service(web::resource("/").route(web::post().to(query_course)))
    ).bind(bind)
        .expect("Can not bind to port 8000")
        .run()
        .expect("Error on running HTTP server")
}
