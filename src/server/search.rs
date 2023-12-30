use std::{collections::HashSet, path::PathBuf};

use chrono::{DateTime, Utc};
use log::debug;
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    doc,
    merge_policy::LogMergePolicy,
    query::{AllQuery, BooleanQuery, FuzzyTermQuery, Query, QueryParser, RangeQuery},
    query_grammar::Occur,
    schema::{Field, Schema, FAST, STORED, TEXT},
    DocAddress, Index, IndexReader, Order, ReloadPolicy, Term,
};

use super::ClipboardData;

pub struct QueryParam {
    pub query: Option<String>,
    pub sources: HashSet<String>,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub skip: usize,
    pub size: usize,
    pub sort_by_score: bool,
}

#[derive(Clone)]
pub struct Search {
    index: Index,
    reader: IndexReader,
    source: Field,
    content: Field,
    timestamp: Field,
    query_parser: QueryParser,
}

impl Search {
    pub fn new(index_path: Option<PathBuf>) -> Self {
        let mut schema_builder = Schema::builder();
        let source = schema_builder.add_text_field("source", TEXT | STORED);
        let content = schema_builder.add_text_field("content", TEXT | STORED);
        let timestamp = schema_builder.add_i64_field("timestamp", FAST | STORED);
        let schema = schema_builder.build();
        let index = match index_path {
            Some(path) => {
                std::fs::create_dir_all(&path).unwrap();
                Index::open_or_create(MmapDirectory::open(path).unwrap(), schema.clone()).unwrap()
            }
            None => Index::create_in_ram(schema.clone()),
        };
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()
            .unwrap();
        let mut query_parser = QueryParser::for_index(&index, vec![content]);
        query_parser.set_conjunction_by_default();
        query_parser.set_field_fuzzy(content, true, 1, true);
        Self {
            index,
            reader,
            source,
            content,
            timestamp,
            query_parser,
        }
    }

    pub fn add_entry(&self, entry: &ClipboardData) -> anyhow::Result<()> {
        debug!("Adding entry: {:?}", entry);
        let mut index_writer = self.index.writer(50_000_000)?;
        index_writer.set_merge_policy(Box::<LogMergePolicy>::default());
        index_writer.add_document(doc!(
            self.source => entry.source.clone(),
            self.content => entry.data.clone(),
            self.timestamp => entry.timestamp
        ))?;
        index_writer.commit()?;
        Ok(())
    }

    pub fn get_device_list(&self) -> anyhow::Result<HashSet<String>> {
        let searcher = self.reader.searcher();
        let mut device_list = HashSet::new();
        let collector = TopDocs::with_limit(1000);
        let result: Vec<(f64, DocAddress)> = searcher
            .search(&AllQuery, &collector)?
            .into_iter()
            .map(|(ts, d)| (ts as f64, d))
            .collect();
        for (_, doc_address) in result {
            let doc = searcher.doc(doc_address)?;
            let source = doc
                .get_first(self.source)
                .and_then(|v| v.as_text())
                .map(|v| v.to_string())
                .unwrap_or_default();
            device_list.insert(source);
        }
        Ok(device_list)
    }

    pub fn query(&self, param: QueryParam) -> anyhow::Result<Vec<ClipboardData>> {
        let searcher = self.reader.searcher();

        let content_q: Box<dyn Query> = match param.query {
            Some(query) => {
                let query = query.trim();
                if query.is_empty() {
                    Box::new(AllQuery)
                } else {
                    let (q, errors) = self.query_parser.parse_query_lenient(query);
                    if !errors.is_empty() {
                        debug!("Query parse error: {:?}", errors);
                    }
                    q
                }
            }
            None => {
                debug!("Empty keyword query");
                Box::new(AllQuery)
            }
        };
        let source_q: Box<dyn Query> = match param.sources.is_empty() {
            true => {
                debug!("Empty source query");
                Box::new(AllQuery)
            }
            false => {
                debug!("Source query: {:?}", param.sources);
                let source_q = BooleanQuery::new(
                    param
                        .sources
                        .into_iter()
                        .map(|s| {
                            (Occur::Should, {
                                let term = Term::from_field_text(self.source, &s);
                                Box::new(FuzzyTermQuery::new(term, 2, true)) as Box<dyn Query>
                            })
                        })
                        .collect::<Vec<_>>(),
                );

                Box::new(source_q)
            }
        };
        let time_q: Box<dyn Query> = match param.time_range {
            Some((begin, end)) => {
                let begin = begin.timestamp();
                let end = end.timestamp();
                let range = RangeQuery::new_i64("timestamp".to_string(), begin..end);
                Box::new(range)
            }
            None => {
                debug!("Empty time range query");
                Box::new(AllQuery)
            }
        };

        let q = BooleanQuery::new(vec![
            (Occur::Must, content_q),
            (Occur::Must, source_q),
            (Occur::Must, time_q),
        ]);
        let collector = TopDocs::with_limit(param.size).and_offset(param.skip);
        let result: Vec<(f64, DocAddress)> = if param.sort_by_score {
            searcher
                .search(&q, &collector)?
                .into_iter()
                .map(|(ts, d)| (ts as f64, d))
                .collect()
        } else {
            searcher
                .search(&q, &collector.order_by_fast_field("timestamp", Order::Desc))?
                .into_iter()
                .map(|(ts, d): (i64, DocAddress)| (ts as f64, d))
                .collect()
        };
        let ret = result
            .into_iter()
            .map(|(_, doc_address)| {
                let doc = searcher.doc(doc_address);
                doc.map(|d| {
                    debug!("Found doc: {:?}", d);
                    let data = d
                        .get_first(self.content)
                        .and_then(|v| v.as_text())
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    let source = d
                        .get_first(self.source)
                        .and_then(|v| v.as_text())
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    let timestamp = d
                        .get_first(self.timestamp)
                        .and_then(|v| v.as_i64())
                        .unwrap_or_default();
                    ClipboardData {
                        source: source.to_string(),
                        data: data.to_string(),
                        timestamp,
                    }
                })
            })
            .filter_map(|d| d.ok())
            .collect::<Vec<_>>();
        Ok(ret)
    }
}