//! This design a File streamer.
use futures_cpupool::CpuPool;
use hyper;
use futures::{Future, Sink};

use std::fs;
use std::io::{self, BufRead};

use errors::Error;
use response::{Responder, Response};
use Context;
use http::{StatusCode, header};

// size for chunked read
// using 4096, maybe modify for better perfs
const CHUNK_SIZE: usize = 4096;

pub struct File {
    filename: String,
    cpupool : CpuPool,
    head    : bool,
}

impl File {
    pub fn head(context: &Context, filename: String) -> Self {
        File {
            filename, 
            head: true, 
            cpupool: context.get::<CpuPool>().clone(),
        }
    }

    pub fn open(context: &Context, filename: String) -> Self {
        File {
            filename,
            head: false,
            cpupool: context.get::<CpuPool>().clone(),
        }
    }
}

impl Responder for File {
    type Result = Box<Future<Item = Response, Error = hyper::Error>>;

    fn to_response(self) -> Self::Result {
        let Self { filename, head, cpupool } = self;

        let cpupool_clone = cpupool.clone();

        cpupool.spawn_fn(move || -> Result<Response, Error> {
            let filename = filename;
            let head = head;

            let metadata = match fs::metadata(&filename) {
                Ok(metadata) => metadata,
                Err(_) => {
                    // TODO: maybe log error ???
                    return Ok(Response::with(StatusCode::NotFound));
                }
            };
            if !metadata.is_file() {
                // TODO: maybe display full path
                return Ok(Response::with(StatusCode::NotFound));
            }

            let length = metadata.len();

            if head {
                let response = Response::build()
                    .status(StatusCode::Ok)
                    .header(header::ContentLength(length))
                    .into();
                return Ok(response);
            }

            let file = io::BufReader::new(fs::File::open(filename)?);

            let (sender, body) = hyper::Body::pair();

            cpupool_clone.spawn_fn(move || -> Result<(), ()> {
                let mut file = file;
                let mut sender = sender.wait();

                'readloop: loop {
                    let mut data : Vec<u8> = Vec::with_capacity(CHUNK_SIZE);

                    let len = {
                        let buffer = match file.fill_buf() {
                            Ok(buf) => buf,
                            Err(io) => {
                                let _ = sender.send(Err(io.into()));
                                break 'readloop;
                            }
                        };

                        data.extend_from_slice(buffer);
                        buffer.len()
                    };

                    if len == 0 {
                        break;
                    }
                    let _ = sender.send(Ok(data.into()));
                }

                let _ = sender.flush();
                Ok(())
            }).forget();

            let response = Response::build()
                .status(StatusCode::Ok)
                .header(header::ContentLength(length))
                .body(body);
            return Ok(response);
        }).to_response()
    }
}