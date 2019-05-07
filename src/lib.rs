use std::collections::HashMap;
use reqwest::{self, Client, Response};
use select::{
    node::Node,
    document::Document,
    predicate::{Predicate, Attr, Class, Name},
};
use failure::{Error, Fail};

const URL_CAS_LOGIN: &str = "https://cas.sustech.edu.cn/cas/login";
const URL_COURSE_FORM: &str = "http://jwxt.sustech.edu.cn/jsxsd/kscj/cjcx_query";
const URL_COURSE_QUERY: &str = "http://jwxt.sustech.edu.cn/jsxsd/kscj/cjcx_list";


#[derive(Debug, Clone)]
pub struct UserAgent {
    client: Client
}

#[derive(Debug, Clone)]
pub struct LoginedAgent {
    client: Client
}

#[derive(Debug, Clone)]
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
    LoginError {
        message: String,
    }
}

impl From<Client> for UserAgent {
    fn from(client: Client) -> UserAgent {
        UserAgent { client }
    }
}

trait ResponseExt {
    fn parse(self) -> Result<Document, Error>;
}

impl ResponseExt for Response {
    fn parse(mut self) -> Result<Document, Error> {
        let doc = self
            .text()?
            .as_str()
            .into();
        Ok(doc)
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
        let client = Client::builder()
            .gzip(true)
            .cookie_store(true)
            .build()
            .expect("fail to init http client");
        UserAgent { client }
    }

    pub fn login<S: AsRef<str>>(self, username: S, password: S) -> Result<LoginedAgent, Error> {
        let UserAgent { client } = self;

        // Retrive login <form> and all its <input>
        let doc = client.get(URL_CAS_LOGIN)
            .query(&[("service", URL_COURSE_FORM)])
            .send()?.error_for_status()?.parse()?;
        let mut form = doc.extract_form(Attr("id", "fm1"));
        form.insert("username", username.as_ref());
        form.insert("password", password.as_ref());

        // Post form and check result
        let resp = client.post(URL_CAS_LOGIN)
            .form(&form)
            .send()?;
        match resp.error_for_status_ref() {
            Ok(_) => Ok(LoginedAgent { client }),
            Err(err) => match err.status() {
                Some(status) if status.is_client_error() => {
                    // Try to extract err message
                    let predicate = Attr("id", "fm1").descendant(Class("alert"));
                    let message = if let Some(alert) = resp.parse()?.find(predicate).next() {
                        alert.text().trim().to_string()
                    } else {
                        format!("server return {}", status)
                    };
                    Err(CourseError::LoginError { message }.into())
                }
                _ => Err(err.into())
            }
        }
    }
}

impl LoginedAgent {
    pub fn query_course(&mut self, year: u16, term: u8) -> Result<Vec<Course>, Error> {
        // Form form
        let doc = self.client.get(URL_COURSE_FORM)
            .send()?.error_for_status()?.parse()?;
        let mut form = doc.extract_form(Attr("id", "kscjQueryForm"));
        let term = format!("{}-{}-{}", year, year + 1, term);
        form.insert("kksj", term.as_str());

        // Post form
        let doc = self.client.post(URL_COURSE_QUERY)
            .form(&form).send()?.parse()?;

        //let doc = dbg!(doc);
        // Parse
        let rows = Attr("id", "dataList").descendant(Name("tr"));
        let courses = doc.find(rows).skip(1).map(|row| {
            let mut elems = row.find(Name("td"));
            elems.next();  // drop column id
            Course {
                term: elems.next().text(),
                code: elems.next().text(),
                name: elems.next().text(),
                grade: elems.next().text(),
                score: elems.next().text(),
                point: elems.next().text(),
                hours: elems.next().text(),
                eval_method: elems.next().text(),
                course_type: elems.next().text(),
                category: elems.next().text(),
            }
        });

        Ok(courses.collect())
    }
}
