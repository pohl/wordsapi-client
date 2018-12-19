extern crate hyper;
extern crate hyper_tls;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio_core;

use hyper::client::HttpConnector;
use hyper::error::Error;
use hyper::header::HeaderName;
use hyper::rt::Future;
use hyper::rt::Stream;
use hyper::Body;
use hyper::Request;
use hyper_tls::HttpsConnector;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt;
use tokio_core::reactor::Core;

static X_MASHAPE_KEY: &[u8] = b"x-mashape-key";
static X_MASHAPE_HOST: &[u8] = b"x-mashape-host";
static X_RATE_LIMIT_REMAINING: &[u8] = b"x-ratelimit-requests-remaining";
static X_RATE_LIMIT_REQUESTS_LIMIT: &[u8] = b"x-ratelimit-requests-limit";
static API_BASE: &'static str = "https://wordsapiv1.p.mashape.com/words/";
static MASHAPE_HOST: &'static str = "wordsapiv1.p.mashape.com";

#[derive(Debug)]
pub enum RequestError {
    RequestError,
    ResultParseError,
}

#[derive(Debug)]
pub enum RequestType {
    Everything,
    Definitions,
    Synonyms,
    Antonyms,
    Examples,
    Rhymes,
    Frequency,
    IsATypeOf,
    HasTypes,
    PartOf,
    HasParts,
    IsAnInstanceOf,
    HasInstances,
    InRegion,
    RegionOf,
    UsageOf,
    HasUsages,
    IsAMemberOf,
    HasMembers,
    IsASubstanceOf,
    HasSubstances,
    HasAttribute,
    InCategory,
    HasCategories,
    Also,
    PertainsTo,
    SimilarTo,
    Entails,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RequestError::RequestError => f.write_str("RequestError"),
            RequestError::ResultParseError => f.write_str("ResultParseError"),
        }
    }
}
impl StdError for RequestError {
    fn description(&self) -> &str {
        match *self {
            RequestError::RequestError => "WordAPI request failed",
            RequestError::ResultParseError => "Could not parse result",
        }
    }
}

pub struct Client {
    https_client: hyper::Client<HttpsConnector<HttpConnector>, Body>,
    api_base: String,
    api_token: String,
    mashape_host: String,
}

pub struct Response {
    pub response_json: String,
    pub rate_limit_remaining: usize,
    pub rate_limit_requests_limit: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Word {
    pub word: String,
    pub frequency: Option<f32>,
    pub pronunciation: Option<HashMap<String, String>>,
    pub entries: Vec<Entry>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Entry {
    pub definition: String,
    #[serde(rename = "partOfSpeech")]
    pub part_of_speech: Option<String>,
    pub derivation: Option<Vec<String>>,
    #[serde(rename = "hasSubstances")]
    pub has_substances: Option<Vec<String>>,
    #[serde(rename = "typeOf")]
    pub type_of: Option<Vec<String>>,
    #[serde(rename = "verbGroup")]
    pub verb_group: Option<Vec<String>>,
    #[serde(rename = "hasTypes")]
    pub has_types: Option<Vec<String>>,
    #[serde(rename = "hasParts")]
    pub has_parts: Option<Vec<String>>,
    #[serde(rename = "memberOf")]
    pub member_of: Option<Vec<String>>,
    #[serde(rename = "partOf")]
    pub part_of: Option<Vec<String>>,
    pub synonyms: Option<Vec<String>>,
    pub antonyms: Option<Vec<String>>,
    pub examples: Option<Vec<String>>,
    #[serde(rename = "similarTo")]
    pub similar_to: Option<Vec<String>>,
    #[serde(rename = "pertainsTo")]
    pub pertains_to: Option<Vec<String>>,
}

impl Client {
    pub fn new(token: &str) -> Client {
        let https = HttpsConnector::new(4).unwrap();
        let client = hyper::Client::builder().build::<_, hyper::Body>(https);
        Self {
            https_client: client,
            api_base: API_BASE.to_owned(),
            api_token: token.to_owned(),
            mashape_host: MASHAPE_HOST.to_owned(),
        }
    }

    pub fn look_up(
        &self,
        word: &str,
        request_type: &RequestType,
    ) -> Result<Response, RequestError> {
        let uri = self.request_url(word, request_type);
        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .header(X_MASHAPE_KEY, self.api_token.to_owned())
            .header(X_MASHAPE_HOST, self.mashape_host.to_owned())
            .body(Body::empty())
            .unwrap();
        let work = self
            .https_client
            .request(request)
            .and_then(|response| {
                let remaining = response
                    .headers()
                    .get(HeaderName::from_lowercase(X_RATE_LIMIT_REMAINING).unwrap())
                    .map(|hv| hv.to_str().unwrap().to_string())
                    .map_or(0, |v| v.parse::<usize>().unwrap());
                let allowed = response
                    .headers()
                    .get(HeaderName::from_lowercase(X_RATE_LIMIT_REQUESTS_LIMIT).unwrap())
                    .map(|hv| hv.to_str().unwrap().to_string())
                    .map_or(0, |v| v.parse::<usize>().unwrap());
                response
                    .into_body()
                    .concat2()
                    .map(move |body| {
                        (
                            String::from_utf8(body.to_vec()).unwrap(),
                            allowed,
                            remaining,
                        )
                    })
                    .map_err(Error::from)
            })
            .map_err(|_err| {
                println!("api says {}", _err);
                Err(RequestError::RequestError)
            });
        let mut reactor = Core::new().unwrap();
        let result = reactor.run(work);
        match result {
            Ok(r) => Ok(Response::new(r.0, r.1, r.2)),
            Err(_e) => _e,
        }
    }

    fn request_url(&self, word: &str, request_type: &RequestType) -> String {
        let suffix = match *request_type {
            RequestType::Everything => "",
            RequestType::Definitions => "/definitions",
            RequestType::Synonyms => "/synonyms",
            RequestType::Antonyms => "/antonyms",
            RequestType::Examples => "/examples",
            RequestType::Rhymes => "/rhymes",
            RequestType::Frequency => "/frequency",
            RequestType::IsATypeOf => "/isATypeOf",
            RequestType::HasTypes => "/hasTypes",
            RequestType::PartOf => "/partOf",
            RequestType::HasParts => "/hasParts",
            RequestType::IsAnInstanceOf => "/isAnInstanceOf",
            RequestType::HasInstances => "/hasInstances",
            RequestType::InRegion => "/inRegion",
            RequestType::RegionOf => "/regionOf",
            RequestType::UsageOf => "/usageOf",
            RequestType::HasUsages => "/hasUsages",
            RequestType::IsAMemberOf => "/isAMemberOf",
            RequestType::HasMembers => "/hasMembers",
            RequestType::IsASubstanceOf => "/isASubstanceOf",
            RequestType::HasSubstances => "/hasSubstances",
            RequestType::HasAttribute => "/hasAttribute",
            RequestType::InCategory => "/inCategory",
            RequestType::HasCategories => "/hasCategories",
            RequestType::Also => "/also",
            RequestType::PertainsTo => "/pertainsTo",
            RequestType::SimilarTo => "/similarTo",
            RequestType::Entails => "/entails",
        };
        format!("{}{}{}", self.api_base, word, suffix)
    }
}

impl Response {
    fn new(raw_json: String, allowed: usize, remaining: usize) -> Response {
        Self {
            response_json: raw_json,
            rate_limit_remaining: remaining,
            rate_limit_requests_limit: allowed,
        }
    }

    pub fn try_parse(&self) -> Result<Word, RequestError> {
        try_parse(&self.response_json)
    }
}

pub fn try_parse(word_json: &str) -> Result<Word, RequestError> {
    let result = serde_json::from_str(word_json);
    match result {
        Ok(word_data) => Ok(word_data),
        Err(e) => {
            println!("serde says {}", e);
            Err(RequestError::ResultParseError)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Client;
    use crate::RequestType;
    use crate::API_BASE;
    use crate::MASHAPE_HOST;

    #[test]
    fn it_has_api_token() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        assert_eq!(word_client.api_token, token);
    }

    #[test]
    fn it_has_api_base() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        assert_eq!(word_client.api_base, API_BASE);
    }

    #[test]
    fn it_has_mashape_host() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        assert_eq!(word_client.mashape_host, MASHAPE_HOST);
    }

    #[test]
    fn it_makes_uri_everything() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::Everything);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example"
        );
    }

    #[test]
    fn it_makes_uri_definitions() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::Definitions);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/definitions"
        );
    }

    #[test]
    fn it_makes_uri_synonyms() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::Synonyms);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/synonyms"
        );
    }

    #[test]
    fn it_makes_uri_antonyms() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::Antonyms);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/antonyms"
        );
    }

    #[test]
    fn it_makes_uri_examples() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::Examples);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/examples"
        );
    }

    #[test]
    fn it_makes_uri_rhymes() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::Rhymes);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/rhymes"
        );
    }

    #[test]
    fn it_makes_uri_frequency() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::Frequency);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/frequency"
        );
    }

    #[test]
    fn it_makes_uri_is_a_type_of() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::IsATypeOf);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/isATypeOf"
        );
    }

    #[test]
    fn it_makes_uri_has_types() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::HasTypes);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/hasTypes"
        );
    }

    #[test]
    fn it_makes_uri_part_of() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::PartOf);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/partOf"
        );
    }

    #[test]
    fn it_makes_uri_has_parts() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::HasParts);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/hasParts"
        );
    }

    #[test]
    fn it_makes_uri_is_an_instance_of() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::IsAnInstanceOf);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/isAnInstanceOf"
        );
    }

    #[test]
    fn it_makes_uri_has_instances() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::HasInstances);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/hasInstances"
        );
    }

    #[test]
    fn it_makes_uri_in_region() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::InRegion);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/inRegion"
        );
    }

    #[test]
    fn it_makes_uri_region_of() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::RegionOf);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/regionOf"
        );
    }

    #[test]
    fn it_makes_uri_usage_of() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::UsageOf);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/usageOf"
        );
    }

    #[test]
    fn it_makes_uri_has_usages() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::HasUsages);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/hasUsages"
        );
    }

    #[test]
    fn it_makes_uri_is_a_member_of() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::IsAMemberOf);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/isAMemberOf"
        );
    }

    #[test]
    fn it_makes_uri_has_members() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::HasMembers);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/hasMembers"
        );
    }

    #[test]
    fn it_makes_uri_is_a_substance_of() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::IsASubstanceOf);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/isASubstanceOf"
        );
    }

    #[test]
    fn it_makes_uri_has_substances() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::HasSubstances);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/hasSubstances"
        );
    }

    #[test]
    fn it_makes_uri_has_attribute() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::HasAttribute);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/hasAttribute"
        );
    }

    #[test]
    fn it_makes_uri_in_category() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::InCategory);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/inCategory"
        );
    }

    #[test]
    fn it_makes_uri_has_categories() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::HasCategories);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/hasCategories"
        );
    }

    #[test]
    fn it_makes_uri_also() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::Also);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/also"
        );
    }

    #[test]
    fn it_makes_uri_pertains_to() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::PertainsTo);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/pertainsTo"
        );
    }

    #[test]
    fn it_makes_uri_similar_to() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::SimilarTo);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/similarTo"
        );
    }

    #[test]
    fn it_makes_uri_entails() {
        let token = "TEST_TOKEN";
        let word_client = Client::new(token);
        let word = "example";
        let request_uri = word_client.request_url(word, &RequestType::Entails);
        assert_eq!(
            request_uri,
            "https://wordsapiv1.p.mashape.com/words/example/entails"
        );
    }

}