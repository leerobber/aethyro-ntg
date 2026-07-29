//! I/O traits for genomic data sources and sinks.
//! Enables pluggable input/output without hard-coding to VCF, CSV, etc.

use crate::genomic::SnpRecord;

/// A genomic data source (VCF, CSV, database, etc.).
/// Abstracts over file format and location to support mocking, testing, and extensibility.
pub trait Source {
    type Error: std::fmt::Display;

    /// Iterate over SNP records from this source.
    fn records(&self) -> Box<dyn Iterator<Item = Result<SnpRecord, Self::Error>>>;

    /// Total number of records available (may be None for streaming sources).
    fn record_count(&self) -> Option<u32>;

    /// Human-readable description of the source (e.g., filename, database URL).
    fn description(&self) -> String;
}

/// A genomic data sink (CSV, JSON, database, etc.).
/// Abstracts over file format and location for extensible output.
pub trait Sink {
    type Error: std::fmt::Display;

    /// Write a single SNP record to this sink.
    fn write_record(&mut self, record: &SnpRecord) -> Result<(), Self::Error>;

    /// Flush any buffered data and finalize the sink.
    fn finalize(self) -> Result<(), Self::Error>;

    /// Human-readable description of the sink (e.g., filename, database URL).
    fn description(&self) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSource {
        records: Vec<SnpRecord>,
    }

    impl Source for MockSource {
        type Error = String;

        fn records(&self) -> Box<dyn Iterator<Item = Result<SnpRecord, Self::Error>>> {
            let records = self.records.clone();
            Box::new(records.into_iter().map(Ok))
        }

        fn record_count(&self) -> Option<u32> {
            Some(self.records.len() as u32)
        }

        fn description(&self) -> String {
            "MockSource".to_string()
        }
    }

    #[test]
    fn mock_source_implements_trait() {
        let records = vec![SnpRecord {
            id: "rs1".to_string(),
            position: 1000,
            ref_allele: "A".to_string(),
            alt_allele: "G".to_string(),
            qual: 100.0,
            info: String::new(),
        }];
        let source = MockSource { records };
        assert_eq!(source.record_count(), Some(1));
        assert_eq!(source.description(), "MockSource");
    }
}
