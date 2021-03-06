#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::error::Error;
use std::borrow::Cow;
use std::fs::File;

use serde::{Serialize, Deserialize};
use structopt::StructOpt;

use meilidb_data::{Database, Schema};

#[derive(Debug, StructOpt)]
pub struct Opt {
    /// The destination where the database must be created.
    #[structopt(parse(from_os_str))]
    pub database_path: PathBuf,

    /// The csv file to index.
    #[structopt(parse(from_os_str))]
    pub csv_data_path: PathBuf,

    /// The path to the schema.
    #[structopt(long = "schema", parse(from_os_str))]
    pub schema_path: PathBuf,

    /// The path to the list of stop words (one by line).
    #[structopt(long = "stop-words", parse(from_os_str))]
    pub stop_words_path: Option<PathBuf>,

    #[structopt(long = "update-group-size")]
    pub update_group_size: Option<usize>,
}

#[derive(Serialize, Deserialize)]
struct Document<'a> (
    #[serde(borrow)]
    HashMap<Cow<'a, str>, Cow<'a, str>>
);

fn index(
    schema: Schema,
    database_path: &Path,
    csv_data_path: &Path,
    update_group_size: Option<usize>,
    stop_words: &HashSet<String>,
) -> Result<Database, Box<Error>>
{
    let database = Database::start_default(database_path)?;

    let index = database.create_index("default".to_string(), schema.clone())?;

    let mut rdr = csv::Reader::from_path(csv_data_path)?;
    let mut raw_record = csv::StringRecord::new();
    let headers = rdr.headers()?.clone();

    let mut i = 0;
    let mut end_of_file = false;

    while !end_of_file {
        let mut update = index.documents_addition();

        loop {
            end_of_file = !rdr.read_record(&mut raw_record)?;
            if end_of_file { break }

            let document: Document = match raw_record.deserialize(Some(&headers)) {
                Ok(document) => document,
                Err(e) => {
                    eprintln!("{:?}", e);
                    continue;
                }
            };

            update.update_document(&document)?;

            print!("\rindexing document {}", i);
            i += 1;

            if let Some(group_size) = update_group_size {
                if i % group_size == 0 { break }
            }
        }

        println!();

        println!("committing update...");
        update.finalize()?;
    }

    Ok(database)
}

fn retrieve_stop_words(path: &Path) -> io::Result<HashSet<String>> {
    let f = File::open(path)?;
    let reader = BufReader::new(f);
    let mut words = HashSet::new();

    for line in reader.lines() {
        let line = line?;
        let word = line.trim().to_string();
        words.insert(word);
    }

    Ok(words)
}

fn main() -> Result<(), Box<Error>> {
    let _ = env_logger::init();
    let opt = Opt::from_args();

    let schema = {
        let file = File::open(&opt.schema_path)?;
        Schema::from_toml(file)?
    };

    let stop_words = match opt.stop_words_path {
        Some(ref path) => retrieve_stop_words(path)?,
        None           => HashSet::new(),
    };

    let start = Instant::now();
    let result = index(schema, &opt.database_path, &opt.csv_data_path, opt.update_group_size, &stop_words);

    if let Err(e) = result {
        return Err(e.into())
    }

    println!("database created in {:.2?} at: {:?}", start.elapsed(), opt.database_path);
    Ok(())
}
