use std::borrow::Cow;
use std::fs;

use axum::response::Html;
use chrono::{NaiveDateTime, NaiveTime};
use minijinja::{filters::safe, value::ValueKind, Environment, Error, ErrorKind, Value};
use serde::Serialize;

use tracing::warn;

use anyhow::{Context, Result};

fn value_to_float(value: Value) -> Result<f64, Error> {
    Ok(match value.kind() {
        ValueKind::Number => value.try_into().unwrap(),
        ValueKind::String => value.as_str().unwrap().parse().map_err(|e| {
            Error::new(
                ErrorKind::InvalidOperation,
                format!("coloredfloat: Can't convert '{}' to float: {}", value, e),
            )
        })?,
        unknown => {
            return Err(Error::new(
                ErrorKind::InvalidOperation,
                format!("Value kind {unknown:#?} is not supported by coloredfloat"),
            ))
        }
    })
}

fn coloredfloatnegative(value: Value) -> Result<Value, Error> {
    let f = value_to_float(value)?;
    if f > 0.0 {
        Ok(safe(format!("{:.2}", f)))
    } else {
        Ok(safe(format!("<span class=\"negative\">{:.2}</span>", f)))
    }
}
fn coloredfloat(value: Value) -> Result<Value, Error> {
    let f = value_to_float(value)?;
    if f > 0.0 {
        Ok(safe(format!("<span class=\"positive\">{:.2}</span>", f)))
    } else {
        Ok(safe(format!("<span class=\"negative\">{:.2}</span>", f)))
    }
}

fn orempty(value: Option<String>) -> Value {
    match value {
        Some(x) => x.into(),
        None => "".into(),
    }
}

fn markdown(value: String) -> String {
    let parsed = pulldown_cmark::Parser::new(&value);
    let mut output = String::new();
    pulldown_cmark::html::push_html(&mut output, parsed);
    output
}

fn hourminutes(value: Option<String>) -> Result<Value, Error> {
    match value {
        Some(x) => {
            return if let Ok(ndt) = NaiveTime::parse_from_str(&x, "%H:%M:%S") {
                Ok(Value::from(ndt.format("%H:%M").to_string()))
            } else if let Ok(ndt) = NaiveDateTime::parse_from_str(&x, "%Y-%m-%dT%H:%M:%S") {
                Ok(Value::from(ndt.format("%H:%M").to_string()))
            } else {
                Err(Error::new(
                    ErrorKind::InvalidOperation,
                    format!("{} is not a date time", x),
                ))
            }
        }
        None => Ok(Value::from("")),
    }
}

#[derive(Clone)]
pub struct CachedEnvironment {
    environment: Option<Environment<'static>>,
}

impl CachedEnvironment {
    pub fn new(use_cache: bool) -> Self {
        CachedEnvironment {
            environment: if use_cache {
                Some(create_environment().expect("Failed to create environment"))
            } else {
                None
            },
        }
    }

    pub fn render<S: Serialize>(&self, template_path: &str, context: S) -> Html<String> {
        let env = match &self.environment {
            Some(cached) => Cow::Borrowed(cached),
            None => Cow::Owned(create_environment().expect("Failed to create environment")),
        };

        let template = match env.get_template(template_path) {
            Ok(t) => t,
            Err(e) => return Html(format!("Failed getting template: {:#?}", e)),
        };
        match template.render(context) {
            Ok(rendered) => Html(rendered),
            Err(e) => Html(format!("Rendering error: {:#?}", e)),
        }
    }
}

fn create_environment<'source>() -> Result<Environment<'source>> {
    let mut environment = Environment::new();
    let uuid = &uuid::Uuid::new_v4().to_string()[..8];
    environment.add_global("buildNumber", uuid);
    environment.add_filter("coloredfloat", coloredfloat);
    environment.add_filter("coloredfloatnegative", coloredfloatnegative);
    environment.add_filter("hourminutes", hourminutes);
    environment.add_filter("orempty", orempty);
    environment.add_filter("markdown", markdown);
    environment.add_filter("floatfmt", |f: f64| format!("{:.2}", f));
    environment.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
    for file in fs::read_dir("ui/jinja").context("ui/jinja read dir failed")? {
        let file = file?;
        let name = file
            .path()
            .file_name()
            .and_then(|x| x.to_str())
            .map(|x| x.to_string());
        let name = match name {
            Some(x) => x,
            None => {
                warn!("Template {:?} is not loaded", file);
                continue;
            }
        };
        let contents = fs::read_to_string(file.path())?;

        environment.add_template_owned(name, contents)?;
    }
    Ok(environment)
}
