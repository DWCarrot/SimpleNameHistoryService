use std::time::SystemTime;

use hyper::header;
use hyper::Response;
use hyper::Body;
use hyper::StatusCode;
use hyper::http::request;
use serde::Serialize;
use serde::ser::SerializeStruct;
use uuid::Uuid;
use warp::Rejection;
use warp::Reply;

use crate::client::JsonRequesterError;
use crate::client::data::Profile;
use crate::storage::data::NameHistory;
use crate::storage::data::NameHistoryElement;
use crate::storage::data::Update;

use super::Context;

pub const UPDATE_BY_PROFILE: u32 = 1;

pub async fn handle_get_name_history(uuid: Uuid, context: Context) -> Result<Response<Body>, Rejection> {
    match handle_get_name_history_inner(uuid, context).await {
        Ok(data) => Ok(warp::reply::json(&data).into_response()),
        Err(resp) => Ok(resp)
    }
}


async fn handle_get_name_history_inner(uuid: Uuid, context: Context) -> Result<NameHistory, Response<Body>> {
    let now = SystemTime::now();
    let update = context.database.get_update(&uuid).await.map_err(into_error_response_db)?;
    let (no_update_record, need_request) = if let Some(update) = update {
        (false, !update.use_cache(&now, context.use_cache_config.as_ref()))
    } else {
        (true, true)
    };
    let mut data = context.database.get_name_history(&uuid).await.map_err(into_error_response_db)?;
    if need_request {
        let profile = context.requester.request_profile(&uuid).await.map_err(into_error_response_req)?;
        tracing::debug!("request new profile @{}", &uuid);
        let mut update_record = Update::new(now, false);
        let need_update = if let Some(last) = data.last() {
            if last.name == profile.name {
                None
            } else {
                Some(NameHistoryElement::new(profile.name, now))
            }
        } else {
            Some(NameHistoryElement::new(profile.name, now))
        };
        if let Some(record) = need_update {
            update_record.changed = true;
            context.database.add_name_history(&uuid, &record, UPDATE_BY_PROFILE).await.map_err(into_error_response_db)?;
            tracing::debug!("update @{}: {:?}", &uuid, &record);
            data.push(record);
        }
        if no_update_record {
            context.database.insert_update(&uuid, &update_record).await.map_err(into_error_response_db)?;
        } else {
            context.database.refresh_update(&uuid, &update_record).await.map_err(into_error_response_db)?;
        }
    }
    Ok(data)
}

pub(crate) struct ErrorWrapper<E>(E);

pub(crate) fn into_error_response_db(e: sqlx::Error) -> Response<Body> {
    tracing::error!("database error {}", &e);
    let e = ErrorWrapper(e);
    match serde_json::to_vec(&e) {
        Ok(body) => {
            let mut resp = Response::new(body.into());
            resp.headers_mut().insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
            *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            resp
        }
        Err(e) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub(crate) fn into_error_response_req(e: JsonRequesterError) -> Response<Body> {
    let s = match &e {
        JsonRequesterError::StatusCode(s) => {
            tracing::warn!("request failed: {}", s);
            StatusCode::SERVICE_UNAVAILABLE
        },
        JsonRequesterError::Hyper(e) => {
            tracing::error!("request error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        },
        JsonRequesterError::Deserialize(e) => {
            tracing::error!("request error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        },
    };
    let e = ErrorWrapper(e);
    match serde_json::to_vec(&e) {
        Ok(body) => {
            let mut resp = Response::new(body.into());
            resp.headers_mut().insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
            *resp.status_mut() = s;
            resp
        }
        Err(e) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}


impl Serialize for ErrorWrapper<JsonRequesterError> {

    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        let mut s = serializer.serialize_struct("ErrorWrapper", 2)?;
        match &self.0 {
            JsonRequesterError::Deserialize(e) => {
                s.serialize_field("type", "inner")?;
                s.serialize_field("error", e.to_string().as_str())?;
            },
            JsonRequesterError::Hyper(e) => {
                s.serialize_field("type", "request")?;
                s.serialize_field("error", e.to_string().as_str())?;
            },
            JsonRequesterError::StatusCode(c) => {
                s.serialize_field("type", "request")?;
                s.serialize_field("code", &c.as_u16())?;
            },
        }
        s.end()
    }
}

impl Serialize for ErrorWrapper<sqlx::Error> {

    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        let mut s = serializer.serialize_struct("ErrorWrapper", 2)?;
        s.serialize_field("type", "database")?;
        s.serialize_field("error", self.0.to_string().as_str())?;
        s.end()
    }
}


// fn from_request_error_status(e: JsonRequesterError) -> Response<Body> {
//     let body = match e {
//         JsonRequesterError::StatusCode(s) => format!("{{\"type\":\"request\",\"status\":{}}}", s.as_u16()),
//         JsonRequesterError::Hyper(e) => format!("{{\"type\":\"request\",\"message\":{:?}}}", e.to_string()),
//         JsonRequesterError::Deserialize(e) => format!("{{\"type\":\"request\",\"message\":{:?}}}", e.to_string()),
//     };
//     let mut resp = Response::new(body.into());
//     resp.headers_mut().insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
//     *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
//     resp
// }

// fn from_database_error(e: sqlx::Error) -> Response<Body> {
//     let body = format!("{{\"type\":\"database\",\"message\":{:?}}}", e.to_string());
//     let mut resp = Response::new(body.into());
//     resp.headers_mut().insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
//     *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
//     resp
// }