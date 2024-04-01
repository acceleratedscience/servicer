#![allow(dead_code)] // Remove this later

use std::fmt::Debug;

use reqwest::Client;

pub struct Dispatcher<T>
where
    T: Debug,
{
    data: T,
    client: Client,
}

impl<T> Dispatcher<T>
where
    T: Debug,
{
    pub fn new(data: T) -> Self {
        Self {
            data,
            client: Client::new(),
        }
    }
}
