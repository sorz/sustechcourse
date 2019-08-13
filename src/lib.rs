use failure::{Error, Fail};
use reqwest::{
    r#async::Client,
    header::{USER_AGENT, REFERER, HeaderMap},
};
use select::{
    document::Document,
    node::Node,
    predicate::{Attr, Name, Predicate},
};
use futures::Future;
use log::debug;
use serde::Serialize;
use std::collections::HashMap;

const URL_CAS_LOGIN: &str = "https://cas.sustech.edu.cn/cas/login";
const URL_COURSE_FORM: &str = "https://jwxt.sustech.edu.cn/jsxsd/kscj/cjcx_query";
const URL_COURSE_QUERY: &str = "https://jwxt.sustech.edu.cn/jsxsd/kscj/cjcx_list";

const USER_AGENT_STRING: &str = "sustechcourse/0.1.0 (citric-acid.com.cn)";


#[derive(Debug, Clone)]
pub struct UserAgent {
    client: Client,
}

#[derive(Debug, Clone)]
pub struct LoginedAgent {
    client: Client,
}

#[derive(Debug, Clone, Serialize)]
pub struct Course {
    pub code: String,
    pub term: String,
    pub name: String,
    pub grade: String,
    pub score: String,
    pub point: String,
    pub hours: String,
    pub eval_method: String,
    pub course_type: String,
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct CourseQuery<'a> {
    agent: &'a LoginedAgent,
    form: HashMap<String, String>,
    years: Vec<String>,
}

#[derive(Debug, Fail)]
enum CourseError {
    #[fail(display = "cannot login: {}", message)]
    LoginError { message: String },
}

impl From<Client> for UserAgent {
    fn from(client: Client) -> UserAgent {
        UserAgent { client }
    }
}

trait NodeExt {
    fn text(self) -> String;
}

impl NodeExt for Option<Node<'_>> {
    fn text(self) -> String {
        match self {
            Some(node) => node.text(),
            None => String::default(),
        }
    }
}

trait FormFieldExtract {
    fn extract_form<P: Predicate>(&self, form: P) -> HashMap<&str, &str>;
}

impl FormFieldExtract for Document {
    fn extract_form<P: Predicate>(&self, form: P) -> HashMap<&str, &str> {
        let mut fields = HashMap::new();
        for input in self.find(form.descendant(Name("input"))) {
            if let (Some(name), Some(value)) = (input.attr("name"), input.attr("value")) {
                fields.insert(name, value);
            }
        }
        fields
    }
}

impl UserAgent {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, USER_AGENT_STRING.parse().unwrap());

        let client = Client::builder()
            .gzip(true)
            .cookie_store(true)
            .use_sys_proxy()
            .default_headers(headers)
            .build()
            .expect("fail to init http client");
        UserAgent { client }
    }

    pub fn login(self, username: String, password: String)
        -> impl Future<Item=LoginedAgent, Error=Error>
    {
        let UserAgent { client } = self;
        debug!("loging in as {}", username);

        // Retrive login <form> and all its <input>
        let doc = client
            .get(URL_CAS_LOGIN)
            .query(&[("service", URL_COURSE_FORM)])
            .send()
            .and_then(|resp| resp.error_for_status())
            .and_then(|mut resp| resp.text())
            .map(|text| text.as_str().into());
        
        // Fill the form then post
        let post = doc.and_then(move |doc: Document| {
            let mut form = doc.extract_form(Attr("id", "fm1"));
            debug!("login form retrived {:?}", form.keys());
            form.insert("username", username.as_ref());
            form.insert("password", password.as_ref());
            client.post(URL_CAS_LOGIN)
                .form(&form)
                .header(REFERER, URL_CAS_LOGIN)
                .send()
                .map(move |resp| (resp, client))
        }).map_err(|err| err.into());

        // Check response
        post.and_then(|(resp, client)| {
            debug!("login form posted {:?}", resp);
            match resp.error_for_status_ref() {
                Ok(_) => Ok(LoginedAgent { client }),
                Err(_) => {
                    // TODO: extract err message
                    let message = format!("server return {}", resp.status());
                    Err(CourseError::LoginError { message }.into())
                }
            }
        })
    }
}

impl LoginedAgent {
    pub fn all_courses(&mut self) -> impl Future<Item = Vec<Course>, Error = Error> {
        let doc = self.client
            .get(URL_COURSE_QUERY)
            .send()
            .and_then(|resp| resp.error_for_status())
            .and_then(|mut resp| resp.text())
            .map(|text| text.as_str().into())
            .map_err(|err| err.into());
        doc.map(|doc: Document| {
            let rows = Attr("id", "dataList").descendant(Name("tr"));
            doc.find(rows).skip(1).filter_map(|row| {
                let mut elems = row.find(Name("td"));
                elems.next(); // drop column id
                if let (Some(term), Some(code)) = (elems.next(), elems.next()) {
                    // First two elem is requried
                    Some(Course {
                        term: term.text(),
                        code: code.text(),
                        name: elems.next().text(),
                        grade: elems.next().text(),
                        score: elems.next().text(),
                        point: elems.next().text(),
                        hours: elems.next().text(),
                        eval_method: elems.next().text(),
                        course_type: elems.next().text(),
                        category: elems.next().text(),
                    })
                } else {
                    None
                }
            }).collect()
        })
    }
}

#[test]
fn test_query_course() {
    let mut agent = UserAgent::new()
        .login(env!("USER"), env!("PASS"))
        .unwrap();
    let courses = agent.query_course(2018, 01).unwrap();
    assert!(courses.len() > 0);
    println!("courses: {:?}", courses);
}

#[test]
fn test_all_courses() {
    let mut agent = UserAgent::new()
        .login(env!("USER"), env!("PASS"))
        .unwrap();
    let courses = agent.all_courses().unwrap();
    assert!(courses.len() > 0);
    println!("courses: {:?}", courses);
}
