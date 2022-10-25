use anyhow::Result;
use std::fmt::Display;
use url::Url;

use crate::data::AsQuery;

pub const API_VERSION: &str = "v0alpha1";

pub enum Route {
    Fetch,
    Push,
    Publish,
    Did,
    Identify,
}

impl Display for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fragment = match self {
            Route::Fetch => "fetch",
            Route::Push => "push",
            Route::Publish => "publish",
            Route::Did => "did",
            Route::Identify => "identify",
        };

        write!(f, "/api/{}/{}", API_VERSION, fragment)
    }
}

pub struct RouteUrl<'a, 'b, Params: AsQuery = ()>(pub &'a Url, pub Route, pub Option<&'b Params>);

impl<'a, 'b, Params: AsQuery> TryFrom<RouteUrl<'a, 'b, Params>> for Url {
    type Error = anyhow::Error;

    fn try_from(value: RouteUrl<'a, 'b, Params>) -> Result<Self, Self::Error> {
        let RouteUrl(api_base, route, params) = value;
        let mut url = api_base.clone();
        url.set_path(&route.to_string());
        if let Some(params) = params {
            url.set_query(params.as_query()?.as_deref());
        } else {
            url.set_query(None);
        }
        Ok(url)
    }
}
