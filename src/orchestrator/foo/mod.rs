#![allow(unused_variables)]
use std::path::PathBuf;

use reqwest::Client;

use crate::{
    dispatch::ServiceCache,
    errors::Result,
    models::{Orchestrator, UserProvidedConfig},
};

#[allow(dead_code)]
struct Foo;

impl Orchestrator for Foo {
    fn setup(
        &mut self,
        cache: ServiceCache,
        pwd: PathBuf,
        name: String,
        userconfig: Option<&UserProvidedConfig>,
    ) -> Result<PathBuf> {
        todo!()
    }

    fn remove(&mut self, cache: ServiceCache, name: String) -> Result<()> {
        todo!()
    }

    fn update(&mut self, cache: ServiceCache, name: String) -> Result<()> {
        todo!()
    }

    fn up(
        &mut self,
        client: Client,
        cache: ServiceCache,
        name: String,
        skip_prompt: Option<bool>,
    ) -> Result<()> {
        todo!()
    }

    fn down(
        &mut self,
        client: Client,
        cache: ServiceCache,
        name: String,
        skip_prompt: Option<bool>,
        force: Option<bool>,
    ) -> Result<()> {
        todo!()
    }

    fn status(
        &mut self,
        client: Client,
        cache: ServiceCache,
        name: String,
        pretty: Option<bool>,
    ) -> Result<String> {
        todo!()
    }

    fn replica_check_string(&self) -> &'static str {
        todo!()
    }
}
