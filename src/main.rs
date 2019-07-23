use actix_web::{
    http::Method, server, App, Json, Result, middleware::Logger};
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

fn query_course(info: Json<CourseQueryInfo>) -> Result<Json<Vec<Course>>> {
    let mut agent = UserAgent::new().login(&info.username, &info.password)?;

    if let Some(terms) = &info.terms {
        let mut courses = vec![];
        for (year, term) in terms {
            courses.append(&mut agent.query_course(*year, *term)?);
        }
        Ok(Json(courses))
    } else {
        Ok(Json(agent.all_courses()?))
    }
}

fn main() {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let bind = std::env::var("HTTP_BIND")
        .unwrap_or("127.0.0.1:8000".to_string());
    info!("Start server on {}", bind);

    server::new(|| App::new()
            .middleware(Logger::new("%a"))
            .resource("/", |r| r.method(Method::POST).with(query_course))
        )
        .bind(bind)
        .expect("Can not bind to port 8000")
        .run();
}
