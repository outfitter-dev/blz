use crate::{Error, HeadingBlock, Result, SearchHit};
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, Value, STORED, TEXT, STRING};
use tantivy::{doc, Index, IndexReader};
use tracing::{debug, info};

pub struct SearchIndex {
    index: Index,
    #[allow(dead_code)]
    schema: Schema,
    content_field: Field,
    path_field: Field,
    heading_path_field: Field,
    lines_field: Field,
    alias_field: Field,
    reader: IndexReader,
}

impl SearchIndex {
    pub fn create(index_path: &Path) -> Result<Self> {
        let mut schema_builder = Schema::builder();
        
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let path_field = schema_builder.add_text_field("path", STRING | STORED);
        let heading_path_field = schema_builder.add_text_field("heading_path", TEXT | STORED);
        let lines_field = schema_builder.add_text_field("lines", STRING | STORED);
        let alias_field = schema_builder.add_text_field("alias", STRING | STORED);
        
        let schema = schema_builder.build();
        
        std::fs::create_dir_all(index_path)
            .map_err(|e| Error::Index(format!("Failed to create index directory: {}", e)))?;
        
        let index = Index::create_in_dir(index_path, schema.clone())
            .map_err(|e| Error::Index(format!("Failed to create index: {}", e)))?;
        
        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| Error::Index(format!("Failed to create reader: {}", e)))?;
        
        Ok(Self {
            index,
            schema,
            content_field,
            path_field,
            heading_path_field,
            lines_field,
            alias_field,
            reader,
        })
    }
    
    pub fn open(index_path: &Path) -> Result<Self> {
        let index = Index::open_in_dir(index_path)
            .map_err(|e| Error::Index(format!("Failed to open index: {}", e)))?;
        
        let schema = index.schema();
        
        let content_field = schema
            .get_field("content")
            .map_err(|_| Error::Index("Missing content field".into()))?;
        let path_field = schema
            .get_field("path")
            .map_err(|_| Error::Index("Missing path field".into()))?;
        let heading_path_field = schema
            .get_field("heading_path")
            .map_err(|_| Error::Index("Missing heading_path field".into()))?;
        let lines_field = schema
            .get_field("lines")
            .map_err(|_| Error::Index("Missing lines field".into()))?;
        let alias_field = schema
            .get_field("alias")
            .map_err(|_| Error::Index("Missing alias field".into()))?;
        
        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| Error::Index(format!("Failed to create reader: {}", e)))?;
        
        Ok(Self {
            index,
            schema,
            content_field,
            path_field,
            heading_path_field,
            lines_field,
            alias_field,
            reader,
        })
    }
    
    pub fn index_blocks(
        &mut self,
        alias: &str,
        file_path: &str,
        blocks: &[HeadingBlock],
    ) -> Result<()> {
        let mut writer = self.index
            .writer(50_000_000)
            .map_err(|e| Error::Index(format!("Failed to create writer: {}", e)))?;
        
        let _deleted = writer
            .delete_term(tantivy::Term::from_field_text(self.alias_field, alias));
        
        for block in blocks {
            let heading_path_str = block.path.join(" > ");
            let lines_str = format!("{}-{}", block.start_line, block.end_line);
            
            let doc = doc!(
                self.content_field => block.content.clone(),
                self.path_field => file_path,
                self.heading_path_field => heading_path_str,
                self.lines_field => lines_str,
                self.alias_field => alias
            );
            
            writer.add_document(doc)
                .map_err(|e| Error::Index(format!("Failed to add document: {}", e)))?;
        }
        
        writer.commit()
            .map_err(|e| Error::Index(format!("Failed to commit: {}", e)))?;
        
        self.reader.reload()
            .map_err(|e| Error::Index(format!("Failed to reload reader: {}", e)))?;
        
        info!("Indexed {} blocks for {}", blocks.len(), alias);
        Ok(())
    }
    
    pub fn search(
        &self,
        query_str: &str,
        alias: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchHit>> {
        let searcher = self.reader.searcher();
        
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.content_field, self.heading_path_field],
        );
        
        let mut full_query_str = query_str.to_string();
        if let Some(alias) = alias {
            full_query_str = format!("alias:{} AND ({})", alias, query_str);
        }
        
        let query = query_parser
            .parse_query(&full_query_str)
            .map_err(|e| Error::Index(format!("Failed to parse query: {}", e)))?;
        
        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| Error::Index(format!("Search failed: {}", e)))?;
        
        let mut hits = Vec::new();
        
        for (score, doc_address) in top_docs {
            let doc = searcher
                .doc(doc_address)
                .map_err(|e| Error::Index(format!("Failed to retrieve doc: {}", e)))?;
            
            let alias = self.get_field_text(&doc, self.alias_field)?;
            let file = self.get_field_text(&doc, self.path_field)?;
            let heading_path_str = self.get_field_text(&doc, self.heading_path_field)?;
            let lines = self.get_field_text(&doc, self.lines_field)?;
            let content = self.get_field_text(&doc, self.content_field)?;
            
            let heading_path: Vec<String> = heading_path_str
                .split(" > ")
                .map(|s| s.to_string())
                .collect();
            
            let snippet = self.extract_snippet(&content, query_str, 100);
            
            hits.push(SearchHit {
                alias,
                file,
                heading_path,
                lines,
                snippet,
                score,
                source_url: None,
                checksum: String::new(),
            });
        }
        
        debug!("Found {} hits for query: {}", hits.len(), query_str);
        Ok(hits)
    }
    
    fn get_field_text(&self, doc: &tantivy::TantivyDocument, field: Field) -> Result<String> {
        doc.get_first(field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| Error::Index("Field not found in document".into()))
    }
    
    fn extract_snippet(&self, content: &str, query: &str, max_len: usize) -> String {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        
        if let Some(pos) = content_lower.find(&query_lower) {
            let start = pos.saturating_sub(50);
            let end = (pos + query.len() + 50).min(content.len());
            
            let mut snippet = String::new();
            if start > 0 {
                snippet.push_str("...");
            }
            snippet.push_str(&content[start..end]);
            if end < content.len() {
                snippet.push_str("...");
            }
            
            return snippet;
        }
        
        if content.len() <= max_len {
            content.to_string()
        } else {
            format!("{}...", &content[..max_len])
        }
    }
}