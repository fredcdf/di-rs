use std::any::Any;
use std::collections::{ BTreeMap, HashMap };
use std::collections::btree_map::{ Entry };

use metafactory::{ ToMetaFactory, MetaFactory };
use metafactory::aggregate::{ Aggregate };

use container::Container;

use self::one_of::{ OneOf };
use self::one::{ One };
use self::candidate::{ GroupCandidate, DefinitionCandidate };
use self::error::{ CompileError };

mod candidate;

pub mod argument_builder;
pub mod one_of;
pub mod one;
pub mod error;
pub mod validator;

pub struct Registry {
    /// Contains a list of group candidates.
    maybe_groups: BTreeMap<String, GroupCandidate>,
    /// Contains a list of definition candidates.
    maybe_definitions: BTreeMap<String, DefinitionCandidate>,
    /// Contains a list of definitions that were overriden while building
    /// the registry - so we can at least show some kind of warning.
    overriden_definitions: BTreeMap<String, Vec<DefinitionCandidate>>,

    validators: Vec<Box<validator::Validator + 'static>>,
}

impl Registry {
    pub fn new() -> Registry {
        let mut registry = Registry {
            maybe_groups: BTreeMap::new(),
            maybe_definitions: BTreeMap::new(),
            overriden_definitions: BTreeMap::new(),
            validators: Vec::new(),
        };

        registry.push_validator(validator::argument_count::ArgumentCountValidator);
        registry.push_validator(validator::overrides::NoOverridesValidator);
        registry.push_validator(validator::dependencies::DependencyValidator);

        registry
    }

    pub fn push_validator<T: validator::Validator + 'static>(
        &mut self,
        validator: T
    ) {
        self.validators.push(box validator);
    }

    pub fn compile(&self) -> Result<Container, Vec<CompileError>> {
        let mut error_summary = Vec::<CompileError>::new();

        for validator in self.validators.iter() {
            validator.validate(self, &mut error_summary);
        }

        let factory_map = HashMap::<String, Box<Any>>::new();

        if error_summary.len() == 0 {
            Ok(Container::new(factory_map))
        } else {
            Err(error_summary)
        }
    }

    pub fn has_many<T: 'static>(&mut self, collection_id: &str) {
        self.define_group_if_not_exists(collection_id, Aggregate::new::<T>());
    }

    fn define_group_if_not_exists(&mut self, collection_id: &str, type_aggregate: Aggregate<'static>) {
        if !self.maybe_groups.contains_key(collection_id) {
            println!("insert group {}", collection_id);
            self.maybe_groups.insert(
                collection_id.to_string(),
                GroupCandidate::new(type_aggregate)
            );
        }
    }

    pub fn one_of<'r, T: 'static + ToMetaFactory>(&'r mut self, collection_id: &str, id: &str, value: T)
        -> OneOf<'r>
    {
        let metafactory = value.to_metafactory();

        self.define_group_if_not_exists(collection_id, metafactory.new_aggregate());

        OneOf::new(
            self,
            collection_id,
            id,
            metafactory
        )
    }

    pub fn one<'r, T: 'static + ToMetaFactory>(&'r mut self, id: &str, value: T)
        -> One<'r>
    {
        One::new(
            self,
            id,
            value.to_metafactory()
        )
    }

    pub fn insert_one<T: 'static + ToMetaFactory>(&mut self, id: &str, value: T) {
        self.finalize(
            None,
            id,
            value.to_metafactory(),
            Vec::new()
        );
    }

    pub fn insert_with_args_one<T: 'static + ToMetaFactory>(&mut self, id: &str, arg_sources: &[&str], value: T) {
        self.finalize(
            None,
            id,
            value.to_metafactory(),
            arg_sources.iter()
                .map(|s| s.to_string())
                .collect()
        );
    }

    pub fn insert_with_arg_one<T: 'static + ToMetaFactory>(&mut self, id: &str, arg_source: &str, value: T) {
        self.finalize(
            None,
            id,
            value.to_metafactory(),
            [arg_source].iter()
                .map(|s| s.to_string())
                .collect()
        );
    }

    pub fn insert_one_of<T: 'static + ToMetaFactory>(&mut self, collection_id: &str, id: &str, value: T) {
        self.finalize(
            Some(collection_id),
            id,
            value.to_metafactory(),
            Vec::new()
        );
    }

    pub fn insert_with_args_one_of<T: 'static + ToMetaFactory>(&mut self, collection_id: &str, id: &str, arg_sources: &[&str], value: T) {
        self.finalize(
            Some(collection_id),
            id,
            value.to_metafactory(),
            arg_sources.iter()
                .map(|s| s.to_string())
                .collect()
        );
    }

    pub fn insert_with_arg_one_of<T: 'static + ToMetaFactory>(&mut self, collection_id: &str, id: &str, arg_source: &str, value: T) {
        self.finalize(
            Some(collection_id),
            id,
            value.to_metafactory(),
            [arg_source].iter()
                .map(|s| s.to_string())
                .collect()
        );
    }

    fn finalize(&mut self, collection_id: Option<&str>, id: &str, value: Box<MetaFactory + 'static>, args: Vec<String>) {
        if let Some(overriden_candidate) = self.maybe_definitions.remove(id) {
            match self.overriden_definitions.entry(id.to_string()) {
                Entry::Vacant(entry) => { entry.set(vec![overriden_candidate]); },
                Entry::Occupied(mut entry) => { entry.get_mut().push(overriden_candidate); },
            };
        }

        let candidate = DefinitionCandidate::new(
            value,
            args,
            collection_id
        );

        self.maybe_definitions.insert(
            id.to_string(),
            candidate
        );
    }
}
