use std::io::{Read, Write};

use hashbrown::HashMap;
use meilidb_core::DocumentId;

use crate::{SchemaAttr, Number};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RankedMap(HashMap<(DocumentId, SchemaAttr), Number>);

impl RankedMap {
    pub fn insert(&mut self, document: DocumentId, attribute: SchemaAttr, number: Number) {
        self.0.insert((document, attribute), number);
    }

    pub fn get(&self, document: DocumentId, attribute: SchemaAttr) -> Option<Number> {
        self.0.get(&(document, attribute)).cloned()
    }

    pub fn read_from_bin<R: Read>(reader: R) -> bincode::Result<RankedMap> {
        bincode::deserialize_from(reader).map(RankedMap)
    }

    pub fn write_to_bin<W: Write>(&self, writer: W) -> bincode::Result<()> {
        bincode::serialize_into(writer, &self.0)
    }
}
