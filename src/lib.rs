use std::str;
use std::io::Read;

extern crate encoding;
use encoding::{Encoding, DecoderTrap};
use encoding::all::WINDOWS_1252;
use encoding::label::encoding_from_whatwg_label;

extern crate flate2;
use flate2::read::{GzDecoder, ZlibDecoder};

extern crate hyper;
use hyper::Client;
use hyper::header::{AcceptCharset, AcceptEncoding, ContentEncoding, ContentType, Charset, Headers,
                    qitem, Encoding as HyperEncoding};

/// FetchError described that top level error categories that can result from a fetch operation
#[derive(Debug)]
pub enum FetchError {
    /// RetrieveError encompasses all errors relating to the retrieval of the initla bytes from the server
    RetrieveError(hyper::error::Error),
    /// ReadErrors occur when fetching or decompressing the body errors
    ReadError(std::io::Error),
    /// CharsetErrors are related to problems when trying to convert the body to UTF-8
    CharsetError(String),
}

impl From<hyper::error::Error> for FetchError {
    fn from(err: hyper::error::Error) -> FetchError {
        FetchError::RetrieveError(err)
    }
}
    let client = Client::new();

    let mut headers = Headers::new();
    headers.set(AcceptCharset(vec![qitem(Charset::Ext("utf-8".to_owned()))]));
    headers.set(AcceptEncoding(vec![qitem(HyperEncoding::Gzip),
                                    qitem(HyperEncoding::Deflate),
                                    qitem(HyperEncoding::Chunked),
                                    qitem(HyperEncoding::Identity)]));

    let mut fetch_result = try!(client.get(url).headers(headers).send());

    let mut body_buffer = Vec::new();

    try!(fetch_result.read_to_end(&mut body_buffer).map_err(|e| FetchError::ReadError(e)));

    println!("headers {}", fetch_result.headers);

    match fetch_result.headers.get::<ContentEncoding>() {
        Some(content_encoding_header) => {
            println!("encodings {}", content_encoding_header);
            for encoding in content_encoding_header.iter().rev() {
                match *encoding {
                    HyperEncoding::Gzip => {
                        let mut unzipped_body_buffer = Vec::new();
                        {
                            let mut d = try!(GzDecoder::new(body_buffer.as_slice()).map_err(|e| FetchError::ReadError(e)));

                            try!(d.read_to_end(&mut unzipped_body_buffer).map_err(|e| FetchError::ReadError(e)));
                        }
                        body_buffer = unzipped_body_buffer;
                    }

                    HyperEncoding::Deflate => {
                        let mut unzipped_body_buffer = Vec::new();
                        {
                            let mut d = ZlibDecoder::new(body_buffer.as_slice());
                            let res = d.read_to_end(&mut unzipped_body_buffer);

                            try!(res.map_err(|e| FetchError::ReadError(e)));
                        }
                        body_buffer = unzipped_body_buffer;
                    }
                    HyperEncoding::Chunked => {}
                    HyperEncoding::Identity => {}
                    _ => return Err(FetchError::ReadError(std::io::Error::new(std::io::ErrorKind::Other, "Unsupported encoding"))),
                }
            }
        }
        None => {}
    }

    match fetch_result.headers.get::<ContentType>() {
        Some(content_type_header) => {
                // .ok_or(FetchError::CharsetError("Error getting charset from content-type header".into) {
            match content_type_header.get_param(hyper::mime::Attr::Charset) {
                Some(charset) => {
                    println!("charset: {}", charset);
                    match encoding_from_whatwg_label(charset) {
                        Some(decoder) => {
                            return decoder.decode(&body_buffer, DecoderTrap::Strict)
                                .ok()
                                .ok_or(FetchError::CharsetError("Error decoding page body (using decoder for charset from \
                                        content-type)".into()));
                        }
                        None => {
                            // No decoder found for charset, will try default decoder
                        }
                    }
                }
                None => {
                    // no charset in content-type header, will use default
                }
            }
        }
        None => {
            // no content-type header, will use default charset
        }
    }

    return WINDOWS_1252.decode(&body_buffer, DecoderTrap::Strict)
        .ok()
        .ok_or(FetchError::CharsetError("Error decoding page body (using WINDOWS_1252)".into()));
}

#[test]
fn fetch_gzip_compressed_page() {
    fetch_body("http://httpbin.org/gzip").expect("Fetch to succeed");
}

#[test]
fn fetch_utf8_encoded_page() {
    fetch_body("https://eu.httpbin.org/encoding/utf8").expect("Fetch to succeed");
}

#[test]
fn fetch_win1215_encoded_page() {
    fetch_body("http://www.cbloom.com/rants.html").expect("Fetch to succeed"); // will fail when read as UTF-8
}

#[test]
fn fetch_deflate_compressed_page() {
    fetch_body("http://httpbin.org/deflate").expect("Fetch to succeed");
}
