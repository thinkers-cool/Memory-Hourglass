use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortField {
    Date,
    Name,
    Rating,
    Path,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDir {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortSpec {
    pub field: SortField,
    pub dir: SortDir,
}

impl SortSpec {
    pub fn parse(value: &str) -> Result<Self> {
        let (field, direction) = value
            .split_once(':')
            .ok_or_else(|| AppError::InvalidInput(format!("invalid sort: {}", value)))?;
        if field.is_empty() || direction.is_empty() {
            return Err(AppError::InvalidInput(format!("invalid sort: {}", value)));
        }
        let field = match field {
            "date" => SortField::Date,
            "name" => SortField::Name,
            "rating" => SortField::Rating,
            "path" => SortField::Path,
            other => {
                return Err(AppError::InvalidInput(format!(
                    "invalid sort field: {}",
                    other
                )))
            }
        };
        let dir = match direction.to_ascii_lowercase().as_str() {
            "asc" => SortDir::Asc,
            "desc" => SortDir::Desc,
            other => {
                return Err(AppError::InvalidInput(format!(
                    "invalid sort dir: {}",
                    other
                )))
            }
        };
        Ok(Self { field, dir })
    }

    pub fn encode(self) -> String {
        let field = match self.field {
            SortField::Date => "date",
            SortField::Name => "name",
            SortField::Rating => "rating",
            SortField::Path => "path",
        };
        let dir = match self.dir {
            SortDir::Asc => "asc",
            SortDir::Desc => "desc",
        };
        format!("{}:{}", field, dir)
    }

    pub fn album_order_sql(self) -> String {
        let dir = match self.dir {
            SortDir::Asc => "ASC",
            SortDir::Desc => "DESC",
        };
        match self.field {
            SortField::Name => format!("a.file_name {dir}, ai.position ASC"),
            SortField::Rating => format!("m.rating {dir}, ai.position ASC"),
            SortField::Path => format!("a.rel_path {dir}, ai.position ASC"),
            SortField::Date => format!("{} {dir}, ai.position ASC", crate::dates::CAPTURE_AT_SQL),
        }
    }

    pub fn query_order_sql(self) -> (SortField, bool) {
        (self.field, self.dir == SortDir::Desc)
    }
}

pub fn default_album_sort() -> SortSpec {
    SortSpec {
        field: SortField::Date,
        dir: SortDir::Desc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_query_sort_param() {
        let spec = SortSpec::parse("rating:asc").unwrap();
        assert_eq!(spec.field, SortField::Rating);
        assert_eq!(spec.dir, SortDir::Asc);
    }

    #[test]
    fn parses_album_sort_mode() {
        let spec = SortSpec::parse("date:desc").unwrap();
        assert_eq!(spec.field, SortField::Date);
        assert_eq!(spec.dir, SortDir::Desc);
    }

    #[test]
    fn rejects_invalid_sort_values() {
        assert!(SortSpec::parse("invalid").is_err());
        assert!(SortSpec::parse("date:").is_err());
        assert!(SortSpec::parse(":desc").is_err());
        assert!(SortSpec::parse("size:desc").is_err());
        assert!(SortSpec::parse("date:sideways").is_err());
    }

    #[test]
    fn encodes_and_builds_sql() {
        let spec = SortSpec {
            field: SortField::Name,
            dir: SortDir::Asc,
        };
        assert_eq!(spec.encode(), "name:asc");
        assert!(spec.album_order_sql().contains("file_name ASC"));
        let (field, desc) = spec.query_order_sql();
        assert_eq!(field, SortField::Name);
        assert!(!desc);
    }

    #[test]
    fn default_album_sort_is_date_desc() {
        let spec = default_album_sort();
        assert_eq!(spec.encode(), "date:desc");
    }

    #[test]
    fn album_order_sql_covers_all_fields() {
        assert!(SortSpec {
            field: SortField::Rating,
            dir: SortDir::Desc,
        }
        .album_order_sql()
        .contains("rating DESC"));
        assert!(SortSpec {
            field: SortField::Path,
            dir: SortDir::Asc,
        }
        .album_order_sql()
        .contains("rel_path ASC"));
        assert!(SortSpec {
            field: SortField::Date,
            dir: SortDir::Asc,
        }
        .album_order_sql()
        .contains("ASC"));
        assert_eq!(
            SortSpec {
                field: SortField::Rating,
                dir: SortDir::Desc,
            }
            .encode(),
            "rating:desc"
        );
        assert_eq!(
            SortSpec {
                field: SortField::Path,
                dir: SortDir::Desc,
            }
            .encode(),
            "path:desc"
        );
    }
}
