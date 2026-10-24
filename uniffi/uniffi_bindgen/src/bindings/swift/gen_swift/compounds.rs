/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::CodeType;
use crate::{
    bail,
    interface::{DefaultValue, Literal},
    Result,
};

#[derive(Debug)]
pub struct OptionalCodeType {
    inner: Box<dyn CodeType>,
}

impl OptionalCodeType {
    pub fn new(inner: Box<dyn CodeType>) -> Self {
        Self { inner }
    }
}

impl CodeType for OptionalCodeType {
    fn type_label(&self) -> String {
        format!("{}?", self.inner.type_label())
    }

    fn canonical_name(&self) -> String {
        format!("Option{}", self.inner.canonical_name())
    }

    fn default(&self, default: &DefaultValue) -> Result<String> {
        match default {
            DefaultValue::Default | DefaultValue::Literal(Literal::None) => Ok("nil".into()),
            DefaultValue::Literal(Literal::Some { inner }) => self.inner.default(inner),
            _ => bail!("Invalid literal for Optional type: {default:?}"),
        }
    }
}

#[derive(Debug)]
pub struct SequenceCodeType {
    inner: Box<dyn CodeType>,
}

impl SequenceCodeType {
    pub fn new(inner: Box<dyn CodeType>) -> Self {
        Self { inner }
    }
}

impl CodeType for SequenceCodeType {
    fn type_label(&self) -> String {
        format!("[{}]", self.inner.type_label())
    }

    fn canonical_name(&self) -> String {
        format!("Sequence{}", self.inner.canonical_name())
    }

    fn default(&self, default: &DefaultValue) -> Result<String> {
        match default {
            DefaultValue::Default | DefaultValue::Literal(Literal::EmptySequence) => {
                Ok("[]".into())
            }
            _ => bail!("Invalid literal for sequence type: {default:?}"),
        }
    }
}

#[derive(Debug)]
pub struct MapCodeType {
    key: Box<dyn CodeType>,
    value: Box<dyn CodeType>,
}

impl MapCodeType {
    pub fn new(key: Box<dyn CodeType>, value: Box<dyn CodeType>) -> Self {
        Self { key, value }
    }
}

impl CodeType for MapCodeType {
    fn type_label(&self) -> String {
        format!("[{}: {}]", self.key.type_label(), self.value.type_label())
    }

    fn canonical_name(&self) -> String {
        format!(
            "Dictionary{}{}",
            self.key.canonical_name(),
            self.value.canonical_name()
        )
    }

    fn default(&self, default: &DefaultValue) -> Result<String> {
        match default {
            DefaultValue::Default | DefaultValue::Literal(Literal::EmptyMap) => Ok("[:]".into()),
            _ => bail!("Invalid literal for map type: {default:?}"),
        }
    }
}

#[derive(Debug)]
pub struct SetCodeType {
    inner: Box<dyn CodeType>,
}

impl SetCodeType {
    pub fn new(inner: Box<dyn CodeType>) -> Self {
        Self { inner }
    }
}

impl CodeType for SetCodeType {
    fn type_label(&self) -> String {
        format!("Set<{}>", self.inner.type_label())
    }

    fn canonical_name(&self) -> String {
        format!("Set{}", self.inner.canonical_name())
    }

    fn default(&self, default: &DefaultValue) -> Result<String> {
        match default {
            DefaultValue::Default | DefaultValue::Literal(Literal::EmptySequence) => {
                Ok("Set()".into())
            }
            _ => bail!("Invalid literal for set type: {default:?}"),
        }
    }
}
