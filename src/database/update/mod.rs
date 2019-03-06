use std::collections::{HashSet, BTreeMap};
use std::error::Error;

use rocksdb::rocksdb::{Writable, WriteBatch};
use hashbrown::HashMap;
use sdset::{Set, SetBuf};
use serde::Serialize;

use crate::database::document_key::{DocumentKey, DocumentKeyAttr};
use crate::database::serde::serializer::Serializer;
use crate::database::serde::SerializerError;
use crate::database::schema::SchemaAttr;
use crate::database::schema::Schema;
use crate::database::index::IndexBuilder;
use crate::database::{DATA_INDEX, DATA_RANKED_MAP};
use crate::database::{RankedMap, Number};
use crate::tokenizer::TokenizerBuilder;
use crate::write_to_bytes::WriteToBytes;
use crate::data::DocIds;
use crate::{DocumentId, DocIndex};

pub use self::index_event::{ReadIndexEvent, WriteIndexEvent};
pub use self::ranked_map_event::{ReadRankedMapEvent, WriteRankedMapEvent};

mod index_event;
mod ranked_map_event;

pub type Token = Vec<u8>; // TODO could be replaced by a SmallVec

pub struct Update {
    schema: Schema,
    raw_builder: RawUpdateBuilder,
}

impl Update {
    pub(crate) fn new(schema: Schema) -> Update {
        Update { schema, raw_builder: RawUpdateBuilder::new() }
    }

    pub fn update_document<T, B>(
        &mut self,
        document: T,
        tokenizer_builder: &B,
        stop_words: &HashSet<String>,
    ) -> Result<DocumentId, SerializerError>
    where T: Serialize,
          B: TokenizerBuilder,
    {
        let document_id = self.schema.document_id(&document)?;

        let serializer = Serializer {
            schema: &self.schema,
            document_id: document_id,
            tokenizer_builder: tokenizer_builder,
            update: &mut self.raw_builder.document_update(document_id)?,
            stop_words: stop_words,
        };

        document.serialize(serializer)?;

        Ok(document_id)
    }

    pub fn remove_document<T>(&mut self, document: T) -> Result<DocumentId, SerializerError>
    where T: Serialize,
    {
        let document_id = self.schema.document_id(&document)?;
        self.raw_builder.document_update(document_id)?.remove()?;
        Ok(document_id)
    }

    pub(crate) fn build(self) -> Result<WriteBatch, Box<Error>> {
        self.raw_builder.build()
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum UpdateType {
    Updated,
    Deleted,
}

use UpdateType::{Updated, Deleted};

pub struct RawUpdateBuilder {
    documents_update: hashbrown::HashSet<DocumentId>,
    documents_ranked_fields: RankedMap,
    indexed_words: BTreeMap<Token, Vec<DocIndex>>,
    batch: WriteBatch,
}

impl RawUpdateBuilder {
    pub fn new() -> RawUpdateBuilder {
        RawUpdateBuilder {
            documents_update: hashbrown::HashSet::new(),
            documents_ranked_fields: HashMap::new(),
            indexed_words: BTreeMap::new(),
            batch: WriteBatch::new(),
        }
    }

    pub fn document_update(&mut self, document_id: DocumentId) -> Result<DocumentUpdate, SerializerError> {
        self.documents_update.insert(document_id);
        DocumentUpdate::new(document_id, self)
    }

    pub fn build(self) -> Result<WriteBatch, Box<Error>> {
        // create the list of all the removed documents
        let removed_documents = {
            // we remove the updated documents to have a fresh update
            // without the old document fields values
            let mut document_ids: Vec<_> = self.documents_update.into_iter().collect();

            document_ids.sort_unstable();
            let setbuf = SetBuf::new_unchecked(document_ids);
            DocIds::new(&setbuf)
        };

        // create the Index of all the document updates
        let index = {
            let mut builder = IndexBuilder::new();
            for (key, mut indexes) in self.indexed_words {
                indexes.sort_unstable();
                let indexes = Set::new_unchecked(&indexes);
                builder.insert(key, indexes).unwrap();
            }
            builder.build()
        };

        // WARN: removed documents must absolutely
        //       be merged *before* document updates

        // === index ===

        // remove the documents using the appropriate IndexEvent
        let event_bytes = WriteIndexEvent::RemovedDocuments(&removed_documents).into_bytes();
        self.batch.merge(DATA_INDEX, &event_bytes)?;

        // update the documents using the appropriate IndexEvent
        let event_bytes = WriteIndexEvent::UpdatedDocuments(&index).into_bytes();
        self.batch.merge(DATA_INDEX, &event_bytes)?;

        // === ranked map ===

        // update the ranked map using the appropriate RankedMapEvent
        let event_bytes = WriteRankedMapEvent::RemovedDocuments(&removed_documents).into_bytes();
        self.batch.merge(DATA_RANKED_MAP, &event_bytes)?;

        // update the documents using the appropriate IndexEvent
        let event_bytes = WriteRankedMapEvent::UpdatedDocuments(&self.documents_ranked_fields).into_bytes();
        self.batch.merge(DATA_RANKED_MAP, &event_bytes)?;

        Ok(self.batch)
    }
}

pub struct DocumentUpdate<'a> {
    document_id: DocumentId,
    inner: &'a mut RawUpdateBuilder,
}

impl<'a> DocumentUpdate<'a> {
    pub fn new(document_id: DocumentId, inner: &'a mut RawUpdateBuilder) -> Result<DocumentUpdate, SerializerError> {
        let start = DocumentKey::new(document_id).with_attribute_min();
        let end = DocumentKey::new(document_id).with_attribute_max(); // FIXME max + 1
        inner.batch.delete_range(start.as_ref(), end.as_ref())?;

        Ok(DocumentUpdate { document_id, inner })
    }

    pub fn remove(&mut self) -> Result<(), SerializerError> {
        // a remove of the whole document is always done!
        Ok(())
    }

    pub fn insert_attribute_value(&mut self, attr: SchemaAttr, value: &[u8]) -> Result<(), SerializerError> {
        let key = DocumentKeyAttr::new(self.document_id, attr);
        self.inner.batch.put(key.as_ref(), &value)?;

        Ok(())
    }

    pub fn insert_doc_index(&mut self, token: Token, doc_index: DocIndex) -> Result<(), SerializerError> {
        self.inner.indexed_words.entry(token).or_insert_with(Vec::new).push(doc_index);

        Ok(())
    }

    pub fn register_ranked_attribute(
        &mut self,
        attr: SchemaAttr,
        number: Number,
    ) -> Result<(), SerializerError>
    {
        self.inner.documents_ranked_fields.insert((self.document_id, attr), number);

        Ok(())
    }
}
