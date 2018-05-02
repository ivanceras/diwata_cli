#![deny(warnings)]
extern crate diwata_intel as intel;
extern crate diwata_server as server;
extern crate futures;
extern crate hyper;
extern crate rustorm;
extern crate serde;
extern crate serde_json;
extern crate structopt;

use structopt::StructOpt;

use futures::Future;
use futures::Stream;
use hyper::error::Error;
use hyper::header::ContentType;
use hyper::server::Request;
use hyper::server::Response;
use hyper::server::Service;
use hyper::Headers;
use hyper::StatusCode;

use hyper::server::Http;
use intel::data_container::SaveContainer;
use intel::data_container::{Filter, Sort};
use intel::data_modify;
use intel::data_read;
use intel::tab;
use intel::tab::Tab;
use intel::window;
use rustorm::pool;
use rustorm::Rows;
use rustorm::TableName;
use serde::Serialize;
use server::context::Context;
use server::Opt;
use server::ServiceError;
use std::sync::{Arc, Mutex};

const HTML: &'static str = include_str!("../public/static/inline-cli.html");

///
/// Derive from:
/// https://github.com/nrc/cargo-src/blob/master/src/server.rs
///
/// An instance of the server. Runs a session of rustw.
pub struct Server {}

#[derive(Clone)]
pub struct Instance {
    server: Arc<Mutex<Server>>,
}

impl Instance {
    pub fn new(server: Server) -> Instance {
        Instance {
            server: Arc::new(Mutex::new(server)),
        }
    }
}

impl Service for Instance {
    type Request = Request;
    type Response = Response;
    type Error = Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let uri = req.uri().clone();
        self.server
            .lock()
            .unwrap()
            .route(uri.path(), uri.query(), req)
    }
}

impl Server {
    pub fn new() -> Self {
        Server {}
    }

    pub fn route(
        &self,
        mut path: &str,
        query: Option<&str>,
        req: Request,
    ) -> <Instance as Service>::Future {
        println!("route: path: {:?}, query: {:?}", path, query);

        path = path.trim_matches('/');
        let path: Vec<_> = path.split('/').collect();
        let head = path[0];
        let tail = &path[1..];

        let result = if head == "" {
            self.handle_index(req)
        } else if head == "windows" {
            create_response(self.handle_windows(req))
        } else if head == "window" {
            create_response(self.handle_window(req, tail))
        } else if head == "data" {
            create_response(self.handle_data(req, tail))
        } else if head == "select" {
            create_response(self.handle_select(req, tail))
        } else if head == "has_many_select" {
            create_response(self.handle_has_many(req, tail))
        } else if head == "indirect_select" {
            create_response(self.handle_indirect(req, tail))
        } else if head == "lookup" {
            create_response(self.handle_lookup(req, tail))
        } else if head == "lookup_all" {
            create_response(self.handle_lookup_all(req, tail))
        } else if head == "test" {
            create_response(self.handle_test(req))
        } else if head == "db_url" {
            create_response(self.handle_db_url(req))
        } else if head == "delete" {
            return self.handle_delete(req, tail);
        } else if head == "tab_changeset" {
            return self.handle_tab_changeset(req);
        } else {
            self.handle_error(req, StatusCode::NotFound, "Page not found".to_owned())
        };

        Box::new(futures::future::ok(result))
    }
    fn handle_test(&self, _req: Request) -> Result<(), ServiceError> {
        let db_url = &server::get_db_url()?;
        println!("test db_url: {}", db_url);
        let ret = pool::test_connection(&db_url)?;
        Ok(ret)
    }

    fn handle_db_url(&self, _req: Request) -> Result<String, ServiceError> {
        server::get_db_url()
    }

    fn handle_index(&self, _req: Request) -> Response {
        let mut res = Response::new();
        res.headers_mut().set(ContentType::html());
        return res.with_body(HTML.as_bytes());
    }

    fn handle_error(&self, _req: Request, status: StatusCode, msg: String) -> Response {
        println!("ERROR: {} ({})", msg, status);

        Response::new().with_status(status).with_body(msg)
    }

    fn handle_windows(&self, _req: Request) -> Result<impl Serialize, ServiceError> {
        let em = server::get_pool_em()?;
        let db_url = &server::get_db_url()?;
        let ret = window::get_grouped_windows_using_cache(&em, db_url)?;
        Ok(ret)
    }

    fn handle_window(&self, _req: Request, tail: &[&str]) -> Result<impl Serialize, ServiceError> {
        let table_name = &tail[0];
        let context = Context::create()?;
        let table_name = TableName::from(&table_name);
        let window = window::get_window(&table_name, &context.windows);
        match window {
            Some(window) => Ok(window.to_owned()),
            None => Err(ServiceError::NotFound),
        }
    }

    ///
    /// /data/<table_name>/page/<page>/filter/<filter>/sort/<sort>/
    ///
    fn handle_data(&self, _req: Request, path: &[&str]) -> Result<impl Serialize, ServiceError> {
        println!("path:{:?}", path);
        let table_name = path[0];
        let tail = &path[1..];
        println!("tail {:?}", tail);
        let key_value: Vec<(&str, &str)> =
            tail.chunks(2).map(|chunk| (chunk[0], chunk[1])).collect();
        let mut page = 1;
        let mut filter_str = None;
        let mut sort_str = None;
        for (k, v) in key_value {
            println!("{} = {}", k, v);
            if k == "page" {
                page = v.parse().unwrap();
            } else if k == "filter" {
                filter_str = Some(v);
            } else if k == "sort" {
                sort_str = Some(v);
            }
        }
        let context = Context::create()?;
        let table_name = TableName::from(&table_name);
        let window = window::get_window(&table_name, &context.windows);
        let filter = filter_str.map(|s| Filter::from_str(s));
        let sort = sort_str.map(|s| Sort::from_str(s));
        match window {
            Some(window) => {
                let rows = data_read::get_maintable_data(
                    &context.em,
                    &context.dm,
                    &context.tables,
                    &window,
                    filter,
                    sort,
                    page,
                    server::PAGE_SIZE,
                )?;
                Ok(rows)
            }
            None => Err(ServiceError::NotFound),
        }
    }

    ///
    /// /select/<table_name>/<record_id>
    ///
    fn handle_select(&self, _req: Request, path: &[&str]) -> Result<impl Serialize, ServiceError> {
        let table_name = path[0];
        let record_id = path[1];
        let table_name = TableName::from(&table_name);
        let context = Context::create()?;
        let window = window::get_window(&table_name, &context.windows);
        match window {
            Some(window) => {
                let dao = data_read::get_selected_record_detail(
                    &context.dm,
                    &context.tables,
                    &window,
                    &record_id,
                    server::PAGE_SIZE,
                )?;
                match dao {
                    Some(dao) => Ok(dao),
                    None => Err(ServiceError::NotFound),
                }
            }
            None => Err(ServiceError::NotFound),
        }
    }

    ///
    ///
    ///  /has_many_select/<table_name>/<record_id>/<has_many_table>/page/<page>/filter/<filter>/sort/<sort>
    ///
    ///
    fn handle_has_many(
        &self,
        _req: Request,
        path: &[&str],
    ) -> Result<impl Serialize, ServiceError> {
        let table_name = path[0];
        let record_id = path[1];
        let has_many_table = path[2];
        let tail = &path[3..];
        let key_value: Vec<(&str, &str)> =
            tail.chunks(2).map(|chunk| (chunk[0], chunk[1])).collect();
        let mut page = 1;
        let mut filter_str = None;
        let mut sort_str = None;
        for (k, v) in key_value {
            println!("{} = {}", k, v);
            if k == "page" {
                page = v.parse().unwrap();
            } else if k == "filter" {
                filter_str = Some(v);
            } else if k == "sort" {
                sort_str = Some(v);
            }
        }
        let _filter = filter_str.map(|s| Filter::from_str(s));
        let _sort = sort_str.map(|s| Sort::from_str(s));
        let context = Context::create()?;
        let table_name = TableName::from(&table_name);
        let window = window::get_window(&table_name, &context.windows);
        let has_many_table_name = TableName::from(&has_many_table);
        match window {
            Some(window) => {
                let main_table = data_read::get_main_table(window, &context.tables);
                assert!(main_table.is_some());
                let main_table = main_table.unwrap();
                let has_many_tab = tab::find_tab(&window.has_many_tabs, &has_many_table_name);
                match has_many_tab {
                    Some(has_many_tab) => {
                        let rows = data_read::get_has_many_records_service(
                            &context.dm,
                            &context.tables,
                            &main_table,
                            &record_id,
                            has_many_tab,
                            server::PAGE_SIZE,
                            page,
                        )?;
                        Ok(rows)
                    }
                    None => Err(ServiceError::NotFound),
                }
            }
            None => Err(ServiceError::NotFound),
        }
    }

    ///
    ///
    ///  /indirect_select/<table_name>/<record_id>/<indirect_table>/page/<page>/filter/<filter>/sort/<sort>
    ///
    ///
    fn handle_indirect(
        &self,
        _req: Request,
        path: &[&str],
    ) -> Result<impl Serialize, ServiceError> {
        let table_name = path[0];
        let record_id = path[1];
        let indirect_table = path[2];
        let tail = &path[3..];
        let key_value: Vec<(&str, &str)> =
            tail.chunks(2).map(|chunk| (chunk[0], chunk[1])).collect();
        let mut page = 1;
        let mut filter_str = None;
        let mut sort_str = None;
        for (k, v) in key_value {
            println!("{} = {}", k, v);
            if k == "page" {
                page = v.parse().unwrap();
            } else if k == "filter" {
                filter_str = Some(v);
            } else if k == "sort" {
                sort_str = Some(v);
            }
        }
        let _filter = filter_str.map(|s| Filter::from_str(s));
        let _sort = sort_str.map(|s| Sort::from_str(s));
        let context = Context::create()?;

        let table_name = TableName::from(&table_name);
        let window = window::get_window(&table_name, &context.windows);
        let indirect_table_name = TableName::from(&indirect_table);
        match window {
            Some(window) => {
                let main_table = data_read::get_main_table(window, &context.tables);
                assert!(main_table.is_some());
                let main_table = main_table.unwrap();

                let indirect_tab: Option<&(TableName, Tab)> = window
                    .indirect_tabs
                    .iter()
                    .find(|&(_linker_table, tab)| tab.table_name == indirect_table_name);

                match indirect_tab {
                    Some(&(ref linker_table, ref indirect_tab)) => {
                        let rows = data_read::get_indirect_records_service(
                            &context.dm,
                            &context.tables,
                            &main_table,
                            &record_id,
                            &indirect_tab,
                            &linker_table,
                            server::PAGE_SIZE,
                            page,
                        )?;
                        Ok(rows)
                    }
                    None => Err(ServiceError::NotFound),
                }
            }
            None => Err(ServiceError::NotFound),
        }
    }

    /// retrieve the lookup data of this table at next page
    /// Usually the first page of the lookup data is preloaded with the window that
    /// may display them in order for the user to see something when clicking on the dropdown list.
    /// When the user scrolls to the bottom of the dropdown, a http request is done to retrieve the
    /// next page. All other lookup that points to the same table is also updated
    fn handle_lookup(&self, _req: Request, path: &[&str]) -> Result<impl Serialize, ServiceError> {
        println!("path:{:?}", path);
        let table_name = path[0];
        let page: u32 = path[1].parse().unwrap();
        let context = Context::create()?;

        let table_name = TableName::from(&table_name);
        let window = window::get_window(&table_name, &context.windows);
        match window {
            Some(window) => {
                let rows = data_read::get_lookup_data_of_tab(
                    &context.dm,
                    &context.tables,
                    &window.main_tab,
                    server::PAGE_SIZE,
                    page,
                )?;
                Ok(rows)
            }
            None => Err(ServiceError::NotFound),
        }
    }

    /// retrieve the first page of all lookup data
    /// used in this window
    /// Note: window is identified by it's table name of the main tab
    fn handle_lookup_all(
        &self,
        _req: Request,
        path: &[&str],
    ) -> Result<impl Serialize, ServiceError> {
        let table_name = path[0];
        let context = Context::create()?;
        let table_name = TableName::from(&table_name);
        let window = window::get_window(&table_name, &context.windows);
        match window {
            Some(window) => {
                let lookup = data_read::get_all_lookup_for_window(
                    &context.dm,
                    &context.tables,
                    &window,
                    server::PAGE_SIZE,
                )?;
                Ok(lookup)
            }
            None => Err(ServiceError::NotFound),
        }
    }

    // https://stackoverflow.com/questions/43419974/how-do-i-read-the-entire-body-of-a-tokio-based-hyper-request?rq=1
    // https://hyper.rs/guides/server/echo/
    fn handle_delete(
        &self,
        req: Request,
        path: &[&str],
    ) -> Box<Future<Item = Response, Error = Error>> {
        let table_name = path[0].to_string();
        let f = req.body().concat2().map(move |chunk| {
            let body = chunk.into_iter().collect::<Vec<u8>>();
            let body_str = String::from_utf8(body.clone()).unwrap();
            let record_ids: Vec<String> = serde_json::from_str(&body_str).unwrap();
            let result = delete_records(&table_name, &record_ids);
            create_response(result)
        });
        Box::new(f)
    }

    fn handle_tab_changeset(&self, req: Request) -> Box<Future<Item = Response, Error = Error>> {
        let f = req.body().concat2().map(move |chunk| {
            let body = chunk.into_iter().collect::<Vec<u8>>();
            let body_str = String::from_utf8(body).unwrap();
            let container: SaveContainer = serde_json::from_str(&body_str).unwrap();
            let result = update_tab_changeset(&container);
            create_response(result)
        });
        Box::new(f)
    }
}

fn delete_records(table_name: &str, record_ids: &Vec<String>) -> Result<Rows, ServiceError> {
    let context = Context::create()?;
    let table_name = TableName::from(&table_name);
    let window = window::get_window(&table_name, &context.windows);
    match window {
        Some(window) => {
            let main_table = data_read::get_main_table(window, &context.tables);
            assert!(main_table.is_some());
            let main_table = main_table.unwrap();
            println!(
                "delete these records: {:?} from table: {:?}",
                record_ids, table_name
            );
            let rows = data_modify::delete_records(&context.dm, &main_table, &record_ids)?;
            Ok(rows)
        }
        None => Err(ServiceError::NotFound),
    }
}

fn update_tab_changeset(container: &SaveContainer) -> Result<Rows, ServiceError> {
    let context = Context::create()?;
    let rows = data_modify::save_container(&context.dm, &context.tables, &container)?;
    Ok(rows)
}

fn create_response<B: Serialize>(body: Result<B, ServiceError>) -> Response {
    match body {
        Ok(body) => {
            let json = serde_json::to_string(&body).unwrap();
            println!("json:{}", json);
            let mut headers = Headers::new();
            headers.set(ContentType::json());
            let mut resp = Response::new().with_headers(headers).with_body(json);
            resp
        }
        Err(e) => match e {
            ServiceError::NotFound => Response::new().with_status(StatusCode::NotFound),
            _ => Response::new().with_status(StatusCode::BadRequest),
        },
    }
}

pub fn run(address: Option<String>, port: Option<u16>) {
    let ip = address.unwrap();
    let port = port.unwrap();
    let addr = format!("{}:{}", ip, port).parse().unwrap();
    let server = Server::new();
    let instance = Instance::new(server);
    Http::new()
        .bind(&addr, move || Ok(instance.clone()))
        .unwrap()
        .run()
        .unwrap();
}

fn main() {
    let opt = Opt::from_args();
    println!("opt: {:?}", opt);
    if let Some(db_url) = opt.db_url {
        match server::set_db_url(&db_url) {
            Ok(_) => println!("url is set"),
            Err(_) => println!("unable to set db_url"),
        }
    }
    run(opt.address, opt.port);
}
