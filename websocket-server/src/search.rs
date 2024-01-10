use std::{collections::HashSet, path::PathBuf};

use client_interface::{ServerClipboardContent, ServerClipboardRecord};
use log::debug;
use tantivy::{
    collector::{Count, FruitHandle, MultiCollector, TopDocs},
    directory::MmapDirectory,
    doc,
    merge_policy::LogMergePolicy,
    query::{AllQuery, BooleanQuery, Query, QueryParser, RangeQuery, TermQuery, TermSetQuery},
    query_grammar::Occur,
    schema::{Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, FAST, STORED},
    tokenizer::{LowerCaser, NgramTokenizer, TextAnalyzer},
    DocAddress, Index, IndexReader, Order, ReloadPolicy, Term,
};

use super::{ClipboardMessage, QueryParam, QueryResult};

const TOKENIZER_NAME: &str = "ngram_m_n";

#[derive(Clone)]
pub struct Search {
    index: Index,
    reader: IndexReader,
    id: Field,
    source: Field,
    content: Field,
    url: Field,
    timestamp: Field,
    query_parser: QueryParser,
}

impl Search {
    pub fn new(index_path: Option<PathBuf>) -> Self {
        let mut schema_builder = Schema::builder();

        let token_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("raw")
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();
        let tokenizer = TextAnalyzer::builder(NgramTokenizer::new(2, 4, false).unwrap())
            .filter(LowerCaser)
            .build();

        let text_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer(TOKENIZER_NAME)
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();
        let id = schema_builder.add_text_field("id", token_options.clone());
        let source = schema_builder.add_text_field("source", token_options.clone());
        let content = schema_builder.add_text_field("content", text_options);
        let url = schema_builder.add_text_field("url", token_options);
        let timestamp = schema_builder.add_i64_field("timestamp", FAST | STORED);
        let schema = schema_builder.build();
        let index = match index_path {
            Some(path) => {
                std::fs::create_dir_all(&path).unwrap();
                Index::open_or_create(MmapDirectory::open(path).unwrap(), schema.clone()).unwrap()
            }
            None => Index::create_in_ram(schema.clone()),
        };
        index.tokenizers().register(TOKENIZER_NAME, tokenizer);
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
            id,
            source,
            content,
            url,
            timestamp,
            query_parser,
        }
    }

    pub fn get_entry_by_id(&self, id: &str) -> anyhow::Result<Option<ClipboardMessage>> {
        let q = TermQuery::new(Term::from_field_text(self.id, id), IndexRecordOption::Basic);
        let collector = TopDocs::with_limit(1);
        let searcher = self.reader.searcher();
        let result: Vec<(f64, DocAddress)> = searcher
            .search(&q, &collector)?
            .into_iter()
            .map(|(ts, d)| (ts as f64, d))
            .collect();
        if result.is_empty() {
            return Ok(None);
        }
        let (_, doc_address) = result[0];
        let doc = searcher.doc(doc_address)?;
        let data = doc
            .get_first(self.content)
            .and_then(|v| v.as_text())
            .map(|v| v.to_string())
            .unwrap_or_default();
        let id = doc
            .get_first(self.id)
            .and_then(|v| v.as_text())
            .map(|v| v.to_string())
            .unwrap_or_default();
        let source = doc
            .get_first(self.source)
            .and_then(|v| v.as_text())
            .map(|v| v.to_string())
            .unwrap_or_default();
        let url = doc
            .get_first(self.url)
            .and_then(|v| v.as_text())
            .map(|v| v.to_string())
            .unwrap_or_default();
        let timestamp = doc
            .get_first(self.timestamp)
            .and_then(|v| v.as_i64())
            .unwrap_or_default();
        if url.is_empty() {
            Ok(Some(ClipboardMessage {
                entry: ServerClipboardRecord {
                    id: Some(id),
                    source: source.to_string(),
                    content: ServerClipboardContent::Text(data.to_string()),
                },
                timestamp,
            }))
        } else {
            Ok(Some(ClipboardMessage {
                entry: ServerClipboardRecord {
                    id: Some(id),
                    source: source.to_string(),
                    content: ServerClipboardContent::ImageUrl(url.to_string()),
                },
                timestamp,
            }))
        }
    }

    pub fn add_entry(&self, entry: &ClipboardMessage) -> anyhow::Result<()> {
        debug!("Adding entry: from {}", entry.entry.source);
        assert!(entry.entry.id.is_some());
        let id = entry.entry.id.as_ref().unwrap().clone();
        let q = TermQuery::new(
            Term::from_field_text(self.id, &id),
            IndexRecordOption::Basic,
        );
        let collector = TopDocs::with_limit(1);
        let searcher = self.reader.searcher();
        let result: Vec<(f64, DocAddress)> = searcher
            .search(&q, &collector)?
            .into_iter()
            .map(|(ts, d)| (ts as f64, d))
            .collect();
        if !result.is_empty() {
            debug!("Entry already exists, skipping");
            return Ok(());
        }
        let mut index_writer = self.index.writer(50_000_000)?;
        index_writer.set_merge_policy(Box::<LogMergePolicy>::default());
        index_writer.add_document(match &entry.entry.content {
            ServerClipboardContent::Text(text) => {
                doc!(
                    self.id => id,
                    self.source => entry.entry.source.clone(),
                    self.content => text.clone(),
                    self.timestamp => entry.timestamp
                )
            }
            ServerClipboardContent::ImageUrl(url) => {
                doc!(
                    self.id => id,
                    self.source => entry.entry.source.clone(),
                    self.url => url.clone(),
                    self.timestamp => entry.timestamp
                )
            }
        })?;
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

    pub fn query(&self, param: QueryParam) -> anyhow::Result<QueryResult> {
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
                let source_q = TermSetQuery::new(
                    param
                        .sources
                        .into_iter()
                        .map(|s| Term::from_field_text(self.source, &s))
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
        let mut collectors = MultiCollector::new();
        let count_handle = collectors.add_collector(Count);
        let (count, ret) = if param.sort_by_score {
            let top_docs_handle =
                collectors.add_collector(TopDocs::with_limit(param.size).and_offset(param.skip));
            let mut multi_fruit = searcher.search(&q, &collectors)?;
            let count = count_handle.extract(&mut multi_fruit);
            let ret = top_docs_handle
                .extract(&mut multi_fruit)
                .into_iter()
                .map(|(v, d)| (v as i64, d))
                .collect::<Vec<_>>();
            (count, ret)
        } else {
            let top_docs_handle: FruitHandle<Vec<(i64, DocAddress)>> = collectors.add_collector(
                TopDocs::with_limit(param.size)
                    .and_offset(param.skip)
                    .order_by_fast_field("timestamp", Order::Desc),
            );
            let mut multi_fruit = searcher.search(&q, &collectors)?;
            let count = count_handle.extract(&mut multi_fruit);
            let ret = top_docs_handle.extract(&mut multi_fruit);
            (count, ret)
        };
        let ret = ret
            .into_iter()
            .map(|(_, doc_address)| {
                let doc = searcher.doc(doc_address);
                doc.map(|d| {
                    debug!("Found doc at {:?}", doc_address);
                    let data = d
                        .get_first(self.content)
                        .and_then(|v| v.as_text())
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    let id = d
                        .get_first(self.id)
                        .and_then(|v| v.as_text())
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    let source = d
                        .get_first(self.source)
                        .and_then(|v| v.as_text())
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    let url = d
                        .get_first(self.url)
                        .and_then(|v| v.as_text())
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    let timestamp = d
                        .get_first(self.timestamp)
                        .and_then(|v| v.as_i64())
                        .unwrap_or_default();
                    if url.is_empty() {
                        ClipboardMessage {
                            entry: ServerClipboardRecord {
                                id: Some(id),
                                source: source.to_string(),
                                content: ServerClipboardContent::Text(data.to_string()),
                            },
                            timestamp,
                        }
                    } else {
                        ClipboardMessage {
                            entry: ServerClipboardRecord {
                                id: Some(id),
                                source: source.to_string(),
                                content: ServerClipboardContent::ImageUrl(url.to_string()),
                            },
                            timestamp,
                        }
                    }
                })
            })
            .filter_map(|d| d.ok())
            .collect::<Vec<_>>();
        Ok(QueryResult {
            total: count,
            skip: param.skip,
            data: ret,
        })
    }
}
