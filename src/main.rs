use actix_web::{
    http::Method, server, App, Json, Result, middleware::Logger};
use serde::Deserialize;
use env_logger;
use sustechcourse::{Course, UserAgent};

#[derive(Deserialize)]
struct CourseQueryInfo {
    username: String,
    password: String,
    terms: Vec<(u16, u8)>,
}

fn query_course(info: Json<CourseQueryInfo>) -> Result<Json<Vec<Course>>> {
    let mut agent = UserAgent::new().login(&info.username, &info.password)?;

    let mut courses = vec![];
    for (year, term) in &info.terms {
        courses.append(&mut agent.query_course(*year, *term)?);
    }
    Ok(Json(courses))
}

fn main() {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();
    server::new(|| App::new()
            .middleware(Logger::new("%a"))
            .resource("/", |r| r.method(Method::POST).with(query_course))
        )
        .bind("127.0.0.1:8000")
        .expect("Can not bind to port 8000")
        .run();
}
