use log::*;
use std::collections::HashMap;
use std::error;
use std::time;

use meilidb::SortByAttr;
use meilidb::database::{Database, Schema, SchemaAttr, RankingOrdering};
use meilidb_core::Match as MeiliMatch;
use meilidb_core::criterion::*;
use meilidb_core::QueryBuilder;
use serde_derive::{Deserialize, Serialize};

pub struct Query {
    //The text to search in the index.
    pub query: String,

    // Specify the offset of the first hit to return.
    pub offset: Option<usize>, // 0
    // Set the number of hits to retrieve (used only with offset).
    pub length: Option<usize>, // 20

    // limit the max size of a returned field. The content returned will at least contain one match
    // if the document contain one.
    pub cropped_fields: Option<HashMap<String, usize>>, // field_name : size

    // Gives control over which attributes to retrieve and which not to retrieve.
    pub attributes_to_retrieve: Option<Vec<String>>, //All

    // Restricts a given query to look in only a subset of your searchable attributes.
    //This setting overrides `searchableAttributes` on indexing config for specific searches.
    pub restrict_searchable_attributes: Option<Vec<String>>, //All
}

#[derive(Serialize, Deserialize)]
pub struct Match {
    attribute: String,
    start: u16,
    length: u16,
}

#[derive(Serialize, Deserialize)]
pub struct QueryResult {
    pub hits: HashMap<String, String>,
    pub matches: Vec<Match>,
}

fn crop_text(
    text: &str,
    matches: impl IntoIterator<Item=MeiliMatch>,
    context: usize,
) -> (String, Vec<MeiliMatch>)
{
    let mut matches = matches.into_iter().peekable();

    let char_index = matches.peek().map(|m| m.char_index as usize).unwrap_or(0);
    let start = char_index.saturating_sub(context);
    let text = text.chars().skip(start).take(context * 2).collect();

    let matches = matches
        .take_while(|m| {
            (m.char_index as usize) + (m.char_length as usize) <= start + (context * 2)
        })
        .map(|match_| {
            MeiliMatch { char_index: match_.char_index - start as u16, ..match_ }
        })
        .collect();

    (text, matches)
}

fn crop_document(
    document: &mut HashMap<String, String>,
    matches: &mut Vec<MeiliMatch>,
    schema: &Schema,
    field: &str,
    length: usize,
) {
    matches.sort_unstable_by_key(|m| (m.char_index, m.char_length));

    let attribute = match schema.attribute(field) {
        Some(attr) => attr,
        None => {
            warn!("Not attribute {} found", field);
            return
        }
    };
    let selected_matches = matches.iter().filter(|m| SchemaAttr::new(m.attribute) == attribute).cloned();
    let original_text = match document.get(field) {
        Some(text) => text,
        None => {
            warn!("attribute {} not found", field);
            return
        }
    };
    let (cropped_text, cropped_matches) = crop_text(&original_text, selected_matches, length);

    document.insert(field.to_string(), cropped_text);
    matches.retain(|m| SchemaAttr::new(m.attribute) != attribute);
    matches.extend_from_slice(&cropped_matches);
}

pub fn search(
    index: String,
    query: Query,
    db: &Database,
) -> Result<(Vec<QueryResult>), Box<error::Error>> {
    info!("search - Start handler - index: {:?}", index);
    let view = db.view(&index.clone())?;

    let config = view.config();
    let schema = view.schema();
    let ranked_map = view.ranked_map();

    let offset = query.offset.unwrap_or(0);
    let length = query.length.unwrap_or(20);

    let attributes_to_retrieve = query.attributes_to_retrieve;

    let start = time::Instant::now();

    // Apply the criteria ranks find in config
    let empty = HashMap::new();
    let ranking_rules_custom = config.ranking_rules.as_ref().unwrap_or(&empty);
    let number_of_custom_ranking = ranking_rules_custom.len();
    let mut builder = CriteriaBuilder::with_capacity(7 + number_of_custom_ranking);
    let mut query_builder = if let Some(ranking_rules_order) = &config.ranking_order {
        for rule in ranking_rules_order {
            match rule.as_str() {
                "_sum_of_typos" => builder.push(SumOfTypos),
                "_number_of_words" => builder.push(NumberOfWords),
                "_word_proximity" => builder.push(WordsProximity),
                "_sum_of_words_attribute" => builder.push(SumOfWordsAttribute),
                "_sum_of_words_position" => builder.push(SumOfWordsPosition),
                "_exact" => builder.push(Exact),
                _ => {
                    let order = match ranking_rules_custom.get(rule) {
                        Some(o) => o,
                        None => {
                            warn!("search - rule {} not exist", rule);
                            continue;
                        }
                    };

                    let custom_ranking = match order {
                        RankingOrdering::Asc => SortByAttr::lower_is_better(&ranked_map, &schema, &rule).unwrap(),
                        RankingOrdering::Dsc => SortByAttr::higher_is_better(&ranked_map, &schema, &rule).unwrap(),
                    };

                    builder.push(custom_ranking);
                }
            }
        }
        builder.push(DocumentId);
        let criteria_rules = builder.build();
        debug!("search - custom ranking created");
        QueryBuilder::with_criteria(&view.index(), criteria_rules)
    } else {
        debug!("search - default ranking used");
        QueryBuilder::new(&view.index())
    };


    // Filter searchable fields
    if let Some(fields) = query.restrict_searchable_attributes {
        for attribute in fields.iter().filter_map(|f| schema.attribute(f)) {
            query_builder.add_searchable_attribute(attribute.0);
        }
    }

    // Apply Distinct if it's wished
    let docs = match &config.distinct_field {
        Some(field) => {
            query_builder.with_distinct(|id| {
                match view.raw_field_by_document_id(&field, id) {
                    Ok(raw) => raw,
                    Err(err) => {
                        warn!("Error on distinct with field {}; {}", field, err);
                        None
                    }
                }
            }, 1).query(&query.query, offset..(offset + length))
        },
        None => query_builder.query(&query.query, offset..(offset + length))
    };

    let mut results = Vec::with_capacity(length);
    for doc in docs {
        // retrieve the content of document in kv store
        let mut hits: HashMap<String, String> = match view.document_by_id(doc.id) {
            Ok(doc) => doc,
            Err(err) => {
                warn!("Impossible to retrive a document; {}", err);
                continue
            },
        };

        let mut matches = doc.matches.clone();

        // Crops fields if needed
        if let Some(fields) = query.cropped_fields.clone() {
            for (field, length) in fields {
                crop_document(&mut hits, &mut matches, schema, &field, length);
            }
        }

        // Transform to readable matches
        let mut matches: Vec<Match> = matches.iter().map(|m| Match {
            attribute: schema.attribute_name(SchemaAttr::new(m.attribute)).to_string(),
            start: m.char_index,
            length: m.char_length,
        }).collect();

        // Remove unwanted fields
        if let Some(fields) = attributes_to_retrieve.clone() {
            hits.retain(|k, _| fields.contains(&k));
            matches.retain(|m| fields.contains(&m.attribute));
        }

        let query_result = QueryResult { hits, matches };

        results.push(query_result);
    }

    info!(
        "index: {} - search: {:?} - length: {:?} - time: {:.2?}",
        index, query.query, query.length, start.elapsed()
    );

    Ok(results)
}
